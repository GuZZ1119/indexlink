//! SQLite adapter for the local opportunity-bucket cash ledger.

use rust_decimal::Decimal;
use sqlx::SqlitePool;
use uuid::Uuid;

use investment_plans::OpportunityCashPolicy;

use crate::sqlite::{decode_amount, encode_amount};

/// One idempotently persisted opportunity-cash settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpportunityCashSettlement {
    /// Cash available before this accepted decision.
    pub balance_before: Decimal,
    /// Balance retained for a later accepted decision.
    pub balance_after: Decimal,
    /// Whether this call created the unique decision settlement.
    pub applied: bool,
}

/// SQLite repository which keeps opportunity cash separate from account cash flows.
#[derive(Clone, Debug)]
pub struct SqliteOpportunityCashRepository {
    pool: SqlitePool,
}

impl SqliteOpportunityCashRepository {
    /// Build an opportunity-cash repository from a migrated SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Return the currently carried opportunity cash for a plan.
    pub async fn balance(&self, plan_id: Uuid) -> Result<Decimal, sqlx::Error> {
        let value = sqlx::query_scalar::<_, String>(
            "SELECT balance FROM opportunity_cash_balances WHERE plan_id = ?1",
        )
        .bind(plan_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(value
            .as_deref()
            .and_then(decode_amount)
            .unwrap_or(Decimal::ZERO))
    }

    /// Atomically settle one accepted decision once.
    ///
    /// `allocated_amount` is the estimate used to form the accepted order. Actual fill
    /// reconciliation remains the responsibility of the existing paper-performance ledger.
    pub async fn settle(
        &self,
        plan_id: Uuid,
        decision_record_id: Uuid,
        scheduled_for: &str,
        policy: OpportunityCashPolicy,
        period_budget: Decimal,
        allocated_amount: Decimal,
    ) -> Result<OpportunityCashSettlement, sqlx::Error> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM opportunity_cash_events WHERE decision_record_id = ?1",
        )
        .bind(decision_record_id.to_string())
        .fetch_one(&mut *transaction)
        .await?;
        let before = query_balance(&mut transaction, plan_id).await?;
        if existing != 0 {
            transaction.commit().await?;
            return Ok(OpportunityCashSettlement {
                balance_before: before,
                balance_after: before,
                applied: false,
            });
        }
        if period_budget < Decimal::ZERO || allocated_amount < Decimal::ZERO {
            return Err(sqlx::Error::Protocol(
                "negative opportunity settlement".to_owned(),
            ));
        }
        let after = match policy {
            OpportunityCashPolicy::ExpireEachPeriod => Decimal::ZERO,
            OpportunityCashPolicy::CarryForward => {
                (before + period_budget - allocated_amount).max(Decimal::ZERO)
            }
        };
        let before_text = encode_non_negative(before)?;
        let budget_text = encode_non_negative(period_budget)?;
        let allocated_text = encode_non_negative(allocated_amount)?;
        let after_text = encode_non_negative(after)?;
        sqlx::query(
            "INSERT INTO opportunity_cash_events \
             (id, plan_id, decision_record_id, scheduled_for, policy, balance_before, period_budget, allocated_amount, balance_after) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(plan_id.to_string())
        .bind(decision_record_id.to_string())
        .bind(scheduled_for)
        .bind(policy_name(policy))
        .bind(before_text)
        .bind(budget_text)
        .bind(allocated_text)
        .bind(after_text.clone())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO opportunity_cash_balances (plan_id, balance, updated_at) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
             ON CONFLICT(plan_id) DO UPDATE SET balance = excluded.balance, updated_at = excluded.updated_at",
        )
        .bind(plan_id.to_string())
        .bind(after_text)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(OpportunityCashSettlement {
            balance_before: before,
            balance_after: after,
            applied: true,
        })
    }
}

async fn query_balance(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    plan_id: Uuid,
) -> Result<Decimal, sqlx::Error> {
    let value = sqlx::query_scalar::<_, String>(
        "SELECT balance FROM opportunity_cash_balances WHERE plan_id = ?1",
    )
    .bind(plan_id.to_string())
    .fetch_optional(&mut **transaction)
    .await?;
    match value {
        Some(value) => decode_amount(&value)
            .ok_or_else(|| sqlx::Error::Protocol("invalid opportunity balance".to_owned())),
        None => Ok(Decimal::ZERO),
    }
}

fn encode_non_negative(value: Decimal) -> Result<String, sqlx::Error> {
    if value.is_zero() {
        return Ok("000000000000.00000000".to_owned());
    }
    encode_amount(value)
        .ok_or_else(|| sqlx::Error::Protocol("invalid opportunity amount".to_owned()))
}

fn policy_name(policy: OpportunityCashPolicy) -> &'static str {
    match policy {
        OpportunityCashPolicy::ExpireEachPeriod => "expire_each_period",
        OpportunityCashPolicy::CarryForward => "carry_forward",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteStorage;
    use std::time::Duration;

    /// Verify carry-forward settlement is atomic and decision-record idempotent.
    #[tokio::test]
    async fn carries_only_unallocated_cash_once() {
        let storage =
            SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
                .await
                .unwrap();
        storage.migrate().await.unwrap();
        let plan_id = Uuid::new_v4();
        let record_id = Uuid::new_v4();
        sqlx::query("INSERT INTO investment_plans (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution) VALUES (?1, 'plan', 'VOO', '000000000100.00000000', 'USD', 1, '000000000100.00000000')")
            .bind(plan_id.to_string()).execute(storage.pool()).await.unwrap();
        sqlx::query("INSERT INTO decision_records (id, plan_id, symbol, currency, execution_status, execution_snapshot, fundamental_snapshot, trend_snapshot, decision_snapshot, summary) VALUES (?1, ?2, 'VOO', 'USD', 'due', '{}', '{}', '{}', '{}', 'test')")
            .bind(record_id.to_string()).bind(plan_id.to_string()).execute(storage.pool()).await.unwrap();
        let repo = SqliteOpportunityCashRepository::new(storage.pool().clone());
        let first = repo
            .settle(
                plan_id,
                record_id,
                "2026-08-20",
                OpportunityCashPolicy::CarryForward,
                Decimal::new(30, 0),
                Decimal::new(10, 0),
            )
            .await
            .unwrap();
        assert!(first.applied);
        assert_eq!(first.balance_after, Decimal::new(20, 0));
        let again = repo
            .settle(
                plan_id,
                record_id,
                "2026-08-20",
                OpportunityCashPolicy::CarryForward,
                Decimal::new(30, 0),
                Decimal::new(10, 0),
            )
            .await
            .unwrap();
        assert!(!again.applied);
        assert_eq!(repo.balance(plan_id).await.unwrap(), Decimal::new(20, 0));
    }
}

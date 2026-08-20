//! SQLite adapter for the local opportunity-bucket cash ledger.

use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};
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

/// Inputs required to persist one accepted opportunity-cash settlement.
#[derive(Debug, Clone, Copy)]
pub struct OpportunityCashSettlementInput<'a> {
    /// Plan that owns the opportunity-cash balance.
    pub plan_id: Uuid,
    /// Immutable decision record that makes the settlement idempotent.
    pub decision_record_id: Uuid,
    /// UTC planned date represented by the decision.
    pub scheduled_for: &'a str,
    /// Selected opportunity-cash policy.
    pub policy: OpportunityCashPolicy,
    /// Optional maximum retained opportunity-cash balance.
    pub cash_cap: Option<Decimal>,
    /// Current-period opportunity-bucket budget.
    pub period_budget: Decimal,
    /// Core-bucket amount that has first claim on a later actual fill.
    pub core_contribution: Decimal,
    /// Accepted-order estimate for the opportunity-bucket amount.
    pub allocated_amount: Decimal,
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
        input: OpportunityCashSettlementInput<'_>,
    ) -> Result<OpportunityCashSettlement, sqlx::Error> {
        let OpportunityCashSettlementInput {
            plan_id,
            decision_record_id,
            scheduled_for,
            policy,
            cash_cap,
            period_budget,
            core_contribution,
            allocated_amount,
        } = input;
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
        if period_budget < Decimal::ZERO
            || core_contribution < Decimal::ZERO
            || allocated_amount < Decimal::ZERO
        {
            return Err(sqlx::Error::Protocol(
                "negative opportunity settlement".to_owned(),
            ));
        }
        let after = match policy {
            OpportunityCashPolicy::ExpireEachPeriod => Decimal::ZERO,
            OpportunityCashPolicy::CarryForward | OpportunityCashPolicy::CarryWithCap => {
                (before + period_budget - allocated_amount).max(Decimal::ZERO)
            }
        };
        let after = cash_cap.map_or(after, |cap| after.min(cap));
        let before_text = encode_non_negative(before)?;
        let budget_text = encode_non_negative(period_budget)?;
        let allocated_text = encode_non_negative(allocated_amount)?;
        let core_text = encode_non_negative(core_contribution)?;
        let cap_text = cash_cap.map(encode_non_negative).transpose()?;
        let after_text = encode_non_negative(after)?;
        sqlx::query(
            "INSERT INTO opportunity_cash_events \
             (id, plan_id, decision_record_id, scheduled_for, policy, balance_before, period_budget, core_contribution, allocated_amount, cash_cap, balance_after) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(plan_id.to_string())
        .bind(decision_record_id.to_string())
        .bind(scheduled_for)
        .bind(policy_name(policy))
        .bind(before_text)
        .bind(budget_text)
        .bind(core_text)
        .bind(allocated_text)
        .bind(cap_text)
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

    /// Reconcile terminal broker fills and recompute carried balances from the audited event order.
    ///
    /// Pending and partially filled orders retain their accepted-order estimate, so available
    /// opportunity cash is never released while an OpenD order can still consume it.
    pub async fn reconcile_completed_fills(&self, plan_id: Uuid) -> Result<(), sqlx::Error> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let terminal = sqlx::query(
            "SELECT opportunity_cash_events.id, opportunity_cash_events.core_contribution, \
             opportunity_cash_events.allocated_amount, paper_orders.filled_quantity, paper_orders.average_fill_price \
             FROM opportunity_cash_events JOIN paper_orders \
             ON paper_orders.decision_record_id = opportunity_cash_events.decision_record_id \
             WHERE opportunity_cash_events.plan_id = ?1 AND paper_orders.state IN ('filled', 'closed')",
        )
        .bind(plan_id.to_string())
        .fetch_all(&mut *transaction)
        .await?;
        for row in terminal {
            let event_id: String = row.try_get("id")?;
            let core = decode_non_negative(row.try_get::<String, _>("core_contribution")?)?;
            let estimated = decode_non_negative(row.try_get::<String, _>("allocated_amount")?)?;
            let quantity = decode_non_negative(row.try_get::<String, _>("filled_quantity")?)?;
            let price = decode_non_negative(row.try_get::<String, _>("average_fill_price")?)?;
            let actual = (quantity * price - core).max(Decimal::ZERO).min(estimated);
            sqlx::query(
                "UPDATE opportunity_cash_events SET actual_allocated_amount = ?1, \
                 reconciled_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?2",
            )
            .bind(encode_non_negative(actual)?)
            .bind(event_id)
            .execute(&mut *transaction)
            .await?;
        }
        let rows = sqlx::query(
            "SELECT id, policy, period_budget, allocated_amount, actual_allocated_amount, cash_cap \
             FROM opportunity_cash_events WHERE plan_id = ?1 ORDER BY scheduled_for, created_at, id",
        )
        .bind(plan_id.to_string())
        .fetch_all(&mut *transaction)
        .await?;
        let mut balance = Decimal::ZERO;
        for row in rows {
            let id: String = row.try_get("id")?;
            let policy: String = row.try_get("policy")?;
            let budget = decode_non_negative(row.try_get::<String, _>("period_budget")?)?;
            let estimated = decode_non_negative(row.try_get::<String, _>("allocated_amount")?)?;
            let actual = match row.try_get::<Option<String>, _>("actual_allocated_amount")? {
                Some(value) => decode_non_negative(value)?,
                None => estimated,
            };
            let cap = match row.try_get::<Option<String>, _>("cash_cap")? {
                Some(value) => Some(decode_non_negative(value)?),
                None => None,
            };
            balance = if policy == "expire_each_period" {
                Decimal::ZERO
            } else {
                (balance + budget - actual).max(Decimal::ZERO)
            };
            balance = cap.map_or(balance, |value| balance.min(value));
            sqlx::query("UPDATE opportunity_cash_events SET balance_after = ?1 WHERE id = ?2")
                .bind(encode_non_negative(balance)?)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
        }
        sqlx::query(
            "INSERT INTO opportunity_cash_balances (plan_id, balance, updated_at) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
             ON CONFLICT(plan_id) DO UPDATE SET balance = excluded.balance, updated_at = excluded.updated_at",
        )
        .bind(plan_id.to_string())
        .bind(encode_non_negative(balance)?)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await
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

/// Decode the fixed-scale SQLite amount and reject negative/corrupt values.
fn decode_non_negative(value: impl AsRef<str>) -> Result<Decimal, sqlx::Error> {
    let value = value.as_ref();
    let amount = if value == "000000000000.00000000" {
        Decimal::ZERO
    } else {
        decode_amount(value)
            .ok_or_else(|| sqlx::Error::Protocol("invalid opportunity amount".to_owned()))?
    };
    (amount >= Decimal::ZERO)
        .then_some(amount)
        .ok_or_else(|| sqlx::Error::Protocol("negative opportunity amount".to_owned()))
}

fn policy_name(policy: OpportunityCashPolicy) -> &'static str {
    match policy {
        OpportunityCashPolicy::ExpireEachPeriod => "expire_each_period",
        OpportunityCashPolicy::CarryForward => "carry_forward",
        // The cap is stored beside the event; SQLite's prior CHECK constraint only permits
        // this stable carry-forward label and remains compatible with pre-V1.1 databases.
        OpportunityCashPolicy::CarryWithCap => "carry_forward",
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
            .settle(OpportunityCashSettlementInput {
                plan_id,
                decision_record_id: record_id,
                scheduled_for: "2026-08-20",
                policy: OpportunityCashPolicy::CarryForward,
                cash_cap: None,
                period_budget: Decimal::new(30, 0),
                core_contribution: Decimal::new(0, 0),
                allocated_amount: Decimal::new(10, 0),
            })
            .await
            .unwrap();
        assert!(first.applied);
        assert_eq!(first.balance_after, Decimal::new(20, 0));
        let again = repo
            .settle(OpportunityCashSettlementInput {
                plan_id,
                decision_record_id: record_id,
                scheduled_for: "2026-08-20",
                policy: OpportunityCashPolicy::CarryForward,
                cash_cap: None,
                period_budget: Decimal::new(30, 0),
                core_contribution: Decimal::new(0, 0),
                allocated_amount: Decimal::new(10, 0),
            })
            .await
            .unwrap();
        assert!(!again.applied);
        assert_eq!(repo.balance(plan_id).await.unwrap(), Decimal::new(20, 0));
    }

    /// Verify capped carry-forward cannot accumulate past the persisted cash boundary.
    #[tokio::test]
    async fn carry_with_cap_clamps_the_local_balance() {
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
        let result = repo
            .settle(OpportunityCashSettlementInput {
                plan_id,
                decision_record_id: record_id,
                scheduled_for: "2026-08-20",
                policy: OpportunityCashPolicy::CarryWithCap,
                cash_cap: Some(Decimal::new(25, 0)),
                period_budget: Decimal::new(30, 0),
                core_contribution: Decimal::ZERO,
                allocated_amount: Decimal::ZERO,
            })
            .await
            .unwrap();
        assert_eq!(result.balance_after, Decimal::new(25, 0));
    }

    /// Verify a terminal partial fill corrects the earlier accepted-order estimate.
    #[tokio::test]
    async fn terminal_fill_reconciles_opportunity_cash_from_actual_spend() {
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
        sqlx::query("INSERT INTO paper_orders (order_id, plan_id, decision_record_id, symbol, side, requested_quantity, state, filled_quantity, average_fill_price, submitted_at, observed_at) VALUES ('order', ?1, ?2, 'VOO', 'buy', '000000000001.00000000', 'closed', '000000000000.50000000', '000000000100.00000000', '2026-08-20T00:00:00.000Z', '2026-08-20T00:00:00.000Z')")
            .bind(plan_id.to_string()).bind(record_id.to_string()).execute(storage.pool()).await.unwrap();
        let repo = SqliteOpportunityCashRepository::new(storage.pool().clone());
        repo.settle(OpportunityCashSettlementInput {
            plan_id,
            decision_record_id: record_id,
            scheduled_for: "2026-08-20",
            policy: OpportunityCashPolicy::CarryForward,
            cash_cap: None,
            period_budget: Decimal::new(30, 0),
            core_contribution: Decimal::new(70, 0),
            allocated_amount: Decimal::new(20, 0),
        })
        .await
        .unwrap();
        repo.reconcile_completed_fills(plan_id).await.unwrap();
        assert_eq!(repo.balance(plan_id).await.unwrap(), Decimal::new(30, 0));
    }
}

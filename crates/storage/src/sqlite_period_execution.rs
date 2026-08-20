//! SQLite atomic per-period execution-budget reservations.

use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::sqlite::{decode_amount, encode_amount};

/// SQLite repository for plan-level period execution limits.
#[derive(Clone, Debug)]
pub struct SqlitePeriodExecutionRepository {
    pool: SqlitePool,
}

impl SqlitePeriodExecutionRepository {
    /// Build the repository from a migrated SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Atomically reserve an amount under one plan and weekly/monthly period key.
    ///
    /// A returned `false` means the configured period limit would be exceeded.
    pub async fn reserve(
        &self,
        plan_id: Uuid,
        decision_record_id: Uuid,
        period_key: &str,
        limit: Decimal,
        amount: Decimal,
    ) -> Result<bool, sqlx::Error> {
        if limit <= Decimal::ZERO || amount < Decimal::ZERO {
            return Err(sqlx::Error::Protocol(
                "invalid period execution amount".to_owned(),
            ));
        }
        let amount = encode_amount(amount)
            .ok_or_else(|| sqlx::Error::Protocol("invalid period execution amount".to_owned()))?;
        let limit = encode_amount(limit)
            .ok_or_else(|| sqlx::Error::Protocol("invalid period execution limit".to_owned()))?;
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM plan_period_execution_reservations WHERE decision_record_id = ?1",
        )
        .bind(decision_record_id.to_string())
        .fetch_one(&mut *transaction)
        .await?;
        if existing != 0 {
            transaction.commit().await?;
            return Ok(true);
        }
        let used_rows = sqlx::query_scalar::<_, String>(
            "SELECT amount FROM plan_period_execution_reservations \
             WHERE plan_id = ?1 AND period_key = ?2 AND state IN ('reserved', 'accepted')",
        )
        .bind(plan_id.to_string())
        .bind(period_key)
        .fetch_all(&mut *transaction)
        .await?;
        let used = used_rows
            .into_iter()
            .try_fold(Decimal::ZERO, |total, value| {
                let amount = decode_amount(&value).ok_or_else(|| {
                    sqlx::Error::Protocol("invalid period execution ledger".to_owned())
                })?;
                Ok::<Decimal, sqlx::Error>(total + amount)
            })?;
        let amount_decimal = decode_amount(&amount)
            .ok_or_else(|| sqlx::Error::Protocol("invalid period execution amount".to_owned()))?;
        let limit_decimal = decode_amount(&limit)
            .ok_or_else(|| sqlx::Error::Protocol("invalid period execution limit".to_owned()))?;
        if used + amount_decimal > limit_decimal {
            transaction.commit().await?;
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO plan_period_execution_reservations \
             (decision_record_id, plan_id, period_key, amount, state) VALUES (?1, ?2, ?3, ?4, 'reserved')",
        )
        .bind(decision_record_id.to_string())
        .bind(plan_id.to_string())
        .bind(period_key)
        .bind(amount)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(true)
    }

    /// Mark a successful broker submission as consuming the period budget.
    pub async fn accept(&self, decision_record_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE plan_period_execution_reservations SET state = 'accepted', \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE decision_record_id = ?1",
        )
        .bind(decision_record_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Release a reservation when its broker submission did not complete.
    pub async fn release(&self, decision_record_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE plan_period_execution_reservations SET state = 'released', \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE decision_record_id = ?1 AND state = 'reserved'",
        )
        .bind(decision_record_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Reconcile terminal paper orders to their observed fill spending.
    pub async fn reconcile_completed_orders(&self, plan_id: Uuid) -> Result<(), sqlx::Error> {
        let rows = sqlx::query(
            "SELECT paper_orders.decision_record_id, paper_orders.filled_quantity, paper_orders.average_fill_price \
             FROM paper_orders JOIN plan_period_execution_reservations \
             ON paper_orders.decision_record_id = plan_period_execution_reservations.decision_record_id \
             WHERE paper_orders.plan_id = ?1 AND paper_orders.state IN ('filled', 'closed') \
             AND plan_period_execution_reservations.state = 'accepted'",
        )
        .bind(plan_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        for row in rows {
            let id: String = row.try_get("decision_record_id")?;
            let quantity = decode_amount(&row.try_get::<String, _>("filled_quantity")?)
                .ok_or_else(|| sqlx::Error::Protocol("invalid filled quantity".to_owned()))?;
            let price = decode_amount(&row.try_get::<String, _>("average_fill_price")?)
                .ok_or_else(|| sqlx::Error::Protocol("invalid fill price".to_owned()))?;
            let amount = encode_amount(quantity * price)
                .ok_or_else(|| sqlx::Error::Protocol("invalid reconciled spend".to_owned()))?;
            sqlx::query(
                "UPDATE plan_period_execution_reservations SET amount = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
                 WHERE decision_record_id = ?2",
            )
            .bind(amount)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::SqliteStorage;

    /// Verify concurrent-style reservations cannot exceed a period's configured cap.
    #[tokio::test]
    async fn reserves_only_within_the_period_cap() {
        let storage =
            SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
                .await
                .unwrap();
        storage.migrate().await.unwrap();
        let plan_id = Uuid::new_v4();
        sqlx::query("INSERT INTO investment_plans (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution) VALUES (?1, 'plan', 'VOO', '000000000100.00000000', 'USD', 1, '000000000100.00000000')")
            .bind(plan_id.to_string())
            .execute(storage.pool())
            .await
            .unwrap();
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        for record in [first, second] {
            sqlx::query("INSERT INTO decision_records (id, plan_id, symbol, currency, execution_status, execution_snapshot, fundamental_snapshot, trend_snapshot, decision_snapshot, summary) VALUES (?1, ?2, 'VOO', 'USD', 'due', '{}', '{}', '{}', '{}', 'test')")
                .bind(record.to_string()).bind(plan_id.to_string()).execute(storage.pool()).await.unwrap();
        }
        let repo = SqlitePeriodExecutionRepository::new(storage.pool().clone());
        assert!(repo
            .reserve(
                plan_id,
                first,
                "2026-08",
                Decimal::new(200, 0),
                Decimal::new(120, 0)
            )
            .await
            .unwrap());
        assert!(!repo
            .reserve(
                plan_id,
                second,
                "2026-08",
                Decimal::new(200, 0),
                Decimal::new(100, 0)
            )
            .await
            .unwrap());
        repo.release(first).await.unwrap();
        assert!(repo
            .reserve(
                plan_id,
                second,
                "2026-08",
                Decimal::new(200, 0),
                Decimal::new(100, 0)
            )
            .await
            .unwrap());
    }
}

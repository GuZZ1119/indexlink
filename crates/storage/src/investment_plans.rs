//! PostgreSQL adapter for investment plan repository port.

use async_trait::async_trait;
use investment_plans::{
    CreateInvestmentPlan, InvestmentPlan, InvestmentPlanRepository, PlanRepositoryError,
    PlanValidationError, ScheduleKind, UpdateInvestmentPlan,
};
use rust_decimal::Decimal;
use sqlx::{postgres::PgRow, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

const PLAN_COLUMNS: &str = "id::text AS id, name, symbol, base_contribution::text AS \
    base_contribution, currency, schedule_kind, schedule_day, max_single_execution::text AS \
    max_single_execution, is_active, (EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS \
    created_at_micros, (EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_micros";

/// PostgreSQL implementation of [`InvestmentPlanRepository`].
#[derive(Clone, Debug)]
pub struct PostgresInvestmentPlanRepository {
    pool: PgPool,
}

impl PostgresInvestmentPlanRepository {
    /// Build a repository from an existing PostgreSQL pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvestmentPlanRepository for PostgresInvestmentPlanRepository {
    /// Insert a normalized investment plan and return the persisted row.
    async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let row = sqlx::query(&format!(
            "INSERT INTO investment_plans \
             (name, symbol, base_contribution, currency, schedule_kind, schedule_day, \
              max_single_execution, is_active) \
             VALUES ($1, $2, $3::numeric, $4, 'monthly', $5, $6::numeric, true) \
             RETURNING {PLAN_COLUMNS}"
        ))
        .bind(input.name)
        .bind(input.symbol)
        .bind(input.base_contribution.to_string())
        .bind(input.currency)
        .bind(input.schedule_day)
        .bind(input.max_single_execution.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        plan_from_row(row)
    }

    /// List plans in deterministic creation order.
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {PLAN_COLUMNS} FROM investment_plans ORDER BY created_at ASC, id ASC"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter().map(plan_from_row).collect()
    }

    /// Fetch one plan by ID.
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {PLAN_COLUMNS} FROM investment_plans WHERE id = $1::uuid"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(PlanRepositoryError::NotFound)?;

        plan_from_row(row)
    }

    /// Merge, validate, and persist an update inside one database transaction.
    async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let current = sqlx::query(
            "SELECT base_contribution::text AS base_contribution, \
             max_single_execution::text AS max_single_execution \
             FROM investment_plans WHERE id = $1::uuid FOR UPDATE",
        )
        .bind(id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(PlanRepositoryError::NotFound)?;

        let base = input.base_contribution.unwrap_or(parse_decimal(
            current
                .try_get("base_contribution")
                .map_err(map_sqlx_error)?,
        )?);
        let max = input.max_single_execution.unwrap_or(parse_decimal(
            current
                .try_get("max_single_execution")
                .map_err(map_sqlx_error)?,
        )?);
        validate_final_amounts(base, max)?;
        let base_contribution = input.base_contribution.map(|value| value.to_string());
        let max_single_execution = input.max_single_execution.map(|value| value.to_string());

        let row = sqlx::query(&format!(
            "UPDATE investment_plans SET \
             name = COALESCE($2, name), \
             base_contribution = COALESCE($3::numeric, base_contribution), \
             schedule_day = COALESCE($4, schedule_day), \
             max_single_execution = COALESCE($5::numeric, max_single_execution), \
             is_active = COALESCE($6, is_active), \
             updated_at = NOW() \
             WHERE id = $1::uuid RETURNING {PLAN_COLUMNS}"
        ))
        .bind(id.to_string())
        .bind(input.name)
        .bind(base_contribution)
        .bind(input.schedule_day)
        .bind(max_single_execution)
        .bind(input.is_active)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;

        plan_from_row(row)
    }

    /// Persist the active flag through the dedicated toggle use case.
    async fn set_active(
        &self,
        id: Uuid,
        is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let row = sqlx::query(&format!(
            "UPDATE investment_plans \
             SET is_active = $2, updated_at = NOW() \
             WHERE id = $1::uuid RETURNING {PLAN_COLUMNS}"
        ))
        .bind(id.to_string())
        .bind(is_active)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(PlanRepositoryError::NotFound)?;

        plan_from_row(row)
    }
}

fn plan_from_row(row: PgRow) -> Result<InvestmentPlan, PlanRepositoryError> {
    let schedule_kind = match row
        .try_get::<String, _>("schedule_kind")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "monthly" => ScheduleKind::Monthly,
        _ => return Err(PlanRepositoryError::Unavailable),
    };

    Ok(InvestmentPlan {
        id: parse_uuid(row.try_get("id").map_err(map_sqlx_error)?)?,
        name: row.try_get("name").map_err(map_sqlx_error)?,
        symbol: row.try_get("symbol").map_err(map_sqlx_error)?,
        base_contribution: parse_decimal(
            row.try_get("base_contribution").map_err(map_sqlx_error)?,
        )?,
        currency: row.try_get("currency").map_err(map_sqlx_error)?,
        schedule_kind,
        schedule_day: row.try_get("schedule_day").map_err(map_sqlx_error)?,
        max_single_execution: parse_decimal(
            row.try_get("max_single_execution")
                .map_err(map_sqlx_error)?,
        )?,
        is_active: row.try_get("is_active").map_err(map_sqlx_error)?,
        created_at: parse_micros(row.try_get("created_at_micros").map_err(map_sqlx_error)?)?,
        updated_at: parse_micros(row.try_get("updated_at_micros").map_err(map_sqlx_error)?)?,
    })
}

fn parse_uuid(value: String) -> Result<Uuid, PlanRepositoryError> {
    value.parse().map_err(|_| PlanRepositoryError::Unavailable)
}

fn parse_decimal(value: String) -> Result<Decimal, PlanRepositoryError> {
    value.parse().map_err(|_| PlanRepositoryError::Unavailable)
}

fn parse_micros(value: i64) -> Result<OffsetDateTime, PlanRepositoryError> {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(value) * 1000)
        .map_err(|_| PlanRepositoryError::Unavailable)
}

fn validate_final_amounts(base: Decimal, max: Decimal) -> Result<(), PlanRepositoryError> {
    if base <= Decimal::ZERO {
        return Err(PlanValidationError::NonPositiveAmount {
            field: "base_contribution",
        }
        .into());
    }
    if max <= Decimal::ZERO {
        return Err(PlanValidationError::NonPositiveAmount {
            field: "max_single_execution",
        }
        .into());
    }
    if max < base {
        return Err(PlanValidationError::MaxBelowBaseContribution.into());
    }
    Ok(())
}

fn map_sqlx_error(error: sqlx::Error) -> PlanRepositoryError {
    match error {
        sqlx::Error::RowNotFound => PlanRepositoryError::NotFound,
        _ => PlanRepositoryError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_sqlx_errors_to_safe_repository_errors() {
        assert_eq!(
            map_sqlx_error(sqlx::Error::RowNotFound),
            PlanRepositoryError::NotFound
        );
        assert_eq!(
            map_sqlx_error(sqlx::Error::PoolClosed),
            PlanRepositoryError::Unavailable
        );
    }

    #[test]
    fn validates_final_update_amount_relationship() {
        assert_eq!(
            validate_final_amounts(Decimal::new(2000, 0), Decimal::new(1500, 0)),
            Err(PlanRepositoryError::Validation(
                PlanValidationError::MaxBelowBaseContribution
            ))
        );
    }
}

//! PostgreSQL adapter for investment plan repository port.

use async_trait::async_trait;
use investment_plans::{
    BucketAllocationRatio, CreateInvestmentPlan, InvestmentPlan, InvestmentPlanRepository,
    OpportunityCashPolicy, PlanExecutionConfiguration, PlanRepositoryError, PlanRiskMode,
    PlanValidationError, ScheduleKind, TwoBucketAllocationConfig, UpdateInvestmentPlan,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use sqlx::{postgres::PgRow, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

const RATIO_UNITS: i64 = 100_000_000;
const PLAN_COLUMNS: &str = "p.id::text AS id, p.name, p.symbol, p.base_contribution::text AS \
    base_contribution, p.currency, c.schedule_kind, c.schedule_day, c.core_ratio_units, \
    c.opportunity_ratio_units, c.risk_mode, c.opportunity_cash_policy, p.max_single_execution::text AS \
    max_single_execution, p.is_active, (EXTRACT(EPOCH FROM p.created_at) * 1000000)::bigint AS \
    created_at_micros, (EXTRACT(EPOCH FROM p.updated_at) * 1000000)::bigint AS updated_at_micros";

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
        // Production uses SQLite, which persists immutable policy bindings.  This retained
        // PostgreSQL adapter has no equivalent migration, so reject values it cannot store
        // rather than silently returning a plan under the wrong policy.
        if input
            .policy
            .as_ref()
            .is_some_and(|policy| *policy != investment_plans::legacy_core_opportunity_v1_policy())
        {
            return Err(PlanRepositoryError::Unavailable);
        }
        let CreateInvestmentPlan {
            name,
            symbol,
            base_contribution,
            currency,
            schedule_kind,
            schedule_day,
            schedule_days: _,
            policy: _,
            execution_configuration,
            max_single_execution,
        } = input;
        let (core_ratio_units, opportunity_ratio_units, risk_mode, opportunity_cash_policy) =
            execution_configuration_values(execution_configuration)?;
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let row = sqlx::query(
            "INSERT INTO investment_plans \
             (name, symbol, base_contribution, currency, schedule_kind, schedule_day, \
              max_single_execution, is_active) \
             VALUES ($1, $2, $3::numeric, $4, 'monthly', $5, $6::numeric, true) \
             RETURNING id::text AS id",
        )
        .bind(name)
        .bind(symbol)
        .bind(base_contribution.to_string())
        .bind(currency)
        .bind(schedule_day)
        .bind(max_single_execution.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let id: String = row.try_get("id").map_err(map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO investment_plan_execution_configurations \
             (plan_id, schedule_kind, schedule_day, core_ratio_units, opportunity_ratio_units, risk_mode, opportunity_cash_policy) \
             VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&id)
        .bind(schedule_kind_name(schedule_kind))
        .bind(schedule_day)
        .bind(core_ratio_units)
        .bind(opportunity_ratio_units)
        .bind(risk_mode_name(risk_mode))
        .bind(opportunity_cash_policy_name(opportunity_cash_policy))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let row = sqlx::query(&format!(
            "SELECT {PLAN_COLUMNS} FROM investment_plans p \
             JOIN investment_plan_execution_configurations c ON c.plan_id = p.id \
             WHERE p.id = $1::uuid"
        ))
        .bind(&id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        plan_from_row(row)
    }

    /// List plans in deterministic creation order.
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {PLAN_COLUMNS} FROM investment_plans p \
             JOIN investment_plan_execution_configurations c ON c.plan_id = p.id \
             ORDER BY p.created_at ASC, p.id ASC"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter().map(plan_from_row).collect()
    }

    /// Fetch one plan by ID.
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {PLAN_COLUMNS} FROM investment_plans p \
             JOIN investment_plan_execution_configurations c ON c.plan_id = p.id \
             WHERE p.id = $1::uuid"
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
        if input.policy.is_some() {
            return Err(PlanRepositoryError::Unavailable);
        }
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let current = sqlx::query(
            "SELECT p.base_contribution::text AS base_contribution, \
             p.max_single_execution::text AS max_single_execution, c.schedule_kind, \
             c.schedule_day, c.core_ratio_units, c.opportunity_ratio_units, c.risk_mode, c.opportunity_cash_policy \
             FROM investment_plans p JOIN investment_plan_execution_configurations c \
             ON c.plan_id = p.id WHERE p.id = $1::uuid FOR UPDATE OF p, c",
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
        let schedule_kind = schedule_kind_from_name(
            current
                .try_get::<String, _>("schedule_kind")
                .map_err(map_sqlx_error)?
                .as_str(),
        )?;
        let current_schedule_day: i16 = current.try_get("schedule_day").map_err(map_sqlx_error)?;
        let schedule_day = input.schedule_day.unwrap_or(current_schedule_day);
        validate_schedule_day(schedule_kind, schedule_day)?;
        let current_configuration = execution_configuration_from_row(&current)?;
        let bucket_allocation = input
            .bucket_allocation
            .unwrap_or(current_configuration.bucket_allocation());
        let risk_mode = input.risk_mode.unwrap_or(current_configuration.risk_mode());
        let opportunity_cash_policy = input
            .opportunity_cash_policy
            .unwrap_or(current_configuration.opportunity_cash_policy());
        let configuration = PlanExecutionConfiguration::new_with_cash_policy(
            bucket_allocation,
            risk_mode,
            opportunity_cash_policy,
        )?;
        let (core_ratio_units, opportunity_ratio_units, risk_mode, opportunity_cash_policy) =
            execution_configuration_values(configuration)?;
        let base_contribution = input.base_contribution.map(|value| value.to_string());
        let max_single_execution = input.max_single_execution.map(|value| value.to_string());

        sqlx::query(
            "UPDATE investment_plans SET \
             name = COALESCE($2, name), \
             base_contribution = COALESCE($3::numeric, base_contribution), \
             schedule_day = COALESCE($4, schedule_day), \
             max_single_execution = COALESCE($5::numeric, max_single_execution), \
             is_active = COALESCE($6, is_active), \
             updated_at = NOW() \
             WHERE id = $1::uuid",
        )
        .bind(id.to_string())
        .bind(input.name)
        .bind(base_contribution)
        .bind(input.schedule_day)
        .bind(max_single_execution)
        .bind(input.is_active)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "UPDATE investment_plan_execution_configurations SET schedule_day = $2, \
             core_ratio_units = $3, opportunity_ratio_units = $4, risk_mode = $5, opportunity_cash_policy = $6 \
             WHERE plan_id = $1::uuid",
        )
        .bind(id.to_string())
        .bind(schedule_day)
        .bind(core_ratio_units)
        .bind(opportunity_ratio_units)
        .bind(risk_mode_name(risk_mode))
        .bind(opportunity_cash_policy_name(opportunity_cash_policy))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let row = sqlx::query(&format!(
            "SELECT {PLAN_COLUMNS} FROM investment_plans p \
             JOIN investment_plan_execution_configurations c ON c.plan_id = p.id \
             WHERE p.id = $1::uuid"
        ))
        .bind(id.to_string())
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
        let result = sqlx::query(
            "UPDATE investment_plans \
             SET is_active = $2, updated_at = NOW() \
             WHERE id = $1::uuid",
        )
        .bind(id.to_string())
        .bind(is_active)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        if result.rows_affected() == 0 {
            return Err(PlanRepositoryError::NotFound);
        }
        let row = sqlx::query(&format!(
            "SELECT {PLAN_COLUMNS} FROM investment_plans p \
             JOIN investment_plan_execution_configurations c ON c.plan_id = p.id \
             WHERE p.id = $1::uuid"
        ))
        .bind(id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        plan_from_row(row)
    }
}

fn plan_from_row(row: PgRow) -> Result<InvestmentPlan, PlanRepositoryError> {
    let schedule_kind = schedule_kind_from_name(
        row.try_get::<String, _>("schedule_kind")
            .map_err(map_sqlx_error)?
            .as_str(),
    )?;
    let schedule_day: i16 = row.try_get("schedule_day").map_err(map_sqlx_error)?;
    validate_schedule_day(schedule_kind, schedule_day)?;

    Ok(InvestmentPlan {
        id: parse_uuid(row.try_get("id").map_err(map_sqlx_error)?)?,
        name: row.try_get("name").map_err(map_sqlx_error)?,
        symbol: row.try_get("symbol").map_err(map_sqlx_error)?,
        base_contribution: parse_decimal(
            row.try_get("base_contribution").map_err(map_sqlx_error)?,
        )?,
        currency: row.try_get("currency").map_err(map_sqlx_error)?,
        schedule_kind,
        schedule_day,
        schedule_days: vec![schedule_day],
        // PostgreSQL is a retained legacy adapter. Its schema has no policy binding
        // migration because production uses SQLite; legacy rows retain V1 semantics.
        policy: investment_plans::legacy_core_opportunity_v1_policy(),
        execution_configuration: execution_configuration_from_row(&row)?,
        max_single_execution: parse_decimal(
            row.try_get("max_single_execution")
                .map_err(map_sqlx_error)?,
        )?,
        is_active: row.try_get("is_active").map_err(map_sqlx_error)?,
        created_at: parse_micros(row.try_get("created_at_micros").map_err(map_sqlx_error)?)?,
        updated_at: parse_micros(row.try_get("updated_at_micros").map_err(map_sqlx_error)?)?,
    })
}

fn schedule_kind_from_name(value: &str) -> Result<ScheduleKind, PlanRepositoryError> {
    match value {
        "monthly" => Ok(ScheduleKind::Monthly),
        "weekly" => Ok(ScheduleKind::Weekly),
        _ => Err(PlanRepositoryError::Unavailable),
    }
}

fn schedule_kind_name(value: ScheduleKind) -> &'static str {
    match value {
        ScheduleKind::Monthly => "monthly",
        ScheduleKind::Weekly => "weekly",
    }
}

fn risk_mode_name(value: PlanRiskMode) -> &'static str {
    match value {
        PlanRiskMode::Fixed => "fixed",
        PlanRiskMode::Autopilot => "autopilot",
        PlanRiskMode::Approval => "approval",
    }
}

/// Encode the opportunity cash policy as a stable PostgreSQL value.
fn opportunity_cash_policy_name(value: OpportunityCashPolicy) -> &'static str {
    match value {
        OpportunityCashPolicy::ExpireEachPeriod => "expire_each_period",
        OpportunityCashPolicy::CarryForward => "carry_forward",
        OpportunityCashPolicy::CarryWithCap => "carry_with_cap",
    }
}

fn execution_configuration_from_row(
    row: &PgRow,
) -> Result<PlanExecutionConfiguration, PlanRepositoryError> {
    let bucket_allocation = TwoBucketAllocationConfig::new(
        decode_ratio_units(row.try_get("core_ratio_units").map_err(map_sqlx_error)?)?,
        decode_ratio_units(
            row.try_get("opportunity_ratio_units")
                .map_err(map_sqlx_error)?,
        )?,
    )?;
    let risk_mode = match row
        .try_get::<String, _>("risk_mode")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "fixed" => PlanRiskMode::Fixed,
        "autopilot" => PlanRiskMode::Autopilot,
        "approval" => PlanRiskMode::Approval,
        _ => return Err(PlanRepositoryError::Unavailable),
    };
    let opportunity_cash_policy = match row
        .try_get::<String, _>("opportunity_cash_policy")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "expire_each_period" => OpportunityCashPolicy::ExpireEachPeriod,
        "carry_forward" => OpportunityCashPolicy::CarryForward,
        _ => return Err(PlanRepositoryError::Unavailable),
    };
    PlanExecutionConfiguration::new_with_cash_policy(
        bucket_allocation,
        risk_mode,
        opportunity_cash_policy,
    )
    .map_err(Into::into)
}

fn execution_configuration_values(
    configuration: PlanExecutionConfiguration,
) -> Result<(i64, i64, PlanRiskMode, OpportunityCashPolicy), PlanRepositoryError> {
    let bucket_allocation = configuration.bucket_allocation();
    Ok((
        encode_ratio_units(bucket_allocation.core_ratio())?,
        encode_ratio_units(bucket_allocation.opportunity_ratio())?,
        configuration.risk_mode(),
        configuration.opportunity_cash_policy(),
    ))
}

fn encode_ratio_units(value: BucketAllocationRatio) -> Result<i64, PlanRepositoryError> {
    let mut decimal = value.value();
    decimal.rescale(8);
    if decimal != value.value() {
        return Err(PlanRepositoryError::Unavailable);
    }
    let units = (decimal * Decimal::from(RATIO_UNITS))
        .to_i64()
        .ok_or(PlanRepositoryError::Unavailable)?;
    (0..=RATIO_UNITS)
        .contains(&units)
        .then_some(units)
        .ok_or(PlanRepositoryError::Unavailable)
}

fn decode_ratio_units(value: i64) -> Result<BucketAllocationRatio, PlanRepositoryError> {
    if !(0..=RATIO_UNITS).contains(&value) {
        return Err(PlanRepositoryError::Unavailable);
    }
    BucketAllocationRatio::new(Decimal::new(value, 8)).map_err(Into::into)
}

fn validate_schedule_day(kind: ScheduleKind, day: i16) -> Result<(), PlanRepositoryError> {
    match kind {
        ScheduleKind::Monthly if (1..=28).contains(&day) => Ok(()),
        ScheduleKind::Weekly if (1..=7).contains(&day) => Ok(()),
        ScheduleKind::Monthly => Err(PlanValidationError::InvalidScheduleDay.into()),
        ScheduleKind::Weekly => Err(PlanValidationError::InvalidWeeklyScheduleDay.into()),
    }
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

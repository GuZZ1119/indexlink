//! SQLite adapter for the investment-plan repository port.

use async_trait::async_trait;
use investment_plans::{
    CreateInvestmentPlan, InvestmentPlan, InvestmentPlanRepository, PlanRepositoryError,
    PlanValidationError, ScheduleKind, UpdateInvestmentPlan,
};
use rust_decimal::Decimal;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::sqlite::{decode_amount, encode_amount};

const INSERT_PLAN_SQL: &str = "INSERT INTO investment_plans \
    (id, name, symbol, base_contribution, currency, schedule_kind, schedule_day, \
     max_single_execution, is_active) \
    VALUES (?1, ?2, ?3, ?4, ?5, 'monthly', ?6, ?7, 1) \
    RETURNING id, name, symbol, base_contribution, currency, schedule_kind, schedule_day, \
    max_single_execution, is_active, created_at, updated_at";
const LIST_PLANS_SQL: &str = "SELECT id, name, symbol, base_contribution, currency, \
    schedule_kind, schedule_day, max_single_execution, is_active, created_at, updated_at \
    FROM investment_plans ORDER BY created_at ASC, id ASC";
const GET_PLAN_SQL: &str = "SELECT id, name, symbol, base_contribution, currency, \
    schedule_kind, schedule_day, max_single_execution, is_active, created_at, updated_at \
    FROM investment_plans WHERE id = ?1";
const SELECT_UPDATE_AMOUNTS_SQL: &str = "SELECT base_contribution, max_single_execution \
    FROM investment_plans WHERE id = ?1";
const UPDATE_PLAN_SQL: &str = "UPDATE investment_plans SET \
    name = COALESCE(?2, name), \
    base_contribution = COALESCE(?3, base_contribution), \
    schedule_day = COALESCE(?4, schedule_day), \
    max_single_execution = COALESCE(?5, max_single_execution), \
    is_active = COALESCE(?6, is_active), \
    updated_at = MAX( \
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
        strftime('%Y-%m-%dT%H:%M:%fZ', updated_at, '+0.001 seconds') \
    ) \
    WHERE id = ?1 \
    RETURNING id, name, symbol, base_contribution, currency, schedule_kind, schedule_day, \
    max_single_execution, is_active, created_at, updated_at";
const SET_ACTIVE_SQL: &str = "UPDATE investment_plans SET \
    is_active = ?2, \
    updated_at = MAX( \
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
        strftime('%Y-%m-%dT%H:%M:%fZ', updated_at, '+0.001 seconds') \
    ) \
    WHERE id = ?1 \
    RETURNING id, name, symbol, base_contribution, currency, schedule_kind, schedule_day, \
    max_single_execution, is_active, created_at, updated_at";

/// SQLite implementation of [`InvestmentPlanRepository`].
#[derive(Clone, Debug)]
pub struct SqliteInvestmentPlanRepository {
    pool: SqlitePool,
}

impl SqliteInvestmentPlanRepository {
    /// Build a repository from an existing SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvestmentPlanRepository for SqliteInvestmentPlanRepository {
    /// Insert a normalized investment plan and return the persisted row.
    async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let base_contribution =
            encode_amount(input.base_contribution).ok_or(PlanRepositoryError::Unavailable)?;
        let max_single_execution =
            encode_amount(input.max_single_execution).ok_or(PlanRepositoryError::Unavailable)?;
        let row = sqlx::query(INSERT_PLAN_SQL)
            .bind(Uuid::new_v4().to_string())
            .bind(input.name)
            .bind(input.symbol)
            .bind(base_contribution)
            .bind(input.currency)
            .bind(input.schedule_day)
            .bind(max_single_execution)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        plan_from_row(row)
    }

    /// List plans in deterministic creation order.
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
        let rows = sqlx::query(LIST_PLANS_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        rows.into_iter().map(plan_from_row).collect()
    }

    /// Fetch one plan by ID.
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
        let row = sqlx::query(GET_PLAN_SQL)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(PlanRepositoryError::NotFound)?;

        plan_from_row(row)
    }

    /// Merge, validate, and persist an update within one SQLite write transaction.
    async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(map_sqlx_error)?;
        let current = sqlx::query(SELECT_UPDATE_AMOUNTS_SQL)
            .bind(id.to_string())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(PlanRepositoryError::NotFound)?;
        let current_base = decode_amount(
            current
                .try_get::<String, _>("base_contribution")
                .map_err(map_sqlx_error)?
                .as_str(),
        )
        .ok_or(PlanRepositoryError::Unavailable)?;
        let current_max = decode_amount(
            current
                .try_get::<String, _>("max_single_execution")
                .map_err(map_sqlx_error)?
                .as_str(),
        )
        .ok_or(PlanRepositoryError::Unavailable)?;
        let base = input.base_contribution.unwrap_or(current_base);
        let max = input.max_single_execution.unwrap_or(current_max);
        validate_final_amounts(base, max)?;

        let base_contribution = input
            .base_contribution
            .map(|value| encode_amount(value).ok_or(PlanRepositoryError::Unavailable))
            .transpose()?;
        let max_single_execution = input
            .max_single_execution
            .map(|value| encode_amount(value).ok_or(PlanRepositoryError::Unavailable))
            .transpose()?;
        let is_active = input.is_active.map(i64::from);
        let row = sqlx::query(UPDATE_PLAN_SQL)
            .bind(id.to_string())
            .bind(input.name)
            .bind(base_contribution)
            .bind(input.schedule_day)
            .bind(max_single_execution)
            .bind(is_active)
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;

        plan_from_row(row)
    }

    /// Persist the active flag through the dedicated toggle use case.
    async fn set_active(
        &self,
        id: Uuid,
        is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let row = sqlx::query(SET_ACTIVE_SQL)
            .bind(id.to_string())
            .bind(i64::from(is_active))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(PlanRepositoryError::NotFound)?;

        plan_from_row(row)
    }
}

/// 将 SQLite 查询结果转换为已验证的领域计划。
fn plan_from_row(row: SqliteRow) -> Result<InvestmentPlan, PlanRepositoryError> {
    let schedule_kind = match row
        .try_get::<String, _>("schedule_kind")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "monthly" => ScheduleKind::Monthly,
        _ => return Err(PlanRepositoryError::Unavailable),
    };
    let is_active = match row.try_get::<i64, _>("is_active").map_err(map_sqlx_error)? {
        0 => false,
        1 => true,
        _ => return Err(PlanRepositoryError::Unavailable),
    };

    Ok(InvestmentPlan {
        id: parse_uuid(row.try_get("id").map_err(map_sqlx_error)?)?,
        name: row.try_get("name").map_err(map_sqlx_error)?,
        symbol: row.try_get("symbol").map_err(map_sqlx_error)?,
        base_contribution: parse_amount(row.try_get("base_contribution").map_err(map_sqlx_error)?)?,
        currency: row.try_get("currency").map_err(map_sqlx_error)?,
        schedule_kind,
        schedule_day: i16::try_from(
            row.try_get::<i64, _>("schedule_day")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| PlanRepositoryError::Unavailable)?,
        max_single_execution: parse_amount(
            row.try_get("max_single_execution")
                .map_err(map_sqlx_error)?,
        )?,
        is_active,
        created_at: parse_timestamp(row.try_get("created_at").map_err(map_sqlx_error)?)?,
        updated_at: parse_timestamp(row.try_get("updated_at").map_err(map_sqlx_error)?)?,
    })
}

/// 解析数据库存储的 UUID 文本。
fn parse_uuid(value: String) -> Result<Uuid, PlanRepositoryError> {
    value.parse().map_err(|_| PlanRepositoryError::Unavailable)
}

/// 解析 schema 强制的固定精度金额文本。
fn parse_amount(value: String) -> Result<Decimal, PlanRepositoryError> {
    decode_amount(&value).ok_or(PlanRepositoryError::Unavailable)
}

/// 解析 schema 强制的 UTC RFC 3339 时间文本。
fn parse_timestamp(value: String) -> Result<OffsetDateTime, PlanRepositoryError> {
    OffsetDateTime::parse(&value, &Rfc3339).map_err(|_| PlanRepositoryError::Unavailable)
}

/// 校验合并更新后的最终金额关系。
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

/// 将底层 SQLite 错误映射为安全的 repository 错误。
fn map_sqlx_error(error: sqlx::Error) -> PlanRepositoryError {
    match error {
        sqlx::Error::RowNotFound => PlanRepositoryError::NotFound,
        _ => PlanRepositoryError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::SqliteStorage;

    /// 创建测试用 Decimal。
    fn amount(value: &str) -> Decimal {
        value.parse().expect("test amount must parse")
    }

    /// 创建合法的测试计划输入。
    fn input() -> CreateInvestmentPlan {
        CreateInvestmentPlan {
            name: "Core plan".to_owned(),
            symbol: "VOO".to_owned(),
            base_contribution: amount("1000.00"),
            currency: "USD".to_owned(),
            schedule_kind: ScheduleKind::Monthly,
            schedule_day: 15,
            max_single_execution: amount("1500.00"),
        }
    }

    /// 创建已执行 migration 的隔离 SQLite repository。
    async fn repository() -> SqliteInvestmentPlanRepository {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .in_memory(true)
                    .foreign_keys(true),
            )
            .await
            .expect("in-memory SQLite pool must connect");
        let storage = SqliteStorage::from_pool(pool);
        storage
            .migrate()
            .await
            .expect("SQLite migration must apply");
        SqliteInvestmentPlanRepository::new(storage.pool().clone())
    }

    /// 验证 SQLite adapter 按固定精度编码金额并实现 create、list、get。
    #[tokio::test]
    async fn creates_lists_and_gets_plans() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();

        assert_eq!(created.base_contribution, amount("1000.00000000"));
        assert_eq!(created.max_single_execution, amount("1500.00000000"));
        assert_eq!(repository.list().await.unwrap(), vec![created.clone()]);
        assert_eq!(repository.get(created.id).await.unwrap(), created);
    }

    /// 验证更新在同一 SQLite 写事务中校验最终金额组合。
    #[tokio::test]
    async fn update_preserves_atomic_amount_validation() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();

        let invalid = repository
            .update(
                created.id,
                UpdateInvestmentPlan {
                    base_contribution: Some(amount("2000.00")),
                    ..Default::default()
                },
            )
            .await;
        assert_eq!(
            invalid,
            Err(PlanRepositoryError::Validation(
                PlanValidationError::MaxBelowBaseContribution
            ))
        );
        assert_eq!(
            repository.get(created.id).await.unwrap().base_contribution,
            amount("1000.00000000")
        );

        let updated = repository
            .update(
                created.id,
                UpdateInvestmentPlan {
                    name: Some("Growth plan".to_owned()),
                    base_contribution: Some(amount("1200.00")),
                    schedule_day: Some(20),
                    max_single_execution: Some(amount("1800.00")),
                    is_active: Some(false),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Growth plan");
        assert_eq!(updated.base_contribution, amount("1200.00000000"));
        assert_eq!(updated.schedule_day, 20);
        assert_eq!(updated.max_single_execution, amount("1800.00000000"));
        assert!(!updated.is_active);
    }

    /// 验证 SQLite adapter 持久化启停状态并正确处理金额精度边界。
    #[tokio::test]
    async fn toggles_activity_and_rejects_unrepresentable_amounts() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();

        let inactive = repository.set_active(created.id, false).await.unwrap();
        assert!(!inactive.is_active);
        let trailing_zero_precision = repository
            .create(CreateInvestmentPlan {
                base_contribution: amount("1.000000000"),
                max_single_execution: amount("2.000000000"),
                ..input()
            })
            .await
            .unwrap();
        assert_eq!(
            trailing_zero_precision.base_contribution,
            amount("1.00000000")
        );
        assert_eq!(
            trailing_zero_precision.max_single_execution,
            amount("2.00000000")
        );
        assert_eq!(
            repository
                .create(CreateInvestmentPlan {
                    base_contribution: amount("1.000000001"),
                    ..input()
                })
                .await,
            Err(PlanRepositoryError::Unavailable)
        );
    }

    /// 验证固定精度金额编码拒绝零、负数和超出整数范围的值。
    #[test]
    fn amount_codec_enforces_sqlite_representation() {
        assert_eq!(
            encode_amount(amount("12.5")).as_deref(),
            Some("000000000012.50000000")
        );
        assert_eq!(
            decode_amount("000000000012.50000000"),
            Some(amount("12.50000000"))
        );
        assert_eq!(encode_amount(Decimal::ZERO), None);
        assert_eq!(encode_amount(amount("-1.00")), None);
        assert_eq!(
            encode_amount(amount("1.000000000")).as_deref(),
            Some("000000000001.00000000")
        );
        assert_eq!(encode_amount(amount("1.000000001")), None);
        assert_eq!(encode_amount(amount("1000000000000.00")), None);
    }

    /// 验证快速更新时仍保证 UTC 更新时间严格递增。
    #[tokio::test]
    async fn updates_advance_timestamp_even_when_clock_does_not() {
        let repository = repository().await;
        let created = repository.create(input()).await.unwrap();
        let future_timestamp = "2099-01-01T00:00:00.000Z";
        sqlx::query("UPDATE investment_plans SET updated_at = ?1 WHERE id = ?2")
            .bind(future_timestamp)
            .bind(created.id.to_string())
            .execute(&repository.pool)
            .await
            .expect("test timestamp override must succeed");
        let future_timestamp =
            OffsetDateTime::parse(future_timestamp, &Rfc3339).expect("test timestamp must parse");

        let updated = repository.set_active(created.id, false).await.unwrap();

        assert!(updated.updated_at > future_timestamp);
    }

    /// 验证 SQLite 错误和损坏金额快照映射为安全 repository 错误。
    #[test]
    fn maps_storage_failures_to_safe_repository_errors() {
        assert_eq!(
            map_sqlx_error(sqlx::Error::RowNotFound),
            PlanRepositoryError::NotFound
        );
        assert_eq!(
            map_sqlx_error(sqlx::Error::PoolClosed),
            PlanRepositoryError::Unavailable
        );
        assert_eq!(
            parse_amount("1000.00".to_owned()),
            Err(PlanRepositoryError::Unavailable)
        );
    }

    /// 验证所有 SQLite 查询保持静态并使用 SQLite 参数占位符。
    #[test]
    fn query_strings_are_static_and_sqlite_compatible() {
        for query in [
            INSERT_PLAN_SQL,
            LIST_PLANS_SQL,
            GET_PLAN_SQL,
            SELECT_UPDATE_AMOUNTS_SQL,
            UPDATE_PLAN_SQL,
            SET_ACTIVE_SQL,
        ] {
            assert!(!query.contains('$'));
        }
        assert!(UPDATE_PLAN_SQL.contains("MAX("));
        assert!(SET_ACTIVE_SQL.contains("+0.001 seconds"));
    }
}

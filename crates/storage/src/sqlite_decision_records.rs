//! SQLite adapter for the decision-record repository port.

use async_trait::async_trait;
use decision_records::{
    CreateDecisionRecord, DecisionExecutionStatus, DecisionRecord, DecisionRecordListQuery,
    DecisionRecordRepository, DecisionRecordRepositoryError,
};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::sqlite::{decode_amount, encode_amount};

const INSERT_RECORD_SQL: &str = concat!(
    "INSERT INTO decision_records ",
    "(id, plan_id, symbol, currency, execution_status, planned_contribution, ",
    "execution_snapshot, fundamental_snapshot, trend_snapshot, sentiment_snapshot, ",
    "decision_snapshot, broker_order_request, broker_order_ack, summary) ",
    "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14) ",
    "RETURNING id, plan_id, symbol, currency, execution_status, planned_contribution, ",
    "execution_snapshot, fundamental_snapshot, trend_snapshot, sentiment_snapshot, ",
    "decision_snapshot, broker_order_request, broker_order_ack, summary, created_at"
);
const LIST_RECORDS_BY_PLAN_SQL: &str = concat!(
    "SELECT id, plan_id, symbol, currency, execution_status, planned_contribution, ",
    "execution_snapshot, fundamental_snapshot, trend_snapshot, sentiment_snapshot, ",
    "decision_snapshot, broker_order_request, broker_order_ack, summary, created_at ",
    "FROM decision_records WHERE plan_id = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2"
);
const GET_RECORD_SQL: &str = concat!(
    "SELECT id, plan_id, symbol, currency, execution_status, planned_contribution, ",
    "execution_snapshot, fundamental_snapshot, trend_snapshot, sentiment_snapshot, ",
    "decision_snapshot, broker_order_request, broker_order_ack, summary, created_at ",
    "FROM decision_records WHERE id = ?1"
);

/// SQLite implementation of [`DecisionRecordRepository`].
#[derive(Clone, Debug)]
pub struct SqliteDecisionRecordRepository {
    pool: SqlitePool,
}

impl SqliteDecisionRecordRepository {
    /// Build a repository from an existing SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DecisionRecordRepository for SqliteDecisionRecordRepository {
    /// Insert a normalized decision record and return the persisted row.
    async fn create(
        &self,
        input: CreateDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        let input = input.normalize()?;
        let planned_contribution = input
            .planned_contribution
            .map(encode_planned_contribution)
            .transpose()?;
        let row = sqlx::query(INSERT_RECORD_SQL)
            .bind(Uuid::new_v4().to_string())
            .bind(input.plan_id.to_string())
            .bind(input.symbol)
            .bind(input.currency)
            .bind(status_to_str(input.execution_status))
            .bind(planned_contribution)
            .bind(input.execution_snapshot.to_string())
            .bind(input.fundamental_snapshot.to_string())
            .bind(input.trend_snapshot.to_string())
            .bind(
                input
                    .sentiment_snapshot
                    .map(|snapshot| snapshot.to_string()),
            )
            .bind(input.decision_snapshot.to_string())
            .bind(
                input
                    .broker_order_request
                    .map(|snapshot| snapshot.to_string()),
            )
            .bind(input.broker_order_ack.map(|snapshot| snapshot.to_string()))
            .bind(input.summary)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        record_from_row(row)
    }

    /// List decision records for one plan with newest records first.
    async fn list_by_plan(
        &self,
        plan_id: Uuid,
        query: DecisionRecordListQuery,
    ) -> Result<Vec<DecisionRecord>, DecisionRecordRepositoryError> {
        let rows = sqlx::query(LIST_RECORDS_BY_PLAN_SQL)
            .bind(plan_id.to_string())
            .bind(i64::from(query.limit()))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        rows.into_iter().map(record_from_row).collect()
    }

    /// Fetch one decision record by ID.
    async fn get(&self, id: Uuid) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        let row = sqlx::query(GET_RECORD_SQL)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(DecisionRecordRepositoryError::NotFound)?;

        record_from_row(row)
    }
}

/// 将经过领域校验的金额字符串编码为 SQLite 固定精度文本。
fn encode_planned_contribution(value: String) -> Result<String, DecisionRecordRepositoryError> {
    value
        .parse::<Decimal>()
        .ok()
        .and_then(encode_amount)
        .ok_or(DecisionRecordRepositoryError::Unavailable)
}

/// 将 SQLite 查询结果转换为已验证的 decision record。
fn record_from_row(row: SqliteRow) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
    Ok(DecisionRecord {
        id: parse_uuid(column(&row, "id")?)?,
        plan_id: parse_uuid(column(&row, "plan_id")?)?,
        symbol: column(&row, "symbol")?,
        currency: column(&row, "currency")?,
        execution_status: status_from_str(column(&row, "execution_status")?)?,
        planned_contribution: column::<Option<String>>(&row, "planned_contribution")?
            .map(decode_planned_contribution)
            .transpose()?,
        execution_snapshot: parse_json(column(&row, "execution_snapshot")?)?,
        fundamental_snapshot: parse_json(column(&row, "fundamental_snapshot")?)?,
        trend_snapshot: parse_json(column(&row, "trend_snapshot")?)?,
        sentiment_snapshot: parse_optional_json(column(&row, "sentiment_snapshot")?)?,
        decision_snapshot: parse_json(column(&row, "decision_snapshot")?)?,
        broker_order_request: parse_optional_json(column(&row, "broker_order_request")?)?,
        broker_order_ack: parse_optional_json(column(&row, "broker_order_ack")?)?,
        summary: column(&row, "summary")?,
        created_at: parse_timestamp(column(&row, "created_at")?)?,
    })
}

/// 从 SQLite row 读取一个列，并将底层错误映射为安全 repository 错误。
fn column<'row, T>(row: &'row SqliteRow, name: &str) -> Result<T, DecisionRecordRepositoryError>
where
    T: sqlx::Decode<'row, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
{
    row.try_get(name).map_err(map_sqlx_error)
}

/// 将 SQLite 固定精度金额文本还原为对 API 安全的字符串。
fn decode_planned_contribution(value: String) -> Result<String, DecisionRecordRepositoryError> {
    decode_amount(&value)
        .map(|amount| amount.to_string())
        .ok_or(DecisionRecordRepositoryError::Unavailable)
}

/// 解析数据库存储的 UUID 文本。
fn parse_uuid(value: String) -> Result<Uuid, DecisionRecordRepositoryError> {
    value
        .parse()
        .map_err(|_| DecisionRecordRepositoryError::Unavailable)
}

/// 解析 schema 强制的 UTC RFC 3339 时间文本。
fn parse_timestamp(value: String) -> Result<OffsetDateTime, DecisionRecordRepositoryError> {
    if !value.ends_with('Z') {
        return Err(DecisionRecordRepositoryError::Unavailable);
    }

    OffsetDateTime::parse(&value, &Rfc3339).map_err(|_| DecisionRecordRepositoryError::Unavailable)
}

/// 解析必填 JSON 快照。
fn parse_json(value: String) -> Result<Value, DecisionRecordRepositoryError> {
    let snapshot = serde_json::from_str::<Value>(&value)
        .map_err(|_| DecisionRecordRepositoryError::Unavailable)?;
    if snapshot.is_null() {
        Err(DecisionRecordRepositoryError::Unavailable)
    } else {
        Ok(snapshot)
    }
}

/// 解析可选 JSON 快照。
fn parse_optional_json(
    value: Option<String>,
) -> Result<Option<Value>, DecisionRecordRepositoryError> {
    value.map(parse_json).transpose()
}

/// 将数据库状态文本还原为领域执行状态。
fn status_from_str(
    value: String,
) -> Result<DecisionExecutionStatus, DecisionRecordRepositoryError> {
    match value.as_str() {
        "due" => Ok(DecisionExecutionStatus::Due),
        "waiting" => Ok(DecisionExecutionStatus::Waiting),
        "inactive" => Ok(DecisionExecutionStatus::Inactive),
        _ => Err(DecisionRecordRepositoryError::Unavailable),
    }
}

/// 将领域执行状态编码为 schema 允许的文本。
fn status_to_str(status: DecisionExecutionStatus) -> &'static str {
    match status {
        DecisionExecutionStatus::Due => "due",
        DecisionExecutionStatus::Waiting => "waiting",
        DecisionExecutionStatus::Inactive => "inactive",
    }
}

/// 将底层 SQLite 错误映射为安全的 repository 错误。
fn map_sqlx_error(error: sqlx::Error) -> DecisionRecordRepositoryError {
    match error {
        sqlx::Error::RowNotFound => DecisionRecordRepositoryError::NotFound,
        other => {
            tracing::warn!(error = %other, "decision record SQLite operation failed");
            DecisionRecordRepositoryError::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use investment_plans::{CreateInvestmentPlan, InvestmentPlanRepository, ScheduleKind};
    use serde_json::json;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::{SqliteInvestmentPlanRepository, SqliteStorage};

    /// 创建已执行 migration 的隔离 SQLite repositories。
    async fn repositories() -> (
        SqliteDecisionRecordRepository,
        SqliteInvestmentPlanRepository,
    ) {
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

        (
            SqliteDecisionRecordRepository::new(storage.pool().clone()),
            SqliteInvestmentPlanRepository::new(storage.pool().clone()),
        )
    }

    /// 创建能通过 decision record 外键约束的测试计划。
    async fn create_plan(repository: &SqliteInvestmentPlanRepository) -> Uuid {
        repository
            .create(CreateInvestmentPlan {
                name: "Core plan".to_owned(),
                symbol: "VOO".to_owned(),
                base_contribution: "1000.00".parse().unwrap(),
                currency: "USD".to_owned(),
                schedule_kind: ScheduleKind::Monthly,
                schedule_day: 15,
                max_single_execution: "1500.00".parse().unwrap(),
            })
            .await
            .expect("test plan must persist")
            .id
    }

    /// 创建合法、包含完整审计快照的 decision record 输入。
    fn input(plan_id: Uuid) -> CreateDecisionRecord {
        CreateDecisionRecord {
            plan_id,
            symbol: " voo ".to_owned(),
            currency: " usd ".to_owned(),
            execution_status: DecisionExecutionStatus::Due,
            planned_contribution: Some("1000.000000000".to_owned()),
            execution_snapshot: json!({"day_of_month": 15}),
            fundamental_snapshot: json!({"score": 0.4}),
            trend_snapshot: json!({"score": 0.5}),
            sentiment_snapshot: Some(json!({"score": 0.1})),
            decision_snapshot: json!({"action": "standard"}),
            broker_order_request: Some(json!({"symbol": "VOO"})),
            broker_order_ack: Some(json!({"order_id": "paper-1"})),
            summary: "  执行本次计划投入。  ".to_owned(),
        }
    }

    /// 验证 SQLite adapter 创建、查询并还原完整审计快照。
    #[tokio::test]
    async fn creates_lists_and_gets_decision_records() {
        let (repository, plans) = repositories().await;
        let plan_id = create_plan(&plans).await;
        let created = repository.create(input(plan_id)).await.unwrap();

        assert_eq!(created.plan_id, plan_id);
        assert_eq!(created.symbol, "VOO");
        assert_eq!(created.currency, "USD");
        assert_eq!(
            created.planned_contribution.as_deref(),
            Some("1000.00000000")
        );
        assert_eq!(created.summary, "执行本次计划投入。");
        assert_eq!(
            repository
                .list_by_plan(plan_id, DecisionRecordListQuery::new(10).unwrap())
                .await
                .unwrap(),
            vec![created.clone()]
        );
        assert_eq!(repository.get(created.id).await.unwrap(), created);
    }

    /// 验证 SQLite adapter 拒绝超出 schema 金额精度的输入与不存在的记录。
    #[tokio::test]
    async fn rejects_unrepresentable_amounts_and_reports_missing_records() {
        let (repository, plans) = repositories().await;
        let plan_id = create_plan(&plans).await;
        assert_eq!(
            repository
                .create(CreateDecisionRecord {
                    planned_contribution: Some("1.000000001".to_owned()),
                    ..input(plan_id)
                })
                .await,
            Err(DecisionRecordRepositoryError::Unavailable)
        );
        assert_eq!(
            repository.get(Uuid::new_v4()).await,
            Err(DecisionRecordRepositoryError::NotFound)
        );
    }

    /// 验证 history 查询在 SQLite 中执行调用方提供的上限。
    #[tokio::test]
    async fn list_query_applies_limit_with_stable_database_order() {
        let (repository, plans) = repositories().await;
        let plan_id = create_plan(&plans).await;
        repository.create(input(plan_id)).await.unwrap();
        repository.create(input(plan_id)).await.unwrap();

        let all = repository
            .list_by_plan(plan_id, DecisionRecordListQuery::new(2).unwrap())
            .await
            .unwrap();
        let limited = repository
            .list_by_plan(plan_id, DecisionRecordListQuery::new(1).unwrap())
            .await
            .unwrap();

        assert_eq!(all.len(), 2);
        assert_eq!(limited, vec![all[0].clone()]);
    }

    /// 验证 SQLite 错误、损坏快照和未知状态均映射为安全错误。
    #[test]
    fn maps_storage_failures_to_safe_repository_errors() {
        assert_eq!(
            map_sqlx_error(sqlx::Error::RowNotFound),
            DecisionRecordRepositoryError::NotFound
        );
        assert_eq!(
            map_sqlx_error(sqlx::Error::PoolClosed),
            DecisionRecordRepositoryError::Unavailable
        );
        assert_eq!(
            decode_planned_contribution("1000.00".to_owned()),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
        assert_eq!(
            parse_timestamp("2026-01-01T00:00:00+08:00".to_owned()),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
        assert_eq!(
            parse_json("not-json".to_owned()),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
        assert_eq!(
            parse_json("null".to_owned()),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
        assert_eq!(
            status_from_str("paused".to_owned()),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
    }

    /// 验证查询保持静态、SQLite 兼容且 history 查询有上限。
    #[test]
    fn query_strings_are_static_and_bounded() {
        for query in [INSERT_RECORD_SQL, LIST_RECORDS_BY_PLAN_SQL, GET_RECORD_SQL] {
            assert!(!query.contains('$'));
        }
        assert!(LIST_RECORDS_BY_PLAN_SQL.contains("LIMIT ?2"));
        assert!(GET_RECORD_SQL.contains("WHERE id = ?1"));
    }

    /// 验证计划外键约束没有被 repository 绕过。
    #[tokio::test]
    async fn nonexistent_plan_is_not_persisted() {
        let (repository, _) = repositories().await;

        assert_eq!(
            repository.create(input(Uuid::new_v4())).await,
            Err(DecisionRecordRepositoryError::Unavailable)
        );
    }
}

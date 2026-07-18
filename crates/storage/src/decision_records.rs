//! PostgreSQL adapter for decision record repository port.

use async_trait::async_trait;
use decision_records::{
    CompleteDecisionRecord, CreateDecisionRecord, DecisionExecutionStatus, DecisionRecord,
    DecisionRecordListQuery, DecisionRecordRepository, DecisionRecordRepositoryError,
};
use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

macro_rules! record_columns {
    () => {
        "id::text AS id, plan_id::text AS plan_id, symbol, currency, \
         execution_status, planned_contribution, execution_snapshot::text AS execution_snapshot, \
         fundamental_snapshot::text AS fundamental_snapshot, trend_snapshot::text AS trend_snapshot, \
         sentiment_snapshot::text AS sentiment_snapshot, decision_snapshot::text AS decision_snapshot, \
         broker_order_request::text AS broker_order_request, broker_order_ack::text AS broker_order_ack, \
         summary, (EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS created_at_micros"
    };
}

const INSERT_RECORD_SQL: &str = concat!(
    "INSERT INTO decision_records \
     (plan_id, symbol, currency, execution_status, planned_contribution, \
      execution_snapshot, fundamental_snapshot, trend_snapshot, sentiment_snapshot, \
      decision_snapshot, broker_order_request, broker_order_ack, summary) \
     VALUES ($1::uuid, $2, $3, $4, $5, $6::jsonb, $7::jsonb, $8::jsonb, $9::jsonb, \
      $10::jsonb, $11::jsonb, $12::jsonb, $13) \
     RETURNING ",
    record_columns!()
);

const LIST_RECORDS_BY_PLAN_SQL: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM decision_records \
      WHERE plan_id = $1::uuid ORDER BY created_at DESC, id DESC LIMIT $2"
);

const GET_RECORD_SQL: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM decision_records WHERE id = $1::uuid"
);

const COMPLETE_BROKER_ORDER_SQL: &str = concat!(
    "UPDATE decision_records SET broker_order_ack = $1::jsonb, summary = $2 ",
    "WHERE id = $3::uuid RETURNING ",
    record_columns!()
);

/// PostgreSQL implementation of [`DecisionRecordRepository`].
#[derive(Clone, Debug)]
pub struct PostgresDecisionRecordRepository {
    pool: PgPool,
}

impl PostgresDecisionRecordRepository {
    /// Build a repository from an existing PostgreSQL pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DecisionRecordRepository for PostgresDecisionRecordRepository {
    /// Insert a normalized decision record and return the persisted row.
    async fn create(
        &self,
        input: CreateDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        let input = input.normalize()?;
        let row = sqlx::query(INSERT_RECORD_SQL)
            .bind(input.plan_id.to_string())
            .bind(input.symbol)
            .bind(input.currency)
            .bind(status_to_str(input.execution_status))
            .bind(input.planned_contribution)
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

    /// Attach a broker acknowledgement to an existing decision record.
    async fn complete_broker_order(
        &self,
        id: Uuid,
        input: CompleteDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        let input = input.normalize()?;
        let row = sqlx::query(COMPLETE_BROKER_ORDER_SQL)
            .bind(input.broker_order_ack.to_string())
            .bind(input.summary)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(DecisionRecordRepositoryError::NotFound)?;

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

fn record_from_row(row: PgRow) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
    Ok(DecisionRecord {
        id: parse_uuid(row.try_get("id").map_err(map_sqlx_error)?)?,
        plan_id: parse_uuid(row.try_get("plan_id").map_err(map_sqlx_error)?)?,
        symbol: row.try_get("symbol").map_err(map_sqlx_error)?,
        currency: row.try_get("currency").map_err(map_sqlx_error)?,
        execution_status: status_from_str(
            row.try_get("execution_status").map_err(map_sqlx_error)?,
        )?,
        planned_contribution: row
            .try_get("planned_contribution")
            .map_err(map_sqlx_error)?,
        execution_snapshot: parse_json(row.try_get("execution_snapshot").map_err(map_sqlx_error)?)?,
        fundamental_snapshot: parse_json(
            row.try_get("fundamental_snapshot")
                .map_err(map_sqlx_error)?,
        )?,
        trend_snapshot: parse_json(row.try_get("trend_snapshot").map_err(map_sqlx_error)?)?,
        sentiment_snapshot: parse_optional_json(
            row.try_get("sentiment_snapshot").map_err(map_sqlx_error)?,
        )?,
        decision_snapshot: parse_json(row.try_get("decision_snapshot").map_err(map_sqlx_error)?)?,
        broker_order_request: parse_optional_json(
            row.try_get("broker_order_request")
                .map_err(map_sqlx_error)?,
        )?,
        broker_order_ack: parse_optional_json(
            row.try_get("broker_order_ack").map_err(map_sqlx_error)?,
        )?,
        summary: row.try_get("summary").map_err(map_sqlx_error)?,
        created_at: parse_micros(row.try_get("created_at_micros").map_err(map_sqlx_error)?)?,
    })
}

fn parse_uuid(value: String) -> Result<Uuid, DecisionRecordRepositoryError> {
    value
        .parse()
        .map_err(|_| DecisionRecordRepositoryError::Unavailable)
}

fn parse_micros(value: i64) -> Result<OffsetDateTime, DecisionRecordRepositoryError> {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(value) * 1000)
        .map_err(|_| DecisionRecordRepositoryError::Unavailable)
}

fn parse_json(value: String) -> Result<Value, DecisionRecordRepositoryError> {
    serde_json::from_str(&value).map_err(|_| DecisionRecordRepositoryError::Unavailable)
}

fn parse_optional_json(
    value: Option<String>,
) -> Result<Option<Value>, DecisionRecordRepositoryError> {
    value.map(parse_json).transpose()
}

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

fn status_to_str(status: DecisionExecutionStatus) -> &'static str {
    match status {
        DecisionExecutionStatus::Due => "due",
        DecisionExecutionStatus::Waiting => "waiting",
        DecisionExecutionStatus::Inactive => "inactive",
    }
}

fn map_sqlx_error(error: sqlx::Error) -> DecisionRecordRepositoryError {
    match error {
        sqlx::Error::RowNotFound => DecisionRecordRepositoryError::NotFound,
        _ => DecisionRecordRepositoryError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_sqlx_errors_to_safe_repository_errors() {
        assert_eq!(
            map_sqlx_error(sqlx::Error::RowNotFound),
            DecisionRecordRepositoryError::NotFound
        );
        assert_eq!(
            map_sqlx_error(sqlx::Error::PoolClosed),
            DecisionRecordRepositoryError::Unavailable
        );
    }

    #[test]
    fn maps_execution_status_roundtrip() {
        for (status, stored) in [
            (DecisionExecutionStatus::Due, "due"),
            (DecisionExecutionStatus::Waiting, "waiting"),
            (DecisionExecutionStatus::Inactive, "inactive"),
        ] {
            assert_eq!(status_to_str(status), stored);
            assert_eq!(status_from_str(stored.to_owned()), Ok(status));
        }
        assert_eq!(
            status_from_str("paused".to_owned()),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
    }

    #[test]
    fn rejects_invalid_database_uuid_snapshot() {
        assert_eq!(
            parse_uuid("not-a-uuid".to_owned()),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
    }

    #[test]
    fn rejects_invalid_database_timestamp_snapshot() {
        assert_eq!(
            parse_micros(i64::MAX),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
    }

    #[test]
    fn rejects_invalid_database_json_snapshot() {
        assert_eq!(
            parse_json("not-json".to_owned()),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
        assert_eq!(
            parse_optional_json(Some("not-json".to_owned())),
            Err(DecisionRecordRepositoryError::Unavailable)
        );
    }

    #[test]
    fn query_strings_are_static_and_bounded() {
        assert!(INSERT_RECORD_SQL.contains("RETURNING id::text AS id"));
        assert!(LIST_RECORDS_BY_PLAN_SQL.contains("LIMIT $2"));
        assert!(GET_RECORD_SQL.contains("WHERE id = $1::uuid"));
    }
}

//! SQLite persistence adapter for versioned restricted DSL strategy documents.

use serde::Serialize;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use strategy_dsl::{StrategyDslDocumentError, StrategySpec, StrategySpecDocument};
use strategy_policy::PolicyRef;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const INSERT_STRATEGY_SQL: &str = concat!(
    "INSERT INTO strategy_specs (policy_id, policy_version, name, spec_json) ",
    "VALUES (?1, ?2, ?3, ?4) ",
    "RETURNING policy_id, policy_version, name, spec_json, created_at"
);
const LIST_STRATEGIES_SQL: &str = concat!(
    "SELECT policy_id, policy_version, name, spec_json, created_at FROM strategy_specs ",
    "ORDER BY created_at DESC, policy_id ASC, policy_version DESC"
);
const GET_STRATEGY_SQL: &str = concat!(
    "SELECT policy_id, policy_version, name, spec_json, created_at FROM strategy_specs ",
    "WHERE policy_id = ?1 AND policy_version = ?2"
);

/// A validated, persisted version of one restricted DSL strategy.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredStrategySpec {
    /// Immutable policy reference reconstructed from the validated document.
    pub policy: PolicyRef,
    /// User-facing normalized strategy name.
    pub name: String,
    /// Persisted restricted DSL document, safe for read-only clients.
    pub document: StrategySpecDocument,
    /// SQLite creation timestamp in UTC RFC3339 form.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// SQLite adapter for immutable versioned DSL strategy definitions.
#[derive(Clone, Debug)]
pub struct SqliteStrategySpecRepository {
    pool: SqlitePool,
}

impl SqliteStrategySpecRepository {
    /// Build the adapter from an existing SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save one already validated immutable strategy version.
    ///
    /// The database persists the canonical document, while reads reconstruct it through
    /// [`StrategySpecDocument::into_strategy_spec`] before returning it.
    pub async fn save(
        &self,
        strategy: &StrategySpec,
    ) -> Result<StoredStrategySpec, StrategySpecRepositoryError> {
        let document = StrategySpecDocument::from_strategy_spec(strategy);
        let spec_json = serde_json::to_string(&document)
            .map_err(|_| StrategySpecRepositoryError::Unavailable)?;
        let row = sqlx::query(INSERT_STRATEGY_SQL)
            .bind(strategy.policy().id().as_str())
            .bind(i64::from(strategy.policy().version().value()))
            .bind(strategy.name())
            .bind(spec_json)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        strategy_from_row(row)
    }

    /// List all immutable strategy versions, newest first.
    pub async fn list(&self) -> Result<Vec<StoredStrategySpec>, StrategySpecRepositoryError> {
        let rows = sqlx::query(LIST_STRATEGIES_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        rows.into_iter().map(strategy_from_row).collect()
    }

    /// Fetch one exact immutable policy version.
    pub async fn get(
        &self,
        policy: &PolicyRef,
    ) -> Result<StoredStrategySpec, StrategySpecRepositoryError> {
        let row = sqlx::query(GET_STRATEGY_SQL)
            .bind(policy.id().as_str())
            .bind(i64::from(policy.version().value()))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StrategySpecRepositoryError::NotFound)?;
        strategy_from_row(row)
    }
}

fn strategy_from_row(row: SqliteRow) -> Result<StoredStrategySpec, StrategySpecRepositoryError> {
    let stored_policy_id: String = row.try_get("policy_id").map_err(map_sqlx_error)?;
    let stored_policy_version: i64 = row.try_get("policy_version").map_err(map_sqlx_error)?;
    let stored_name: String = row.try_get("name").map_err(map_sqlx_error)?;
    let spec_json: String = row.try_get("spec_json").map_err(map_sqlx_error)?;
    let created_at: String = row.try_get("created_at").map_err(map_sqlx_error)?;
    let document: StrategySpecDocument =
        serde_json::from_str(&spec_json).map_err(|_| StrategySpecRepositoryError::Unavailable)?;
    let strategy = document
        .clone()
        .into_strategy_spec()
        .map_err(map_document_error)?;

    if strategy.policy().id().as_str() != stored_policy_id
        || i64::from(strategy.policy().version().value()) != stored_policy_version
        || strategy.name() != stored_name
    {
        return Err(StrategySpecRepositoryError::Unavailable);
    }

    Ok(StoredStrategySpec {
        policy: strategy.policy().clone(),
        name: strategy.name().to_owned(),
        document,
        created_at: OffsetDateTime::parse(&created_at, &Rfc3339)
            .map_err(|_| StrategySpecRepositoryError::Unavailable)?,
    })
}

fn map_document_error(_: StrategyDslDocumentError) -> StrategySpecRepositoryError {
    StrategySpecRepositoryError::Unavailable
}

fn map_sqlx_error(error: sqlx::Error) -> StrategySpecRepositoryError {
    match error {
        sqlx::Error::RowNotFound => StrategySpecRepositoryError::NotFound,
        _ => StrategySpecRepositoryError::Unavailable,
    }
}

/// Error returned by the local immutable strategy store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StrategySpecRepositoryError {
    /// The requested immutable strategy version does not exist.
    #[error("strategy version was not found")]
    NotFound,
    /// The local store is unavailable or contains invalid persisted data.
    #[error("strategy store is unavailable")]
    Unavailable,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rust_decimal::Decimal;
    use strategy_dsl::{
        ComparisonOperator, Condition, IndicatorSpec, LookbackWindow, PolicyAction, StrategyRule,
        ValueExpression,
    };
    use strategy_policy::{PolicyId, PolicyVersion};

    use super::*;
    use crate::SqliteStorage;

    fn strategy() -> StrategySpec {
        StrategySpec::new(
            PolicyRef::new(
                PolicyId::new("dsl_storage_test").unwrap(),
                PolicyVersion::new(1).unwrap(),
            ),
            "Stored RSI guard",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(IndicatorSpec::RelativeStrengthIndex(
                        LookbackWindow::new(14).unwrap(),
                    )),
                    ComparisonOperator::LessThan,
                    Decimal::new(30, 0),
                ),
                PolicyAction::set_opportunity_multiplier(core_domain::Multiplier::new_clamped(1.1)),
            )],
        )
        .unwrap()
    }

    /// Verify SQLite stores canonical DSL JSON and reconstructs it through domain validation.
    #[tokio::test]
    async fn saves_lists_and_gets_a_validated_strategy_version() {
        let storage =
            SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
                .await
                .unwrap();
        storage.migrate().await.unwrap();
        let repository = SqliteStrategySpecRepository::new(storage.pool().clone());
        let strategy = strategy();

        let saved = repository.save(&strategy).await.unwrap();
        assert_eq!(saved.policy, *strategy.policy());
        assert_eq!(saved.name, strategy.name());
        assert_eq!(repository.list().await.unwrap(), vec![saved.clone()]);
        assert_eq!(repository.get(strategy.policy()).await.unwrap(), saved);
    }

    /// Verify malformed persisted JSON never reaches a read-only caller as a strategy.
    #[tokio::test]
    async fn rejects_corrupted_persisted_strategy_documents() {
        let storage =
            SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
                .await
                .unwrap();
        storage.migrate().await.unwrap();
        let repository = SqliteStrategySpecRepository::new(storage.pool().clone());
        let strategy = strategy();
        repository.save(&strategy).await.unwrap();
        sqlx::query("UPDATE strategy_specs SET spec_json = '{\"policy_id\":\"fixed_dca\"}'")
            .execute(storage.pool())
            .await
            .unwrap();

        assert_eq!(
            repository.get(strategy.policy()).await,
            Err(StrategySpecRepositoryError::Unavailable)
        );
    }
}

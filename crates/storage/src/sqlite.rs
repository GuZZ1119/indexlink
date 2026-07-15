//! SQLite 本地存储基础设施与 migration runner。

use std::{str::FromStr, time::Duration};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

use crate::StorageError;

static SQLITE_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/sqlite");

/// SQLite 本地存储连接。
///
/// 此类型在 SQLite adapter 接入前提供连接、迁移与健康检查基础设施；它不会改变
/// 现有 PostgreSQL production wiring。
#[derive(Clone, Debug)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// 使用调用方提供的 SQLite 连接池构建存储句柄。
    #[must_use]
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 使用指定连接池参数连接本地 SQLite 数据库。
    ///
    /// 连接会自动创建尚不存在的数据库文件、启用外键约束，并设置 WAL journal 模式。
    ///
    /// # 错误
    ///
    /// 配置无效、URL 无效、连接超时或 SQLite 无法打开数据库时返回 [`StorageError`]。
    pub async fn connect_with_options(
        database_url: &str,
        max_connections: u32,
        connect_timeout: Duration,
    ) -> Result<Self, StorageError> {
        if max_connections == 0 {
            return Err(StorageError::InvalidConfiguration(
                "database max connections must be greater than zero",
            ));
        }
        if connect_timeout.is_zero() {
            return Err(StorageError::InvalidConfiguration(
                "database connect timeout must be greater than zero",
            ));
        }

        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(StorageError::InvalidDatabaseUrl)?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(connect_timeout);
        let connect = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(connect_timeout)
            .connect_with(options);

        let pool = tokio::time::timeout(connect_timeout, connect)
            .await
            .map_err(|_| StorageError::ConnectionTimeout {
                seconds: connect_timeout.as_secs(),
            })?
            .map_err(StorageError::Connection)?;

        Ok(Self { pool })
    }

    /// 执行编译进二进制的全部 SQLite migration。
    ///
    /// 调用方应在开始提供 HTTP 服务前调用此方法；失败时返回
    /// [`StorageError::Migration`]，以阻止服务运行在不完整 schema 上。
    pub async fn migrate(&self) -> Result<(), StorageError> {
        SQLITE_MIGRATOR
            .run(&self.pool)
            .await
            .map_err(StorageError::Migration)
    }

    /// 检查 SQLite 是否可响应查询。
    ///
    /// # 错误
    ///
    /// 数据库不可用时返回 [`StorageError::Ping`]。
    pub async fn ping(&self) -> Result<(), StorageError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(StorageError::Ping)?;
        Ok(())
    }

    /// 返回底层 SQLite 连接池。
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;

    /// 验证 migration 可在隔离的本地 SQLite 数据库中创建 MVP schema。
    #[tokio::test]
    async fn migration_creates_mvp_schema_in_memory() {
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
        let tables = sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
        )
        .fetch_all(storage.pool())
        .await
        .expect("SQLite schema tables must be queryable");

        assert_eq!(
            tables,
            vec![
                "_sqlx_migrations".to_owned(),
                "decision_records".to_owned(),
                "investment_plans".to_owned(),
            ]
        );
    }

    /// 验证 SQLite 连接配置在接触数据库前拒绝零连接数。
    #[tokio::test]
    async fn connect_rejects_zero_max_connections() {
        let error =
            SqliteStorage::connect_with_options("sqlite::memory:", 0, Duration::from_secs(1))
                .await
                .expect_err("zero max connections must fail");

        assert!(matches!(error, StorageError::InvalidConfiguration(_)));
    }

    /// 验证 SQLite schema 拒绝非规范金额、错误金额关系和非 UTC 时间文本。
    #[tokio::test]
    async fn schema_enforces_amount_and_timestamp_invariants() {
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

        let invalid_amount = sqlx::query(
            "INSERT INTO investment_plans \
             (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution) \
             VALUES ('plan-1', 'Core plan', 'VOO', '1000.00', 'USD', 15, '000000001000.00000000')",
        )
        .execute(storage.pool())
        .await;
        assert!(invalid_amount.is_err());

        let invalid_relationship = sqlx::query(
            "INSERT INTO investment_plans \
             (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution) \
             VALUES ('plan-2', 'Core plan', 'VOO', '000000001000.00000000', 'USD', 15, '000000000900.00000000')",
        )
        .execute(storage.pool())
        .await;
        assert!(invalid_relationship.is_err());

        let invalid_timestamp = sqlx::query(
            "INSERT INTO investment_plans \
             (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution, created_at, updated_at) \
             VALUES ('plan-3', 'Core plan', 'VOO', '000000001000.00000000', 'USD', 15, '000000001000.00000000', '2026-07-15T01:02:03.456+00:00', '2026-07-15T01:02:03.456+00:00')",
        )
        .execute(storage.pool())
        .await;
        assert!(invalid_timestamp.is_err());
    }
}

#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! PostgreSQL 连接与 repository adapter 基础设施。
//!
//! 此 crate 负责连接池的建立、存活检查，以及面向领域 crate 的 outbound adapter。

mod investment_plans;

use std::{str::FromStr, time::Duration};

use sqlx::{postgres::PgPoolOptions, PgPool};

/// Investment Plan repository 的 PostgreSQL adapter。
pub use investment_plans::PostgresInvestmentPlanRepository;

const DEFAULT_MAX_CONNECTIONS: u32 = 10;
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// PostgreSQL 存储连接。
#[derive(Clone, Debug)]
pub struct Storage {
    pool: PgPool,
}

impl Storage {
    /// 使用调用方提供的 PostgreSQL 连接池构建存储句柄。
    ///
    /// 适用于由 composition root 统一配置连接池，或使用 lazy pool 进行隔离测试的场景。
    #[must_use]
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 使用默认连接池参数连接 PostgreSQL。
    ///
    /// # 错误
    ///
    /// URL 无效、连接超时或 PostgreSQL 拒绝连接时返回 [`StorageError`]。
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        Self::connect_with_options(
            database_url,
            DEFAULT_MAX_CONNECTIONS,
            DEFAULT_CONNECT_TIMEOUT,
        )
        .await
    }

    /// 使用指定连接池参数连接 PostgreSQL。
    ///
    /// # 错误
    ///
    /// 配置无效、URL 无效、连接超时或 PostgreSQL 拒绝连接时返回 [`StorageError`]。
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

        let options = sqlx::postgres::PgConnectOptions::from_str(database_url)
            .map_err(StorageError::InvalidDatabaseUrl)?;
        let connect = PgPoolOptions::new()
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

    /// 检查 PostgreSQL 是否可响应查询。
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

    /// 返回底层 PostgreSQL 连接池。
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// 存储基础设施错误。
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// 连接池配置无效。
    #[error("invalid storage configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// 数据库 URL 格式无效。
    #[error("database URL is invalid")]
    InvalidDatabaseUrl(#[source] sqlx::Error),
    /// 在配置的时限内未建立连接。
    #[error("database connection timed out after {seconds} seconds")]
    ConnectionTimeout {
        /// 超时秒数。
        seconds: u64,
    },
    /// 建立数据库连接失败。
    #[error("failed to connect to database")]
    Connection(#[source] sqlx::Error),
    /// 数据库存活检查失败。
    #[error("database ping failed")]
    Ping(#[source] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET_DATABASE_URL: &str = "not a postgres URL containing user:secret-password";

    #[tokio::test]
    async fn rejects_zero_max_connections_without_contacting_database() {
        let error = Storage::connect_with_options(
            "postgres://unused:unused@localhost/unused",
            0,
            Duration::from_secs(1),
        )
        .await
        .expect_err("zero max connections must be rejected");

        assert!(matches!(error, StorageError::InvalidConfiguration(_)));
        assert!(!error.to_string().contains("unused"));
    }

    #[tokio::test]
    async fn rejects_zero_timeout_without_contacting_database() {
        let error = Storage::connect_with_options(
            "postgres://unused:unused@localhost/unused",
            1,
            Duration::ZERO,
        )
        .await
        .expect_err("zero timeout must be rejected");

        assert!(matches!(error, StorageError::InvalidConfiguration(_)));
        assert!(!error.to_string().contains("unused"));
    }

    #[tokio::test]
    async fn connect_rejects_invalid_url_without_exposing_input() {
        let error = Storage::connect(SECRET_DATABASE_URL)
            .await
            .expect_err("invalid URL must be rejected before connecting");

        assert!(matches!(error, StorageError::InvalidDatabaseUrl(_)));
        let display = error.to_string();
        assert_eq!(display, "database URL is invalid");
        assert!(!display.contains("secret-password"));
        assert!(!display.contains(SECRET_DATABASE_URL));
    }

    #[tokio::test]
    async fn pool_returns_lazy_pool_without_contacting_database() {
        let pool = PgPoolOptions::new().connect_lazy_with(
            sqlx::postgres::PgConnectOptions::new()
                .host("localhost")
                .database("indexlink-test"),
        );
        let storage = Storage::from_pool(pool);

        assert!(!storage.pool().is_closed());
        assert_eq!(storage.pool().size(), 0);
    }

    #[tokio::test]
    async fn ping_maps_closed_pool_to_safe_ping_error() {
        let pool = PgPoolOptions::new().connect_lazy_with(
            sqlx::postgres::PgConnectOptions::new()
                .host("localhost")
                .database("indexlink-test"),
        );
        let storage = Storage::from_pool(pool);
        storage.pool().close().await;

        let error = storage
            .ping()
            .await
            .expect_err("closed pool must fail readiness ping");
        assert!(matches!(error, StorageError::Ping(sqlx::Error::PoolClosed)));
        assert_eq!(error.to_string(), "database ping failed");
    }

    #[test]
    fn storage_error_display_is_stable_and_safe() {
        let cases = [
            (
                StorageError::InvalidConfiguration(
                    "database max connections must be greater than zero",
                ),
                "invalid storage configuration: database max connections must be greater than zero",
            ),
            (
                StorageError::ConnectionTimeout { seconds: 5 },
                "database connection timed out after 5 seconds",
            ),
            (
                StorageError::Connection(sqlx::Error::PoolClosed),
                "failed to connect to database",
            ),
            (
                StorageError::Ping(sqlx::Error::PoolClosed),
                "database ping failed",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
            assert!(!error.to_string().contains("postgres://"));
            assert!(!error.to_string().contains("password"));
        }
    }
}

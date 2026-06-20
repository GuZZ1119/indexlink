use std::{fmt, sync::Arc};

use async_trait::async_trait;
use indexlink_storage::Storage;

enum ReadinessBackend {
    Storage(Storage),
    Custom(Arc<dyn ReadinessCheck>),
}

impl fmt::Debug for ReadinessBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(_) => formatter.write_str("Storage"),
            Self::Custom(_) => formatter.write_str("CustomReadinessCheck"),
        }
    }
}

/// HTTP API 的共享应用状态。
#[derive(Clone, Debug)]
pub struct ApiState {
    readiness: Arc<ReadinessBackend>,
    version: Arc<str>,
}

impl ApiState {
    /// 使用生产 PostgreSQL 存储构建应用状态。
    #[must_use]
    pub fn new(storage: Storage, version: impl Into<Arc<str>>) -> Self {
        Self {
            readiness: Arc::new(ReadinessBackend::Storage(storage)),
            version: version.into(),
        }
    }

    /// 使用可替换的 readiness 检查构建状态，供隔离测试和受控适配器使用。
    #[must_use]
    pub fn with_readiness(
        readiness: Arc<dyn ReadinessCheck>,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            readiness: Arc::new(ReadinessBackend::Custom(readiness)),
            version: version.into(),
        }
    }

    pub(crate) async fn check_readiness(&self) -> Result<(), ReadinessError> {
        match self.readiness.as_ref() {
            ReadinessBackend::Storage(storage) => storage
                .ping()
                .await
                .map_err(|error| ReadinessError::new(error.to_string())),
            ReadinessBackend::Custom(check) => check.check().await,
        }
    }

    pub(crate) fn version(&self) -> &str {
        self.version.as_ref()
    }
}

/// 可替换的服务就绪检查。
#[async_trait]
pub trait ReadinessCheck: Send + Sync {
    /// 检查依赖是否可用。
    async fn check(&self) -> Result<(), ReadinessError>;
}

/// readiness 检查的内部错误。
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ReadinessError {
    message: String,
}

impl ReadinessError {
    /// 创建内部 readiness 错误。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;

    struct SecretChecker {
        secret: &'static str,
    }

    #[async_trait]
    impl ReadinessCheck for SecretChecker {
        async fn check(&self) -> Result<(), ReadinessError> {
            Err(ReadinessError::new(self.secret))
        }
    }

    #[test]
    fn readiness_error_display_preserves_internal_diagnostic_for_logs() {
        let error = ReadinessError::new("database connection refused");

        assert_eq!(error.to_string(), "database connection refused");
    }

    #[test]
    fn custom_backend_debug_hides_checker_fields() {
        let state = ApiState::with_readiness(
            Arc::new(SecretChecker {
                secret: "private-checker-detail",
            }),
            "0.1.0",
        );
        let debug = format!("{state:?}");

        assert!(debug.contains("CustomReadinessCheck"));
        assert!(!debug.contains("private-checker-detail"));
        assert!(!debug.contains("secret"));
    }

    #[tokio::test]
    async fn storage_backend_debug_and_error_hide_pool_details() {
        let pool = PgPoolOptions::new().connect_lazy_with(
            PgConnectOptions::new()
                .host("secret-database.internal")
                .username("secret-user")
                .password("secret-password")
                .database("secret-database"),
        );
        pool.close().await;
        let state = ApiState::new(Storage::from_pool(pool), "0.1.0");
        let debug = format!("{state:?}");

        assert!(debug.contains("Storage"));
        assert!(!debug.contains("secret-database"));
        assert!(!debug.contains("secret-user"));
        assert!(!debug.contains("secret-password"));

        let error = state
            .check_readiness()
            .await
            .expect_err("closed pool must fail readiness");
        assert_eq!(error.to_string(), "database ping failed");
        assert!(!error.to_string().contains("secret"));
    }
}

use std::{fmt, sync::Arc};

use ai_client::{fetch_market_sentiment, AiProvider, NewsSource, Sentiment};
use async_trait::async_trait;
use broker::{BrokerClient, MockBroker};
use decision_records::{
    DecisionRecord, DecisionRecordListQuery, DecisionRecordRepository,
    DecisionRecordRepositoryError, DecisionRecordService,
};
use indexlink_storage::{
    SqliteDecisionRecordRepository, SqliteInvestmentPlanRepository, SqliteStorage,
};
use investment_plans::InvestmentPlanService;

use crate::ApiError;

enum ReadinessBackend {
    SqliteStorage(SqliteStorage),
    Custom(Arc<dyn ReadinessCheck>),
}

struct MarketSentimentDependencies {
    news_source: Arc<dyn NewsSource>,
    provider: Arc<dyn AiProvider>,
}

impl fmt::Debug for MarketSentimentDependencies {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MarketSentimentDependencies")
    }
}

impl fmt::Debug for ReadinessBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SqliteStorage(_) => formatter.write_str("SqliteStorage"),
            Self::Custom(_) => formatter.write_str("CustomReadinessCheck"),
        }
    }
}

/// HTTP API 的共享应用状态。
#[derive(Clone)]
pub struct ApiState {
    readiness: Arc<ReadinessBackend>,
    plans: InvestmentPlanService,
    decision_records: DecisionRecordService,
    broker: Arc<dyn BrokerClient>,
    market_sentiment: Option<Arc<MarketSentimentDependencies>>,
    version: Arc<str>,
}

impl fmt::Debug for ApiState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiState")
            .field("readiness", &self.readiness)
            .field("plans", &"InvestmentPlanService")
            .field("decision_records", &"DecisionRecordService")
            .field("broker", &"BrokerClient")
            .field("market_sentiment", &self.market_sentiment)
            .field("version", &self.version)
            .finish()
    }
}

impl ApiState {
    /// 使用生产 SQLite 本地存储构建应用状态。
    #[must_use]
    pub fn new(storage: SqliteStorage, version: impl Into<Arc<str>>) -> Self {
        let plans = InvestmentPlanService::new(Arc::new(SqliteInvestmentPlanRepository::new(
            storage.pool().clone(),
        )));
        let decision_records = DecisionRecordService::new(Arc::new(
            SqliteDecisionRecordRepository::new(storage.pool().clone()),
        ));
        Self {
            readiness: Arc::new(ReadinessBackend::SqliteStorage(storage)),
            plans,
            decision_records,
            broker: Arc::new(MockBroker::paper_only()),
            market_sentiment: None,
            version: version.into(),
        }
    }

    /// 使用可替换的 readiness 检查构建状态，供隔离测试和受控适配器使用。
    #[must_use]
    pub fn with_readiness(
        readiness: Arc<dyn ReadinessCheck>,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self::with_readiness_and_plans(
            readiness,
            InvestmentPlanService::new(Arc::new(UnavailableInvestmentPlans)),
            version,
        )
    }

    /// 使用可替换的 readiness 与 investment plan service 构建状态。
    #[must_use]
    pub fn with_readiness_and_plans(
        readiness: Arc<dyn ReadinessCheck>,
        plans: InvestmentPlanService,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self::with_readiness_plans_and_broker(
            readiness,
            plans,
            Arc::new(MockBroker::paper_only()),
            version,
        )
    }

    /// 使用可替换的 readiness、investment plan service 与 broker 构建状态。
    #[must_use]
    pub fn with_readiness_plans_and_broker(
        readiness: Arc<dyn ReadinessCheck>,
        plans: InvestmentPlanService,
        broker: Arc<dyn BrokerClient>,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self::with_readiness_plans_broker_and_decision_records(
            readiness,
            plans,
            broker,
            DecisionRecordService::new(Arc::new(UnavailableDecisionRecords)),
            version,
        )
    }

    /// 使用可替换的 readiness、计划、broker 与 decision record 服务构建状态。
    #[must_use]
    pub fn with_readiness_plans_broker_and_decision_records(
        readiness: Arc<dyn ReadinessCheck>,
        plans: InvestmentPlanService,
        broker: Arc<dyn BrokerClient>,
        decision_records: DecisionRecordService,
        version: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            readiness: Arc::new(ReadinessBackend::Custom(readiness)),
            plans,
            decision_records,
            broker,
            market_sentiment: None,
            version: version.into(),
        }
    }

    /// 注入市场新闻源与 AI provider，启用真实市场情绪预览。
    ///
    /// provider 的凭据必须只由 server 配置层持有，不能进入 HTTP 请求、响应或审计快照。
    #[must_use]
    pub fn with_market_sentiment(
        mut self,
        news_source: Arc<dyn NewsSource>,
        provider: Arc<dyn AiProvider>,
    ) -> Self {
        self.market_sentiment = Some(Arc::new(MarketSentimentDependencies {
            news_source,
            provider,
        }));
        self
    }

    /// 检查 API 依赖是否可用。
    pub(crate) async fn check_readiness(&self) -> Result<(), ReadinessError> {
        match self.readiness.as_ref() {
            ReadinessBackend::SqliteStorage(storage) => storage
                .ping()
                .await
                .map_err(|error| ReadinessError::new(error.to_string())),
            ReadinessBackend::Custom(check) => check.check().await,
        }
    }

    /// 返回运行中的服务版本。
    pub(crate) fn version(&self) -> &str {
        self.version.as_ref()
    }

    /// 返回 investment plan 应用服务。
    pub(crate) fn plans(&self) -> &InvestmentPlanService {
        &self.plans
    }

    /// 返回受配置保护的 broker port。
    pub(crate) fn broker(&self) -> &dyn BrokerClient {
        self.broker.as_ref()
    }

    /// 返回 decision record 应用服务。
    pub(crate) fn decision_records(&self) -> &DecisionRecordService {
        &self.decision_records
    }

    /// 拉取新闻并调用已配置的 AI provider 生成市场情绪。
    pub(crate) async fn market_sentiment(&self) -> Result<Sentiment, ApiError> {
        let dependencies = self
            .market_sentiment
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        fetch_market_sentiment(
            dependencies.news_source.as_ref(),
            dependencies.provider.as_ref(),
        )
        .await
        .inspect_err(|error| tracing::error!(%error, "market sentiment pipeline failed"))
        .map_err(Into::into)
    }
}

/// 可替换的服务就绪检查。
#[async_trait]
pub trait ReadinessCheck: Send + Sync {
    /// 检查依赖是否可用。
    async fn check(&self) -> Result<(), ReadinessError>;
}

/// 未配置计划存储时使用的显式不可用 repository。
struct UnavailableInvestmentPlans;

/// Fallback repository used when decision records are not configured in isolated tests.
struct UnavailableDecisionRecords;

#[async_trait]
impl investment_plans::InvestmentPlanRepository for UnavailableInvestmentPlans {
    async fn create(
        &self,
        _input: investment_plans::CreateInvestmentPlan,
    ) -> Result<investment_plans::InvestmentPlan, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }

    async fn list(
        &self,
    ) -> Result<Vec<investment_plans::InvestmentPlan>, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }

    async fn get(
        &self,
        _id: uuid::Uuid,
    ) -> Result<investment_plans::InvestmentPlan, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }

    async fn update(
        &self,
        _id: uuid::Uuid,
        _input: investment_plans::UpdateInvestmentPlan,
    ) -> Result<investment_plans::InvestmentPlan, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }

    async fn set_active(
        &self,
        _id: uuid::Uuid,
        _is_active: bool,
    ) -> Result<investment_plans::InvestmentPlan, investment_plans::PlanRepositoryError> {
        Err(investment_plans::PlanRepositoryError::Unavailable)
    }
}

#[async_trait]
impl DecisionRecordRepository for UnavailableDecisionRecords {
    /// Reject creates because no decision-record backend is configured.
    async fn create(
        &self,
        _input: decision_records::CreateDecisionRecord,
    ) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject list queries because no decision-record backend is configured.
    async fn list_by_plan(
        &self,
        _plan_id: uuid::Uuid,
        _query: DecisionRecordListQuery,
    ) -> Result<Vec<DecisionRecord>, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }

    /// Reject record lookups because no decision-record backend is configured.
    async fn get(&self, _id: uuid::Uuid) -> Result<DecisionRecord, DecisionRecordRepositoryError> {
        Err(DecisionRecordRepositoryError::Unavailable)
    }
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
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

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
    async fn sqlite_backend_debug_and_error_hide_pool_details() {
        let pool = SqlitePoolOptions::new()
            .connect_lazy_with(SqliteConnectOptions::new().filename("secret-database.sqlite"));
        pool.close().await;
        let state = ApiState::new(SqliteStorage::from_pool(pool), "0.1.0");
        let debug = format!("{state:?}");

        assert!(debug.contains("SqliteStorage"));
        assert!(!debug.contains("secret-database"));

        let error = state
            .check_readiness()
            .await
            .expect_err("closed pool must fail readiness");
        assert_eq!(error.to_string(), "database ping failed");
        assert!(!error.to_string().contains("secret"));
    }
}

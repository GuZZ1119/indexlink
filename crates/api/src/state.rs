use std::{fmt, sync::Arc};

use ai_client::{fetch_market_sentiment, AiProvider, NewsSource, Sentiment};
use async_trait::async_trait;
use broker::{
    BrokerClient, BrokerOrderAck, BrokerOrderRequest, MockBroker, PaperPortfolioSnapshot,
};
use chrono::Datelike;
use decision_records::{
    DecisionRecord, DecisionRecordListQuery, DecisionRecordRepository,
    DecisionRecordRepositoryError, DecisionRecordService,
};
use indexlink_storage::{
    PaperPerformance, PaperPerformanceError, PaperPerformancePlan, PaperPerformancePoint,
    PaperTradeMarker, SqliteDecisionRecordRepository, SqliteInvestmentPlanRepository,
    SqlitePaperPerformanceRepository, SqliteStorage,
};
use investment_plans::InvestmentPlanService;
use market_data::{MarketDataError, MarketPricePoint, MarketSignalInput, MarketSignalProvider};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::ApiError;

enum ReadinessBackend {
    SqliteStorage(SqliteStorage),
    Custom(Arc<dyn ReadinessCheck>),
}

struct MarketSentimentDependencies {
    news_source: Arc<dyn NewsSource>,
    provider: Arc<dyn AiProvider>,
}

/// One real local-paper series belonging to a configured recurring holding.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActualPerformanceSeries {
    /// Local holding identifier.
    pub plan_id: uuid::Uuid,
    /// User-facing holding name.
    pub name: String,
    /// Normalized symbol.
    pub symbol: String,
    /// Local snapshot points in chronological order.
    pub points: Vec<PaperPerformancePoint>,
}

/// Combined real local-paper trajectory across all configured recurring holdings.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActualPerformance {
    /// Shared display currency. MVP only aggregates one currency at a time.
    pub currency: String,
    /// Per-holding trajectories.
    pub series: Vec<ActualPerformanceSeries>,
    /// Sum of per-holding adaptive values at each shared observation timestamp.
    pub total_points: Vec<PaperPerformancePoint>,
}

/// Read-only price history and local trade markers for one configured holding.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct HoldingPriceHistory {
    /// Local holding identifier.
    pub plan_id: uuid::Uuid,
    /// User-facing holding name.
    pub name: String,
    /// Normalized symbol.
    pub symbol: String,
    /// Actual OpenD daily closes in the requested window.
    pub prices: Vec<MarketPricePoint>,
    /// Locally confirmed paper fills within the requested window.
    pub trades: Vec<PaperTradeMarker>,
}

/// One monthly point from the transparent historical price-only DCA replay.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct HistoricalBacktestPoint {
    /// Last available trading date in the replay month.
    pub date: String,
    /// Value of equal scheduled contributions without adaptation.
    pub plain_dca_value: f64,
    /// Value of the same schedule after the documented price-distance adjustment.
    pub adaptive_value: f64,
}

/// One-year historical comparison of plain and adaptive contribution schedules.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct HistoricalBacktest {
    /// Shared display currency. MVP only aggregates one currency at a time.
    pub currency: String,
    /// Explains the exact first-version replay boundary without presenting it as realised return.
    pub methodology: &'static str,
    /// Monthly points, oldest first.
    pub points: Vec<HistoricalBacktestPoint>,
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
    market_data: Option<Arc<dyn MarketSignalProvider>>,
    paper_performance: Option<SqlitePaperPerformanceRepository>,
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
            .field(
                "market_data",
                &self.market_data.as_ref().map(|_| "MarketSignalProvider"),
            )
            .field("version", &self.version)
            .finish()
    }
}

impl ApiState {
    /// 使用生产 SQLite 本地存储构建应用状态。
    #[must_use]
    pub fn new(storage: SqliteStorage, version: impl Into<Arc<str>>) -> Self {
        let pool = storage.pool().clone();
        let plans =
            InvestmentPlanService::new(Arc::new(SqliteInvestmentPlanRepository::new(pool.clone())));
        let decision_records =
            DecisionRecordService::new(Arc::new(SqliteDecisionRecordRepository::new(pool.clone())));
        Self {
            readiness: Arc::new(ReadinessBackend::SqliteStorage(storage)),
            plans,
            decision_records,
            broker: Arc::new(MockBroker::paper_only()),
            market_sentiment: None,
            market_data: None,
            paper_performance: Some(SqlitePaperPerformanceRepository::new(pool)),
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
            market_data: None,
            paper_performance: None,
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

    /// 注入只读市场信号 provider，启用自动数据刷新。
    ///
    /// provider 只返回可审计的指标输入，不得持有交易账户、下单权限或任何密钥快照。
    #[must_use]
    pub fn with_market_data(mut self, provider: Arc<dyn MarketSignalProvider>) -> Self {
        self.market_data = Some(provider);
        self
    }

    /// 注入受配置保护的 broker 实现，替换默认的本地 mock broker。
    ///
    /// 生产装配只能传入已验证的 paper-only adapter；凭据和账户标识不得进入
    /// HTTP 请求、响应、审计快照或日志。
    #[must_use]
    pub fn with_broker(mut self, broker: Arc<dyn BrokerClient>) -> Self {
        self.broker = broker;
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

    /// 从已配置的 paper broker 读取账户、持仓和订单快照。
    ///
    /// 读取失败仅对客户端返回统一不可用错误；OpenD 协议细节、账户标识和
    /// provider 错误文本只保留在服务端日志中。
    pub(crate) async fn paper_portfolio(&self) -> Result<PaperPortfolioSnapshot, ApiError> {
        self.broker
            .read_paper_portfolio()
            .await
            .inspect_err(|error| tracing::error!(%error, "paper portfolio refresh failed"))
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// 保存一个由用户确认的本地模拟账户起始资金基准。
    pub(crate) async fn set_paper_opening_balance(
        &self,
        plan_id: uuid::Uuid,
        amount: rust_decimal::Decimal,
        occurred_at: &str,
    ) -> Result<(), ApiError> {
        self.plans().get(plan_id).await?;
        self.paper_performance
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?
            .set_opening_balance(plan_id, amount, occurred_at)
            .await
            .map_err(|error| match error {
                PaperPerformanceError::InvalidInput => ApiError::BadRequest,
                PaperPerformanceError::Unavailable => ApiError::ServiceUnavailable,
            })
    }

    /// 刷新并返回一个计划的本地模拟账户收益与对比曲线。
    pub(crate) async fn paper_performance(
        &self,
        plan_id: uuid::Uuid,
    ) -> Result<PaperPerformance, ApiError> {
        let plan = self.plans().get(plan_id).await?;
        let portfolio = self.paper_portfolio().await?;
        self.paper_performance
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?
            .refresh(
                &PaperPerformancePlan {
                    id: plan.id,
                    symbol: plan.symbol,
                    currency: plan.currency,
                    base_contribution: plan.base_contribution,
                },
                &portfolio,
            )
            .await
            .inspect_err(|error| tracing::error!(%error, "paper performance refresh failed"))
            .map_err(|_| ApiError::ServiceUnavailable)
    }

    /// Refresh every configured holding from one read-only paper-account snapshot and return
    /// their local trajectories plus an explicitly summed total line.
    pub(crate) async fn actual_performance(&self) -> Result<ActualPerformance, ApiError> {
        let plans = self.plans().list().await?;
        let active: Vec<_> = plans.into_iter().filter(|plan| plan.is_active).collect();
        let currency = active
            .first()
            .map(|plan| plan.currency.clone())
            .unwrap_or_else(|| "USD".to_owned());
        if active.iter().any(|plan| plan.currency != currency) {
            return Err(ApiError::BadRequest);
        }
        if active.is_empty() {
            return Ok(ActualPerformance {
                currency,
                series: Vec::new(),
                total_points: Vec::new(),
            });
        }
        let portfolio = self.paper_portfolio().await?;
        let repository = self
            .paper_performance
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let mut series = Vec::with_capacity(active.len());
        for plan in active {
            repository
                .refresh(
                    &PaperPerformancePlan {
                        id: plan.id,
                        symbol: plan.symbol.clone(),
                        currency: plan.currency,
                        base_contribution: plan.base_contribution,
                    },
                    &portfolio,
                )
                .await
                .inspect_err(|error| tracing::error!(%error, "actual performance refresh failed"))
                .map_err(|_| ApiError::ServiceUnavailable)?;
            series.push(ActualPerformanceSeries {
                plan_id: plan.id,
                name: plan.name,
                symbol: plan.symbol,
                points: repository
                    .history(plan.id)
                    .await
                    .map_err(|_| ApiError::ServiceUnavailable)?,
            });
        }
        let mut daily = BTreeMap::<String, BTreeMap<uuid::Uuid, PaperPerformancePoint>>::new();
        for item in &series {
            for point in &item.points {
                let day = point
                    .observed_at
                    .get(..10)
                    .unwrap_or(&point.observed_at)
                    .to_owned();
                // A manual refresh may create several same-day snapshots.  Keep the newest
                // point per holding so the aggregate is never a sum of duplicate states.
                daily
                    .entry(day)
                    .or_default()
                    .insert(item.plan_id, point.clone());
            }
        }
        let total_points = daily
            .into_iter()
            .map(|(day, per_plan)| {
                let (adaptive_value, plain_dca_value, net_contributions) =
                    per_plan.into_values().fold(
                        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
                        |total, point| {
                            (
                                total.0 + point.adaptive_value,
                                total.1 + point.plain_dca_value,
                                total.2 + point.net_contributions,
                            )
                        },
                    );
                PaperPerformancePoint {
                    observed_at: format!("{day}T00:00:00.000Z"),
                    adaptive_value,
                    plain_dca_value,
                    net_contributions,
                }
            })
            .collect();
        Ok(ActualPerformance {
            currency,
            series,
            total_points,
        })
    }

    /// Return read-only price histories and local buy/sell markers for all active holdings.
    pub(crate) async fn holding_price_history(
        &self,
        lookback_days: i64,
    ) -> Result<Vec<HoldingPriceHistory>, ApiError> {
        let provider = self
            .market_data
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let repository = self
            .paper_performance
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(lookback_days);
        let mut output = Vec::new();
        for plan in self
            .plans()
            .list()
            .await?
            .into_iter()
            .filter(|plan| plan.is_active)
        {
            let prices = provider
                .fetch_price_history(&plan.symbol, lookback_days)
                .await
                .inspect_err(|error| tracing::error!(%error, symbol = %plan.symbol, "price history refresh failed"))
                .map_err(|_| ApiError::ServiceUnavailable)?;
            let trades = repository
                .trade_markers(plan.id)
                .await
                .map_err(|_| ApiError::ServiceUnavailable)?
                .into_iter()
                .filter(|trade| {
                    chrono::DateTime::parse_from_rfc3339(&trade.observed_at)
                        .is_ok_and(|at| at.with_timezone(&chrono::Utc) >= cutoff)
                })
                .collect();
            output.push(HoldingPriceHistory {
                plan_id: plan.id,
                name: plan.name,
                symbol: plan.symbol,
                prices,
                trades,
            });
        }
        Ok(output)
    }

    /// Simulate one historical year for all active holdings using actual OpenD prices.
    ///
    /// This first MVP replay deliberately does not invent unavailable historical AI output or
    /// macro snapshots.  It applies a bounded contribution adjustment from each symbol's
    /// real 200-day moving-average distance and compares it with the same-date plain schedule.
    pub(crate) async fn historical_backtest(&self) -> Result<HistoricalBacktest, ApiError> {
        let provider = self
            .market_data
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        let plans: Vec<_> = self
            .plans()
            .list()
            .await?
            .into_iter()
            .filter(|plan| plan.is_active)
            .collect();
        let currency = plans
            .first()
            .map(|plan| plan.currency.clone())
            .unwrap_or_else(|| "USD".to_owned());
        if plans.iter().any(|plan| plan.currency != currency) {
            return Err(ApiError::BadRequest);
        }
        let cutoff = chrono::Utc::now().date_naive() - chrono::Duration::days(366);
        let mut totals = BTreeMap::<String, (f64, f64)>::new();
        for plan in plans {
            let prices = provider
                .fetch_price_history(&plan.symbol, 365 * 3 + 1)
                .await
                .inspect_err(|error| tracing::error!(%error, symbol = %plan.symbol, "historical replay data refresh failed"))
                .map_err(|_| ApiError::ServiceUnavailable)?;
            let parsed: Vec<_> = prices
                .iter()
                .filter_map(|point| {
                    chrono::NaiveDate::parse_from_str(&point.date, "%Y-%m-%d")
                        .ok()
                        .map(|date| (date, point.close))
                })
                .collect();
            if parsed.len() < 201 {
                return Err(ApiError::ServiceUnavailable);
            }
            let mut monthly = BTreeMap::<(i32, u32), (usize, chrono::NaiveDate, f64)>::new();
            for (index, (date, close)) in parsed.iter().enumerate() {
                if *date >= cutoff && index >= 199 {
                    monthly.insert((date.year(), date.month()), (index, *date, *close));
                }
            }
            let mut plain_units = 0.0;
            let mut adaptive_units = 0.0;
            let base = plan
                .base_contribution
                .to_f64()
                .ok_or(ApiError::BadRequest)?;
            for (_, (index, date, close)) in monthly {
                let average = parsed[index + 1 - 200..=index]
                    .iter()
                    .map(|(_, value)| *value)
                    .sum::<f64>()
                    / 200.0;
                let distance = close / average - 1.0;
                let multiplier = (1.0 - distance * 2.5).clamp(0.5, 1.5);
                plain_units += base / close;
                adaptive_units += base * multiplier / close;
                let entry = totals
                    .entry(date.format("%Y-%m-%d").to_string())
                    .or_insert((0.0, 0.0));
                entry.0 += plain_units * close;
                entry.1 += adaptive_units * close;
            }
        }
        Ok(HistoricalBacktest {
            currency,
            methodology: "一年前开始的真实 OpenD 日线月度回放；普通定投每月固定投入，自适应定投仅按当月相对 MA200 距离在 0.5x–1.5x 调整。历史 AI 情绪与宏观快照未被伪造，因此这不是已实现收益，也不是完整 70/20/10 审计回放。",
            points: totals
                .into_iter()
                .map(|(date, (plain_dca_value, adaptive_value))| HistoricalBacktestPoint {
                    date,
                    plain_dca_value,
                    adaptive_value,
                })
                .collect(),
        })
    }

    /// 记录已被 broker 接受的订单意图，供后续只读对账生成本地成交账本。
    pub(crate) async fn record_accepted_paper_order(
        &self,
        plan_id: uuid::Uuid,
        acknowledgement: &BrokerOrderAck,
        request: &BrokerOrderRequest,
    ) {
        let Some(repository) = &self.paper_performance else {
            return;
        };
        if let Err(error) = repository
            .record_accepted_order(plan_id, acknowledgement, request)
            .await
        {
            tracing::error!(%error, order_id = %acknowledgement.order_id(), "accepted paper order was not added to local ledger");
        }
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

    /// 拉取一份自动市场信号输入，并在边界保留内部失败日志。
    pub(crate) async fn market_signal_input(
        &self,
        symbol: &str,
    ) -> Result<MarketSignalInput, ApiError> {
        let provider = self
            .market_data
            .as_ref()
            .ok_or(ApiError::ServiceUnavailable)?;
        provider
            .fetch(symbol)
            .await
            .inspect_err(|error| tracing::error!(%error, "market signal refresh failed"))
            .map_err(market_data_error)
    }
}

fn market_data_error(error: MarketDataError) -> ApiError {
    match error {
        MarketDataError::InvalidSymbol => ApiError::BadRequest,
        _ => ApiError::ServiceUnavailable,
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

    /// Reject broker completions because no decision-record backend is configured.
    async fn complete_broker_order(
        &self,
        _id: uuid::Uuid,
        _input: decision_records::CompleteDecisionRecord,
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
    use broker::{BrokerEnvironment, BrokerError, BrokerOrderRequest, BrokerOrderSide};
    use rust_decimal::Decimal;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;

    struct SecretChecker {
        secret: &'static str,
    }

    /// Broker double that proves composition can replace the default mock safely.
    #[derive(Debug)]
    struct UnavailableBroker;

    #[async_trait]
    impl ReadinessCheck for SecretChecker {
        async fn check(&self) -> Result<(), ReadinessError> {
            Err(ReadinessError::new(self.secret))
        }
    }

    #[async_trait]
    impl BrokerClient for UnavailableBroker {
        async fn submit_order(
            &self,
            _request: BrokerOrderRequest,
        ) -> Result<broker::BrokerOrderAck, BrokerError> {
            Err(BrokerError::Unavailable)
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

    /// Verify a configured adapter replaces the local mock at the broker port.
    #[tokio::test]
    async fn with_broker_replaces_default_mock_broker() {
        let pool =
            SqlitePoolOptions::new().connect_lazy_with(SqliteConnectOptions::new().in_memory(true));
        let state = ApiState::new(SqliteStorage::from_pool(pool), "0.1.0")
            .with_broker(Arc::new(UnavailableBroker));
        let request = BrokerOrderRequest::market(
            "configured-broker-test",
            "VOO",
            BrokerOrderSide::Buy,
            Decimal::ONE,
            BrokerEnvironment::Paper,
        )
        .expect("paper order fixture should be valid");

        assert_eq!(
            state.broker().submit_order(request).await,
            Err(BrokerError::Unavailable)
        );
    }
}

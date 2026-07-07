#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Broker boundary for demo and future real trading adapters.
//!
//! This crate defines a provider-neutral broker port plus a safe mock
//! implementation. Real Futu/Moomoo OpenD integration should implement
//! [`BrokerClient`] in an adapter crate while preserving these invariants:
//! paper trading is the default demo path, live trading must be explicitly
//! enabled, and public errors must not expose account credentials.

use std::{fmt, sync::Mutex};

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Serialize;

const MAX_SYMBOL_LEN: usize = 32;
const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;
const MAX_OPEND_HOST_LEN: usize = 253;
const MAX_OPEND_ACCOUNT_ID_LEN: usize = 128;

/// Broker execution environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerEnvironment {
    /// Paper trading or virtual-account execution.
    Paper,
    /// Real account execution; must be explicitly enabled by the adapter.
    Live,
}

/// Broker provider family backed by an OpenD gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerProvider {
    /// Futu OpenD.
    Futu,
    /// Moomoo OpenD.
    Moomoo,
}

/// Validated OpenD connection settings for paper-trading adapters.
#[derive(Clone, PartialEq, Eq)]
pub struct OpenDConnectionConfig {
    /// Target OpenD-compatible provider.
    provider: BrokerProvider,
    /// OpenD host, usually localhost or an internal gateway host.
    host: String,
    /// OpenD TCP port.
    port: u16,
    /// Configured execution environment.
    environment: BrokerEnvironment,
    /// Optional broker account identifier, redacted from debug output.
    account_id: Option<String>,
    /// Whether live trading requests are allowed by this configuration.
    live_trading_enabled: bool,
}

impl OpenDConnectionConfig {
    /// Build validated OpenD settings.
    ///
    /// Live trading stays disabled unless `live_trading_enabled` is set
    /// explicitly, even when `environment` is [`BrokerEnvironment::Live`].
    pub fn new(
        provider: BrokerProvider,
        host: impl Into<String>,
        port: u16,
        environment: BrokerEnvironment,
        account_id: Option<String>,
        live_trading_enabled: bool,
    ) -> Result<Self, BrokerValidationError> {
        let host = normalize_opend_host(host.into())?;
        validate_opend_port(port)?;
        let account_id = account_id.map(normalize_opend_account_id).transpose()?;

        Ok(Self {
            provider,
            host,
            port,
            environment,
            account_id,
            live_trading_enabled,
        })
    }

    /// Build paper-trading OpenD settings without an account identifier.
    pub fn paper(
        provider: BrokerProvider,
        host: impl Into<String>,
        port: u16,
    ) -> Result<Self, BrokerValidationError> {
        Self::new(provider, host, port, BrokerEnvironment::Paper, None, false)
    }

    /// Build paper-trading OpenD settings with a redacted account identifier.
    pub fn paper_with_account(
        provider: BrokerProvider,
        host: impl Into<String>,
        port: u16,
        account_id: impl Into<String>,
    ) -> Result<Self, BrokerValidationError> {
        Self::new(
            provider,
            host,
            port,
            BrokerEnvironment::Paper,
            Some(account_id.into()),
            false,
        )
    }

    /// Return the configured broker provider.
    #[must_use]
    pub fn provider(&self) -> BrokerProvider {
        self.provider
    }

    /// Return the OpenD host.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Return the OpenD TCP port.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Return the configured execution environment.
    #[must_use]
    pub fn environment(&self) -> BrokerEnvironment {
        self.environment
    }

    /// Return the optional broker account identifier.
    #[must_use]
    pub fn account_id(&self) -> Option<&str> {
        self.account_id.as_deref()
    }

    /// Return whether live trading requests are explicitly allowed.
    #[must_use]
    pub fn live_trading_enabled(&self) -> bool {
        self.live_trading_enabled
    }

    /// Validate that an order request matches the configured environment gate.
    pub fn validate_order_environment(
        &self,
        request: &BrokerOrderRequest,
    ) -> Result<(), BrokerError> {
        if request.environment() != self.environment {
            return Err(BrokerError::EnvironmentMismatch {
                configured: self.environment,
                requested: request.environment(),
            });
        }

        if request.environment() == BrokerEnvironment::Live && !self.live_trading_enabled {
            return Err(BrokerError::LiveTradingDisabled);
        }

        Ok(())
    }
}

impl fmt::Debug for OpenDConnectionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let account_id = self.account_id.as_ref().map(|_| "<redacted>");
        f.debug_struct("OpenDConnectionConfig")
            .field("provider", &self.provider)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("environment", &self.environment)
            .field("account_id", &account_id)
            .field("live_trading_enabled", &self.live_trading_enabled)
            .finish()
    }
}

/// Broker order side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerOrderSide {
    /// Buy the instrument.
    Buy,
    /// Sell the instrument.
    Sell,
}

/// Broker order type supported by the MVP boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerOrderType {
    /// Market order; no limit price is accepted.
    Market,
    /// Limit order; a positive limit price is required.
    Limit,
}

/// Validated broker order request.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BrokerOrderRequest {
    /// Stable idempotency key generated by the execution layer.
    idempotency_key: String,
    /// Broker symbol, normalized to uppercase ASCII.
    symbol: String,
    /// Order side.
    side: BrokerOrderSide,
    /// Order type.
    order_type: BrokerOrderType,
    /// Positive order quantity.
    #[serde(with = "rust_decimal::serde::str")]
    quantity: Decimal,
    /// Positive limit price when `order_type` is limit.
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    limit_price: Option<Decimal>,
    /// Paper or live trading environment.
    environment: BrokerEnvironment,
}

impl BrokerOrderRequest {
    /// Build a validated market order request.
    pub fn market(
        idempotency_key: impl Into<String>,
        symbol: impl Into<String>,
        side: BrokerOrderSide,
        quantity: Decimal,
        environment: BrokerEnvironment,
    ) -> Result<Self, BrokerValidationError> {
        Self::new(
            idempotency_key,
            symbol,
            side,
            BrokerOrderType::Market,
            quantity,
            None,
            environment,
        )
    }

    /// Build a validated limit order request.
    pub fn limit(
        idempotency_key: impl Into<String>,
        symbol: impl Into<String>,
        side: BrokerOrderSide,
        quantity: Decimal,
        limit_price: Decimal,
        environment: BrokerEnvironment,
    ) -> Result<Self, BrokerValidationError> {
        Self::new(
            idempotency_key,
            symbol,
            side,
            BrokerOrderType::Limit,
            quantity,
            Some(limit_price),
            environment,
        )
    }

    fn new(
        idempotency_key: impl Into<String>,
        symbol: impl Into<String>,
        side: BrokerOrderSide,
        order_type: BrokerOrderType,
        quantity: Decimal,
        limit_price: Option<Decimal>,
        environment: BrokerEnvironment,
    ) -> Result<Self, BrokerValidationError> {
        let idempotency_key = normalize_idempotency_key(idempotency_key.into())?;
        let symbol = normalize_symbol(symbol.into())?;
        validate_positive("quantity", quantity)?;

        match (order_type, limit_price) {
            (BrokerOrderType::Market, None) => {}
            (BrokerOrderType::Market, Some(_)) => {
                return Err(BrokerValidationError::UnexpectedLimitPrice)
            }
            (BrokerOrderType::Limit, Some(price)) => validate_positive("limit_price", price)?,
            (BrokerOrderType::Limit, None) => return Err(BrokerValidationError::MissingLimitPrice),
        }

        Ok(Self {
            idempotency_key,
            symbol,
            side,
            order_type,
            quantity,
            limit_price,
            environment,
        })
    }

    /// Return the stable idempotency key.
    #[must_use]
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    /// Return the normalized symbol.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Return the order side.
    #[must_use]
    pub fn side(&self) -> BrokerOrderSide {
        self.side
    }

    /// Return the order type.
    #[must_use]
    pub fn order_type(&self) -> BrokerOrderType {
        self.order_type
    }

    /// Return the positive quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// Return the optional limit price.
    #[must_use]
    pub fn limit_price(&self) -> Option<Decimal> {
        self.limit_price
    }

    /// Return the execution environment.
    #[must_use]
    pub fn environment(&self) -> BrokerEnvironment {
        self.environment
    }
}

/// Broker order response returned by adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BrokerOrderAck {
    /// Adapter-specific order ID.
    order_id: String,
    /// Accepted execution environment.
    environment: BrokerEnvironment,
    /// Initial broker-side status.
    status: BrokerOrderStatus,
}

impl BrokerOrderAck {
    /// Build a broker order acknowledgement.
    pub fn new(
        order_id: impl Into<String>,
        environment: BrokerEnvironment,
        status: BrokerOrderStatus,
    ) -> Result<Self, BrokerValidationError> {
        let order_id = normalize_idempotency_key(order_id.into())?;
        Ok(Self {
            order_id,
            environment,
            status,
        })
    }

    /// Return the adapter-specific order ID.
    #[must_use]
    pub fn order_id(&self) -> &str {
        &self.order_id
    }

    /// Return the accepted execution environment.
    #[must_use]
    pub fn environment(&self) -> BrokerEnvironment {
        self.environment
    }

    /// Return the initial broker-side status.
    #[must_use]
    pub fn status(&self) -> BrokerOrderStatus {
        self.status
    }
}

/// Initial broker order status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerOrderStatus {
    /// Adapter accepted the order.
    Accepted,
    /// Adapter deduplicated a repeated idempotency key.
    Duplicate,
}

/// Broker port implemented by mock, paper, and live adapters.
#[async_trait]
pub trait BrokerClient: Send + Sync {
    /// Submit one validated broker order.
    async fn submit_order(
        &self,
        request: BrokerOrderRequest,
    ) -> Result<BrokerOrderAck, BrokerError>;
}

/// OpenD order gateway implemented by a TCP or SDK transport.
#[async_trait]
pub trait OpenDOrderGateway: Send + Sync {
    /// Submit one paper-trading order through OpenD.
    async fn submit_paper_order(
        &self,
        config: &OpenDConnectionConfig,
        request: &BrokerOrderRequest,
    ) -> Result<BrokerOrderAck, BrokerError>;
}

/// Paper-trading broker adapter for Futu/Moomoo OpenD.
#[derive(Debug)]
pub struct OpenDPaperBroker<G> {
    config: OpenDConnectionConfig,
    gateway: G,
}

impl<G> OpenDPaperBroker<G> {
    /// Build a paper-only OpenD broker adapter.
    pub fn new(config: OpenDConnectionConfig, gateway: G) -> Result<Self, BrokerError> {
        if config.environment() != BrokerEnvironment::Paper || config.live_trading_enabled() {
            return Err(BrokerError::PaperTradingRequired {
                configured: config.environment(),
            });
        }

        Ok(Self { config, gateway })
    }

    /// Return validated OpenD connection settings.
    #[must_use]
    pub fn config(&self) -> &OpenDConnectionConfig {
        &self.config
    }

    /// Return the underlying OpenD gateway transport.
    #[must_use]
    pub fn gateway(&self) -> &G {
        &self.gateway
    }
}

#[async_trait]
impl<G> BrokerClient for OpenDPaperBroker<G>
where
    G: OpenDOrderGateway,
{
    async fn submit_order(
        &self,
        request: BrokerOrderRequest,
    ) -> Result<BrokerOrderAck, BrokerError> {
        self.config.validate_order_environment(&request)?;
        self.gateway
            .submit_paper_order(&self.config, &request)
            .await
    }
}

/// Safe broker implementation for demos and tests.
#[derive(Debug, Default)]
pub struct MockBroker {
    accepted: Mutex<Vec<BrokerOrderRequest>>,
    allow_live: bool,
}

impl MockBroker {
    /// Create a mock broker that accepts paper orders and rejects live orders.
    #[must_use]
    pub fn paper_only() -> Self {
        Self::default()
    }

    /// Create a mock broker that also accepts live-mode requests.
    #[must_use]
    pub fn allow_live() -> Self {
        Self {
            accepted: Mutex::new(Vec::new()),
            allow_live: true,
        }
    }

    /// Return a snapshot of accepted orders.
    #[must_use]
    pub fn accepted_orders(&self) -> Vec<BrokerOrderRequest> {
        self.accepted.lock().unwrap().clone()
    }
}

#[async_trait]
impl BrokerClient for MockBroker {
    async fn submit_order(
        &self,
        request: BrokerOrderRequest,
    ) -> Result<BrokerOrderAck, BrokerError> {
        if request.environment() == BrokerEnvironment::Live && !self.allow_live {
            return Err(BrokerError::LiveTradingDisabled);
        }

        let mut accepted = self.accepted.lock().unwrap();
        if accepted
            .iter()
            .any(|order| order.idempotency_key() == request.idempotency_key())
        {
            return Ok(BrokerOrderAck::new(
                format!("MOCK-{}", request.idempotency_key()),
                request.environment(),
                BrokerOrderStatus::Duplicate,
            )?);
        }

        let ack = BrokerOrderAck::new(
            format!("MOCK-{}", accepted.len() + 1),
            request.environment(),
            BrokerOrderStatus::Accepted,
        )?;
        accepted.push(request);
        Ok(ack)
    }
}

/// Broker validation error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BrokerValidationError {
    /// Idempotency key is empty or too long.
    #[error("idempotency key must be 1..=128 ASCII characters after trimming")]
    InvalidIdempotencyKey,
    /// Symbol is empty, too long, or not ASCII.
    #[error("broker symbol must be 1..=32 ASCII characters after trimming")]
    InvalidSymbol,
    /// Decimal field is not positive.
    #[error("{field} must be greater than zero")]
    NonPositiveDecimal {
        /// Field name.
        field: &'static str,
    },
    /// Limit orders require a positive limit price.
    #[error("limit order requires a limit price")]
    MissingLimitPrice,
    /// Market orders must not include a limit price.
    #[error("market order must not include a limit price")]
    UnexpectedLimitPrice,
    /// OpenD host is empty, too long, non-ASCII, or contains control characters.
    #[error("opend host must be 1..=253 printable ASCII characters after trimming")]
    InvalidOpenDHost,
    /// OpenD TCP port cannot be zero.
    #[error("opend port must be greater than zero")]
    InvalidOpenDPort,
    /// Broker account identifier is empty, too long, or unsafe to log.
    #[error("broker account id must be 1..=128 printable ASCII characters after trimming")]
    InvalidAccountId,
}

/// Broker adapter error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BrokerError {
    /// Request failed validation.
    #[error(transparent)]
    Validation(#[from] BrokerValidationError),
    /// Live trading is disabled by configuration.
    #[error("live trading is disabled")]
    LiveTradingDisabled,
    /// Order environment does not match the adapter configuration.
    #[error(
        "order environment {requested:?} does not match configured environment {configured:?}"
    )]
    EnvironmentMismatch {
        /// Configured adapter environment.
        configured: BrokerEnvironment,
        /// Requested order environment.
        requested: BrokerEnvironment,
    },
    /// OpenD paper adapter was created with a non-paper environment.
    #[error("opend paper adapter requires Paper environment, got {configured:?}")]
    PaperTradingRequired {
        /// Configured adapter environment.
        configured: BrokerEnvironment,
    },
    /// Broker adapter or gateway is unavailable.
    #[error("broker is unavailable")]
    Unavailable,
}

fn normalize_idempotency_key(value: String) -> Result<String, BrokerValidationError> {
    let normalized = value.trim().to_owned();
    if normalized.is_empty() || normalized.len() > MAX_IDEMPOTENCY_KEY_LEN || !normalized.is_ascii()
    {
        Err(BrokerValidationError::InvalidIdempotencyKey)
    } else {
        Ok(normalized)
    }
}

fn normalize_symbol(value: String) -> Result<String, BrokerValidationError> {
    let normalized = value.trim().to_owned();
    if normalized.is_empty() || normalized.len() > MAX_SYMBOL_LEN || !normalized.is_ascii() {
        Err(BrokerValidationError::InvalidSymbol)
    } else {
        Ok(normalized.to_ascii_uppercase())
    }
}

fn validate_positive(field: &'static str, value: Decimal) -> Result<(), BrokerValidationError> {
    if value > Decimal::ZERO {
        Ok(())
    } else {
        Err(BrokerValidationError::NonPositiveDecimal { field })
    }
}

fn normalize_opend_host(value: String) -> Result<String, BrokerValidationError> {
    normalize_printable_ascii(
        value,
        MAX_OPEND_HOST_LEN,
        BrokerValidationError::InvalidOpenDHost,
    )
}

fn validate_opend_port(port: u16) -> Result<(), BrokerValidationError> {
    if port == 0 {
        Err(BrokerValidationError::InvalidOpenDPort)
    } else {
        Ok(())
    }
}

fn normalize_opend_account_id(value: String) -> Result<String, BrokerValidationError> {
    normalize_printable_ascii(
        value,
        MAX_OPEND_ACCOUNT_ID_LEN,
        BrokerValidationError::InvalidAccountId,
    )
}

fn normalize_printable_ascii(
    value: String,
    max_len: usize,
    error: BrokerValidationError,
) -> Result<String, BrokerValidationError> {
    let normalized = value.trim().to_owned();
    if normalized.is_empty()
        || normalized.len() > max_len
        || !normalized.is_ascii()
        || normalized.chars().any(char::is_control)
    {
        Err(error)
    } else {
        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct FakeOpenDGateway {
        calls: Mutex<Vec<String>>,
        fail: bool,
    }

    #[async_trait]
    impl OpenDOrderGateway for FakeOpenDGateway {
        async fn submit_paper_order(
            &self,
            config: &OpenDConnectionConfig,
            request: &BrokerOrderRequest,
        ) -> Result<BrokerOrderAck, BrokerError> {
            if self.fail {
                return Err(BrokerError::Unavailable);
            }

            self.calls.lock().unwrap().push(format!(
                "{:?}:{}:{}",
                config.provider(),
                config.host(),
                request.idempotency_key()
            ));
            BrokerOrderAck::new(
                format!("OPEND-PAPER-{}", request.idempotency_key()),
                request.environment(),
                BrokerOrderStatus::Accepted,
            )
            .map_err(Into::into)
        }
    }

    impl FakeOpenDGateway {
        fn failing() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                fail: true,
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    /// Parse decimal fixtures without floating-point literals.
    fn money(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    /// Build a valid paper-market order fixture.
    fn paper_market_order(key: &str) -> BrokerOrderRequest {
        BrokerOrderRequest::market(
            key,
            " voo ",
            BrokerOrderSide::Buy,
            money("1.25"),
            BrokerEnvironment::Paper,
        )
        .unwrap()
    }

    /// Verify OpenD paper adapter submits paper orders through the gateway.
    #[tokio::test]
    async fn opend_paper_broker_submits_order_through_gateway() {
        let broker = OpenDPaperBroker::new(
            OpenDConnectionConfig::paper(BrokerProvider::Moomoo, "127.0.0.1", 11111).unwrap(),
            FakeOpenDGateway::default(),
        )
        .unwrap();
        let ack = broker
            .submit_order(paper_market_order("demo-opend-1"))
            .await
            .unwrap();

        assert_eq!(broker.config().provider(), BrokerProvider::Moomoo);
        assert_eq!(ack.order_id(), "OPEND-PAPER-demo-opend-1");
        assert_eq!(ack.environment(), BrokerEnvironment::Paper);
        assert_eq!(ack.status(), BrokerOrderStatus::Accepted);
        assert_eq!(
            broker.gateway().calls(),
            vec!["Moomoo:127.0.0.1:demo-opend-1"]
        );
    }

    /// Verify OpenD paper adapter rejects non-paper configuration.
    #[test]
    fn opend_paper_broker_rejects_live_config() {
        let config = OpenDConnectionConfig::new(
            BrokerProvider::Futu,
            "127.0.0.1",
            11111,
            BrokerEnvironment::Live,
            None,
            true,
        )
        .unwrap();

        assert_eq!(
            OpenDPaperBroker::new(config, FakeOpenDGateway::default()).map(|_| ()),
            Err(BrokerError::PaperTradingRequired {
                configured: BrokerEnvironment::Live,
            })
        );
    }

    /// Verify OpenD paper adapter rejects paper config with live gate enabled.
    #[test]
    fn opend_paper_broker_rejects_enabled_live_gate() {
        let config = OpenDConnectionConfig::new(
            BrokerProvider::Futu,
            "127.0.0.1",
            11111,
            BrokerEnvironment::Paper,
            None,
            true,
        )
        .unwrap();

        assert_eq!(
            OpenDPaperBroker::new(config, FakeOpenDGateway::default()).map(|_| ()),
            Err(BrokerError::PaperTradingRequired {
                configured: BrokerEnvironment::Paper,
            })
        );
    }

    /// Verify live orders are rejected before the OpenD gateway is called.
    #[tokio::test]
    async fn opend_paper_broker_rejects_live_order_before_gateway() {
        let broker = OpenDPaperBroker::new(
            OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", 11111).unwrap(),
            FakeOpenDGateway::default(),
        )
        .unwrap();
        let request = BrokerOrderRequest::market(
            "demo-opend-live",
            "VOO",
            BrokerOrderSide::Buy,
            money("1.00"),
            BrokerEnvironment::Live,
        )
        .unwrap();

        assert_eq!(
            broker.submit_order(request).await,
            Err(BrokerError::EnvironmentMismatch {
                configured: BrokerEnvironment::Paper,
                requested: BrokerEnvironment::Live,
            })
        );
        assert!(broker.gateway().calls().is_empty());
    }

    /// Verify gateway failures are surfaced as safe broker errors.
    #[tokio::test]
    async fn opend_paper_broker_propagates_gateway_failure() {
        let broker = OpenDPaperBroker::new(
            OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", 11111).unwrap(),
            FakeOpenDGateway::failing(),
        )
        .unwrap();

        assert_eq!(
            broker
                .submit_order(paper_market_order("demo-opend-2"))
                .await,
            Err(BrokerError::Unavailable)
        );
    }

    /// Verify OpenD paper settings normalize text and keep live trading disabled.
    #[test]
    fn opend_paper_config_normalizes_host_and_disables_live() {
        let config =
            OpenDConnectionConfig::paper(BrokerProvider::Futu, " 127.0.0.1 ", 11111).unwrap();

        assert_eq!(config.provider(), BrokerProvider::Futu);
        assert_eq!(config.host(), "127.0.0.1");
        assert_eq!(config.port(), 11111);
        assert_eq!(config.environment(), BrokerEnvironment::Paper);
        assert_eq!(config.account_id(), None);
        assert!(!config.live_trading_enabled());
    }

    /// Verify OpenD settings reject unsafe host, port, and account values.
    #[test]
    fn opend_config_rejects_invalid_connection_fields() {
        assert_eq!(
            OpenDConnectionConfig::paper(BrokerProvider::Futu, "", 11111),
            Err(BrokerValidationError::InvalidOpenDHost)
        );
        assert_eq!(
            OpenDConnectionConfig::paper(BrokerProvider::Futu, "open\nd", 11111),
            Err(BrokerValidationError::InvalidOpenDHost)
        );
        assert_eq!(
            OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", 0),
            Err(BrokerValidationError::InvalidOpenDPort)
        );
        assert_eq!(
            OpenDConnectionConfig::paper_with_account(
                BrokerProvider::Moomoo,
                "127.0.0.1",
                11111,
                " "
            ),
            Err(BrokerValidationError::InvalidAccountId)
        );
    }

    /// Verify account identifiers are available to adapters but redacted from debug logs.
    #[test]
    fn opend_config_debug_redacts_account_id() {
        let config = OpenDConnectionConfig::paper_with_account(
            BrokerProvider::Moomoo,
            "127.0.0.1",
            11111,
            "paper-account-1",
        )
        .unwrap();
        let debug = format!("{config:?}");

        assert_eq!(config.account_id(), Some("paper-account-1"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("paper-account-1"));
    }

    /// Verify OpenD settings reject requests for the wrong environment.
    #[test]
    fn opend_config_rejects_environment_mismatch() {
        let config =
            OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", 11111).unwrap();
        let live_order = BrokerOrderRequest::market(
            "demo-live",
            "VOO",
            BrokerOrderSide::Buy,
            money("1.00"),
            BrokerEnvironment::Live,
        )
        .unwrap();

        assert_eq!(
            config.validate_order_environment(&live_order),
            Err(BrokerError::EnvironmentMismatch {
                configured: BrokerEnvironment::Paper,
                requested: BrokerEnvironment::Live,
            })
        );
    }

    /// Verify live-mode OpenD settings still require the explicit live gate.
    #[test]
    fn opend_config_requires_live_gate_for_live_orders() {
        let live_order = BrokerOrderRequest::market(
            "demo-live",
            "VOO",
            BrokerOrderSide::Buy,
            money("1.00"),
            BrokerEnvironment::Live,
        )
        .unwrap();
        let disabled = OpenDConnectionConfig::new(
            BrokerProvider::Futu,
            "127.0.0.1",
            11111,
            BrokerEnvironment::Live,
            None,
            false,
        )
        .unwrap();
        let enabled = OpenDConnectionConfig::new(
            BrokerProvider::Futu,
            "127.0.0.1",
            11111,
            BrokerEnvironment::Live,
            None,
            true,
        )
        .unwrap();

        assert_eq!(
            disabled.validate_order_environment(&live_order),
            Err(BrokerError::LiveTradingDisabled)
        );
        assert_eq!(enabled.validate_order_environment(&live_order), Ok(()));
    }

    /// Verify order constructors normalize text and keep decimal precision.
    #[test]
    fn market_order_normalizes_symbol_and_idempotency_key() {
        let request = paper_market_order(" demo-1 ");

        assert_eq!(request.idempotency_key(), "demo-1");
        assert_eq!(request.symbol(), "VOO");
        assert_eq!(request.side(), BrokerOrderSide::Buy);
        assert_eq!(request.order_type(), BrokerOrderType::Market);
        assert_eq!(request.quantity(), money("1.25"));
        assert_eq!(request.limit_price(), None);
        assert_eq!(request.environment(), BrokerEnvironment::Paper);
    }

    /// Verify limit orders require an explicit positive limit price.
    #[test]
    fn limit_order_requires_positive_limit_price() {
        let request = BrokerOrderRequest::limit(
            "demo-2",
            "VOO",
            BrokerOrderSide::Buy,
            money("1.00"),
            money("400.12"),
            BrokerEnvironment::Paper,
        )
        .unwrap();

        assert_eq!(request.order_type(), BrokerOrderType::Limit);
        assert_eq!(request.limit_price(), Some(money("400.12")));
        assert_eq!(
            BrokerOrderRequest::limit(
                "demo-2",
                "VOO",
                BrokerOrderSide::Buy,
                money("1.00"),
                Decimal::ZERO,
                BrokerEnvironment::Paper,
            ),
            Err(BrokerValidationError::NonPositiveDecimal {
                field: "limit_price"
            })
        );
    }

    /// Verify invalid orders are rejected before any adapter is called.
    #[test]
    fn order_request_rejects_invalid_fields() {
        assert_eq!(
            BrokerOrderRequest::market(
                "",
                "VOO",
                BrokerOrderSide::Buy,
                money("1.00"),
                BrokerEnvironment::Paper,
            ),
            Err(BrokerValidationError::InvalidIdempotencyKey)
        );
        assert_eq!(
            BrokerOrderRequest::market(
                "demo-3",
                "åapl",
                BrokerOrderSide::Buy,
                money("1.00"),
                BrokerEnvironment::Paper,
            ),
            Err(BrokerValidationError::InvalidSymbol)
        );
        assert_eq!(
            BrokerOrderRequest::market(
                "demo-3",
                "AAPL",
                BrokerOrderSide::Buy,
                Decimal::ZERO,
                BrokerEnvironment::Paper,
            ),
            Err(BrokerValidationError::NonPositiveDecimal { field: "quantity" })
        );
    }

    /// Verify mock broker accepts paper orders and records them for demo assertions.
    #[tokio::test]
    async fn mock_broker_accepts_paper_orders() {
        let broker = MockBroker::paper_only();
        let ack = broker
            .submit_order(paper_market_order("demo-4"))
            .await
            .unwrap();

        assert_eq!(ack.order_id(), "MOCK-1");
        assert_eq!(ack.status(), BrokerOrderStatus::Accepted);
        assert_eq!(ack.environment(), BrokerEnvironment::Paper);
        assert_eq!(broker.accepted_orders().len(), 1);
    }

    /// Verify mock broker deduplicates idempotency keys.
    #[tokio::test]
    async fn mock_broker_deduplicates_repeated_idempotency_key() {
        let broker = MockBroker::paper_only();
        broker
            .submit_order(paper_market_order("demo-5"))
            .await
            .unwrap();
        let duplicate = broker
            .submit_order(paper_market_order("demo-5"))
            .await
            .unwrap();

        assert_eq!(duplicate.status(), BrokerOrderStatus::Duplicate);
        assert_eq!(broker.accepted_orders().len(), 1);
    }

    /// Verify live orders are rejected unless explicitly enabled.
    #[tokio::test]
    async fn mock_broker_rejects_live_orders_by_default() {
        let request = BrokerOrderRequest::market(
            "demo-6",
            "VOO",
            BrokerOrderSide::Buy,
            money("1.00"),
            BrokerEnvironment::Live,
        )
        .unwrap();

        assert_eq!(
            MockBroker::paper_only().submit_order(request).await,
            Err(BrokerError::LiveTradingDisabled)
        );
    }
}

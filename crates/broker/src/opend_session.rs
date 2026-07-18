//! Futu/Moomoo OpenD raw TCP session transport.
//!
//! This module implements the official fixed-size OpenD frame and the session
//! bootstrap protocols needed before a paper order can be submitted. Order
//! conversion and submission deliberately remain in the next adapter PR.

use std::{
    fmt,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
    task::JoinHandle,
    time::{sleep, timeout},
};

use crate::{
    BrokerEnvironment, BrokerError, BrokerOrderAck, BrokerOrderRequest, BrokerOrderSide,
    BrokerOrderStatus, BrokerOrderType, OpenDConnectionConfig, OpenDOrderGateway,
};

const HEADER_LENGTH: usize = 44;
const MAX_BODY_LENGTH: usize = 1024 * 1024;
const JSON_FORMAT: u8 = 1;
const INIT_CONNECT_PROTOCOL_ID: u32 = 1001;
const GET_GLOBAL_STATE_PROTOCOL_ID: u32 = 1002;
const KEEP_ALIVE_PROTOCOL_ID: u32 = 1004;
const GET_ACCOUNT_LIST_PROTOCOL_ID: u32 = 2001;
const PLACE_ORDER_PROTOCOL_ID: u32 = 2202;
const PAPER_TRADING_ENVIRONMENT: i64 = 0;
const US_TRADING_MARKET: i64 = 2;
const US_SECURITY_MARKET: i64 = 2;
const SESSION_TIMEOUT: Duration = Duration::from_secs(5);
const CLIENT_ID: &str = "indexlink-rust";
const CLIENT_VERSION: i64 = 1;

/// Session bootstrap failure for a raw OpenD paper connection.
///
/// Error strings intentionally do not reveal host names, account IDs, packet
/// contents, credentials, or provider error messages.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OpenDSessionError {
    /// The session was requested for anything other than paper trading.
    #[error("opend paper session requires Paper environment")]
    PaperTradingRequired,
    /// The raw transport was configured for a host other than local OpenD.
    #[error("opend raw TCP transport requires a loopback host")]
    RemoteHostUnsupported,
    /// Connecting to OpenD timed out before a session request was sent.
    #[error("opend session request timed out")]
    Timeout,
    /// OpenD could not be reached or closed the TCP connection.
    #[error("opend is unavailable")]
    Unavailable,
    /// An OpenD request may have been sent, but no trustworthy result arrived.
    #[error("opend request outcome is unknown")]
    OutcomeUnknown,
    /// OpenD returned a malformed or mismatched protocol frame.
    #[error("opend returned an invalid protocol response")]
    InvalidResponse,
    /// OpenD rejected a bootstrap request.
    #[error("opend rejected the session request")]
    Rejected,
    /// OpenD is reachable but has not authenticated its trading service.
    #[error("opend trading service is not authenticated")]
    TradingNotLoggedIn,
    /// OpenD did not return any simulated trading account.
    #[error("opend did not return a paper trading account")]
    NoPaperAccount,
    /// The configured paper account was absent or was not simulated.
    #[error("configured opend paper account was not available")]
    ConfiguredPaperAccountNotFound,
    /// More than one paper account is available without an explicit selection.
    #[error("multiple opend paper accounts are available; configure an account id")]
    AmbiguousPaperAccounts,
}

/// Connected, initialized, and paper-account-selected OpenD TCP session.
///
/// This transport is deliberately limited to a loopback OpenD instance. The
/// official protocol supports optional packet encryption, but that RSA setup is
/// outside this PR; refusing a remote plaintext TCP connection is safer than
/// silently forwarding trading metadata across the network.
pub struct OpenDPaperSession {
    // Shared by the heartbeat task and the follow-up order gateway, so protocol
    // serial numbers and socket writes remain strictly ordered.
    _transport: Arc<Mutex<OpenDTcpTransport>>,
    // Aborted on drop so a detached timer cannot outlive this session.
    heartbeat_task: JoinHandle<()>,
    connection_id: u64,
    keep_alive_interval: Duration,
    selected_account_id: String,
}

impl OpenDPaperSession {
    /// Connect to local OpenD, initialize a JSON session, verify trade login,
    /// and select one simulated account.
    ///
    /// If [`OpenDConnectionConfig::account_id`] is set, it must match a
    /// simulated account returned by OpenD. Without an explicit account ID,
    /// exactly one simulated account must be available.
    pub async fn connect(config: &OpenDConnectionConfig) -> Result<Self, OpenDSessionError> {
        if config.environment() != BrokerEnvironment::Paper || config.live_trading_enabled() {
            return Err(OpenDSessionError::PaperTradingRequired);
        }
        if !is_loopback_host(config.host()) {
            return Err(OpenDSessionError::RemoteHostUnsupported);
        }

        let mut transport = OpenDTcpTransport::connect(config).await?;
        let initialized = initialize_connection(&mut transport).await?;
        verify_trading_login(&mut transport).await?;
        let accounts = fetch_paper_accounts(&mut transport).await?;
        let selected_account_id = select_paper_account(config.account_id(), accounts)?;
        let transport = Arc::new(Mutex::new(transport));
        let heartbeat_task = spawn_heartbeat(&transport, initialized.keep_alive_interval);

        Ok(Self {
            _transport: transport,
            heartbeat_task,
            connection_id: initialized.connection_id,
            keep_alive_interval: initialized.keep_alive_interval,
            selected_account_id,
        })
    }

    /// Return the OpenD connection ID assigned during `InitConnect`.
    #[must_use]
    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }

    /// Return the explicitly selected simulated account ID.
    ///
    /// Callers must treat this identifier as sensitive operational metadata and
    /// must not place it in HTTP responses, audit snapshots, or unredacted logs.
    #[must_use]
    pub fn selected_account_id(&self) -> &str {
        &self.selected_account_id
    }

    /// Submit one paper order through this initialized OpenD session.
    ///
    /// The MVP intentionally supports only US stocks and ETFs. The request is
    /// sent only after its paper environment has been checked; the response
    /// must confirm the selected simulated account and the same US market.
    pub async fn submit_paper_order(
        &self,
        request: &BrokerOrderRequest,
    ) -> Result<BrokerOrderAck, BrokerError> {
        if request.environment() != BrokerEnvironment::Paper {
            return Err(BrokerError::EnvironmentMismatch {
                configured: BrokerEnvironment::Paper,
                requested: request.environment(),
            });
        }

        let mut transport = self._transport.lock().await;
        let packet_serial = transport
            .next_trade_packet_serial()
            .map_err(map_order_error)?;
        let response = transport
            .request(
                PLACE_ORDER_PROTOCOL_ID,
                place_order_payload(
                    self.connection_id,
                    packet_serial,
                    &self.selected_account_id,
                    request,
                )?,
            )
            .await
            .map_err(map_order_error)?;
        let payload = successful_payload(&response).map_err(map_order_error)?;
        order_ack_from_payload(payload, &self.selected_account_id)
    }
}

#[async_trait]
impl OpenDOrderGateway for OpenDPaperSession {
    async fn submit_paper_order(
        &self,
        config: &OpenDConnectionConfig,
        request: &BrokerOrderRequest,
    ) -> Result<BrokerOrderAck, BrokerError> {
        if config.environment() != BrokerEnvironment::Paper || config.live_trading_enabled() {
            return Err(BrokerError::PaperTradingRequired {
                configured: config.environment(),
            });
        }
        config.validate_order_environment(request)?;
        if config
            .account_id()
            .is_some_and(|account_id| account_id != self.selected_account_id)
        {
            return Err(BrokerError::Rejected);
        }

        OpenDPaperSession::submit_paper_order(self, request).await
    }
}

impl fmt::Debug for OpenDPaperSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenDPaperSession")
            .field("connection_id", &self.connection_id)
            .field("keep_alive_interval", &self.keep_alive_interval)
            .field("selected_account_id", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl Drop for OpenDPaperSession {
    fn drop(&mut self) {
        self.heartbeat_task.abort();
    }
}

struct OpenDTcpTransport {
    stream: TcpStream,
    next_serial: u32,
    next_trade_packet_serial: u32,
}

impl OpenDTcpTransport {
    async fn connect(config: &OpenDConnectionConfig) -> Result<Self, OpenDSessionError> {
        let address = socket_address(config.host(), config.port());
        let stream = timeout(SESSION_TIMEOUT, TcpStream::connect(address))
            .await
            .map_err(|_| OpenDSessionError::Timeout)?
            .map_err(|_| OpenDSessionError::Unavailable)?;

        Ok(Self {
            stream,
            next_serial: 1,
            next_trade_packet_serial: 0,
        })
    }

    fn next_trade_packet_serial(&mut self) -> Result<u32, OpenDSessionError> {
        let serial = self.next_trade_packet_serial;
        self.next_trade_packet_serial = self
            .next_trade_packet_serial
            .checked_add(1)
            .ok_or(OpenDSessionError::InvalidResponse)?;
        Ok(serial)
    }

    async fn request(&mut self, protocol_id: u32, body: Value) -> Result<Value, OpenDSessionError> {
        let serial = self.next_serial;
        self.next_serial = self
            .next_serial
            .checked_add(1)
            .ok_or(OpenDSessionError::InvalidResponse)?;
        let body = serde_json::to_vec(&body).map_err(|_| OpenDSessionError::InvalidResponse)?;
        let request = encode_frame(protocol_id, serial, &body);

        timeout(SESSION_TIMEOUT, self.stream.write_all(&request))
            .await
            .map_err(|_| OpenDSessionError::OutcomeUnknown)?
            .map_err(|_| OpenDSessionError::OutcomeUnknown)?;
        timeout(SESSION_TIMEOUT, self.stream.flush())
            .await
            .map_err(|_| OpenDSessionError::OutcomeUnknown)?
            .map_err(|_| OpenDSessionError::OutcomeUnknown)?;
        let response = timeout(SESSION_TIMEOUT, read_frame(&mut self.stream))
            .await
            .map_err(|_| OpenDSessionError::OutcomeUnknown)?
            .map_err(|_| OpenDSessionError::OutcomeUnknown)?;

        if response.protocol_id != protocol_id
            || response.serial != serial
            || response.format != JSON_FORMAT
        {
            return Err(OpenDSessionError::OutcomeUnknown);
        }

        serde_json::from_slice(&response.body).map_err(|_| OpenDSessionError::OutcomeUnknown)
    }
}

struct OpenDFrame {
    protocol_id: u32,
    format: u8,
    serial: u32,
    body: Vec<u8>,
}

struct InitializedConnection {
    connection_id: u64,
    keep_alive_interval: Duration,
}

async fn initialize_connection(
    transport: &mut OpenDTcpTransport,
) -> Result<InitializedConnection, OpenDSessionError> {
    let response = transport
        .request(
            INIT_CONNECT_PROTOCOL_ID,
            json!({
                "c2s": {
                    "clientVer": CLIENT_VERSION,
                    "clientID": CLIENT_ID,
                    "recvNotify": false,
                    "packetEncAlgo": 0,
                    "pushProtoFmt": JSON_FORMAT,
                }
            }),
        )
        .await?;
    let payload = successful_payload(&response)?;
    Ok(InitializedConnection {
        connection_id: u64_field(payload, "connID")?,
        keep_alive_interval: positive_seconds(payload, "keepAliveInterval")?,
    })
}

async fn verify_trading_login(transport: &mut OpenDTcpTransport) -> Result<(), OpenDSessionError> {
    let response = transport
        .request(
            GET_GLOBAL_STATE_PROTOCOL_ID,
            json!({"c2s": {"userID": "0"}}),
        )
        .await?;
    let payload = successful_payload(&response)?;
    match payload.get("trdLogined").and_then(Value::as_bool) {
        Some(true) => Ok(()),
        Some(false) => Err(OpenDSessionError::TradingNotLoggedIn),
        None => Err(OpenDSessionError::InvalidResponse),
    }
}

async fn fetch_paper_accounts(
    transport: &mut OpenDTcpTransport,
) -> Result<Vec<String>, OpenDSessionError> {
    let response = transport
        .request(
            GET_ACCOUNT_LIST_PROTOCOL_ID,
            json!({
                "c2s": {
                    "userID": "0",
                    "needGeneralSecAccount": true,
                }
            }),
        )
        .await?;
    let payload = successful_payload(&response)?;
    let accounts = payload
        .get("accList")
        .and_then(Value::as_array)
        .ok_or(OpenDSessionError::InvalidResponse)?;

    let mut paper_accounts = Vec::new();
    for account in accounts {
        let account = account
            .as_object()
            .ok_or(OpenDSessionError::InvalidResponse)?;
        if integer_field(account, "trdEnv")? == PAPER_TRADING_ENVIRONMENT {
            paper_accounts.push(string_field(account, "accID")?);
        }
    }
    Ok(paper_accounts)
}

fn select_paper_account(
    configured_account_id: Option<&str>,
    accounts: Vec<String>,
) -> Result<String, OpenDSessionError> {
    if let Some(configured_account_id) = configured_account_id {
        return accounts
            .into_iter()
            .find(|account_id| account_id == configured_account_id)
            .ok_or(OpenDSessionError::ConfiguredPaperAccountNotFound);
    }

    match accounts.len() {
        0 => Err(OpenDSessionError::NoPaperAccount),
        1 => Ok(accounts.into_iter().next().expect("length checked")),
        _ => Err(OpenDSessionError::AmbiguousPaperAccounts),
    }
}

fn successful_payload(response: &Value) -> Result<&Map<String, Value>, OpenDSessionError> {
    match integer_value(response.get("retType")) {
        Some(0) => {}
        Some(_) => return Err(OpenDSessionError::Rejected),
        None => return Err(OpenDSessionError::InvalidResponse),
    }

    response
        .get("s2c")
        .and_then(Value::as_object)
        .ok_or(OpenDSessionError::InvalidResponse)
}

fn u64_field(payload: &Map<String, Value>, name: &str) -> Result<u64, OpenDSessionError> {
    let value = payload
        .get(name)
        .ok_or(OpenDSessionError::InvalidResponse)?;
    match value {
        Value::Number(number) => number.as_u64().ok_or(OpenDSessionError::InvalidResponse),
        Value::String(value) => value
            .parse::<u64>()
            .map_err(|_| OpenDSessionError::InvalidResponse),
        _ => Err(OpenDSessionError::InvalidResponse),
    }
}

fn integer_field(payload: &Map<String, Value>, name: &str) -> Result<i64, OpenDSessionError> {
    integer_value(payload.get(name)).ok_or(OpenDSessionError::InvalidResponse)
}

fn positive_seconds(
    payload: &Map<String, Value>,
    name: &str,
) -> Result<Duration, OpenDSessionError> {
    let seconds = u64::try_from(integer_field(payload, name)?)
        .ok()
        .filter(|seconds| *seconds > 0)
        .ok_or(OpenDSessionError::InvalidResponse)?;
    Ok(Duration::from_secs(seconds))
}

fn integer_value(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(number) => number.as_i64(),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn string_field(payload: &Map<String, Value>, name: &str) -> Result<String, OpenDSessionError> {
    let value = payload
        .get(name)
        .ok_or(OpenDSessionError::InvalidResponse)?;
    match value {
        Value::String(value) if !value.is_empty() => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err(OpenDSessionError::InvalidResponse),
    }
}

fn is_loopback_host(host: &str) -> bool {
    host == "127.0.0.1" || host == "::1" || host.eq_ignore_ascii_case("localhost")
}

fn socket_address(host: &str, port: u16) -> String {
    if host.eq_ignore_ascii_case("localhost") {
        format!("127.0.0.1:{port}")
    } else if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn spawn_heartbeat(
    transport: &Arc<Mutex<OpenDTcpTransport>>,
    interval: Duration,
) -> JoinHandle<()> {
    let transport = Arc::downgrade(transport);
    tokio::spawn(async move {
        loop {
            sleep(interval).await;
            let Some(transport) = transport.upgrade() else {
                return;
            };
            let mut transport = transport.lock().await;
            if send_keep_alive(&mut transport).await.is_err() {
                return;
            }
        }
    })
}

async fn send_keep_alive(transport: &mut OpenDTcpTransport) -> Result<(), OpenDSessionError> {
    let sent_at: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| OpenDSessionError::InvalidResponse)?
        .as_secs()
        .try_into()
        .map_err(|_| OpenDSessionError::InvalidResponse)?;
    let response = transport
        .request(KEEP_ALIVE_PROTOCOL_ID, json!({"c2s": {"time": sent_at}}))
        .await?;
    let payload = successful_payload(&response)?;
    integer_field(payload, "time")?;
    Ok(())
}

fn place_order_payload(
    connection_id: u64,
    packet_serial: u32,
    account_id: &str,
    request: &BrokerOrderRequest,
) -> Result<Value, BrokerError> {
    let quantity = decimal_to_f64(request.quantity())?;
    let price = request
        .limit_price()
        .map(decimal_to_f64)
        .transpose()?
        .unwrap_or(0.0);
    let order_type = match request.order_type() {
        BrokerOrderType::Limit => 1,
        BrokerOrderType::Market => 2,
    };
    let side = match request.side() {
        BrokerOrderSide::Buy => 1,
        BrokerOrderSide::Sell => 2,
    };

    Ok(json!({
        "c2s": {
            "packetID": {"connID": connection_id, "serialNo": packet_serial},
            "header": {
                "trdEnv": PAPER_TRADING_ENVIRONMENT,
                "accID": account_id,
                "trdMarket": US_TRADING_MARKET,
            },
            "trdSide": side,
            "orderType": order_type,
            "code": request.symbol(),
            "qty": quantity,
            "price": price,
            "secMarket": US_SECURITY_MARKET,
            "remark": order_remark(request.idempotency_key()),
        }
    }))
}

fn decimal_to_f64(value: rust_decimal::Decimal) -> Result<f64, BrokerError> {
    value
        .to_string()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or(BrokerError::Rejected)
}

fn order_remark(idempotency_key: &str) -> String {
    format!("indexlink-{:x}", Sha1::digest(idempotency_key.as_bytes()))
}

fn order_ack_from_payload(
    payload: &Map<String, Value>,
    selected_account_id: &str,
) -> Result<BrokerOrderAck, BrokerError> {
    let header = payload
        .get("header")
        .and_then(Value::as_object)
        .ok_or(BrokerError::Unavailable)?;
    if integer_field(header, "trdEnv").map_err(map_order_error)? != PAPER_TRADING_ENVIRONMENT
        || integer_field(header, "trdMarket").map_err(map_order_error)? != US_TRADING_MARKET
        || string_field(header, "accID").map_err(map_order_error)? != selected_account_id
    {
        return Err(BrokerError::Unavailable);
    }

    let order_id = non_empty_string_field(payload, "orderIDEx")
        .or_else(|| non_empty_string_field(payload, "orderID"))
        .ok_or(BrokerError::Unavailable)?;
    BrokerOrderAck::new(
        order_id,
        BrokerEnvironment::Paper,
        BrokerOrderStatus::Accepted,
    )
    .map_err(|_| BrokerError::Unavailable)
}

fn non_empty_string_field(payload: &Map<String, Value>, name: &str) -> Option<String> {
    string_field(payload, name)
        .ok()
        .filter(|value| !value.is_empty())
}

fn map_order_error(error: OpenDSessionError) -> BrokerError {
    match error {
        OpenDSessionError::Rejected => BrokerError::Rejected,
        OpenDSessionError::OutcomeUnknown => BrokerError::OutcomeUnknown,
        _ => BrokerError::Unavailable,
    }
}

fn encode_frame(protocol_id: u32, serial: u32, body: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(HEADER_LENGTH + body.len());
    let body_hash = Sha1::digest(body);
    frame.extend_from_slice(b"FT");
    frame.extend_from_slice(&protocol_id.to_le_bytes());
    frame.push(JSON_FORMAT);
    frame.push(0);
    frame.extend_from_slice(&serial.to_le_bytes());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(&body_hash);
    frame.extend_from_slice(&[0; 8]);
    frame.extend_from_slice(body);
    frame
}

async fn read_frame(stream: &mut TcpStream) -> Result<OpenDFrame, OpenDSessionError> {
    let mut header = [0_u8; HEADER_LENGTH];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|_| OpenDSessionError::Unavailable)?;
    if header[..2] != *b"FT" {
        return Err(OpenDSessionError::InvalidResponse);
    }

    let protocol_id = u32::from_le_bytes([header[2], header[3], header[4], header[5]]);
    let format = header[6];
    let serial = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
    let body_length = u32::from_le_bytes([header[12], header[13], header[14], header[15]]) as usize;
    if body_length > MAX_BODY_LENGTH {
        return Err(OpenDSessionError::InvalidResponse);
    }

    let mut body = vec![0_u8; body_length];
    stream
        .read_exact(&mut body)
        .await
        .map_err(|_| OpenDSessionError::Unavailable)?;
    if header[7] != 0 || Sha1::digest(&body).as_slice() != &header[16..36] {
        return Err(OpenDSessionError::InvalidResponse);
    }

    Ok(OpenDFrame {
        protocol_id,
        format,
        serial,
        body,
    })
}

#[cfg(test)]
mod tests {
    use tokio::{io::AsyncWriteExt, net::TcpListener};

    use super::*;
    use crate::{BrokerOrderRequest, BrokerProvider, OpenDConnectionConfig};

    const GOLDEN_KEEP_ALIVE_FRAME: [u8; 46] = [
        b'F', b'T', 0xec, 0x03, 0x00, 0x00, 0x01, 0x00, 0x07, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
        0x00, 0xbf, 0x21, 0xa9, 0xe8, 0xfb, 0xc5, 0xa3, 0x84, 0x6f, 0xb0, 0x5b, 0x4f, 0xa0, 0x85,
        0x9e, 0x09, 0x17, 0xb2, 0x20, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, b'{',
        b'}',
    ];

    enum PostBootstrapRequest {
        None,
        NoPlaceOrder,
        KeepAlive,
        PlaceOrder { expected: Value, response: Value },
        CloseAfterPlaceOrder { expected: Value },
    }

    /// Start a local fake that verifies session requests and returns supplied state.
    async fn spawn_opend(
        trading_logged_in: bool,
        accounts: Vec<Value>,
    ) -> (u16, tokio::task::JoinHandle<()>) {
        spawn_opend_with_request(trading_logged_in, accounts, PostBootstrapRequest::None, 60).await
    }

    /// Start a local fake that can additionally verify the first KeepAlive request.
    async fn spawn_opend_with_heartbeat(
        trading_logged_in: bool,
        accounts: Vec<Value>,
        expect_heartbeat: bool,
    ) -> (u16, tokio::task::JoinHandle<()>) {
        let request = if expect_heartbeat {
            PostBootstrapRequest::KeepAlive
        } else {
            PostBootstrapRequest::None
        };
        spawn_opend_with_request(trading_logged_in, accounts, request, 1).await
    }

    /// Start a local fake that verifies the next post-bootstrap protocol request.
    async fn spawn_opend_with_request(
        trading_logged_in: bool,
        accounts: Vec<Value>,
        post_bootstrap_request: PostBootstrapRequest,
        keep_alive_interval: i64,
    ) -> (u16, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("client should connect");

            let init = read_frame(&mut stream).await.expect("valid init frame");
            assert_eq!(init.protocol_id, INIT_CONNECT_PROTOCOL_ID);
            assert_eq!(init.serial, 1);
            assert_eq!(
                serde_json::from_slice::<Value>(&init.body).unwrap(),
                json!({
                    "c2s": {
                        "clientVer": CLIENT_VERSION,
                        "clientID": CLIENT_ID,
                        "recvNotify": false,
                        "packetEncAlgo": 0,
                        "pushProtoFmt": JSON_FORMAT,
                    }
                })
            );
            stream
                .write_all(&encode_frame(
                    INIT_CONNECT_PROTOCOL_ID,
                    init.serial,
                    &serde_json::to_vec(&json!({
                        "retType": 0,
                        "s2c": {"connID": "42", "keepAliveInterval": keep_alive_interval}
                    }))
                    .unwrap(),
                ))
                .await
                .unwrap();

            let state = read_frame(&mut stream).await.expect("valid state frame");
            assert_eq!(state.protocol_id, GET_GLOBAL_STATE_PROTOCOL_ID);
            assert_eq!(state.serial, 2);
            stream
                .write_all(&encode_frame(
                    GET_GLOBAL_STATE_PROTOCOL_ID,
                    state.serial,
                    &serde_json::to_vec(
                        &json!({"retType": 0, "s2c": {"trdLogined": trading_logged_in}}),
                    )
                    .unwrap(),
                ))
                .await
                .unwrap();

            if !trading_logged_in {
                return;
            }

            let account_list = read_frame(&mut stream)
                .await
                .expect("valid account-list frame");
            assert_eq!(account_list.protocol_id, GET_ACCOUNT_LIST_PROTOCOL_ID);
            assert_eq!(account_list.serial, 3);
            stream
                .write_all(&encode_frame(
                    GET_ACCOUNT_LIST_PROTOCOL_ID,
                    account_list.serial,
                    &serde_json::to_vec(&json!({"retType": 0, "s2c": {"accList": accounts}}))
                        .unwrap(),
                ))
                .await
                .unwrap();

            match post_bootstrap_request {
                PostBootstrapRequest::None => {}
                PostBootstrapRequest::NoPlaceOrder => {
                    match timeout(Duration::from_millis(100), read_frame(&mut stream)).await {
                        Err(_) | Ok(Err(OpenDSessionError::Unavailable)) => {}
                        Ok(Ok(frame)) => panic!(
                            "unexpected protocol {} sent while order should be rejected locally",
                            frame.protocol_id
                        ),
                        Ok(Err(error)) => {
                            panic!("unexpected local-rejection read error: {error:?}")
                        }
                    }
                }
                PostBootstrapRequest::KeepAlive => {
                    let keep_alive = read_frame(&mut stream)
                        .await
                        .expect("valid keep-alive frame");
                    assert_eq!(keep_alive.protocol_id, KEEP_ALIVE_PROTOCOL_ID);
                    assert_eq!(keep_alive.serial, 4);
                    assert!(serde_json::from_slice::<Value>(&keep_alive.body)
                        .unwrap()
                        .pointer("/c2s/time")
                        .and_then(Value::as_i64)
                        .is_some());
                    stream
                        .write_all(&encode_frame(
                            KEEP_ALIVE_PROTOCOL_ID,
                            keep_alive.serial,
                            &serde_json::to_vec(&json!({"retType": 0, "s2c": {"time": 1}}))
                                .unwrap(),
                        ))
                        .await
                        .unwrap();
                }
                PostBootstrapRequest::PlaceOrder { expected, response } => {
                    let order = read_frame(&mut stream).await.expect("valid order frame");
                    assert_eq!(order.protocol_id, PLACE_ORDER_PROTOCOL_ID);
                    assert_eq!(order.serial, 4);
                    assert_eq!(
                        serde_json::from_slice::<Value>(&order.body).unwrap(),
                        expected
                    );
                    stream
                        .write_all(&encode_frame(
                            PLACE_ORDER_PROTOCOL_ID,
                            order.serial,
                            &serde_json::to_vec(&response).unwrap(),
                        ))
                        .await
                        .unwrap();
                }
                PostBootstrapRequest::CloseAfterPlaceOrder { expected } => {
                    let order = read_frame(&mut stream).await.expect("valid order frame");
                    assert_eq!(order.protocol_id, PLACE_ORDER_PROTOCOL_ID);
                    assert_eq!(order.serial, 4);
                    assert_eq!(
                        serde_json::from_slice::<Value>(&order.body).unwrap(),
                        expected
                    );
                }
            }
        });

        (port, task)
    }

    /// Verify the encoder matches an OpenD frame captured as a fixed golden value.
    #[test]
    fn frame_encoding_matches_independent_golden_frame() {
        assert_eq!(
            encode_frame(KEEP_ALIVE_PROTOCOL_ID, 7, b"{}"),
            GOLDEN_KEEP_ALIVE_FRAME
        );
    }

    /// Verify a missing or non-positive server heartbeat interval is rejected.
    #[test]
    fn heartbeat_interval_requires_positive_seconds() {
        let mut payload = Map::new();
        assert_eq!(
            positive_seconds(&payload, "keepAliveInterval"),
            Err(OpenDSessionError::InvalidResponse)
        );

        payload.insert("keepAliveInterval".to_owned(), json!(0));
        assert_eq!(
            positive_seconds(&payload, "keepAliveInterval"),
            Err(OpenDSessionError::InvalidResponse)
        );
    }

    /// Verify a raw frame with a corrupted SHA-1 digest is rejected.
    #[tokio::test]
    async fn frame_reader_rejects_corrupted_golden_frame() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("client should connect");
            let mut corrupted = GOLDEN_KEEP_ALIVE_FRAME;
            corrupted[16] ^= 0xff;
            stream.write_all(&corrupted).await.unwrap();
        });
        let mut client = TcpStream::connect(address)
            .await
            .expect("test client should connect");

        assert!(matches!(
            read_frame(&mut client).await,
            Err(OpenDSessionError::InvalidResponse)
        ));
        task.await.unwrap();
    }

    /// Verify the official raw frames initialize and select the sole paper account.
    #[tokio::test]
    async fn session_initializes_and_selects_single_paper_account() {
        let (port, server) = spawn_opend(
            true,
            vec![
                json!({"trdEnv": 1, "accID": "live-account"}),
                json!({"trdEnv": 0, "accID": "paper-account"}),
            ],
        )
        .await;
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");

        let session = OpenDPaperSession::connect(&config)
            .await
            .expect("paper session should initialize");

        assert_eq!(session.connection_id(), 42);
        assert_eq!(session.selected_account_id(), "paper-account");
        assert!(!format!("{session:?}").contains("paper-account"));
        server.await.unwrap();
    }

    /// Verify an explicit configuration chooses the matching paper account.
    #[tokio::test]
    async fn session_uses_explicit_paper_account_selection() {
        let (port, server) = spawn_opend(
            true,
            vec![
                json!({"trdEnv": 0, "accID": "paper-a"}),
                json!({"trdEnv": 0, "accID": "paper-b"}),
            ],
        )
        .await;
        let config = OpenDConnectionConfig::paper_with_account(
            BrokerProvider::Moomoo,
            "localhost",
            port,
            "paper-b",
        )
        .expect("paper config should be valid");

        let session = OpenDPaperSession::connect(&config)
            .await
            .expect("configured paper account should be selected");

        assert_eq!(session.selected_account_id(), "paper-b");
        server.await.unwrap();
    }

    /// Verify the session sends protocol 1004 at the interval supplied by OpenD.
    #[tokio::test]
    async fn session_sends_keep_alive_at_server_interval() {
        let (port, server) = spawn_opend_with_heartbeat(
            true,
            vec![json!({"trdEnv": 0, "accID": "paper-account"})],
            true,
        )
        .await;
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");

        let session = OpenDPaperSession::connect(&config)
            .await
            .expect("paper session should initialize");

        assert_eq!(session.keep_alive_interval, Duration::from_secs(1));
        timeout(SESSION_TIMEOUT, server)
            .await
            .expect("keep-alive should arrive before timeout")
            .unwrap();
    }

    /// Build a paper or live market-order fixture sharing the MVP US symbol.
    fn market_order(environment: BrokerEnvironment) -> BrokerOrderRequest {
        BrokerOrderRequest::market(
            "demo-order-1",
            "VOO",
            BrokerOrderSide::Buy,
            "1.25".parse().unwrap(),
            environment,
        )
        .expect("market request should be valid")
    }

    /// Return the raw OpenD body expected for the shared paper market-order fixture.
    fn expected_market_order() -> Value {
        json!({
            "c2s": {
                "packetID": {"connID": 42, "serialNo": 0},
                "header": {"trdEnv": 0, "accID": "paper-account", "trdMarket": 2},
                "trdSide": 1,
                "orderType": 2,
                "code": "VOO",
                "qty": 1.25,
                "price": 0.0,
                "secMarket": 2,
                "remark": "indexlink-93b20997c68861289ecc064f21e1e7a8ed839a72",
            }
        })
    }

    /// Verify a paper market order maps to the official US OpenD request and ack.
    #[tokio::test]
    async fn session_submits_us_market_order_and_maps_ack() {
        let response = json!({
            "retType": 0,
            "s2c": {
                "header": {"trdEnv": 0, "accID": "paper-account", "trdMarket": 2},
                "orderID": "order-42",
            }
        });
        let (port, server) = spawn_opend_with_request(
            true,
            vec![json!({"trdEnv": 0, "accID": "paper-account"})],
            PostBootstrapRequest::PlaceOrder {
                expected: expected_market_order(),
                response,
            },
            60,
        )
        .await;
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");
        let session = OpenDPaperSession::connect(&config)
            .await
            .expect("paper session should initialize");
        let request = market_order(BrokerEnvironment::Paper);

        let ack = OpenDOrderGateway::submit_paper_order(&session, &config, &request)
            .await
            .expect("paper order should be accepted");

        assert_eq!(ack.order_id(), "order-42");
        assert_eq!(ack.environment(), BrokerEnvironment::Paper);
        assert_eq!(ack.status(), BrokerOrderStatus::Accepted);
        server.await.unwrap();
    }

    /// Verify a limit order preserves its explicit price and uses normal order type.
    #[test]
    fn limit_order_payload_preserves_price() {
        let request = BrokerOrderRequest::limit(
            "demo-order-2",
            "VOO",
            BrokerOrderSide::Sell,
            "2.5".parse().unwrap(),
            "400.12".parse().unwrap(),
            BrokerEnvironment::Paper,
        )
        .expect("limit request should be valid");

        assert_eq!(
            place_order_payload(42, 3, "paper-account", &request).unwrap(),
            json!({
                "c2s": {
                    "packetID": {"connID": 42, "serialNo": 3},
                    "header": {"trdEnv": 0, "accID": "paper-account", "trdMarket": 2},
                    "trdSide": 2,
                    "orderType": 1,
                    "code": "VOO",
                    "qty": 2.5,
                    "price": 400.12,
                    "secMarket": 2,
                    "remark": "indexlink-4589de32d6501fc22e1c888d63e9f53aac9e05b1",
                }
            })
        );
    }

    /// Verify OpenD rejection is preserved without leaking its provider message.
    #[tokio::test]
    async fn session_maps_opend_rejection_to_safe_broker_error() {
        let (port, server) = spawn_opend_with_request(
            true,
            vec![json!({"trdEnv": 0, "accID": "paper-account"})],
            PostBootstrapRequest::PlaceOrder {
                expected: expected_market_order(),
                response: json!({"retType": -1, "retMsg": "provider message must not escape"}),
            },
            60,
        )
        .await;
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");
        let session = OpenDPaperSession::connect(&config)
            .await
            .expect("paper session should initialize");
        let request = market_order(BrokerEnvironment::Paper);

        assert_eq!(
            OpenDOrderGateway::submit_paper_order(&session, &config, &request).await,
            Err(BrokerError::Rejected)
        );
        server.await.unwrap();
    }

    /// Verify a non-paper request is rejected before any PlaceOrder frame is sent.
    #[tokio::test]
    async fn session_rejects_non_paper_request_before_sending_order() {
        let (port, server) = spawn_opend_with_request(
            true,
            vec![json!({"trdEnv": 0, "accID": "paper-account"})],
            PostBootstrapRequest::NoPlaceOrder,
            60,
        )
        .await;
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");
        let session = OpenDPaperSession::connect(&config)
            .await
            .expect("paper session should initialize");

        assert_eq!(
            session
                .submit_paper_order(&market_order(BrokerEnvironment::Live))
                .await,
            Err(BrokerError::EnvironmentMismatch {
                configured: BrokerEnvironment::Paper,
                requested: BrokerEnvironment::Live,
            })
        );
        server.await.unwrap();
    }

    /// Verify a live adapter configuration is rejected before any PlaceOrder frame is sent.
    #[tokio::test]
    async fn gateway_rejects_live_configuration_before_sending_order() {
        let (port, server) = spawn_opend_with_request(
            true,
            vec![json!({"trdEnv": 0, "accID": "paper-account"})],
            PostBootstrapRequest::NoPlaceOrder,
            60,
        )
        .await;
        let session_config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");
        let session = OpenDPaperSession::connect(&session_config)
            .await
            .expect("paper session should initialize");
        let live_config = OpenDConnectionConfig::new(
            BrokerProvider::Futu,
            "127.0.0.1",
            port,
            BrokerEnvironment::Live,
            None,
            true,
        )
        .expect("live config should be valid");

        assert_eq!(
            OpenDOrderGateway::submit_paper_order(
                &session,
                &live_config,
                &market_order(BrokerEnvironment::Paper),
            )
            .await,
            Err(BrokerError::PaperTradingRequired {
                configured: BrokerEnvironment::Live,
            })
        );
        server.await.unwrap();
    }

    /// Verify a different configured account is rejected before any PlaceOrder frame is sent.
    #[tokio::test]
    async fn gateway_rejects_mismatched_account_before_sending_order() {
        let (port, server) = spawn_opend_with_request(
            true,
            vec![json!({"trdEnv": 0, "accID": "paper-account"})],
            PostBootstrapRequest::NoPlaceOrder,
            60,
        )
        .await;
        let session_config = OpenDConnectionConfig::paper_with_account(
            BrokerProvider::Futu,
            "127.0.0.1",
            port,
            "paper-account",
        )
        .expect("paper config should be valid");
        let session = OpenDPaperSession::connect(&session_config)
            .await
            .expect("paper session should initialize");
        let different_account = OpenDConnectionConfig::paper_with_account(
            BrokerProvider::Futu,
            "127.0.0.1",
            port,
            "other-paper-account",
        )
        .expect("paper config should be valid");

        assert_eq!(
            OpenDOrderGateway::submit_paper_order(
                &session,
                &different_account,
                &market_order(BrokerEnvironment::Paper),
            )
            .await,
            Err(BrokerError::Rejected)
        );
        server.await.unwrap();
    }

    /// Verify a sent order without a response is marked as unsafe to retry.
    #[tokio::test]
    async fn session_marks_sent_order_without_response_as_outcome_unknown() {
        let (port, server) = spawn_opend_with_request(
            true,
            vec![json!({"trdEnv": 0, "accID": "paper-account"})],
            PostBootstrapRequest::CloseAfterPlaceOrder {
                expected: expected_market_order(),
            },
            60,
        )
        .await;
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");
        let session = OpenDPaperSession::connect(&config)
            .await
            .expect("paper session should initialize");

        assert_eq!(
            OpenDOrderGateway::submit_paper_order(
                &session,
                &config,
                &market_order(BrokerEnvironment::Paper),
            )
            .await,
            Err(BrokerError::OutcomeUnknown)
        );
        server.await.unwrap();
    }

    /// Verify acknowledgements for a different account or environment are rejected.
    #[test]
    fn order_ack_requires_selected_paper_account_and_us_market() {
        let mut payload = Map::new();
        payload.insert(
            "header".to_owned(),
            json!({"trdEnv": 1, "accID": "paper-account", "trdMarket": 2}),
        );
        payload.insert("orderID".to_owned(), json!("order-42"));

        assert_eq!(
            order_ack_from_payload(&payload, "paper-account"),
            Err(BrokerError::Unavailable)
        );

        payload.insert(
            "header".to_owned(),
            json!({"trdEnv": 0, "accID": "paper-account", "trdMarket": 2}),
        );
        payload.insert("orderID".to_owned(), json!(" "));
        assert_eq!(
            order_ack_from_payload(&payload, "paper-account"),
            Err(BrokerError::Unavailable)
        );
    }

    /// Verify a non-authenticated OpenD trading service cannot form a session.
    #[tokio::test]
    async fn session_rejects_opend_without_trade_login() {
        let (port, server) = spawn_opend(false, Vec::new()).await;
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");

        assert!(matches!(
            OpenDPaperSession::connect(&config).await,
            Err(OpenDSessionError::TradingNotLoggedIn)
        ));
        server.await.unwrap();
    }

    /// Verify malformed simulated-account payloads never become an implicit selection.
    #[tokio::test]
    async fn session_rejects_malformed_paper_account() {
        let (port, server) = spawn_opend(true, vec![json!({"trdEnv": 0})]).await;
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "127.0.0.1", port)
            .expect("paper config should be valid");

        assert!(matches!(
            OpenDPaperSession::connect(&config).await,
            Err(OpenDSessionError::InvalidResponse)
        ));
        server.await.unwrap();
    }

    /// Verify raw TCP never attempts a non-loopback plaintext OpenD connection.
    #[tokio::test]
    async fn session_rejects_remote_raw_tcp_host() {
        let config = OpenDConnectionConfig::paper(BrokerProvider::Futu, "opend.example", 11111)
            .expect("paper config should be valid");

        assert!(matches!(
            OpenDPaperSession::connect(&config).await,
            Err(OpenDSessionError::RemoteHostUnsupported)
        ));
    }

    /// Verify account selection never falls back to a live account or an ambiguous paper account.
    #[test]
    fn paper_account_selection_requires_one_safe_candidate() {
        assert_eq!(
            select_paper_account(None, Vec::new()),
            Err(OpenDSessionError::NoPaperAccount)
        );
        assert_eq!(
            select_paper_account(None, vec!["paper-a".to_owned(), "paper-b".to_owned()]),
            Err(OpenDSessionError::AmbiguousPaperAccounts)
        );
        assert_eq!(
            select_paper_account(Some("paper-a"), vec!["paper-b".to_owned()]),
            Err(OpenDSessionError::ConfiguredPaperAccountNotFound)
        );
    }

    /// Verify localhost always resolves to a literal loopback socket address.
    #[test]
    fn localhost_uses_a_literal_loopback_address() {
        assert_eq!(socket_address("localhost", 11111), "127.0.0.1:11111");
        assert_eq!(socket_address("LOCALHOST", 11111), "127.0.0.1:11111");
    }
}

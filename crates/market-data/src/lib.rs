//! 本机 OpenD 与公开宏观数据源的只读市场快照适配器。
//!
//! 本 crate 只生成 Decision Preview 所需的输入，不提交订单、不读取交易账户，
//! 也不保存 API 凭据。调用方应在决策记录中保存生成后的输入快照。

use std::{collections::BTreeMap, net::IpAddr, time::Duration};

use async_trait::async_trait;
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, Utc};
use reqwest::Client;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

const OPEND_HEADER_LENGTH: usize = 44;
const OPEND_JSON_FORMAT: u8 = 1;
const OPEND_INIT_CONNECT: u32 = 1001;
const OPEND_HISTORY_KLINE: u32 = 3103;
const OPEND_US_MARKET: i64 = 11;
const OPEND_DAY_KLINE: i64 = 2;
const OPEND_TIMEOUT: Duration = Duration::from_secs(10);
const MIN_HISTORY: usize = 60;
const MA_WINDOW: usize = 200;
const RSI_WINDOW: usize = 14;
const CAPE_URL: &str = "https://www.multpl.com/shiller-pe/table/by-month";
const VIX_URL: &str = "https://cdn.cboe.com/api/global/us_indices/daily_prices/VIX_History.csv";

type MonthlySeries = BTreeMap<(i32, u32), f64>;

/// 自动拉取后的、可直接输入现有信号 API 的市场快照。
#[derive(Debug, Clone, PartialEq)]
pub struct MarketSignalInput {
    /// 标的代码，统一为大写 ASCII。
    pub symbol: String,
    /// 快照对应的最新交易日，使用 UTC `YYYY-MM-DD`。
    pub as_of: String,
    /// Shiller CAPE 月度历史，最旧在前。
    pub cape_history: Vec<f64>,
    /// 最新 Shiller CAPE。
    pub cape_current: f64,
    /// 以 `100 / CAPE - DGS10` 计算的 ERP 代理月度历史，最旧在前。
    pub erp_history: Vec<f64>,
    /// 最新 ERP 代理值。
    pub erp_current: f64,
    /// 标的收盘价相对 MA200 的月度历史，最旧在前。
    pub ma_distance_history: Vec<f64>,
    /// 最新 MA200 距离。
    pub ma_distance_current: f64,
    /// 14 日 RSI 的月度历史，最旧在前。
    pub rsi_history: Vec<f64>,
    /// 最新 RSI。
    pub rsi_current: f64,
    /// VIX 月度历史，最旧在前。
    pub vix_history: Vec<f64>,
    /// 最新 VIX。
    pub vix_current: f64,
}

/// 自动市场信号快照的可替换读取端口。
#[async_trait]
pub trait MarketSignalProvider: Send + Sync {
    /// 为一个美股 ETF/股票读取并计算最新市场信号输入。
    async fn fetch(&self, symbol: &str) -> Result<MarketSignalInput, MarketDataError>;
}

/// 自动市场数据读取的安全错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MarketDataError {
    /// 标的不符合本适配器允许的安全格式。
    #[error("market symbol is invalid")]
    InvalidSymbol,
    /// 本机 OpenD 无法返回可用日线数据。
    #[error("local OpenD market data is unavailable")]
    OpenDUnavailable,
    /// 公开宏观数据源无法返回可验证数据。
    #[error("market macro data is unavailable")]
    MacroUnavailable,
    /// 返回的历史不足以计算受支持的指标。
    #[error("market history is insufficient")]
    InsufficientHistory,
}

/// 使用本机 OpenD、Shiller CAPE、Cboe VIX 和美国财政部收益率的实际市场信号 provider。
#[derive(Debug, Clone)]
pub struct OpenDMarketSignalProvider {
    host: String,
    port: u16,
    client: Client,
}

impl OpenDMarketSignalProvider {
    /// 创建只读 provider；OpenD 地址必须是字面回环地址。
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, MarketDataError> {
        let host = host.into();
        if !host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
            || port == 0
        {
            return Err(MarketDataError::OpenDUnavailable);
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(90))
            .http1_only()
            .user_agent("IndexLink/0.1 local market-signal refresh")
            .build()
            .map_err(|_| MarketDataError::MacroUnavailable)?;
        Ok(Self { host, port, client })
    }
}

#[async_trait]
impl MarketSignalProvider for OpenDMarketSignalProvider {
    async fn fetch(&self, symbol: &str) -> Result<MarketSignalInput, MarketDataError> {
        let symbol = normalize_symbol(symbol)?;
        let (daily, as_of) = self.fetch_daily_closes(&symbol).await?;
        let cape_task = self.client.get(CAPE_URL).send();
        let vix_task = self.client.get(VIX_URL).send();
        let (cape_response, vix_response) = tokio::join!(cape_task, vix_task);
        let cape_html = cape_response
            .map_err(|_| MarketDataError::MacroUnavailable)?
            .error_for_status()
            .map_err(|_| MarketDataError::MacroUnavailable)?
            .text()
            .await
            .map_err(|_| MarketDataError::MacroUnavailable)?;
        let vix_csv = vix_response
            .map_err(|_| MarketDataError::MacroUnavailable)?
            .error_for_status()
            .map_err(|_| MarketDataError::MacroUnavailable)?
            .text()
            .await
            .map_err(|_| MarketDataError::MacroUnavailable)?;
        let cape = parse_cape_history(&cape_html)?;
        let vix = parse_vix_csv(&vix_csv)?;
        let treasury = self.fetch_treasury_history().await?;
        let (ma_history, rsi_history) = technical_history(&daily)?;
        let (cape_history, erp_history) = cape_and_erp_history(&cape, &treasury)?;
        let vix_history = last_values(vix.into_values().collect())?;
        Ok(MarketSignalInput {
            symbol,
            as_of,
            cape_current: *cape_history
                .last()
                .ok_or(MarketDataError::InsufficientHistory)?,
            erp_current: *erp_history
                .last()
                .ok_or(MarketDataError::InsufficientHistory)?,
            ma_distance_current: *ma_history
                .last()
                .ok_or(MarketDataError::InsufficientHistory)?,
            rsi_current: *rsi_history
                .last()
                .ok_or(MarketDataError::InsufficientHistory)?,
            vix_current: *vix_history
                .last()
                .ok_or(MarketDataError::InsufficientHistory)?,
            cape_history,
            erp_history,
            ma_distance_history: ma_history,
            rsi_history,
            vix_history,
        })
    }
}

impl OpenDMarketSignalProvider {
    async fn fetch_treasury_history(&self) -> Result<MonthlySeries, MarketDataError> {
        let current_year = Utc::now().year();
        let mut values = BTreeMap::new();
        for year in current_year - 6..=current_year {
            let url = format!(
                "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/daily-treasury-rates.csv/{year}/all?type=daily_treasury_yield_curve&field_tdr_date_value={year}&page&_format=csv"
            );
            let csv = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|_| MarketDataError::MacroUnavailable)?
                .error_for_status()
                .map_err(|_| MarketDataError::MacroUnavailable)?
                .text()
                .await
                .map_err(|_| MarketDataError::MacroUnavailable)?;
            values.extend(parse_treasury_csv(&csv)?);
        }
        if values.len() < MIN_HISTORY {
            Err(MarketDataError::InsufficientHistory)
        } else {
            Ok(values)
        }
    }

    async fn fetch_daily_closes(
        &self,
        symbol: &str,
    ) -> Result<(Vec<(NaiveDate, f64)>, String), MarketDataError> {
        let end = Utc::now().date_naive();
        let start = end - ChronoDuration::days(365 * 7);
        let mut stream = timeout(
            OPEND_TIMEOUT,
            TcpStream::connect(format!("{}:{}", self.host, self.port)),
        )
        .await
        .map_err(|_| MarketDataError::OpenDUnavailable)?
        .map_err(|_| MarketDataError::OpenDUnavailable)?;
        let mut serial = 1_u32;
        let _ = opend_request(
            &mut stream,
            OPEND_INIT_CONNECT,
            serial,
            json!({"c2s": {"clientVer": 1, "clientID": "indexlink-market-data", "recvNotify": false, "packetEncAlgo": 0, "pushProtoFmt": 1}}),
        )
        .await?;
        serial += 1;
        let mut closes = Vec::new();
        let mut next_request_key = None;
        for _ in 0..4 {
            let mut request = json!({"c2s": {"rehabType": 1, "klType": OPEND_DAY_KLINE, "security": {"market": OPEND_US_MARKET, "code": symbol}, "beginTime": start.format("%Y-%m-%d").to_string(), "endTime": end.format("%Y-%m-%d").to_string(), "maxAckKLNum": 1000}});
            if let Some(next_request_key) = next_request_key.take() {
                request["c2s"]["nextReqKey"] = Value::String(next_request_key);
            }
            let response = opend_request(&mut stream, OPEND_HISTORY_KLINE, serial, request).await?;
            serial = serial
                .checked_add(1)
                .ok_or(MarketDataError::OpenDUnavailable)?;
            let payload = response
                .get("s2c")
                .and_then(Value::as_object)
                .ok_or(MarketDataError::OpenDUnavailable)?;
            let rows = payload
                .get("klList")
                .and_then(Value::as_array)
                .ok_or(MarketDataError::OpenDUnavailable)?;
            for row in rows {
                let time = row
                    .get("time")
                    .and_then(Value::as_str)
                    .ok_or(MarketDataError::OpenDUnavailable)?;
                let close = row
                    .get("closePrice")
                    .and_then(Value::as_f64)
                    .ok_or(MarketDataError::OpenDUnavailable)?;
                let date = NaiveDate::parse_from_str(&time[..10], "%Y-%m-%d")
                    .map_err(|_| MarketDataError::OpenDUnavailable)?;
                if close.is_finite() && close > 0.0 {
                    closes.push((date, close));
                }
            }
            next_request_key = payload
                .get("nextReqKey")
                .and_then(Value::as_str)
                .filter(|key| !key.is_empty())
                .map(ToOwned::to_owned);
            if next_request_key.is_none() {
                break;
            }
        }
        closes.sort_by_key(|(date, _)| *date);
        let as_of = closes
            .last()
            .map(|(date, _)| date.format("%Y-%m-%d").to_string())
            .ok_or(MarketDataError::InsufficientHistory)?;
        if closes.len() < MA_WINDOW + MIN_HISTORY {
            return Err(MarketDataError::InsufficientHistory);
        }
        Ok((closes, as_of))
    }
}

async fn opend_request(
    stream: &mut TcpStream,
    protocol_id: u32,
    serial: u32,
    body: Value,
) -> Result<Value, MarketDataError> {
    let body = serde_json::to_vec(&body).map_err(|_| MarketDataError::OpenDUnavailable)?;
    let digest = Sha1::digest(&body);
    let mut frame = Vec::with_capacity(OPEND_HEADER_LENGTH + body.len());
    frame.extend_from_slice(b"FT");
    frame.extend_from_slice(&protocol_id.to_le_bytes());
    frame.push(OPEND_JSON_FORMAT);
    frame.push(0);
    frame.extend_from_slice(&serial.to_le_bytes());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(&digest);
    frame.extend_from_slice(&[0; 8]);
    frame.extend_from_slice(&body);
    timeout(OPEND_TIMEOUT, stream.write_all(&frame))
        .await
        .map_err(|_| MarketDataError::OpenDUnavailable)?
        .map_err(|_| MarketDataError::OpenDUnavailable)?;
    timeout(OPEND_TIMEOUT, stream.flush())
        .await
        .map_err(|_| MarketDataError::OpenDUnavailable)?
        .map_err(|_| MarketDataError::OpenDUnavailable)?;
    let mut header = [0_u8; OPEND_HEADER_LENGTH];
    timeout(OPEND_TIMEOUT, stream.read_exact(&mut header))
        .await
        .map_err(|_| MarketDataError::OpenDUnavailable)?
        .map_err(|_| MarketDataError::OpenDUnavailable)?;
    if &header[..2] != b"FT"
        || u32::from_le_bytes(
            header[2..6]
                .try_into()
                .map_err(|_| MarketDataError::OpenDUnavailable)?,
        ) != protocol_id
    {
        return Err(MarketDataError::OpenDUnavailable);
    }
    let size = u32::from_le_bytes(
        header[12..16]
            .try_into()
            .map_err(|_| MarketDataError::OpenDUnavailable)?,
    ) as usize;
    if size > 4 * 1024 * 1024 {
        return Err(MarketDataError::OpenDUnavailable);
    }
    let mut response = vec![0_u8; size];
    timeout(OPEND_TIMEOUT, stream.read_exact(&mut response))
        .await
        .map_err(|_| MarketDataError::OpenDUnavailable)?
        .map_err(|_| MarketDataError::OpenDUnavailable)?;
    if Sha1::digest(&response).as_slice() != &header[16..36] {
        return Err(MarketDataError::OpenDUnavailable);
    }
    serde_json::from_slice(&response).map_err(|_| MarketDataError::OpenDUnavailable)
}

fn normalize_symbol(symbol: &str) -> Result<String, MarketDataError> {
    let symbol = symbol.trim().to_ascii_uppercase();
    if symbol.is_empty()
        || symbol.len() > 10
        || !symbol.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-'
        })
    {
        Err(MarketDataError::InvalidSymbol)
    } else {
        Ok(symbol)
    }
}

fn parse_cape_history(html: &str) -> Result<MonthlySeries, MarketDataError> {
    let mut values = BTreeMap::new();
    for row in html.split("<tr").skip(1) {
        let Some(row) = row.split("</tr>").next() else {
            continue;
        };
        let cells: Vec<String> = row
            .split("</td>")
            .filter_map(|cell| cell.split('>').next_back())
            .map(|cell| cell.replace("&#x2002;", "").trim().to_owned())
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let date = NaiveDate::parse_from_str(&cells[0], "%b %d, %Y").ok();
        let value = cells[1].parse::<f64>().ok();
        if let (Some(date), Some(value)) = (date, value) {
            if value.is_finite() && value > 0.0 {
                values.insert((date.year(), date.month()), value);
            }
        }
    }
    if values.is_empty() {
        Err(MarketDataError::InsufficientHistory)
    } else {
        Ok(values)
    }
}

fn parse_vix_csv(text: &str) -> Result<MonthlySeries, MarketDataError> {
    let mut values = BTreeMap::new();
    for row in text.lines().skip(1) {
        let columns: Vec<&str> = row.split(',').collect();
        if columns.len() < 5 {
            continue;
        }
        let Ok(date) = NaiveDate::parse_from_str(columns[0], "%m/%d/%Y") else {
            continue;
        };
        let Ok(value) = columns[4].parse::<f64>() else {
            continue;
        };
        if value.is_finite() {
            values.insert((date.year(), date.month()), value);
        }
    }
    if values.is_empty() {
        Err(MarketDataError::InsufficientHistory)
    } else {
        Ok(values)
    }
}

fn parse_treasury_csv(text: &str) -> Result<MonthlySeries, MarketDataError> {
    let mut rows = text.lines();
    let header = rows.next().ok_or(MarketDataError::MacroUnavailable)?;
    let ten_year_index = header
        .split(',')
        .position(|column| column.trim_matches('"') == "10 Yr")
        .ok_or(MarketDataError::MacroUnavailable)?;
    let mut values = BTreeMap::new();
    for row in rows {
        let columns: Vec<&str> = row.split(',').collect();
        let Some(date) = columns.first() else {
            continue;
        };
        let Some(value) = columns.get(ten_year_index) else {
            continue;
        };
        let Ok(date) = NaiveDate::parse_from_str(date, "%m/%d/%Y") else {
            continue;
        };
        let Ok(value) = value.parse::<f64>() else {
            continue;
        };
        if value.is_finite() {
            values.insert((date.year(), date.month()), value);
        }
    }
    if values.is_empty() {
        Err(MarketDataError::InsufficientHistory)
    } else {
        Ok(values)
    }
}

fn technical_history(daily: &[(NaiveDate, f64)]) -> Result<(Vec<f64>, Vec<f64>), MarketDataError> {
    let mut months = BTreeMap::new();
    for index in MA_WINDOW..daily.len() {
        let closes: Vec<f64> = daily[index + 1 - MA_WINDOW..=index]
            .iter()
            .map(|(_, close)| *close)
            .collect();
        let average = closes.iter().sum::<f64>() / MA_WINDOW as f64;
        let (gains, losses) = daily[index + 1 - RSI_WINDOW..=index].windows(2).fold(
            (0.0, 0.0),
            |(gains, losses), pair| {
                let change = pair[1].1 - pair[0].1;
                if change >= 0.0 {
                    (gains + change, losses)
                } else {
                    (gains, losses - change)
                }
            },
        );
        let rsi = if losses == 0.0 {
            100.0
        } else {
            100.0 - 100.0 / (1.0 + gains / losses)
        };
        months.insert(
            (daily[index].0.year(), daily[index].0.month()),
            (daily[index].1 / average - 1.0, rsi),
        );
    }
    let values: Vec<(f64, f64)> = months.into_values().collect();
    if values.len() < MIN_HISTORY {
        return Err(MarketDataError::InsufficientHistory);
    }
    let values = &values[values.len() - MIN_HISTORY..];
    Ok((
        values.iter().map(|(ma, _)| *ma).collect(),
        values.iter().map(|(_, rsi)| *rsi).collect(),
    ))
}

fn cape_and_erp_history(
    cape: &MonthlySeries,
    treasury: &MonthlySeries,
) -> Result<(Vec<f64>, Vec<f64>), MarketDataError> {
    let values: Vec<(f64, f64)> = cape
        .iter()
        .filter_map(|(month, cape)| {
            treasury
                .get(month)
                .map(|yield_rate| (*cape, 100.0 / *cape - *yield_rate))
        })
        .filter(|(_, erp)| erp.is_finite())
        .collect();
    if values.len() < MIN_HISTORY {
        return Err(MarketDataError::InsufficientHistory);
    }
    let values = &values[values.len() - MIN_HISTORY..];
    Ok((
        values.iter().map(|(cape, _)| *cape).collect(),
        values.iter().map(|(_, erp)| *erp).collect(),
    ))
}

fn last_values(values: Vec<f64>) -> Result<Vec<f64>, MarketDataError> {
    if values.len() < MIN_HISTORY {
        Err(MarketDataError::InsufficientHistory)
    } else {
        Ok(values[values.len() - MIN_HISTORY..].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify technical snapshots retain the latest sixty monthly observations.
    #[test]
    fn technical_history_returns_monthly_indicators() {
        let start = NaiveDate::from_ymd_opt(2018, 1, 1).unwrap();
        let daily: Vec<_> = (0..(365 * 8))
            .map(|offset| {
                (
                    start + ChronoDuration::days(offset),
                    100.0 + offset as f64 / 10.0,
                )
            })
            .collect();
        let (ma, rsi) = technical_history(&daily).unwrap();
        assert_eq!(ma.len(), MIN_HISTORY);
        assert_eq!(rsi.len(), MIN_HISTORY);
        assert!(rsi.iter().all(|value| *value == 100.0));
    }

    /// Verify unsafe or unsupported symbols cannot become an OpenD request.
    #[test]
    fn symbol_normalization_rejects_unsafe_values() {
        assert_eq!(normalize_symbol(" voo ").unwrap(), "VOO");
        assert_eq!(normalize_symbol("VOO\n").unwrap(), "VOO");
        assert!(normalize_symbol("VOO/../../").is_err());
    }

    /// Exercise the configured local OpenD and public read-only sources without trading.
    #[tokio::test]
    #[ignore = "requires local OpenD and public market-data network access"]
    async fn local_opend_market_signal_smoke() {
        let provider = OpenDMarketSignalProvider::new("127.0.0.1", 11111).unwrap();
        let input = provider.fetch("VOO").await.unwrap();
        assert_eq!(input.symbol, "VOO");
        assert_eq!(input.cape_history.len(), MIN_HISTORY);
        assert_eq!(input.erp_history.len(), MIN_HISTORY);
        assert_eq!(input.ma_distance_history.len(), MIN_HISTORY);
        assert_eq!(input.rsi_history.len(), MIN_HISTORY);
        assert_eq!(input.vix_history.len(), MIN_HISTORY);
    }
}

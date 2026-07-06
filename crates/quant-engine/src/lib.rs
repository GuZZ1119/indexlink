//! Quant Engine — 70/20/10 框架的前两层。
//!
//! **设计原则：纯函数，零 IO。**
//!
//! 所有函数只接受数据、返回结果，没有任何网络请求、文件读写或全局状态。
//! 这使得同一份代码可同时用于实盘和历史回测，结果完全可复现。
//!
//! # 当前实现范围（MVP 第一阶段）
//!
//! - **第一层（70% 基本面）**：Shiller CAPE 分位 + ERP 分位 → 综合基本面得分
//! - **第二层（20% 趋势）**：MA200 距离 / RSI / VIX 加权分位 → 综合趋势得分 + 节奏体制
//!   （[`evaluate_trend`] 已实现；过渡期 [`evaluate_trend_or_stub`] 仍保留兼容入口）

pub mod fundamental;
pub mod percentile;
pub mod trend;
pub mod weight;

pub use fundamental::{
    evaluate_fundamental, FundamentalConfig, FundamentalSignal, FundamentalSnapshot,
};
pub use percentile::{percentile_of, weighted_percentile_of, EwPercentileConfig};
#[allow(deprecated)]
pub use trend::{
    evaluate_trend, evaluate_trend_or_stub, evaluate_trend_stub, TrendConfig, TrendRegime,
    TrendSignal, TrendSnapshot, TrendWeights,
};
pub use weight::Weight;

// ─── 错误类型 ────────────────────────────────────────────────────────────────

/// Quant Engine 可能产生的错误。
#[derive(Debug, Clone, PartialEq)]
pub enum QuantError {
    /// 历史数据点不足。
    InsufficientHistory {
        indicator: &'static str,
        required: usize,
        found: usize,
    },
    /// 配置权重无效。
    InvalidWeight { value: f64 },
    /// 最少历史数据点数无效。
    InvalidMinHistoryLen { value: usize },
    /// 当前指标读数无效。
    InvalidCurrentValue { indicator: &'static str, value: f64 },
    /// 指数加权分位的半衰期无效（非有限数或 ≤ 0）。
    InvalidHalfLife { value: f64 },
    /// 指数加权分位的衰减系数无效（不在 `(0.0, 1.0]`）。
    InvalidDecay { alpha: f64 },
    /// 分位阈值无效。
    InvalidPercentileThreshold { name: &'static str, value: f64 },
    /// 公开 API 尚未实现（非输入/配置错误）。
    ///
    /// 调用方须显式处理：降级为 [`evaluate_trend_stub`](crate::evaluate_trend_stub)，
    /// 或由 Decision Engine 返回 [`core_domain::Action::Skip`]。
    NotImplemented,
}

impl std::fmt::Display for QuantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientHistory {
                indicator,
                required,
                found,
            } => {
                write!(
                    f,
                    "{indicator}: requires at least {required} historical data points, found {found}"
                )
            }
            Self::InvalidWeight { value } => {
                write!(f, "weight must be finite and in [0.0, 1.0], got {value}")
            }
            Self::InvalidMinHistoryLen { value } => {
                write!(f, "min_history_len must be greater than 0, got {value}")
            }
            Self::InvalidCurrentValue { indicator, value } => {
                write!(f, "{indicator} current value must be finite, got {value}")
            }
            Self::InvalidHalfLife { value } => {
                write!(
                    f,
                    "half_life must be finite and greater than 0, got {value}"
                )
            }
            Self::InvalidDecay { alpha } => {
                write!(
                    f,
                    "decay alpha must be finite and in (0.0, 1.0], got {alpha}"
                )
            }
            Self::InvalidPercentileThreshold { name, value } => {
                write!(
                    f,
                    "{name} threshold must be finite and in [0.0, 1.0], got {value}"
                )
            }
            Self::NotImplemented => {
                write!(f, "quant engine API not yet implemented")
            }
        }
    }
}

impl std::error::Error for QuantError {}

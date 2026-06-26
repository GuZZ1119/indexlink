//! 第二层（20% 趋势）评估。
//!
//! 将 200 日均线距离、RSI、VIX 三项指标各自转为在自身历史分布中的
//! **指数加权分位**，再按子权重合成趋势综合得分（`score`），并以阈值
//! 判定节奏体制（[`TrendRegime`]）供 Decision Engine 触发 `TacticalDelay`。
//!
//! # 方向约定
//!
//! | 指标 | 原始分位高代表 | 计入 score 时 |
//! |------|---------------|--------------|
//! | MA200 距离 `(price−ma)/ma` | 价格远高于均线（强势上涨） | 反向 `1 − p` |
//! | RSI | 超买（过热） | 反向 `1 − p` |
//! | VIX | 高恐慌/急跌 | 正向 `p` |
//!
//! 最终 `score = 0.0` 对应强势上涨/赶顶风险，`1.0` 对应强势下跌/接飞刀风险。
//!
//! # 数据频率（默认契约）
//!
//! **默认按月度样本处理**，与第一层（70% 基本面）及 MVP 月度定投节奏对齐：
//!
//! - 历史序列中**每个元素 = 一个评估月**（通常为每月最后一个交易日的指标读数）。
//! - [`TrendConfig::default`] 的半衰期 `H = 36` 表示 **36 个月**；`min_len = 60` 表示至少 **5 年**
//!   月度有效样本。
//! - 指标本身可在日频上计算（如 200 日均线、14 日 RSI），但接入层须在评估日**采样为月度序列**
//!   后再传入本模块；勿将日频序列直接配合 `H = 36` 使用（那等价于仅 36 个交易日记忆）。
//!
//! 若接入层确需日频序列，须显式构造 [`EwPercentileConfig`]（例如 `H ≈ 756` 交易日、
//! `min_len ≈ 252`），且不得使用 [`TrendConfig::default`]。
//!
//! # 注意
//!
//! 本模块为**纯函数，零 IO**；所有计算均无副作用，可用于实盘与回测。

use core_domain::Percentile;

// weighted_percentile_of 将在 evaluate_trend 实现时使用；存根阶段暂时抑制警告。
#[allow(unused_imports)]
use crate::{weighted_percentile_of, EwPercentileConfig, QuantError, Weight};

// ─── 配置常量 ────────────────────────────────────────────────────────────────

/// 默认 MA200 子权重。
const DEFAULT_MA_WEIGHT: f64 = 0.4;
/// 默认 RSI 子权重。
const DEFAULT_RSI_WEIGHT: f64 = 0.3;
/// 默认 VIX 子权重（= 1 − MA − RSI）。
const DEFAULT_VIX_WEIGHT: f64 = 0.3;
/// 默认指数加权分位半衰期（月频）：36 个月；与基本面层同源。
const DEFAULT_HALF_LIFE_MONTHS: f64 = 36.0;
/// 默认最少历史样本数（月频）：5 年月度数据；与基本面层同源。
const DEFAULT_MIN_HISTORY_LEN: usize = 60;
/// 默认赶顶阈值：RSI/MA 分位高于此值视为过热。
const DEFAULT_OVERHEATED_ABOVE: f64 = 0.90;
/// 默认接飞刀阈值：VIX 分位高于此值视为高恐慌急跌。
const DEFAULT_FALLING_KNIFE_ABOVE: f64 = 0.90;
/// 三权重之和的容许误差（浮点精度）。
const WEIGHT_SUM_TOLERANCE: f64 = 1e-9;

// ─── TrendWeights ────────────────────────────────────────────────────────────

/// 趋势三指标（MA200 距离、RSI、VIX）在 20% 层内部的子权重。
///
/// 三者之和须在 `[1.0 - ε, 1.0 + ε]`（ε = 1e-9），构造时强制校验。
#[derive(Debug, Clone, PartialEq)]
pub struct TrendWeights {
    /// MA200 距离在趋势合成中的占比。
    pub ma_weight: Weight,
    /// RSI 在趋势合成中的占比。
    pub rsi_weight: Weight,
    /// VIX 在趋势合成中的占比。
    pub vix_weight: Weight,
}

impl TrendWeights {
    /// 构造三指标子权重。
    ///
    /// 所有参数须各自在 `[0.0, 1.0]` 且三者之和 ≈ 1.0（误差 <= 1e-9），
    /// 否则返回 [`QuantError::InvalidWeight`]。
    ///
    /// # 错误
    ///
    /// - [`QuantError::InvalidWeight`]：任一权重越界，或三者之和不为 1。
    pub fn new(ma: f64, rsi: f64, vix: f64) -> Result<Self, QuantError> {
        let ma_weight = Weight::new(ma)?;
        let rsi_weight = Weight::new(rsi)?;
        let vix_weight = Weight::new(vix)?;

        let sum = ma + rsi + vix;
        if (sum - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
            return Err(QuantError::InvalidWeight { value: sum });
        }

        Ok(Self {
            ma_weight,
            rsi_weight,
            vix_weight,
        })
    }
}

impl Default for TrendWeights {
    fn default() -> Self {
        Self::new(DEFAULT_MA_WEIGHT, DEFAULT_RSI_WEIGHT, DEFAULT_VIX_WEIGHT)
            .expect("默认趋势子权重有效且和为 1.0")
    }
}

// ─── TrendConfig ─────────────────────────────────────────────────────────────

/// 第二层（20% 趋势）配置参数。
#[derive(Debug, Clone, PartialEq)]
pub struct TrendConfig {
    /// 三指标子权重（和须 ≈ 1.0）。
    pub weights: TrendWeights,
    /// 指数加权历史分位配置（半衰期 + 最少样本数）。
    ///
    /// **默认契约为月频**：`half_life` 单位为月，`min_len` 为月度样本数。
    /// 与 [`TrendSnapshot`] 的历史序列频率须一致；详见模块级「数据频率」说明。
    pub percentile_config: EwPercentileConfig,
    /// RSI 或 MA200 原始分位超过此阈值时判定为「赶顶/过热」。
    ///
    /// `0.0 = 强势上涨/赶顶` 方向——**注意**：score 是反向的，
    /// 此阈值比较的是**原始**（未反向）MA/RSI 分位。
    pub overheated_above: Percentile,
    /// VIX 原始分位超过此阈值时判定为「接飞刀」。
    pub falling_knife_above: Percentile,
}

impl TrendConfig {
    /// 构造趋势层配置。
    ///
    /// # 检查
    ///
    /// 因为入参 weights: TrendWeights 表示权重已通过构造校验
    /// 且入参 percentile_config: EwPercentileConfig 表示分位配置已通过构造校验
    /// 因此本函数只负责校验两个阈值 raw f64
    ///
    /// # 错误
    ///
    /// - [`QuantError::InvalidPercentileThreshold`]：`overheated_above` 或
    ///   `falling_knife_above` 不是有限数，或不在 `[0.0, 1.0]`。
    ///
    /// `weights` 与 `percentile_config` 须由调用方预先构造；
    /// 其校验错误分别见 [`TrendWeights::new`] 与 [`EwPercentileConfig::from_half_life`]。
    pub fn new(
        weights: TrendWeights,
        percentile_config: EwPercentileConfig,
        overheated_above: f64,
        falling_knife_above: f64,
    ) -> Result<Self, QuantError> {
        let overheated_above =
            Percentile::new(overheated_above).ok_or(QuantError::InvalidPercentileThreshold {
                name: "overheated_above",
                value: overheated_above,
            })?;
        let falling_knife_above =
            Percentile::new(falling_knife_above).ok_or(QuantError::InvalidPercentileThreshold {
                name: "falling_knife_above",
                value: falling_knife_above,
            })?;

        Ok(Self {
            weights,
            percentile_config,
            overheated_above,
            falling_knife_above,
        })
    }
}

impl Default for TrendConfig {
    fn default() -> Self {
        Self {
            weights: TrendWeights::default(),
            percentile_config: EwPercentileConfig::from_half_life(
                DEFAULT_HALF_LIFE_MONTHS,
                DEFAULT_MIN_HISTORY_LEN,
            )
            .expect("默认半衰期与最少历史长度有效"),
            overheated_above: Percentile::new(DEFAULT_OVERHEATED_ABOVE)
                .expect("默认过热阈值在 [0.0, 1.0]"),
            falling_knife_above: Percentile::new(DEFAULT_FALLING_KNIFE_ABOVE)
                .expect("默认接飞刀阈值在 [0.0, 1.0]"),
        }
    }
}

// ─── TrendSnapshot ───────────────────────────────────────────────────────────

/// 单次趋势评估所需的输入快照。
///
/// 各指标**自带历史序列**，与 [`crate::fundamental::FundamentalSnapshot`] 同构，
/// 以便各指标独立使用不同长度的历史数据。
///
/// # 数据频率
///
/// **默认契约为月度样本**（见模块级说明）：索引 `0` 为最旧月，末尾为最近评估月。
/// 接入层通常先在日频上计算指标，再于每月评估日取最后一个读数写入序列。
/// 若传入日频序列，须配合日频 [`EwPercentileConfig`]，不可使用 [`TrendConfig::default`]。
///
/// 历史序列须按时间正序排列。
#[derive(Debug, Clone, PartialEq)]
pub struct TrendSnapshot {
    /// MA200 距离历史序列（月频采样）：`(price − ma200) / ma200`，正值表示价格在均线上方。
    pub ma_distance_history: Vec<f64>,
    /// 当前 MA200 距离读数（最近评估月）。
    pub ma_distance_current: f64,

    /// RSI 历史序列（月频采样；指标可在日频上计算，取值范围 [0, 100]）。
    pub rsi_history: Vec<f64>,
    /// 当前 RSI 读数（最近评估月）。
    pub rsi_current: f64,

    /// VIX 历史序列（月频采样；CBOE 波动率指数）。
    pub vix_history: Vec<f64>,
    /// 当前 VIX 读数（最近评估月）。
    pub vix_current: f64,
}

// ─── TrendRegime ─────────────────────────────────────────────────────────────

/// 趋势节奏体制标签，供 Decision Engine 决定是否触发 [`core_domain::Action::TacticalDelay`]。
///
/// - 连续 `score` 负责 20% 节奏的数值微调；
/// - `regime` 是离散事件信号，两者解耦。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendRegime {
    /// 强势上涨/赶顶：MA200 或 RSI 的原始分位 > `overheated_above`。
    Overheated,
    /// 中性节奏：既未过热也未进入接飞刀区间。
    Neutral,
    /// 急跌/接飞刀：VIX 原始分位 > `falling_knife_above`。
    FallingKnife,
}

// ─── TrendSignal ─────────────────────────────────────────────────────────────

/// 第二层（20% 趋势）评估结果。
#[derive(Debug, Clone, PartialEq)]
pub struct TrendSignal {
    /// 趋势综合得分（`0.0 = 强势上涨/赶顶风险，1.0 = 强势下跌/接飞刀风险`）。
    ///
    /// 该值用于 70/20/10 加法管线中的 20% 节奏微调；
    /// 节奏体制的离散判定由 [`TrendSignal::regime`] 承载。
    pub score: Percentile,

    /// 原始 MA200 距离分位（**未反向**，高值 = 均线上方距离大 = 赶顶风险）。
    pub ma_distance_percentile: Percentile,
    /// 原始 RSI 分位（**未反向**，高值 = 超买/过热）。
    pub rsi_percentile: Percentile,
    /// 原始 VIX 分位（正向使用，高值 = 高恐慌/接飞刀）。
    pub vix_percentile: Percentile,

    /// 节奏体制标签（`TacticalDelay` 的判定依据）。
    pub regime: TrendRegime,
}

// ─── evaluate_trend ──────────────────────────────────────────────────────────

/// 计算第二层（20% 趋势）综合得分与节奏体制。纯函数，无 IO。
///
/// # 实现状态
///
/// **当前尚未实现**，始终返回 [`QuantError::NotImplemented`]。
/// 调用方（Decision Engine）须显式处理，不可 `unwrap` 或忽略：
///
/// - **降级（推荐过渡期）**：改用 [`evaluate_trend_or_stub`] 或手动匹配
///   `NotImplemented` 后调用 [`evaluate_trend_stub`]，20% 层保持中性、不触发 [`TrendRegime`] 偏移；
/// - **Skip**：若趋势输入不可用或不允许降级，整条定投管线返回
///   [`core_domain::Action::Skip`]（由上层决策，非本函数职责）。
///
/// # 合成逻辑（落地后）
///
/// ```text
/// ma_p  = weighted_percentile_of(ma_distance_history, ma_distance_current)
/// rsi_p = weighted_percentile_of(rsi_history, rsi_current)
/// vix_p = weighted_percentile_of(vix_history, vix_current)
///
/// score = ma_weight  × (1 − ma_p)   // MA 高分位 → 赶顶 → score 趋 0
///       + rsi_weight × (1 − rsi_p)  // RSI 高分位 → 超买 → score 趋 0
///       + vix_weight × vix_p        // VIX 高分位 → 恐慌 → score 趋 1
/// ```
///
/// # 节奏体制（`regime`）
///
/// 以**原始**（未反向）分位与阈值比较：
/// - `ma_p > overheated_above` 或 `rsi_p > overheated_above` → [`TrendRegime::Overheated`]
/// - `vix_p > falling_knife_above` → [`TrendRegime::FallingKnife`]
/// - 其余 → [`TrendRegime::Neutral`]
///
/// 当 `Overheated` 与 `FallingKnife` 同时满足时，优先判定为 `FallingKnife`（急跌优先）。
///
/// # 错误
///
/// - [`QuantError::NotImplemented`]：**当前唯一返回值**（实现完成前）。
/// - [`QuantError::InsufficientHistory`]：任一指标有效样本数 < `min_len`（实现后）。
/// - [`QuantError::InvalidCurrentValue`]：任一 `current` 非有限数（实现后）。
pub fn evaluate_trend(
    snapshot: &TrendSnapshot,
    config: &TrendConfig,
) -> Result<TrendSignal, QuantError> {
    let _ = (snapshot, config);
    Err(QuantError::NotImplemented)
}

/// 调用 [`evaluate_trend`]，[`QuantError::NotImplemented`] 时降级为 [`evaluate_trend_stub`]。
///
/// Decision Engine 在趋势层落地前的推荐入口：数据/配置错误仍向上传播，
/// 仅「尚未实现」时使用中性 stub，避免 panic 且保持 20% 层无方向偏移。
///
/// # 错误
///
/// 除 [`QuantError::NotImplemented`] 外的所有 [`QuantError`] 原样返回（实现后含
/// `InsufficientHistory`、`InvalidCurrentValue` 等）。
#[allow(deprecated)]
pub fn evaluate_trend_or_stub(
    snapshot: &TrendSnapshot,
    config: &TrendConfig,
) -> Result<TrendSignal, QuantError> {
    match evaluate_trend(snapshot, config) {
        Ok(signal) => Ok(signal),
        Err(QuantError::NotImplemented) => Ok(evaluate_trend_stub()),
        Err(err) => Err(err),
    }
}

// ─── 过渡期存根（已废弃） ────────────────────────────────────────────────────

/// 第二层（20% 趋势）评估的占位实现，始终返回中性。
///
/// # 废弃说明
///
/// 请改用 [`evaluate_trend`]（实现后）或过渡期 [`evaluate_trend_or_stub`]。
/// 此函数在 [`evaluate_trend`] 落地后将被移除。
///
/// # 注意
///
/// 仅应在 [`QuantError::NotImplemented`] 降级路径中使用；返回值本身不携带输入快照信息。
#[deprecated(
    since = "0.2.0",
    note = "请改用 evaluate_trend，此存根将在下一版本移除"
)]
pub fn evaluate_trend_stub() -> TrendSignal {
    TrendSignal {
        score: Percentile::new(0.5).unwrap(),
        ma_distance_percentile: Percentile::new(0.5).unwrap(),
        rsi_percentile: Percentile::new(0.5).unwrap(),
        vix_percentile: Percentile::new(0.5).unwrap(),
        regime: TrendRegime::Neutral,
    }
}

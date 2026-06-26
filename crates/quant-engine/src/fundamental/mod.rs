//! 第一层（70% 基本面）评估。
//!
//! 使用 CAPE 与 ERP 的历史分位合成基本面位置锚。

use core_domain::Percentile;

use crate::{weighted_percentile_of, EwPercentileConfig, QuantError, Weight};

/// 默认 CAPE/ERP 均衡权重。
const DEFAULT_CAPE_WEIGHT: f64 = 0.5;
/// 默认指数加权分位半衰期：36 个月月度数据。
const DEFAULT_HALF_LIFE_MONTHS: f64 = 36.0;
/// 默认最少历史长度：5 年月度数据。
const DEFAULT_MIN_HISTORY_LEN: usize = 60;

/// 第一层（70% 基本面）的配置参数。
#[derive(Debug, Clone, PartialEq)]
pub struct FundamentalConfig {
    /// CAPE 在综合基本面得分中的权重（另一部分为 ERP）。
    /// ERP 权重 = `1.0 - cape_weight`。
    pub cape_weight: Weight,
    /// 指数加权历史分位的配置（半衰期 + 最少有效样本数）。
    pub percentile_config: EwPercentileConfig,
}

impl FundamentalConfig {
    /// 构造第一层（70% 基本面）的配置参数。
    ///
    /// `cape_weight` 必须在 `[0.0, 1.0]`；`percentile_config` 由调用方按数据频率构造。
    pub fn new(
        cape_weight: f64,
        percentile_config: EwPercentileConfig,
    ) -> Result<Self, QuantError> {
        Ok(Self {
            cape_weight: Weight::new(cape_weight)?,
            percentile_config,
        })
    }
}

impl Default for FundamentalConfig {
    fn default() -> Self {
        Self {
            cape_weight: Weight::new(DEFAULT_CAPE_WEIGHT).expect("默认 CAPE 权重在 [0.0, 1.0]"),
            percentile_config: EwPercentileConfig::from_half_life(
                DEFAULT_HALF_LIFE_MONTHS,
                DEFAULT_MIN_HISTORY_LEN,
            )
            .expect("默认半衰期与最少历史长度有效"),
        }
    }
}

/// 单次基本面评估所需的基本面快照。
#[derive(Debug, Clone, PartialEq)]
pub struct FundamentalSnapshot {
    /// Shiller CAPE 历史序列（建议月度，至少 5 年）。
    pub cape_history: Vec<f64>,
    /// 当前 Shiller CAPE 读数。
    pub cape_current: f64,

    /// 股权风险溢价（ERP）历史序列，与 CAPE 同周期。
    /// ERP = 股票预期收益率 - 无风险利率；值越高代表市场越便宜。
    pub erp_history: Vec<f64>,
    /// 当前 ERP 读数。
    pub erp_current: f64,
}

/// 第一层（70% 基本面）的评估结果。
#[derive(Debug, Clone, PartialEq)]
pub struct FundamentalSignal {
    /// 综合基本面得分（0.0 = 历史最便宜，1.0 = 历史最贵）。
    /// 这是 70% 层向 Decision Engine 输出的核心数字。
    pub score: Percentile,

    /// 原始 CAPE 分位（供审计/调试用）。
    pub cape_percentile: Percentile,

    /// 原始 ERP 分位（供审计/调试用；**未倒置**，高值 = 便宜）。
    pub erp_percentile: Percentile,
}

/// 计算第一层（70% 基本面）的综合得分。
///
/// 综合逻辑：
/// - CAPE 分位**直接使用**：CAPE 越高 → 市场越贵 → 得分越高
/// - ERP 分位**倒置使用**：ERP 越高 → 市场越便宜 → 倒置后得分越低
///
/// `score = cape_weight × cape_p + (1 - cape_weight) × (1 - erp_p)`
///
/// 最终得分 0.0 = 历史最便宜（建议加码），1.0 = 历史最贵（建议减量）。
pub fn evaluate_fundamental(
    snapshot: &FundamentalSnapshot,
    config: &FundamentalConfig,
) -> Result<FundamentalSignal, QuantError> {
    let cape_p = weighted_percentile_of(
        "CAPE",
        &snapshot.cape_history,
        snapshot.cape_current,
        &config.percentile_config,
    )?;

    let erp_p = weighted_percentile_of(
        "ERP",
        &snapshot.erp_history,
        snapshot.erp_current,
        &config.percentile_config,
    )?;

    let cape_weight = config.cape_weight.value();
    let erp_weight = config.cape_weight.complement().value();
    // erp_p.invert() 将"高ERP=便宜"转换为"高值=贵"，与CAPE方向对齐
    let composite = cape_weight * cape_p.value() + erp_weight * erp_p.invert().value();

    // composite 是两个 [0,1] 值的加权平均，结果必然在 [0,1]
    let score = Percentile::new(composite).expect("加权平均结果必然在 [0.0, 1.0]");

    Ok(FundamentalSignal {
        score,
        cape_percentile: cape_p,
        erp_percentile: erp_p,
    })
}

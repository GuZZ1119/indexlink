// quant-engine 集成测试共享夹具：统一领域阈值、历史序列与测试配置。

#![allow(dead_code)]

use quant_engine::{
    EwPercentileConfig, FundamentalConfig, TrendConfig, TrendSnapshot, TrendWeights,
};

/// 测试中常用的标准历史长度（便于构造精确百分位）。
pub const STANDARD_HISTORY_LEN: usize = 100;
/// 默认配置契约：5 年月度数据。
pub const DEFAULT_MIN_HISTORY_LEN: usize = 60;
/// 默认配置契约：指数加权分位半衰期为 36 个月。
pub const DEFAULT_HALF_LIFE_MONTHS: f64 = 36.0;
/// 集成测试使用的低门槛历史长度，避免夹具过重。
pub const TEST_MIN_HISTORY_LEN: usize = 10;
/// 集成测试使用的较短半衰期，便于构造可读的加权分位场景。
pub const TEST_HALF_LIFE_MONTHS: f64 = 12.0;

/// 第一层默认均衡权重。
pub const BALANCED_CAPE_WEIGHT: f64 = 0.50;
/// 仅使用 CAPE 维度。
pub const CAPE_ONLY_WEIGHT: f64 = 1.0;
/// 仅使用倒置后的 ERP 维度。
pub const ERP_ONLY_WEIGHT: f64 = 0.0;

/// 位置语言中的中性分位。
pub const NEUTRAL_PERCENTILE: f64 = 0.50;
/// 中性场景允许的近似误差。
pub const NEUTRAL_TOLERANCE: f64 = 0.05;
/// 精确分位断言允许的浮点误差。
pub const EXACT_FLOAT_TOLERANCE: f64 = 1e-9;

/// 低位 / 便宜区域的测试上界。
pub const CHEAP_SCORE_UPPER_BOUND: f64 = 0.20;
/// 高位 / 贵区域的测试下界。
pub const EXPENSIVE_SCORE_LOWER_BOUND: f64 = 0.80;
/// 极低原始分位的测试上界。
pub const VERY_LOW_PERCENTILE_UPPER_BOUND: f64 = 0.10;

/// 分位值边界。
pub const MIN_PERCENTILE: f64 = 0.0;
pub const MAX_PERCENTILE: f64 = 1.0;

pub fn make_history(n: usize) -> Vec<f64> {
    (1..=n).map(|i| i as f64).collect()
}

pub fn standard_history() -> Vec<f64> {
    make_history(STANDARD_HISTORY_LEN)
}

pub fn test_percentile_config() -> EwPercentileConfig {
    EwPercentileConfig::from_half_life(TEST_HALF_LIFE_MONTHS, TEST_MIN_HISTORY_LEN)
        .expect("测试半衰期与历史长度有效")
}

pub fn test_config(cape_weight: f64) -> FundamentalConfig {
    FundamentalConfig::new(cape_weight, test_percentile_config()).expect("测试配置有效")
}

pub fn balanced_test_config() -> FundamentalConfig {
    test_config(BALANCED_CAPE_WEIGHT)
}

// ─── 趋势层夹具 ───────────────────────────────────────────────────────────────

/// 趋势测试使用的最少历史长度（低门槛，绕开默认 60 月度的重量级要求）。
pub const TREND_TEST_MIN_HISTORY_LEN: usize = 10;
/// 趋势测试使用的半衰期（月频单位；测试中用较短值便于构造场景）。
pub const TREND_TEST_HALF_LIFE: f64 = 12.0;

/// 默认均衡三子权重（MA/RSI/VIX 各 1/3）。
pub const TREND_EQUAL_MA_WEIGHT: f64 = 1.0 / 3.0;
pub const TREND_EQUAL_RSI_WEIGHT: f64 = 1.0 / 3.0;
pub const TREND_EQUAL_VIX_WEIGHT: f64 = 1.0 / 3.0;

/// 单指标隔离用权重：全给 MA。
pub const TREND_MA_ONLY_MA: f64 = 1.0;
pub const TREND_MA_ONLY_RSI: f64 = 0.0;
pub const TREND_MA_ONLY_VIX: f64 = 0.0;

/// 单指标隔离用权重：全给 RSI。
pub const TREND_RSI_ONLY_MA: f64 = 0.0;
pub const TREND_RSI_ONLY_RSI: f64 = 1.0;
pub const TREND_RSI_ONLY_VIX: f64 = 0.0;

/// 单指标隔离用权重：全给 VIX。
pub const TREND_VIX_ONLY_MA: f64 = 0.0;
pub const TREND_VIX_ONLY_RSI: f64 = 0.0;
pub const TREND_VIX_ONLY_VIX: f64 = 1.0;

/// 赶顶体制阈值（与默认 TrendConfig 保持一致）。
pub const TREND_OVERHEATED_ABOVE: f64 = 0.90;
/// 接飞刀体制阈值（与默认 TrendConfig 保持一致）。
pub const TREND_FALLING_KNIFE_ABOVE: f64 = 0.90;

/// 构造趋势测试用分位配置（低门槛历史长度）。
pub fn trend_test_percentile_config() -> EwPercentileConfig {
    EwPercentileConfig::from_half_life(TREND_TEST_HALF_LIFE, TREND_TEST_MIN_HISTORY_LEN)
        .expect("测试趋势分位配置有效")
}

/// 构造趋势测试用 TrendConfig（低门槛历史、均衡权重、默认阈值）。
pub fn trend_balanced_test_config() -> TrendConfig {
    let weights = TrendWeights::new(
        TREND_EQUAL_MA_WEIGHT,
        TREND_EQUAL_RSI_WEIGHT,
        TREND_EQUAL_VIX_WEIGHT,
    )
    .expect("均衡三子权重有效");
    TrendConfig::new(
        weights,
        trend_test_percentile_config(),
        TREND_OVERHEATED_ABOVE,
        TREND_FALLING_KNIFE_ABOVE,
    )
    .expect("趋势测试配置有效")
}

/// 构造趋势测试用 TrendConfig，仅使用指定权重组合。
pub fn trend_config_with_weights(ma: f64, rsi: f64, vix: f64) -> TrendConfig {
    let weights = TrendWeights::new(ma, rsi, vix).expect("测试权重有效");
    TrendConfig::new(
        weights,
        trend_test_percentile_config(),
        TREND_OVERHEATED_ABOVE,
        TREND_FALLING_KNIFE_ABOVE,
    )
    .expect("趋势测试配置有效")
}

/// 趋势中性夹具使用的当前读数。
pub const TREND_NEUTRAL_CURRENT: f64 = 50.0;

/// 构造加权 ECDF 语义上的中性趋势历史。
///
/// 序列按 `[低于 current, 高于 current]` 成对交错排列；由于指数加权 ECDF 对越新的
/// 样本赋予更高权重，成对交错比单调递增历史更接近“加权后约 0.5”的中性语义。
pub fn neutral_weighted_history() -> Vec<f64> {
    (0..STANDARD_HISTORY_LEN)
        .map(|i| {
            if i % 2 == 0 {
                TREND_NEUTRAL_CURRENT - 1.0
            } else {
                TREND_NEUTRAL_CURRENT + 1.0
            }
        })
        .collect()
}

/// 构造标准中性趋势快照：三指标历史在时间权重上围绕 current 成对交错。
pub fn neutral_trend_snapshot() -> TrendSnapshot {
    let history = neutral_weighted_history();
    TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: TREND_NEUTRAL_CURRENT,
        rsi_history: history.clone(),
        rsi_current: TREND_NEUTRAL_CURRENT,
        vix_history: history.clone(),
        vix_current: TREND_NEUTRAL_CURRENT,
    }
}

/// 构造「强势上涨/赶顶」快照：MA 距离与 RSI 极高，VIX 极低。
pub fn overheated_trend_snapshot() -> TrendSnapshot {
    let history = standard_history();
    TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 9999.0, // 远高于全部历史 → 分位 → 1.0
        rsi_history: history.clone(),
        rsi_current: 9999.0,
        vix_history: history.clone(),
        vix_current: 0.0001, // 极低 → 分位 → 0.0
    }
}

/// 构造「急跌/接飞刀」快照：VIX 极高，MA 距离与 RSI 极低。
pub fn falling_knife_trend_snapshot() -> TrendSnapshot {
    let history = standard_history();
    TrendSnapshot {
        ma_distance_history: history.clone(),
        ma_distance_current: 0.0001, // 极低 → 分位 → 0.0
        rsi_history: history.clone(),
        rsi_current: 0.0001,
        vix_history: history.clone(),
        vix_current: 9999.0, // 极高 → 分位 → 1.0
    }
}

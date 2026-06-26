//! 跨 Quant Engine 各层共享的配置权重类型。

use crate::QuantError;

/// 配置权重，保证在 `[0.0, 1.0]` 区间内。
///
/// 用于表达各指标在综合得分中的占比。`0.0` 表示完全不使用该指标，
/// `1.0` 表示完全使用该指标。
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Weight(f64);

impl Weight {
    /// 构造一个 [`Weight`]。
    ///
    /// 若输入不是有限数或不在 `[0.0, 1.0]` 区间内，返回 [`QuantError::InvalidWeight`]。
    pub fn new(value: f64) -> Result<Self, QuantError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            Err(QuantError::InvalidWeight { value })
        } else {
            Ok(Self(value))
        }
    }

    /// 返回底层 `f64` 值。
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }

    /// 返回互补权重：`1.0 - self`。
    pub fn complement(self) -> Self {
        Self(1.0 - self.0)
    }
}

impl TryFrom<f64> for Weight {
    type Error = QuantError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Weight> for f64 {
    fn from(value: Weight) -> Self {
        value.0
    }
}

impl std::fmt::Display for Weight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0 * 100.0)
    }
}

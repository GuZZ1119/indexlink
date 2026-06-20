#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! 核心领域类型。
//!
//! 此 crate 只包含纯数据结构，**无任何 IO**，供所有其他 crate 引用。

// ─── Percentile ─────────────────────────────────────────────────────────────

/// 历史分位值，保证在 `[0.0, 1.0]` 区间内。
///
/// - `0.0` = 处于历史最低位（最便宜）
/// - `1.0` = 处于历史最高位（最贵）
///
/// 使用 newtype 模式封装，防止在代码中误传裸 `f64`。
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Percentile(f64);

/// 构造 [`Percentile`] 失败的原因。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PercentileError {
    /// 输入值为 NaN。
    Nan,
    /// 输入值不在 `[0.0, 1.0]` 区间内。
    OutOfRange {
        /// 越界的原始输入值。
        value: f64,
    },
}

impl std::fmt::Display for PercentileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nan => write!(f, "percentile must not be NaN"),
            Self::OutOfRange { value } => {
                write!(f, "percentile must be in [0.0, 1.0], got {value}")
            }
        }
    }
}

impl std::error::Error for PercentileError {}

impl Percentile {
    /// 构造一个 [`Percentile`]。若值不在 `[0.0, 1.0]` 或为 NaN，则返回 `None`。
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        Self::try_from(value).ok()
    }

    /// 返回底层f64值
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }

    /// 返回倒置分位：`1.0 - self`。
    ///
    /// 用于ERP等反向指标：ERP越高代表市场越便宜，
    /// 倒置后才与 CAPE 方向一致（值越大 = 市场越贵）。
    pub fn invert(self) -> Self {
        Self(1.0 - self.0)
    }
}

impl TryFrom<f64> for Percentile {
    type Error = PercentileError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_nan() {
            Err(PercentileError::Nan)
        } else if !(0.0..=1.0).contains(&value) {
            Err(PercentileError::OutOfRange { value })
        } else {
            Ok(Self(value))
        }
    }
}

impl From<Percentile> for f64 {
    fn from(value: Percentile) -> Self {
        value.0
    }
}

impl std::fmt::Display for Percentile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0 * 100.0)
    }
}

// ─── Action ─────────────────────────────────────────────────────────────────

/// 系统可输出的五种定投动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// 加码 +20%~50%；历史深度低位触发。
    Overweight,
    /// 标准金额 100%；历史中性区间触发。
    Standard,
    /// 延迟 3-5 天执行；重大宏观事件或技术过热触发。
    TacticalDelay,
    /// 减量 -50%；历史高位触发。
    Underweight,
    /// 本期跳过；极端高估或系统性风险触发。
    Skip,
}

// ─── Multiplier ─────────────────────────────────────────────────────────────

/// 定投倍率，硬限制在 `[0.0, 1.5]`。
///
/// - `0.0` = Skip（不投）
/// - `1.0` = Standard（100%）
/// - `1.5` = 最大 Overweight（150%）
///
/// AI 或任何上游模块**无法突破**此上限，这是金融安全边界。
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Multiplier(f64);

impl Multiplier {
    const MIN_VALUE: f64 = 0.0;
    const MAX_VALUE: f64 = 1.5;
    const SKIP_BELOW_VALUE: f64 = 0.05;
    const UNDERWEIGHT_BELOW_VALUE: f64 = 0.75;
    const STANDARD_MAX_VALUE: f64 = 1.05;

    /// 最小定投倍率，对应跳过本期定投。
    pub const MIN: Self = Self(Self::MIN_VALUE);
    /// 最大定投倍率，对应 150% 定投金额上限。
    pub const MAX: Self = Self(Self::MAX_VALUE);
    /// 小于此倍率时映射为 [`Action::Skip`]。
    pub const SKIP_BELOW: Self = Self(Self::SKIP_BELOW_VALUE);
    /// 小于此倍率时映射为 [`Action::Underweight`]。
    pub const UNDERWEIGHT_BELOW: Self = Self(Self::UNDERWEIGHT_BELOW_VALUE);
    /// 标准定投动作的最大倍率，包含边界值。
    pub const STANDARD_MAX: Self = Self(Self::STANDARD_MAX_VALUE);

    /// 创建倍率，自动 clamp 到 `[0.0, 1.5]`；NaN 视为最保守的 Skip 倍率。
    pub fn new_clamped(value: f64) -> Self {
        let v = if value.is_nan() {
            Self::MIN_VALUE
        } else {
            value
        };
        Self(v.clamp(Self::MIN_VALUE, Self::MAX_VALUE))
    }

    /// 返回底层倍率数值。
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }

    /// 将倍率映射到对应的 [`Action`] 区间标签。
    ///
    /// 注意：[`Action::TacticalDelay`] 由趋势层单独判断，不在此映射范围内。
    #[must_use]
    pub fn to_action(self) -> Action {
        let v = self.0;

        if !(Self::MIN_VALUE..=Self::MAX_VALUE).contains(&v) {
            unreachable!("Multiplier value must be finite and in [0.0, 1.5]");
        }

        if v < Self::SKIP_BELOW_VALUE {
            Action::Skip
        } else if v < Self::UNDERWEIGHT_BELOW_VALUE {
            Action::Underweight
        } else if v <= Self::STANDARD_MAX_VALUE {
            Action::Standard
        } else {
            Action::Overweight
        }
    }
}

impl std::fmt::Display for Multiplier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0 * 100.0)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_valid_range() {
        assert!(Percentile::new(0.0).is_some());
        assert!(Percentile::new(0.5).is_some());
        assert!(Percentile::new(1.0).is_some());
    }

    #[test]
    fn percentile_out_of_range() {
        assert!(Percentile::new(-0.001).is_none());
        assert!(Percentile::new(1.001).is_none());
        assert!(Percentile::new(f64::NAN).is_none());
    }

    #[test]
    fn percentile_try_from_reports_failure_reason() {
        assert_eq!(Percentile::try_from(f64::NAN), Err(PercentileError::Nan));
        assert_eq!(
            Percentile::try_from(-0.001),
            Err(PercentileError::OutOfRange { value: -0.001 })
        );
        assert_eq!(
            Percentile::try_from(1.001),
            Err(PercentileError::OutOfRange { value: 1.001 })
        );
    }

    #[test]
    fn percentile_error_display() {
        assert_eq!(
            PercentileError::Nan.to_string(),
            "percentile must not be NaN"
        );
        assert_eq!(
            PercentileError::OutOfRange { value: 1.2 }.to_string(),
            "percentile must be in [0.0, 1.0], got 1.2"
        );
    }

    #[test]
    fn percentile_display() {
        let p = Percentile::new(0.7).unwrap();

        assert_eq!(p.to_string(), "70.0%");
    }

    #[test]
    fn percentile_converts_into_f64() {
        let p = Percentile::new(0.7).unwrap();
        let value: f64 = p.into();

        assert_eq!(value, 0.7);
    }

    #[test]
    fn percentile_invert() {
        let p = Percentile::new(0.7).unwrap();
        assert!((p.invert().value() - 0.3).abs() < f64::EPSILON * 10.0);
    }

    #[test]
    fn multiplier_clamp() {
        assert_eq!(Multiplier::new_clamped(-1.0), Multiplier::MIN);
        assert_eq!(Multiplier::new_clamped(99.0), Multiplier::MAX);
        assert_eq!(Multiplier::new_clamped(1.0).value(), 1.0);
        assert_eq!(Multiplier::new_clamped(f64::NAN), Multiplier::MIN);
    }

    #[test]
    fn multiplier_display() {
        assert_eq!(Multiplier::new_clamped(1.2).to_string(), "120.0%");
        assert_eq!(Multiplier::MAX.to_string(), "150.0%");
    }

    #[test]
    fn multiplier_to_action() {
        assert_eq!(Multiplier::new_clamped(0.0).to_action(), Action::Skip);
        assert_eq!(
            Multiplier::new_clamped(0.5).to_action(),
            Action::Underweight
        );
        assert_eq!(Multiplier::new_clamped(1.0).to_action(), Action::Standard);
        assert_eq!(Multiplier::new_clamped(1.4).to_action(), Action::Overweight);
    }

    #[test]
    fn multiplier_to_action_boundaries() {
        assert_eq!(Multiplier::SKIP_BELOW.to_action(), Action::Underweight);
        assert_eq!(Multiplier::UNDERWEIGHT_BELOW.to_action(), Action::Standard);
        assert_eq!(Multiplier::STANDARD_MAX.to_action(), Action::Standard);
    }

    #[test]
    #[should_panic(expected = "Multiplier value must be finite and in [0.0, 1.5]")]
    fn multiplier_to_action_rejects_invalid_internal_value() {
        let _ = Multiplier(f64::NAN).to_action();
    }

    #[test]
    #[should_panic(expected = "Multiplier value must be finite and in [0.0, 1.5]")]
    fn multiplier_to_action_rejects_negative_internal_value() {
        let _ = Multiplier(-0.01).to_action();
    }
}

//! AI 情绪得分新类型。
//!
//! [`Sentiment`] 是 `[-1.0, +1.0]` 区间内的有界浮点数，
//! 代表 LLM 对财经新闻/财报的语义感知结果。

// ─── SentimentError ──────────────────────────────────────────────────────────

/// 构造 [`Sentiment`] 失败的原因。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SentimentError {
    /// 输入值为 NaN。
    Nan,
    /// 输入值不在 `[-1.0, 1.0]` 区间内。
    OutOfRange {
        /// 越界的原始输入值。
        value: f64,
    },
}

impl std::fmt::Display for SentimentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nan => write!(f, "sentiment must not be NaN"),
            Self::OutOfRange { value } => {
                write!(f, "sentiment must be in [-1.0, 1.0], got {value}")
            }
        }
    }
}

impl std::error::Error for SentimentError {}

// ─── Sentiment ───────────────────────────────────────────────────────────────

/// AI 情绪得分，硬限制在 `[-1.0, +1.0]`。
///
/// - `-1.0` = 极度悲观（熊市恐慌、系统性风险）
/// - `0.0` = 中性（无明显方向信号）
/// - `+1.0` = 极度乐观（盈利超预期、宏观利好）
///
/// AI 输出**无法突破**此边界：超限值会被 clamp，NaN 视为中性。
///
/// # 注意
///
/// 此类型只表达 AI 语义分析结果。AI 不可用时的降级（70/20/10 → 90/10/0）
/// 发生在 decision engine 层，不在 ai-client 层。ai-client 在 AI 不可用时
/// 返回 [`crate::AiClientError`]，由 engine 决定如何处理权重分配。
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Sentiment(f64);

impl Sentiment {
    /// 情绪值下界。
    pub const MIN_VALUE: f64 = -1.0;
    /// 情绪值上界。
    pub const MAX_VALUE: f64 = 1.0;
    /// 中性情绪值。
    pub const NEUTRAL_VALUE: f64 = 0.0;

    /// 情绪最小值（-1.0）。
    pub const MIN: Self = Self(Self::MIN_VALUE);
    /// 情绪最大值（+1.0）。
    pub const MAX: Self = Self(Self::MAX_VALUE);
    /// 中性情绪（0.0）——无明显方向信号时的语义值。
    pub const NEUTRAL: Self = Self(Self::NEUTRAL_VALUE);

    /// 构造情绪值，自动 clamp 到 `[-1.0, +1.0]`；NaN 视为中性。
    ///
    /// 这是从 LLM 输出解析时的**主要入口**——AI 输出不可靠，
    /// clamp 保证安全性，绝不因异常输入而 panic。
    pub fn new_clamped(value: f64) -> Self {
        let v = if value.is_nan() {
            Self::NEUTRAL_VALUE
        } else {
            value
        };
        Self(v.clamp(Self::MIN_VALUE, Self::MAX_VALUE))
    }

    /// 构造情绪值。若值不在 `[-1.0, +1.0]` 或为 NaN，则返回 `None`。
    ///
    /// 适用于调用方已确认值在有效范围内的场景（如测试、反序列化校验）。
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        Self::try_from(value).ok()
    }

    /// 返回中性情绪值（0.0），即无方向信号。
    #[must_use = "returns a Sentiment value that should not be discarded"]
    pub fn neutral() -> Self {
        Self::NEUTRAL
    }

    /// 返回底层 `f64` 值。
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Sentiment {
    type Error = SentimentError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_nan() {
            Err(SentimentError::Nan)
        } else if !(Self::MIN_VALUE..=Self::MAX_VALUE).contains(&value) {
            Err(SentimentError::OutOfRange { value })
        } else {
            Ok(Self(value))
        }
    }
}

impl From<Sentiment> for f64 {
    fn from(value: Sentiment) -> Self {
        value.0
    }
}

impl std::fmt::Display for Sentiment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.0 > 0.0 { "+" } else { "-" };
        let abs = self.0.abs();
        if abs == 0.0 {
            write!(f, "0.0")
        } else if abs.fract() == 0.0 {
            // 整数（如 1.0 / -1.0）保留一位小数，避免输出 "+1"
            write!(f, "{sign}{abs:.1}")
        } else {
            // 使用 ryu 最短表示，避免 {:.1} 的四舍五入精度丢失（如 0.75 → +0.8）
            write!(f, "{sign}{}", abs)
        }
    }
}

/// 一次 AI 市场情绪分析的可审计输出。
///
/// 分数仍由 [`Sentiment`] 保证在 `[-1.0, 1.0]` 内；依据和风险提示仅用于
/// 解释本次模型输出，不构成投资建议或事实断言。
#[derive(Debug, Clone, PartialEq)]
pub struct SentimentAnalysis {
    sentiment: Sentiment,
    rationale: String,
    warnings: Vec<String>,
}

impl SentimentAnalysis {
    /// 创建一条受长度限制的结构化 AI 分析。
    ///
    /// 空白依据会被拒绝，防止调用方把“只有分数”的结果伪装成可解释分析。
    pub fn new(
        sentiment: Sentiment,
        rationale: String,
        warnings: Vec<String>,
    ) -> Result<Self, SentimentAnalysisError> {
        let rationale = normalize_text(rationale, MAX_RATIONALE_CHARS)
            .ok_or(SentimentAnalysisError::InvalidRationale)?;
        if warnings.len() > MAX_WARNING_COUNT {
            return Err(SentimentAnalysisError::TooManyWarnings);
        }
        let warnings = warnings
            .into_iter()
            .map(|warning| {
                normalize_text(warning, MAX_WARNING_CHARS)
                    .ok_or(SentimentAnalysisError::InvalidWarning)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            sentiment,
            rationale,
            warnings,
        })
    }

    /// 返回本次有界情绪得分。
    pub fn sentiment(&self) -> Sentiment {
        self.sentiment
    }

    /// 返回模型给出的简短依据。
    #[must_use]
    pub fn rationale(&self) -> &str {
        &self.rationale
    }

    /// 返回模型给出的风险提示。
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// 结构化 AI 分析的校验错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SentimentAnalysisError {
    /// 模型未返回可展示的分析依据。
    #[error("AI rationale must be non-empty and within the supported length")]
    InvalidRationale,
    /// 模型返回了过多风险提示。
    #[error("AI returned too many warnings")]
    TooManyWarnings,
    /// 某条风险提示为空或过长。
    #[error("AI warning must be non-empty and within the supported length")]
    InvalidWarning,
}

const MAX_RATIONALE_CHARS: usize = 1_000;
const MAX_WARNING_COUNT: usize = 5;
const MAX_WARNING_CHARS: usize = 300;

fn normalize_text(value: String, max_chars: usize) -> Option<String> {
    let normalized = value.trim();
    (!normalized.is_empty() && normalized.chars().count() <= max_chars)
        .then(|| normalized.to_owned())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── new (strict) ──────────────────────────────────────────────────────

    #[test]
    fn sentiment_new_valid_range() {
        assert!(Sentiment::new(-1.0).is_some());
        assert!(Sentiment::new(-0.5).is_some());
        assert!(Sentiment::new(0.0).is_some());
        assert!(Sentiment::new(0.5).is_some());
        assert!(Sentiment::new(1.0).is_some());
    }

    #[test]
    fn sentiment_new_out_of_range() {
        assert!(Sentiment::new(-1.001).is_none());
        assert!(Sentiment::new(1.001).is_none());
        assert!(Sentiment::new(f64::NAN).is_none());
    }

    #[test]
    fn sentiment_new_rejects_infinity() {
        assert!(Sentiment::new(f64::INFINITY).is_none());
        assert!(Sentiment::new(f64::NEG_INFINITY).is_none());
    }

    // ── new_clamped ───────────────────────────────────────────────────────

    #[test]
    fn sentiment_new_clamped_inside_range() {
        assert_eq!(Sentiment::new_clamped(-0.5).value(), -0.5);
        assert_eq!(Sentiment::new_clamped(0.0).value(), 0.0);
        assert_eq!(Sentiment::new_clamped(0.7).value(), 0.7);
    }

    #[test]
    fn sentiment_new_clamped_above_range() {
        assert_eq!(Sentiment::new_clamped(5.0), Sentiment::MAX);
        assert_eq!(Sentiment::new_clamped(99.0), Sentiment::MAX);
    }

    #[test]
    fn sentiment_new_clamped_below_range() {
        assert_eq!(Sentiment::new_clamped(-5.0), Sentiment::MIN);
        assert_eq!(Sentiment::new_clamped(-99.0), Sentiment::MIN);
    }

    #[test]
    fn sentiment_new_clamped_nan_is_neutral() {
        assert_eq!(Sentiment::new_clamped(f64::NAN), Sentiment::NEUTRAL);
    }

    #[test]
    fn sentiment_new_clamped_infinity_is_clamped() {
        assert_eq!(Sentiment::new_clamped(f64::INFINITY), Sentiment::MAX);
        assert_eq!(Sentiment::new_clamped(f64::NEG_INFINITY), Sentiment::MIN);
    }

    // ── neutral ───────────────────────────────────────────────────────────

    #[test]
    fn sentiment_neutral_is_zero() {
        assert_eq!(Sentiment::neutral().value(), 0.0);
        assert_eq!(Sentiment::neutral(), Sentiment::NEUTRAL);
    }

    // ── TryFrom / Error ───────────────────────────────────────────────────

    #[test]
    fn sentiment_try_from_reports_failure_reason() {
        assert_eq!(Sentiment::try_from(f64::NAN), Err(SentimentError::Nan));
        assert_eq!(
            Sentiment::try_from(-1.001),
            Err(SentimentError::OutOfRange { value: -1.001 })
        );
        assert_eq!(
            Sentiment::try_from(1.001),
            Err(SentimentError::OutOfRange { value: 1.001 })
        );
    }

    #[test]
    fn sentiment_error_display() {
        assert_eq!(SentimentError::Nan.to_string(), "sentiment must not be NaN");
        assert_eq!(
            SentimentError::OutOfRange { value: -2.0 }.to_string(),
            "sentiment must be in [-1.0, 1.0], got -2"
        );
    }

    // ── Display ───────────────────────────────────────────────────────────

    #[test]
    fn sentiment_display_positive() {
        let s = Sentiment::new(0.7).unwrap();
        assert_eq!(s.to_string(), "+0.7");
    }

    #[test]
    fn sentiment_display_negative() {
        let s = Sentiment::new(-0.5).unwrap();
        assert_eq!(s.to_string(), "-0.5");
    }

    #[test]
    fn sentiment_display_neutral() {
        let s = Sentiment::neutral();
        assert_eq!(s.to_string(), "0.0");
    }

    // ── From<Sentiment> for f64 ───────────────────────────────────────────

    #[test]
    fn sentiment_converts_into_f64() {
        let s = Sentiment::new(0.7).unwrap();
        let value: f64 = s.into();
        assert_eq!(value, 0.7);
    }

    // ── PartialOrd ────────────────────────────────────────────────────────

    #[test]
    fn sentiment_ordering() {
        let low = Sentiment::new(-0.8).unwrap();
        let mid = Sentiment::neutral();
        let high = Sentiment::new(0.8).unwrap();

        assert!(low < mid);
        assert!(mid < high);
        assert!(low < high);
    }

    #[test]
    fn analysis_retains_bounded_score_and_evidence() {
        let analysis = SentimentAnalysis::new(
            Sentiment::new(0.2).unwrap(),
            "Inflation data softened.".to_owned(),
            vec!["The sample is limited.".to_owned()],
        )
        .unwrap();

        assert_eq!(analysis.sentiment().value(), 0.2);
        assert_eq!(analysis.rationale(), "Inflation data softened.");
        assert_eq!(analysis.warnings(), &["The sample is limited.".to_owned()]);
    }

    #[test]
    fn analysis_rejects_blank_or_excessive_evidence() {
        assert_eq!(
            SentimentAnalysis::new(Sentiment::NEUTRAL, "  ".to_owned(), Vec::new()),
            Err(SentimentAnalysisError::InvalidRationale)
        );
        assert_eq!(
            SentimentAnalysis::new(
                Sentiment::NEUTRAL,
                "Valid rationale.".to_owned(),
                vec!["warning".to_owned(); 6],
            ),
            Err(SentimentAnalysisError::TooManyWarnings)
        );
    }
}

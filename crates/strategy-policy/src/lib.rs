#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! 与具体指标、数据源和执行适配器无关的投资策略领域契约。
//!
//! 本 crate 不进行 IO，也不依赖 HTTP、SQLite、Qwen 或 broker。策略只接收已经
//! 解析完成的 [`DecisionContext`]，并返回确定性的 [`InvestmentRecommendation`]。

use core_domain::{Action, Multiplier};
use rust_decimal::Decimal;
use serde::Serialize;
use time::Date;

const MAX_POLICY_ID_LEN: usize = 64;

/// 已校验的稳定策略标识。
///
/// 标识仅允许小写 ASCII 字母、数字与下划线，且必须以字母开头。例如
/// `core_opportunity_v1` 或 `fixed_dca`。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct PolicyId(String);

impl PolicyId {
    /// 构造一个已校验的策略标识。
    pub fn new(value: impl Into<String>) -> Result<Self, PolicyValidationError> {
        let value = value.into();
        let is_valid = !value.is_empty()
            && value.len() <= MAX_POLICY_ID_LEN
            && value.is_ascii()
            && value.bytes().enumerate().all(|(index, byte)| match byte {
                b'a'..=b'z' => true,
                b'0'..=b'9' | b'_' => index > 0,
                _ => false,
            });

        if is_valid {
            Ok(Self(value))
        } else {
            Err(PolicyValidationError::InvalidPolicyId)
        }
    }

    /// 返回策略标识的规范字符串。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PolicyId {
    type Error = PolicyValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::fmt::Display for PolicyId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// 不可变策略版本。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct PolicyVersion(u32);

impl PolicyVersion {
    /// 构造一个大于零的策略版本。
    pub fn new(value: u32) -> Result<Self, PolicyValidationError> {
        if value == 0 {
            Err(PolicyValidationError::InvalidPolicyVersion)
        } else {
            Ok(Self(value))
        }
    }

    /// 返回数值版本。
    #[must_use]
    pub fn value(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for PolicyVersion {
    type Error = PolicyValidationError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::fmt::Display for PolicyVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// 策略标识与不可变版本组成的引用。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct PolicyRef {
    id: PolicyId,
    version: PolicyVersion,
}

impl PolicyRef {
    /// 构造一个策略版本引用。
    #[must_use]
    pub fn new(id: PolicyId, version: PolicyVersion) -> Self {
        Self { id, version }
    }

    /// 返回策略标识。
    #[must_use]
    pub fn id(&self) -> &PolicyId {
        &self.id
    }

    /// 返回策略版本。
    #[must_use]
    pub fn version(&self) -> PolicyVersion {
        self.version
    }
}

impl std::fmt::Display for PolicyRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}@{}", self.id, self.version)
    }
}

/// 一次策略评估已解析完成的确定性上下文。
///
/// `Evidence` 是策略专属、但已经由应用层解析和校验的输入。策略运行时不应在此
/// 边界内读取网络、数据库、环境变量或 broker。
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionContext<Evidence> {
    as_of: Date,
    scheduled_contribution: Decimal,
    evidence: Evidence,
}

impl<Evidence> DecisionContext<Evidence> {
    /// 使用已验证的执行日期、周期预算和策略证据构造上下文。
    pub fn new(
        as_of: Date,
        scheduled_contribution: Decimal,
        evidence: Evidence,
    ) -> Result<Self, PolicyValidationError> {
        if scheduled_contribution <= Decimal::ZERO {
            return Err(PolicyValidationError::InvalidScheduledContribution);
        }

        Ok(Self {
            as_of,
            scheduled_contribution,
            evidence,
        })
    }

    /// 返回本次决策可使用的市场数据截至日期。
    #[must_use]
    pub fn as_of(&self) -> Date {
        self.as_of
    }

    /// 返回本次周期的计划投入预算。
    #[must_use]
    pub fn scheduled_contribution(&self) -> Decimal {
        self.scheduled_contribution
    }

    /// 返回策略专属的已解析证据。
    #[must_use]
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
}

/// 一次策略评估的通用、尚未执行的推荐结果。
///
/// 此类型表达策略层的动作和倍率，而不直接生成 broker order。计划服务仍负责将推荐
/// 应用于双桶、周期额度、可用现金和人工审批等执行约束。
#[derive(Debug, Clone, PartialEq)]
pub struct InvestmentRecommendation {
    policy: PolicyRef,
    action: Action,
    multiplier: Multiplier,
    scheduled_contribution: Decimal,
}

impl InvestmentRecommendation {
    /// 从已验证上下文构造一个未执行的策略推荐。
    #[must_use]
    pub fn from_context<Evidence>(
        policy: PolicyRef,
        context: &DecisionContext<Evidence>,
        action: Action,
        multiplier: Multiplier,
    ) -> Self {
        Self {
            policy,
            action,
            multiplier,
            scheduled_contribution: context.scheduled_contribution(),
        }
    }

    /// 返回生成本次推荐的策略版本。
    #[must_use]
    pub fn policy(&self) -> &PolicyRef {
        &self.policy
    }

    /// 返回策略动作标签。
    #[must_use]
    pub fn action(&self) -> Action {
        self.action
    }

    /// 返回策略建议的有界倍率。
    pub fn multiplier(&self) -> Multiplier {
        self.multiplier
    }

    /// 返回评估时的周期计划预算。
    #[must_use]
    pub fn scheduled_contribution(&self) -> Decimal {
        self.scheduled_contribution
    }
}

/// 所有内置或用户定义策略都应实现的确定性评估契约。
///
/// `Evidence` 由具体策略声明，避免平台级接口直接依赖 CAPE、ERP、RSI、VIX 或
/// 某一策略的专属类型。后续 resolver 会在策略版本确认后提供匹配的证据类型。
pub trait InvestmentPolicy {
    /// 此策略需要的已解析证据类型。
    type Evidence;

    /// 返回此策略的稳定标识与不可变版本。
    fn policy_ref(&self) -> PolicyRef;

    /// 在完整、无 IO 的上下文上生成确定性推荐。
    fn evaluate(&self, context: &DecisionContext<Self::Evidence>) -> InvestmentRecommendation;
}

/// 策略领域输入未通过校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PolicyValidationError {
    /// 策略标识不是长度不超过 64 的小写 ASCII 标识符。
    #[error("policy id must be a lowercase ASCII identifier up to 64 characters")]
    InvalidPolicyId,
    /// 策略版本必须大于零。
    #[error("policy version must be greater than zero")]
    InvalidPolicyVersion,
    /// 周期计划投入金额必须大于零。
    #[error("scheduled contribution must be greater than zero")]
    InvalidScheduledContribution,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    /// Verify policy references keep identifier and version validation at the boundary.
    #[test]
    fn policy_reference_requires_a_valid_identifier_and_non_zero_version() {
        let id = PolicyId::new("fixed_dca").unwrap();
        let version = PolicyVersion::new(1).unwrap();
        let policy = PolicyRef::new(id, version);

        assert_eq!(policy.to_string(), "fixed_dca@1");
        assert!(PolicyId::new("Fixed-Dca").is_err());
        assert!(PolicyVersion::new(0).is_err());
    }

    /// Verify a context rejects a non-positive scheduled contribution before evaluation.
    #[test]
    fn context_requires_a_positive_scheduled_contribution() {
        let date = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        assert_eq!(
            DecisionContext::new(date, Decimal::ZERO, ()),
            Err(PolicyValidationError::InvalidScheduledContribution)
        );
    }
}

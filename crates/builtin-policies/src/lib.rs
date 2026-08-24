#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! IndexLink 内置策略实现。
//!
//! 本 crate 仅包含无 IO 的策略适配器。当前 [`CoreOpportunityV1`] 包装既有
//! 70/20/10 决策引擎，不改变它的计算、权重、动作或降级语义。

use decision_engine::{evaluate_decision, DecisionConfig, DecisionInput, DecisionSignal};
use rust_decimal::Decimal;
use strategy_policy::{
    DecisionContext, InvestmentPolicy, InvestmentRecommendation, PolicyId, PolicyRef,
    PolicyValidationError, PolicyVersion,
};
use time::Date;

/// `CoreOpportunityV1` 的稳定策略标识。
pub const CORE_OPPORTUNITY_V1_ID: &str = "core_opportunity_v1";
/// `CoreOpportunityV1` 的不可变初始版本。
pub const CORE_OPPORTUNITY_V1_VERSION: u32 = 1;
/// `FixedDcaPolicy` 的稳定策略标识。
pub const FIXED_DCA_POLICY_ID: &str = "fixed_dca";
/// `FixedDcaPolicy` 的不可变初始版本。
pub const FIXED_DCA_POLICY_VERSION: u32 = 1;

/// `CoreOpportunityV1` 评估所需的旧 70/20/10 输入。
///
/// 该类型是迁移期适配边界：未来其他策略可以声明不同的证据类型，平台级策略契约
/// 本身不会依赖基本面、趋势或 AI 情绪的具体含义。
#[derive(Debug, Clone, PartialEq)]
pub struct CoreOpportunityEvidence {
    input: DecisionInput,
}

impl CoreOpportunityEvidence {
    /// 使用既有决策输入构造已解析的策略证据。
    #[must_use]
    pub fn new(input: DecisionInput) -> Self {
        Self { input }
    }

    /// 返回未修改的既有 70/20/10 决策输入。
    #[must_use]
    pub fn input(&self) -> &DecisionInput {
        &self.input
    }
}

/// 保持既有 70/20/10 行为不变的内置策略包装器。
#[derive(Debug, Clone, PartialEq)]
pub struct CoreOpportunityV1 {
    config: DecisionConfig,
}

impl CoreOpportunityV1 {
    /// 使用指定的既有决策配置构造策略包装器。
    #[must_use]
    pub fn new(config: DecisionConfig) -> Self {
        Self { config }
    }

    /// 返回策略包装器使用的既有决策配置。
    #[must_use]
    pub fn config(&self) -> &DecisionConfig {
        &self.config
    }

    /// 运行未修改的既有决策引擎，供审计适配和回归测试使用。
    #[must_use]
    pub fn evaluate_legacy(&self, evidence: &CoreOpportunityEvidence) -> DecisionSignal {
        evaluate_decision(evidence.input(), &self.config)
    }
}

impl Default for CoreOpportunityV1 {
    fn default() -> Self {
        Self::new(DecisionConfig::default())
    }
}

impl InvestmentPolicy for CoreOpportunityV1 {
    type Evidence = CoreOpportunityEvidence;

    fn policy_ref(&self) -> PolicyRef {
        PolicyRef::new(
            PolicyId::new(CORE_OPPORTUNITY_V1_ID)
                .expect("CoreOpportunityV1 has a valid static policy id"),
            PolicyVersion::new(CORE_OPPORTUNITY_V1_VERSION)
                .expect("CoreOpportunityV1 has a non-zero static policy version"),
        )
    }

    fn evaluate(&self, context: &DecisionContext<Self::Evidence>) -> InvestmentRecommendation {
        let decision = self.evaluate_legacy(context.evidence());
        InvestmentRecommendation::from_context(
            self.policy_ref(),
            context,
            decision.action,
            decision.multiplier,
        )
    }
}

/// 无信号否决、按周期预算固定投入的内置 DCA 策略。
///
/// 它只输出 `Standard` 与 `1.0x`，不读取 CAPE、趋势、AI 或外部状态。计划服务仍会
/// 应用单次上限、周期上限、可用现金与 paper-only 下单安全边界。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FixedDcaPolicy;

impl InvestmentPolicy for FixedDcaPolicy {
    type Evidence = ();

    fn policy_ref(&self) -> PolicyRef {
        PolicyRef::new(
            PolicyId::new(FIXED_DCA_POLICY_ID)
                .expect("FixedDcaPolicy has a valid static policy id"),
            PolicyVersion::new(FIXED_DCA_POLICY_VERSION)
                .expect("FixedDcaPolicy has a non-zero static policy version"),
        )
    }

    fn evaluate(&self, context: &DecisionContext<Self::Evidence>) -> InvestmentRecommendation {
        InvestmentRecommendation::from_context(
            self.policy_ref(),
            context,
            core_domain::Action::Standard,
            core_domain::Multiplier::new_clamped(1.0),
        )
    }
}

/// Resolver 可接受的内置策略证据。
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinPolicyEvidence {
    /// 旧 70/20/10 策略的完整输入。
    CoreOpportunity(CoreOpportunityEvidence),
    /// 固定 DCA 不需要市场信号。
    FixedDca,
}

/// 一项内置策略在评估前需要的证据种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinPolicyEvidenceKind {
    /// 既有策略需要基本面、趋势与可选的 AI 情绪。
    CoreOpportunity,
    /// 固定 DCA 不读取市场信号。
    FixedDca,
}

/// 一次内置策略评估的结果。
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinPolicyDecision {
    /// 旧策略结果，保留完整信号以支持兼容审计快照。
    CoreOpportunity {
        /// 对外通用的策略推荐。
        recommendation: InvestmentRecommendation,
        /// 未修改的既有决策信号。
        signal: DecisionSignal,
    },
    /// 固定 DCA 结果，不伪造未使用的市场信号。
    FixedDca {
        /// 对外通用的策略推荐。
        recommendation: InvestmentRecommendation,
    },
}

impl BuiltinPolicyDecision {
    /// 返回通用策略推荐。
    #[must_use]
    pub fn recommendation(&self) -> &InvestmentRecommendation {
        match self {
            Self::CoreOpportunity { recommendation, .. } | Self::FixedDca { recommendation } => {
                recommendation
            }
        }
    }

    /// 当且仅当旧策略被选择时返回其完整兼容决策信号。
    #[must_use]
    pub fn legacy_signal(&self) -> Option<&DecisionSignal> {
        match self {
            Self::CoreOpportunity { signal, .. } => Some(signal),
            Self::FixedDca { .. } => None,
        }
    }
}

/// 可选择当前受支持内置策略的无 IO resolver。
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltinPolicyResolver {
    core_opportunity: CoreOpportunityV1,
    fixed_dca: FixedDcaPolicy,
}

impl BuiltinPolicyResolver {
    /// 使用指定的 Legacy 决策配置构造 resolver。
    #[must_use]
    pub fn new(core_opportunity: CoreOpportunityV1, fixed_dca: FixedDcaPolicy) -> Self {
        Self {
            core_opportunity,
            fixed_dca,
        }
    }

    /// 返回当前可被该 resolver 执行的内置策略引用。
    #[must_use]
    pub fn supports(&self, policy: &PolicyRef) -> bool {
        *policy == self.core_opportunity.policy_ref() || *policy == self.fixed_dca.policy_ref()
    }

    /// 返回一个已支持策略所需的证据种类。
    pub fn evidence_kind(
        &self,
        policy: &PolicyRef,
    ) -> Result<BuiltinPolicyEvidenceKind, BuiltinPolicyError> {
        if *policy == self.core_opportunity.policy_ref() {
            Ok(BuiltinPolicyEvidenceKind::CoreOpportunity)
        } else if *policy == self.fixed_dca.policy_ref() {
            Ok(BuiltinPolicyEvidenceKind::FixedDca)
        } else {
            Err(BuiltinPolicyError::UnsupportedPolicy(policy.clone()))
        }
    }

    /// 根据策略引用和已解析证据生成通用推荐。
    pub fn evaluate(
        &self,
        policy: &PolicyRef,
        as_of: Date,
        scheduled_contribution: Decimal,
        evidence: BuiltinPolicyEvidence,
    ) -> Result<BuiltinPolicyDecision, BuiltinPolicyError> {
        if *policy == self.core_opportunity.policy_ref() {
            let BuiltinPolicyEvidence::CoreOpportunity(evidence) = evidence else {
                return Err(BuiltinPolicyError::EvidenceDoesNotMatchPolicy);
            };
            let context = DecisionContext::new(as_of, scheduled_contribution, evidence)?;
            let signal = self.core_opportunity.evaluate_legacy(context.evidence());
            let recommendation = self.core_opportunity.evaluate(&context);
            return Ok(BuiltinPolicyDecision::CoreOpportunity {
                recommendation,
                signal,
            });
        }

        if *policy == self.fixed_dca.policy_ref() {
            let BuiltinPolicyEvidence::FixedDca = evidence else {
                return Err(BuiltinPolicyError::EvidenceDoesNotMatchPolicy);
            };
            let context = DecisionContext::new(as_of, scheduled_contribution, ())?;
            return Ok(BuiltinPolicyDecision::FixedDca {
                recommendation: self.fixed_dca.evaluate(&context),
            });
        }

        Err(BuiltinPolicyError::UnsupportedPolicy(policy.clone()))
    }
}

impl Default for BuiltinPolicyResolver {
    fn default() -> Self {
        Self::new(CoreOpportunityV1::default(), FixedDcaPolicy)
    }
}

/// 内置策略 resolver 无法生成推荐时的安全错误。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BuiltinPolicyError {
    /// 计划引用的策略当前尚未注册为可执行内置策略。
    #[error("unsupported policy {0}")]
    UnsupportedPolicy(PolicyRef),
    /// 调用方提供的证据与被选择的策略不匹配。
    #[error("policy evidence does not match the selected policy")]
    EvidenceDoesNotMatchPolicy,
    /// 周期预算未通过策略领域校验。
    #[error(transparent)]
    InvalidContext(#[from] PolicyValidationError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_client::Sentiment;
    use core_domain::Percentile;
    use decision_engine::DecisionSentiment;
    use quant_engine::{FundamentalSignal, TrendRegime, TrendSignal};
    use rust_decimal::Decimal;
    use time::{Date, Month};

    fn percentile(value: f64) -> Percentile {
        Percentile::new(value).unwrap()
    }

    fn input(regime: TrendRegime, sentiment: DecisionSentiment) -> DecisionInput {
        DecisionInput {
            fundamental: FundamentalSignal {
                score: percentile(0.2),
                cape_percentile: percentile(0.2),
                erp_percentile: percentile(0.8),
            },
            trend: TrendSignal {
                score: percentile(0.5),
                ma_distance_percentile: percentile(0.5),
                rsi_percentile: percentile(0.5),
                vix_percentile: percentile(0.5),
                regime,
            },
            sentiment,
        }
    }

    fn context(input: DecisionInput) -> DecisionContext<CoreOpportunityEvidence> {
        DecisionContext::new(
            Date::from_calendar_date(2026, Month::January, 15).unwrap(),
            Decimal::new(1_000, 0),
            CoreOpportunityEvidence::new(input),
        )
        .unwrap()
    }

    /// Verify wrapping the legacy engine preserves its complete decision signal.
    #[test]
    fn wrapper_preserves_the_legacy_signal_without_changing_inputs() {
        let policy = CoreOpportunityV1::default();
        let input = input(
            TrendRegime::Neutral,
            DecisionSentiment::Available(Sentiment::new(0.25).unwrap()),
        );
        let expected = evaluate_decision(&input, &DecisionConfig::default());
        let actual = policy.evaluate_legacy(&CoreOpportunityEvidence::new(input));

        assert_eq!(actual, expected);
    }

    /// Verify the generic contract preserves legacy action, multiplier, and budget.
    #[test]
    fn policy_contract_maps_legacy_decision_without_behavior_change() {
        let policy = CoreOpportunityV1::default();
        let context = context(input(
            TrendRegime::FallingKnife,
            DecisionSentiment::Unavailable,
        ));
        let expected = policy.evaluate_legacy(context.evidence());
        let recommendation = policy.evaluate(&context);

        assert_eq!(recommendation.policy().to_string(), "core_opportunity_v1@1");
        assert_eq!(recommendation.action(), expected.action);
        assert_eq!(recommendation.multiplier(), expected.multiplier);
        assert_eq!(
            recommendation.scheduled_contribution(),
            context.scheduled_contribution()
        );
    }

    /// Verify fixed DCA always recommends the full scheduled contribution without signals.
    #[test]
    fn fixed_dca_recommends_the_full_budget_without_market_evidence() {
        let policy = FixedDcaPolicy;
        let context = DecisionContext::new(
            Date::from_calendar_date(2026, Month::January, 15).unwrap(),
            Decimal::new(250, 0),
            (),
        )
        .unwrap();
        let recommendation = policy.evaluate(&context);

        assert_eq!(recommendation.policy().to_string(), "fixed_dca@1");
        assert_eq!(recommendation.action(), core_domain::Action::Standard);
        assert_eq!(recommendation.multiplier().value(), 1.0);
        assert_eq!(
            recommendation.scheduled_contribution(),
            Decimal::new(250, 0)
        );
    }

    /// Verify the resolver never maps a policy to unrelated evidence.
    #[test]
    fn resolver_requires_evidence_matching_the_selected_policy() {
        let resolver = BuiltinPolicyResolver::default();
        let fixed = FixedDcaPolicy.policy_ref();
        let error = resolver
            .evaluate(
                &fixed,
                Date::from_calendar_date(2026, Month::January, 15).unwrap(),
                Decimal::ONE,
                BuiltinPolicyEvidence::CoreOpportunity(CoreOpportunityEvidence::new(input(
                    TrendRegime::Neutral,
                    DecisionSentiment::Unavailable,
                ))),
            )
            .unwrap_err();

        assert_eq!(error, BuiltinPolicyError::EvidenceDoesNotMatchPolicy);
    }
}

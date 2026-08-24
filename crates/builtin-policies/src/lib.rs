#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! IndexLink 内置策略实现。
//!
//! 本 crate 仅包含无 IO 的策略适配器。当前 [`CoreOpportunityV1`] 包装既有
//! 70/20/10 决策引擎，不改变它的计算、权重、动作或降级语义。

use decision_engine::{evaluate_decision, DecisionConfig, DecisionInput, DecisionSignal};
use strategy_policy::{
    DecisionContext, InvestmentPolicy, InvestmentRecommendation, PolicyId, PolicyRef, PolicyVersion,
};

/// `CoreOpportunityV1` 的稳定策略标识。
pub const CORE_OPPORTUNITY_V1_ID: &str = "core_opportunity_v1";
/// `CoreOpportunityV1` 的不可变初始版本。
pub const CORE_OPPORTUNITY_V1_VERSION: u32 = 1;

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
}

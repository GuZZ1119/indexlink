#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! 受限、无 IO 的策略 DSL 抽象语法树与校验器。
//!
//! 本 crate 只定义可保存、可审阅的策略规则，不读取市场数据、环境变量或数据库，
//! 也不执行订单。运行时只解释已校验的 AST，不读取市场数据、环境变量、数据库或
//! 网络；存储和 HTTP API 属于后续阶段。该边界只允许白名单指标与动作，因此不会执行
//! 用户代码或任意脚本。

use std::collections::BTreeMap;

use core_domain::{Action, Multiplier};
use rust_decimal::Decimal;
use strategy_policy::{DecisionContext, InvestmentRecommendation, PolicyRef};

const MAX_NAME_LEN: usize = 120;
const MAX_RULES: usize = 32;
const MAX_EXPRESSION_DEPTH: usize = 8;
const MAX_EXPRESSION_NODES: usize = 128;
const MAX_LOOKBACK_DAYS: u16 = 365;

/// 一个已校验的指标回看窗口（以交易日计）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LookbackWindow(u16);

impl LookbackWindow {
    /// 构造介于 2 至 365 个交易日的回看窗口。
    pub fn new(days: u16) -> Result<Self, StrategyDslValidationError> {
        if !(2..=MAX_LOOKBACK_DAYS).contains(&days) {
            return Err(StrategyDslValidationError::InvalidLookbackWindow);
        }
        Ok(Self(days))
    }

    /// 返回窗口包含的交易日数量。
    #[must_use]
    pub fn days(self) -> u16 {
        self.0
    }
}

/// 除法表达式可使用的已校验非零常数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonZeroDecimal(Decimal);

impl NonZeroDecimal {
    /// 构造一个非零常数除数。
    pub fn new(value: Decimal) -> Result<Self, StrategyDslValidationError> {
        if value.is_zero() {
            return Err(StrategyDslValidationError::ZeroDivisor);
        }
        Ok(Self(value))
    }

    /// 返回已校验的除数。
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// DSL 首版允许读取的市场指标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndicatorSpec {
    /// 当期可得收盘价。
    ClosePrice,
    /// 简单移动平均线。
    SimpleMovingAverage(LookbackWindow),
    /// 指数移动平均线。
    ExponentialMovingAverage(LookbackWindow),
    /// 相对强弱指数。
    RelativeStrengthIndex(LookbackWindow),
    /// 相对过去峰值的回撤。
    Drawdown(LookbackWindow),
    /// Cboe VIX 水平。
    Vix,
}

/// 白名单表达式的不可变值节点。
#[derive(Debug, Clone, PartialEq)]
pub struct ValueExpression(ExpressionKind);

#[derive(Debug, Clone, PartialEq)]
enum ExpressionKind {
    Constant(Decimal),
    Indicator(IndicatorSpec),
    Add(Box<ValueExpression>, Box<ValueExpression>),
    Subtract(Box<ValueExpression>, Box<ValueExpression>),
    Multiply(Box<ValueExpression>, Decimal),
    Divide(Box<ValueExpression>, NonZeroDecimal),
}

impl ValueExpression {
    /// 构造一个固定数值表达式。
    #[must_use]
    pub fn constant(value: Decimal) -> Self {
        Self(ExpressionKind::Constant(value))
    }

    /// 构造一个白名单指标表达式。
    #[must_use]
    pub fn indicator(indicator: IndicatorSpec) -> Self {
        Self(ExpressionKind::Indicator(indicator))
    }

    /// 构造两个表达式的加法。
    #[must_use]
    pub fn sum(left: Self, right: Self) -> Self {
        Self(ExpressionKind::Add(Box::new(left), Box::new(right)))
    }

    /// 构造两个表达式的减法。
    #[must_use]
    pub fn subtract(left: Self, right: Self) -> Self {
        Self(ExpressionKind::Subtract(Box::new(left), Box::new(right)))
    }

    /// 构造一个固定常数乘法。
    #[must_use]
    pub fn multiply(expression: Self, factor: Decimal) -> Self {
        Self(ExpressionKind::Multiply(Box::new(expression), factor))
    }

    /// 构造一个除以非零常数的表达式。
    #[must_use]
    pub fn divide(expression: Self, divisor: NonZeroDecimal) -> Self {
        Self(ExpressionKind::Divide(Box::new(expression), divisor))
    }

    fn complexity(&self) -> (usize, usize) {
        match &self.0 {
            ExpressionKind::Constant(_) | ExpressionKind::Indicator(_) => (1, 1),
            ExpressionKind::Add(left, right) | ExpressionKind::Subtract(left, right) => {
                let (left_depth, left_nodes) = left.complexity();
                let (right_depth, right_nodes) = right.complexity();
                (
                    left_depth.max(right_depth) + 1,
                    left_nodes + right_nodes + 1,
                )
            }
            ExpressionKind::Multiply(expression, _) | ExpressionKind::Divide(expression, _) => {
                let (depth, nodes) = expression.complexity();
                (depth + 1, nodes + 1)
            }
        }
    }
}

/// 受限 DSL 可用的比较运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// 大于阈值。
    GreaterThan,
    /// 大于或等于阈值。
    GreaterThanOrEqual,
    /// 小于阈值。
    LessThan,
    /// 小于或等于阈值。
    LessThanOrEqual,
}

/// 由白名单表达式构成的不可变条件树。
#[derive(Debug, Clone, PartialEq)]
pub struct Condition(ConditionKind);

#[derive(Debug, Clone, PartialEq)]
enum ConditionKind {
    Comparison {
        expression: ValueExpression,
        operator: ComparisonOperator,
        threshold: Decimal,
    },
    All(Vec<Condition>),
    Any(Vec<Condition>),
}

impl Condition {
    /// 构造一个表达式与固定阈值的比较条件。
    #[must_use]
    pub fn compare(
        expression: ValueExpression,
        operator: ComparisonOperator,
        threshold: Decimal,
    ) -> Self {
        Self(ConditionKind::Comparison {
            expression,
            operator,
            threshold,
        })
    }

    /// 构造必须全部满足的条件组合。
    pub fn all(conditions: Vec<Self>) -> Result<Self, StrategyDslValidationError> {
        if conditions.is_empty() {
            return Err(StrategyDslValidationError::EmptyConditionGroup);
        }
        Ok(Self(ConditionKind::All(conditions)))
    }

    /// 构造任一满足即可的条件组合。
    pub fn any(conditions: Vec<Self>) -> Result<Self, StrategyDslValidationError> {
        if conditions.is_empty() {
            return Err(StrategyDslValidationError::EmptyConditionGroup);
        }
        Ok(Self(ConditionKind::Any(conditions)))
    }

    fn complexity(&self) -> (usize, usize) {
        match &self.0 {
            ConditionKind::Comparison { expression, .. } => expression.complexity(),
            ConditionKind::All(conditions) | ConditionKind::Any(conditions) => conditions
                .iter()
                .map(Self::complexity)
                .fold((1, 1), |(depth, nodes), (child_depth, child_nodes)| {
                    (depth.max(child_depth + 1), nodes + child_nodes)
                }),
        }
    }
}

/// DSL 首版允许生成的、尚未执行的动作。
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyAction(PolicyActionKind);

#[derive(Debug, Clone, PartialEq)]
enum PolicyActionKind {
    /// 设置机会桶的固定建议金额；核心桶不受该动作影响。
    SetOpportunityFixedAmount(Decimal),
    /// 调整机会桶的倍率；核心桶始终由计划配置决定。
    SetOpportunityMultiplier(Multiplier),
    /// 跳过当前周期的机会桶；不会删除或否决核心桶。
    SkipOpportunity,
}

impl PolicyAction {
    /// 构造一个金额大于零的机会桶固定金额动作。
    pub fn set_opportunity_fixed_amount(
        amount: Decimal,
    ) -> Result<Self, StrategyDslValidationError> {
        if amount <= Decimal::ZERO {
            return Err(StrategyDslValidationError::InvalidFixedAmount);
        }
        Ok(Self(PolicyActionKind::SetOpportunityFixedAmount(amount)))
    }

    /// 构造一个已由 [`Multiplier`] 限定的机会桶倍率动作。
    #[must_use]
    pub fn set_opportunity_multiplier(multiplier: Multiplier) -> Self {
        Self(PolicyActionKind::SetOpportunityMultiplier(multiplier))
    }

    /// 构造一个只跳过当前机会桶的动作。
    #[must_use]
    pub fn skip_opportunity() -> Self {
        Self(PolicyActionKind::SkipOpportunity)
    }

    /// 判断该动作是否只影响机会桶而不会否决核心桶。
    #[must_use]
    pub fn is_opportunity_only(&self) -> bool {
        matches!(
            self.0,
            PolicyActionKind::SetOpportunityFixedAmount(_)
                | PolicyActionKind::SetOpportunityMultiplier(_)
                | PolicyActionKind::SkipOpportunity
        )
    }

    fn validate_for_budget(&self, budget: Decimal) -> Result<(), StrategyDslValidationError> {
        if let PolicyActionKind::SetOpportunityFixedAmount(amount) = &self.0 {
            if *amount > budget {
                return Err(StrategyDslValidationError::ActionExceedsBudget);
            }
        }
        Ok(())
    }

    fn runtime_action(&self) -> DslRuntimeAction {
        match self.0 {
            PolicyActionKind::SetOpportunityFixedAmount(amount) => {
                DslRuntimeAction::OpportunityFixedAmount(amount)
            }
            PolicyActionKind::SetOpportunityMultiplier(multiplier) => {
                DslRuntimeAction::OpportunityMultiplier(multiplier)
            }
            PolicyActionKind::SkipOpportunity => DslRuntimeAction::SkipOpportunity,
        }
    }
}

/// 一个受限条件与单个白名单动作组成的策略规则。
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyRule {
    condition: Condition,
    action: PolicyAction,
}

impl StrategyRule {
    /// 构造一条条件规则。
    #[must_use]
    pub fn new(condition: Condition, action: PolicyAction) -> Self {
        Self { condition, action }
    }

    /// 返回规则条件。
    #[must_use]
    pub fn condition(&self) -> &Condition {
        &self.condition
    }

    /// 返回规则动作。
    #[must_use]
    pub fn action(&self) -> &PolicyAction {
        &self.action
    }
}

/// 可保存、可审阅且可由确定性解释器执行的受限策略定义。
#[derive(Debug, Clone, PartialEq)]
pub struct StrategySpec {
    policy: PolicyRef,
    name: String,
    rules: Vec<StrategyRule>,
}

impl StrategySpec {
    /// 构造并校验一个版本化的自定义策略定义。
    pub fn new(
        policy: PolicyRef,
        name: impl Into<String>,
        rules: Vec<StrategyRule>,
    ) -> Result<Self, StrategyDslValidationError> {
        let name = normalize_name(name.into())?;
        if !policy.id().as_str().starts_with("dsl_") {
            return Err(StrategyDslValidationError::CustomPolicyIdRequired);
        }
        if rules.is_empty() || rules.len() > MAX_RULES {
            return Err(StrategyDslValidationError::InvalidRuleCount);
        }
        for rule in &rules {
            let (depth, nodes) = rule.condition.complexity();
            if depth > MAX_EXPRESSION_DEPTH || nodes > MAX_EXPRESSION_NODES {
                return Err(StrategyDslValidationError::ExpressionTooComplex);
            }
        }

        Ok(Self {
            policy,
            name,
            rules,
        })
    }

    /// 返回该策略不可变的标识与版本。
    #[must_use]
    pub fn policy(&self) -> &PolicyRef {
        &self.policy
    }

    /// 返回已规范化的策略名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回已校验的规则列表。
    #[must_use]
    pub fn rules(&self) -> &[StrategyRule] {
        &self.rules
    }

    /// 使用某个计划周期预算再次校验固定金额动作。
    ///
    /// 该方法只校验 DSL 意图，不替代实际执行时的周期累计、机会现金、可用现金或
    /// paper-only 约束。
    pub fn validate_for_budget(&self, budget: Decimal) -> Result<(), StrategyDslValidationError> {
        if budget <= Decimal::ZERO {
            return Err(StrategyDslValidationError::InvalidBudget);
        }
        self.rules
            .iter()
            .try_for_each(|rule| rule.action.validate_for_budget(budget))
    }

    /// 在已解析的证据上以固定规则顺序执行本策略。
    ///
    /// 第一个满足条件的规则生效；没有规则满足时，运行时返回机会桶的标准倍率。该
    /// 解释器不会读取或写入外部状态，也不会生成 broker 订单。调用方仍必须将返回的
    /// [`InvestmentRecommendation`] 与核心桶、周期上限、可用现金及审批约束合并。
    pub fn evaluate(
        &self,
        context: &DecisionContext<DslEvidence>,
    ) -> Result<DslEvaluation, StrategyDslRuntimeError> {
        self.validate_for_budget(context.scheduled_contribution())?;

        for (index, rule) in self.rules.iter().enumerate() {
            if rule.condition.matches(context.evidence())? {
                return Ok(DslEvaluation::from_action(
                    self.policy.clone(),
                    context,
                    Some(index),
                    rule.action.runtime_action(),
                ));
            }
        }

        Ok(DslEvaluation::from_action(
            self.policy.clone(),
            context,
            None,
            DslRuntimeAction::StandardOpportunity,
        ))
    }
}

impl ValueExpression {
    fn evaluate(&self, evidence: &DslEvidence) -> Result<Decimal, StrategyDslRuntimeError> {
        match &self.0 {
            ExpressionKind::Constant(value) => Ok(*value),
            ExpressionKind::Indicator(indicator) => evidence.value(*indicator),
            ExpressionKind::Add(left, right) => left
                .evaluate(evidence)?
                .checked_add(right.evaluate(evidence)?)
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow),
            ExpressionKind::Subtract(left, right) => left
                .evaluate(evidence)?
                .checked_sub(right.evaluate(evidence)?)
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow),
            ExpressionKind::Multiply(expression, factor) => expression
                .evaluate(evidence)?
                .checked_mul(*factor)
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow),
            ExpressionKind::Divide(expression, divisor) => expression
                .evaluate(evidence)?
                .checked_div(divisor.value())
                .ok_or(StrategyDslRuntimeError::ArithmeticOverflow),
        }
    }
}

impl Condition {
    fn matches(&self, evidence: &DslEvidence) -> Result<bool, StrategyDslRuntimeError> {
        match &self.0 {
            ConditionKind::Comparison {
                expression,
                operator,
                threshold,
            } => {
                let value = expression.evaluate(evidence)?;
                Ok(match operator {
                    ComparisonOperator::GreaterThan => value > *threshold,
                    ComparisonOperator::GreaterThanOrEqual => value >= *threshold,
                    ComparisonOperator::LessThan => value < *threshold,
                    ComparisonOperator::LessThanOrEqual => value <= *threshold,
                })
            }
            ConditionKind::All(conditions) => {
                for condition in conditions {
                    if !condition.matches(evidence)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConditionKind::Any(conditions) => {
                for condition in conditions {
                    if condition.matches(evidence)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }
}

/// 一组与策略声明完全匹配的已解析指标数值。
///
/// 证据必须由应用层或研究器在 `as_of` 时点之前准备。运行时只读取该快照，因此不会
/// 隐式引入网络、数据库或未来数据。
#[derive(Debug, Clone, PartialEq)]
pub struct DslEvidence {
    values: BTreeMap<IndicatorSpec, Decimal>,
}

impl DslEvidence {
    /// 从白名单指标及其同一时点数值构造证据快照。
    ///
    /// 同一指标重复出现会被拒绝，避免调用方依赖插入顺序覆盖证据。
    pub fn new(
        values: impl IntoIterator<Item = (IndicatorSpec, Decimal)>,
    ) -> Result<Self, StrategyDslRuntimeError> {
        let mut normalized = BTreeMap::new();
        for (indicator, value) in values {
            if normalized.insert(indicator, value).is_some() {
                return Err(StrategyDslRuntimeError::DuplicateIndicator);
            }
        }
        Ok(Self { values: normalized })
    }

    /// 返回某个已提供指标的快照值。
    pub fn value(&self, indicator: IndicatorSpec) -> Result<Decimal, StrategyDslRuntimeError> {
        self.values
            .get(&indicator)
            .copied()
            .ok_or(StrategyDslRuntimeError::MissingIndicator)
    }
}

/// 解释器实际命中的、只影响机会桶的动作。
#[derive(Debug, Clone, PartialEq)]
pub enum DslRuntimeAction {
    /// 用固定金额建议机会桶投入；实际金额仍受计划执行约束。
    OpportunityFixedAmount(Decimal),
    /// 用有界倍率建议机会桶投入。
    OpportunityMultiplier(Multiplier),
    /// 跳过当前机会桶；核心桶不受影响。
    SkipOpportunity,
    /// 没有匹配规则时的标准机会桶投入。
    StandardOpportunity,
}

impl DslRuntimeAction {
    fn action_and_multiplier(&self) -> (Action, Multiplier) {
        match self {
            Self::OpportunityFixedAmount(_) | Self::StandardOpportunity => {
                (Action::Standard, Multiplier::new_clamped(1.0))
            }
            Self::OpportunityMultiplier(multiplier) => (multiplier.to_action(), *multiplier),
            Self::SkipOpportunity => (Action::Skip, Multiplier::MIN),
        }
    }
}

/// 一次 DSL 解释的确定性结果。
#[derive(Debug, Clone, PartialEq)]
pub struct DslEvaluation {
    recommendation: InvestmentRecommendation,
    matched_rule_index: Option<usize>,
    action: DslRuntimeAction,
}

impl DslEvaluation {
    fn from_action(
        policy: PolicyRef,
        context: &DecisionContext<DslEvidence>,
        matched_rule_index: Option<usize>,
        action: DslRuntimeAction,
    ) -> Self {
        let (recommendation_action, multiplier) = action.action_and_multiplier();
        Self {
            recommendation: InvestmentRecommendation::from_context(
                policy,
                context,
                recommendation_action,
                multiplier,
            ),
            matched_rule_index,
            action,
        }
    }

    /// 返回策略契约可消费的通用推荐。
    #[must_use]
    pub fn recommendation(&self) -> &InvestmentRecommendation {
        &self.recommendation
    }

    /// 返回首条命中的规则位置；没有命中时为 `None`。
    #[must_use]
    pub fn matched_rule_index(&self) -> Option<usize> {
        self.matched_rule_index
    }

    /// 返回不会影响核心桶的具体 DSL 动作。
    #[must_use]
    pub fn action(&self) -> &DslRuntimeAction {
        &self.action
    }
}

fn normalize_name(value: String) -> Result<String, StrategyDslValidationError> {
    let normalized = value.trim();
    if normalized.is_empty() || normalized.len() > MAX_NAME_LEN {
        Err(StrategyDslValidationError::InvalidName)
    } else {
        Ok(normalized.to_owned())
    }
}

/// 受限策略定义未通过安全或可执行性校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StrategyDslValidationError {
    /// 策略名称为空白或超过长度上限。
    #[error("strategy name must be non-blank and at most 120 characters")]
    InvalidName,
    /// 自定义策略必须使用 `dsl_` 前缀的策略标识。
    #[error("custom strategy policy id must start with dsl_")]
    CustomPolicyIdRequired,
    /// 规则数量为空或超过安全上限。
    #[error("strategy must contain between 1 and 32 rules")]
    InvalidRuleCount,
    /// 回看窗口不在支持范围内。
    #[error("lookback window must be between 2 and 365 trading days")]
    InvalidLookbackWindow,
    /// 除数不能为零。
    #[error("strategy expression divisor must not be zero")]
    ZeroDivisor,
    /// 条件组不能为空。
    #[error("condition group must not be empty")]
    EmptyConditionGroup,
    /// 条件树超过固定的深度或节点安全上限。
    #[error("strategy expression exceeds the supported complexity limit")]
    ExpressionTooComplex,
    /// 固定金额动作必须大于零。
    #[error("fixed contribution action must be greater than zero")]
    InvalidFixedAmount,
    /// DSL 固定金额超过调用方提供的周期预算。
    #[error("strategy action exceeds the plan period budget")]
    ActionExceedsBudget,
    /// 调用方提供的计划周期预算无效。
    #[error("plan period budget must be greater than zero")]
    InvalidBudget,
}

/// 已校验 DSL 在已解析证据上执行失败。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StrategyDslRuntimeError {
    /// 当前证据快照没有策略表达式所需的白名单指标。
    #[error("strategy evidence is missing a required indicator")]
    MissingIndicator,
    /// 同一指标在一份证据快照中被提供多次。
    #[error("strategy evidence contains a duplicate indicator")]
    DuplicateIndicator,
    /// 有界 AST 的 Decimal 运算超出可表示范围。
    #[error("strategy expression arithmetic overflowed")]
    ArithmeticOverflow,
    /// 调用时的周期预算无法满足已保存 DSL 的固定金额约束。
    #[error(transparent)]
    Validation(#[from] StrategyDslValidationError),
}

#[cfg(test)]
mod tests {
    use core_domain::Multiplier;
    use strategy_policy::{PolicyId, PolicyVersion};
    use time::{Date, Month};

    use super::*;

    fn policy() -> PolicyRef {
        PolicyRef::new(
            PolicyId::new("dsl_value_guard").unwrap(),
            PolicyVersion::new(1).unwrap(),
        )
    }

    fn condition() -> Condition {
        Condition::compare(
            ValueExpression::indicator(IndicatorSpec::RelativeStrengthIndex(
                LookbackWindow::new(14).unwrap(),
            )),
            ComparisonOperator::LessThan,
            Decimal::new(30, 0),
        )
    }

    fn context(evidence: DslEvidence) -> DecisionContext<DslEvidence> {
        DecisionContext::new(
            Date::from_calendar_date(2026, Month::January, 15).unwrap(),
            Decimal::new(100, 0),
            evidence,
        )
        .unwrap()
    }

    /// Verify a bounded, white-listed rule can be saved and checked against a plan budget.
    #[test]
    fn accepts_a_safe_custom_strategy_and_budget() {
        let strategy = StrategySpec::new(
            policy(),
            "  RSI opportunity guard  ",
            vec![StrategyRule::new(
                condition(),
                PolicyAction::set_opportunity_fixed_amount(Decimal::new(100, 0)).unwrap(),
            )],
        )
        .unwrap();

        assert_eq!(strategy.name(), "RSI opportunity guard");
        assert_eq!(strategy.policy().to_string(), "dsl_value_guard@1");
        assert_eq!(strategy.rules().len(), 1);
        assert_eq!(strategy.validate_for_budget(Decimal::new(100, 0)), Ok(()));
    }

    /// Verify public constructors reject invalid invariant values before a strategy can contain them.
    #[test]
    fn rejects_invalid_windows_divisors_condition_groups_and_fixed_amounts() {
        assert_eq!(
            LookbackWindow::new(1),
            Err(StrategyDslValidationError::InvalidLookbackWindow)
        );
        assert_eq!(
            NonZeroDecimal::new(Decimal::ZERO),
            Err(StrategyDslValidationError::ZeroDivisor)
        );
        assert_eq!(
            Condition::all(vec![]),
            Err(StrategyDslValidationError::EmptyConditionGroup)
        );
        assert_eq!(
            PolicyAction::set_opportunity_fixed_amount(Decimal::ZERO),
            Err(StrategyDslValidationError::InvalidFixedAmount)
        );
    }

    /// Verify only versioned custom policy identifiers can define DSL rules.
    #[test]
    fn rejects_builtin_policy_ids_and_empty_rule_sets() {
        let builtin = PolicyRef::new(
            PolicyId::new("fixed_dca").unwrap(),
            PolicyVersion::new(1).unwrap(),
        );
        assert_eq!(
            StrategySpec::new(
                builtin,
                "Fixed DCA copy",
                vec![StrategyRule::new(
                    condition(),
                    PolicyAction::skip_opportunity()
                )]
            ),
            Err(StrategyDslValidationError::CustomPolicyIdRequired)
        );
        assert_eq!(
            StrategySpec::new(policy(), "Empty", vec![]),
            Err(StrategyDslValidationError::InvalidRuleCount)
        );
    }

    /// Verify expression-tree bounds prevent unbounded user-authored nesting.
    #[test]
    fn rejects_an_expression_that_exceeds_the_depth_limit() {
        let mut expression = ValueExpression::indicator(IndicatorSpec::ClosePrice);
        for _ in 0..MAX_EXPRESSION_DEPTH {
            expression = ValueExpression::multiply(expression, Decimal::ONE);
        }
        let deep_condition =
            Condition::compare(expression, ComparisonOperator::GreaterThan, Decimal::ZERO);

        assert_eq!(
            StrategySpec::new(
                policy(),
                "Too deep",
                vec![StrategyRule::new(
                    deep_condition,
                    PolicyAction::skip_opportunity()
                )],
            ),
            Err(StrategyDslValidationError::ExpressionTooComplex)
        );
    }

    /// Verify a policy definition cannot claim a fixed amount above its plan period budget.
    #[test]
    fn rejects_fixed_actions_above_the_plan_budget() {
        let strategy = StrategySpec::new(
            policy(),
            "Budget guard",
            vec![StrategyRule::new(
                condition(),
                PolicyAction::set_opportunity_fixed_amount(Decimal::new(101, 0)).unwrap(),
            )],
        )
        .unwrap();

        assert_eq!(
            strategy.validate_for_budget(Decimal::new(100, 0)),
            Err(StrategyDslValidationError::ActionExceedsBudget)
        );
        assert_eq!(
            strategy.validate_for_budget(Decimal::ZERO),
            Err(StrategyDslValidationError::InvalidBudget)
        );
    }

    /// Verify only core-safe opportunity actions are representable by the first DSL revision.
    #[test]
    fn models_opportunity_actions_without_a_core_bucket_veto() {
        assert!(PolicyAction::set_opportunity_fixed_amount(Decimal::ONE)
            .unwrap()
            .is_opportunity_only());
        let action = PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.2));
        assert!(action.is_opportunity_only());
        assert!(PolicyAction::skip_opportunity().is_opportunity_only());
    }

    /// Verify the interpreter uses first-match order and returns a policy recommendation.
    #[test]
    fn evaluates_the_first_matching_rule_deterministically() {
        let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14).unwrap());
        let strategy = StrategySpec::new(
            policy(),
            "Ordered RSI rules",
            vec![
                StrategyRule::new(
                    Condition::compare(
                        ValueExpression::indicator(rsi),
                        ComparisonOperator::LessThan,
                        Decimal::new(40, 0),
                    ),
                    PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.2)),
                ),
                StrategyRule::new(condition(), PolicyAction::skip_opportunity()),
            ],
        )
        .unwrap();
        let evaluation = strategy
            .evaluate(&context(
                DslEvidence::new([(rsi, Decimal::new(25, 0))]).unwrap(),
            ))
            .unwrap();

        assert_eq!(evaluation.matched_rule_index(), Some(0));
        assert_eq!(evaluation.recommendation().multiplier().value(), 1.2);
        assert_eq!(evaluation.recommendation().action(), Action::Overweight);
        assert_eq!(
            evaluation.action(),
            &DslRuntimeAction::OpportunityMultiplier(Multiplier::new_clamped(1.2))
        );
    }

    /// Verify absent evidence fails closed instead of silently substituting an indicator value.
    #[test]
    fn rejects_execution_when_required_evidence_is_missing() {
        let strategy = StrategySpec::new(
            policy(),
            "Required RSI",
            vec![StrategyRule::new(
                condition(),
                PolicyAction::set_opportunity_multiplier(Multiplier::new_clamped(1.1)),
            )],
        )
        .unwrap();

        assert_eq!(
            strategy.evaluate(&context(DslEvidence::new([]).unwrap())),
            Err(StrategyDslRuntimeError::MissingIndicator)
        );
    }

    /// Verify a no-match result leaves the opportunity bucket at its standard multiplier.
    #[test]
    fn defaults_to_standard_opportunity_when_no_rule_matches() {
        let rsi = IndicatorSpec::RelativeStrengthIndex(LookbackWindow::new(14).unwrap());
        let strategy = StrategySpec::new(
            policy(),
            "No match default",
            vec![StrategyRule::new(
                Condition::compare(
                    ValueExpression::indicator(rsi),
                    ComparisonOperator::LessThan,
                    Decimal::new(30, 0),
                ),
                PolicyAction::skip_opportunity(),
            )],
        )
        .unwrap();
        let evaluation = strategy
            .evaluate(&context(
                DslEvidence::new([(rsi, Decimal::new(55, 0))]).unwrap(),
            ))
            .unwrap();

        assert_eq!(evaluation.matched_rule_index(), None);
        assert_eq!(evaluation.recommendation().action(), Action::Standard);
        assert_eq!(evaluation.recommendation().multiplier().value(), 1.0);
        assert_eq!(evaluation.action(), &DslRuntimeAction::StandardOpportunity);
    }
}

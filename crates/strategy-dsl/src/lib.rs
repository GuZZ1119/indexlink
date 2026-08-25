#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! 受限、无 IO 的策略 DSL 抽象语法树与校验器。
//!
//! 本 crate 只定义可保存、可审阅的策略规则，不读取市场数据、环境变量或数据库，
//! 也不执行订单。运行时解释器、存储和 HTTP API 属于后续阶段。该边界只允许白名单
//! 指标与动作，因此不会执行用户代码或任意脚本。

use core_domain::Multiplier;
use rust_decimal::Decimal;
use strategy_policy::PolicyRef;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// 可保存、可审阅但尚不可执行的受限策略定义。
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

#[cfg(test)]
mod tests {
    use core_domain::Multiplier;
    use strategy_policy::{PolicyId, PolicyVersion};

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
}

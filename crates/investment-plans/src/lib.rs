#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Investment Plan 领域与应用层基础。
//!
//! 本 crate 采用模块化单体内的轻量六边形边界：这里定义投资计划的领域模型、
//! 输入校验、应用服务和 repository port；PostgreSQL、Axum、Broker、Qwen、
//! Scheduler 与执行计划生成均属于外部 adapter 或后续阶段。
//!
//! MVP 假设：单用户系统、仅支持 monthly、无计划级 timezone、不验证 symbol 是否
//! 真实可交易、不计算任何本期买入金额或双桶资金分配。
//!
//! 金额统一使用 [`rust_decimal::Decimal`]。HTTP/JSON 边界必须以字符串编码金额，
//! 避免 JavaScript Number 或 JSON 浮点转换造成精度损失。领域类型不直接实现
//! `Deserialize`；入站 adapter 应先反序列化到 DTO，再调用 `normalize()` 进入领域模型。

use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

const MAX_NAME_LEN: usize = 100;
const MAX_SYMBOL_LEN: usize = 32;

/// MVP 支持的投资计划周期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind {
    /// 每月固定日期触发。
    Monthly,
}

/// 持久化后的投资计划。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InvestmentPlan {
    /// 计划 ID。
    pub id: Uuid,
    /// 用户可读计划名称。
    pub name: String,
    /// 投资标的代码，已规范化为大写。
    pub symbol: String,
    /// 基准定投金额，不是本期实际执行金额。
    #[serde(with = "rust_decimal::serde::str")]
    pub base_contribution: Decimal,
    /// ISO 风格三位币种代码，已规范化为大写。
    pub currency: String,
    /// MVP 仅支持 monthly。
    pub schedule_kind: ScheduleKind,
    /// 每月执行日，范围为 1..=28。
    pub schedule_day: i16,
    /// 单次执行金额硬上限，不是 planner 输出。
    #[serde(with = "rust_decimal::serde::str")]
    pub max_single_execution: Decimal,
    /// 是否启用计划。
    pub is_active: bool,
    /// 创建时间。
    pub created_at: OffsetDateTime,
    /// 最近更新时间。
    pub updated_at: OffsetDateTime,
}

/// 创建投资计划的领域输入。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreateInvestmentPlan {
    /// 用户可读计划名称。
    pub name: String,
    /// 投资标的代码。
    pub symbol: String,
    /// 基准定投金额，不是本期实际执行金额。
    #[serde(with = "rust_decimal::serde::str")]
    pub base_contribution: Decimal,
    /// ISO 风格三位币种代码。
    pub currency: String,
    /// MVP 仅支持 monthly。
    pub schedule_kind: ScheduleKind,
    /// 每月执行日。
    pub schedule_day: i16,
    /// 单次执行金额硬上限。
    #[serde(with = "rust_decimal::serde::str")]
    pub max_single_execution: Decimal,
}

impl CreateInvestmentPlan {
    /// 规范化并校验创建输入。
    ///
    /// 本方法只处理计划配置本身，不计算任何本期买入金额。
    pub fn normalize(self) -> Result<Self, PlanValidationError> {
        let name = normalize_non_empty(self.name, MAX_NAME_LEN, PlanValidationError::InvalidName)?;
        let symbol = normalize_symbol(self.symbol)?;
        let currency = normalize_currency(self.currency)?;
        validate_day(self.schedule_day)?;
        validate_amounts(self.base_contribution, self.max_single_execution)?;

        Ok(Self {
            name,
            symbol,
            currency,
            ..self
        })
    }
}

/// 更新投资计划的领域输入。
///
/// 不包含 `symbol`、`currency` 或 `schedule_kind`；更换标的、币种或周期应创建新计划并停用旧计划。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UpdateInvestmentPlan {
    /// 可选的新名称。
    pub name: Option<String>,
    /// 可选的新基准定投金额。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub base_contribution: Option<Decimal>,
    /// 可选的新每月执行日。
    pub schedule_day: Option<i16>,
    /// 可选的新单次执行金额硬上限。
    #[serde(
        default,
        with = "rust_decimal::serde::str_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_single_execution: Option<Decimal>,
    /// 可选启停状态。
    pub is_active: Option<bool>,
}

impl UpdateInvestmentPlan {
    /// 规范化并校验更新输入。
    ///
    /// 当同时更新 `base_contribution` 与 `max_single_execution` 时，会校验二者关系；
    /// 只更新其中一项时，后续 application service 需结合当前计划再次校验最终状态。
    pub fn normalize(self) -> Result<Self, PlanValidationError> {
        if self.name.is_none()
            && self.base_contribution.is_none()
            && self.schedule_day.is_none()
            && self.max_single_execution.is_none()
            && self.is_active.is_none()
        {
            return Err(PlanValidationError::EmptyUpdate);
        }

        let name = self
            .name
            .map(|name| normalize_non_empty(name, MAX_NAME_LEN, PlanValidationError::InvalidName))
            .transpose()?;
        if let Some(day) = self.schedule_day {
            validate_day(day)?;
        }
        if let Some(base) = self.base_contribution {
            validate_positive("base_contribution", base)?;
        }
        if let Some(max) = self.max_single_execution {
            validate_positive("max_single_execution", max)?;
        }
        if let (Some(base), Some(max)) = (self.base_contribution, self.max_single_execution) {
            validate_amounts(base, max)?;
        }

        Ok(Self { name, ..self })
    }
}

/// 投资计划字段校验错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanValidationError {
    /// 名称为空或超过长度限制。
    #[error("investment plan name must be 1..=100 characters after trimming")]
    InvalidName,
    /// 标的为空或超过长度限制。
    #[error("investment plan symbol must be 1..=32 characters after trimming")]
    InvalidSymbol,
    /// 币种不是三位 ASCII 大写字母。
    #[error("currency must be exactly 3 ASCII uppercase letters")]
    InvalidCurrency,
    /// 每月执行日不在 1..=28。
    #[error("monthly schedule day must be between 1 and 28")]
    InvalidScheduleDay,
    /// 金额不是正数。
    #[error("{field} must be greater than zero")]
    NonPositiveAmount {
        /// 字段名。
        field: &'static str,
    },
    /// 单次执行上限低于基准定投金额。
    #[error("max_single_execution must be greater than or equal to base_contribution")]
    MaxBelowBaseContribution,
    /// PATCH 没有任何字段。
    #[error("update must include at least one field")]
    EmptyUpdate,
}

fn normalize_non_empty(
    value: String,
    max_len: usize,
    error: PlanValidationError,
) -> Result<String, PlanValidationError> {
    let normalized = value.trim().to_owned();
    if normalized.is_empty() || normalized.chars().count() > max_len {
        Err(error)
    } else {
        Ok(normalized)
    }
}

fn normalize_symbol(value: String) -> Result<String, PlanValidationError> {
    let symbol = normalize_non_empty(value, MAX_SYMBOL_LEN, PlanValidationError::InvalidSymbol)?;
    if symbol.is_ascii() {
        Ok(symbol.to_ascii_uppercase())
    } else {
        Err(PlanValidationError::InvalidSymbol)
    }
}

fn normalize_currency(value: String) -> Result<String, PlanValidationError> {
    let currency = value.trim().to_ascii_uppercase();
    if currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
        Ok(currency)
    } else {
        Err(PlanValidationError::InvalidCurrency)
    }
}

fn validate_day(day: i16) -> Result<(), PlanValidationError> {
    if (1..=28).contains(&day) {
        Ok(())
    } else {
        Err(PlanValidationError::InvalidScheduleDay)
    }
}

fn validate_positive(field: &'static str, value: Decimal) -> Result<(), PlanValidationError> {
    if value > Decimal::ZERO {
        Ok(())
    } else {
        Err(PlanValidationError::NonPositiveAmount { field })
    }
}

fn validate_amounts(base: Decimal, max: Decimal) -> Result<(), PlanValidationError> {
    validate_positive("base_contribution", base)?;
    validate_positive("max_single_execution", max)?;
    if max < base {
        Err(PlanValidationError::MaxBelowBaseContribution)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    fn money(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    fn create_input() -> CreateInvestmentPlan {
        CreateInvestmentPlan {
            name: "  VOO monthly DCA  ".to_owned(),
            symbol: " voo ".to_owned(),
            base_contribution: money("1000.00"),
            currency: " usd ".to_owned(),
            schedule_kind: ScheduleKind::Monthly,
            schedule_day: 15,
            max_single_execution: money("1500.00"),
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct DecimalContract {
        #[serde(with = "rust_decimal::serde::str")]
        amount: Decimal,
    }

    #[test]
    fn decimal_deserializes_from_json_string_without_float() {
        let payload = r#"{"amount":"1000.0001"}"#;

        let decoded: DecimalContract = serde_json::from_str(payload).unwrap();

        assert_eq!(decoded.amount.to_string(), "1000.0001");
    }

    #[test]
    fn decimal_serializes_to_json_string_without_float() {
        let payload = DecimalContract {
            amount: "1500.25".parse().unwrap(),
        };

        let encoded = serde_json::to_value(payload).unwrap();

        assert_eq!(encoded, json!({"amount": "1500.25"}));
        assert!(matches!(encoded["amount"], Value::String(_)));
    }

    #[test]
    fn decimal_rejects_json_number_at_api_boundary() {
        let result = serde_json::from_value::<DecimalContract>(json!({"amount": 1000.00}));

        assert!(result.is_err());
    }

    #[test]
    fn create_plan_normalizes_text_fields() {
        let input = create_input().normalize().unwrap();

        assert_eq!(input.name, "VOO monthly DCA");
        assert_eq!(input.symbol, "VOO");
        assert_eq!(input.currency, "USD");
    }

    #[test]
    fn create_plan_rejects_invalid_amount_relationship() {
        let mut input = create_input();
        input.max_single_execution = money("999.99");

        assert_eq!(
            input.normalize(),
            Err(PlanValidationError::MaxBelowBaseContribution)
        );
    }

    #[test]
    fn create_plan_rejects_invalid_schedule_day_and_currency() {
        let mut bad_day = create_input();
        bad_day.schedule_day = 29;
        assert_eq!(
            bad_day.normalize(),
            Err(PlanValidationError::InvalidScheduleDay)
        );

        let mut bad_currency = create_input();
        bad_currency.currency = "US1".to_owned();
        assert_eq!(
            bad_currency.normalize(),
            Err(PlanValidationError::InvalidCurrency)
        );
    }

    #[test]
    fn create_plan_rejects_non_ascii_symbol() {
        let mut input = create_input();
        input.symbol = "åapl".to_owned();

        assert_eq!(input.normalize(), Err(PlanValidationError::InvalidSymbol));
    }

    #[test]
    fn update_plan_rejects_empty_patch() {
        assert_eq!(
            UpdateInvestmentPlan::default().normalize(),
            Err(PlanValidationError::EmptyUpdate)
        );
    }

    #[test]
    fn update_plan_normalizes_name_and_validates_amounts() {
        let update = UpdateInvestmentPlan {
            name: Some("  Core plan  ".to_owned()),
            base_contribution: Some(money("1000.00")),
            max_single_execution: Some(money("1500.00")),
            ..Default::default()
        }
        .normalize()
        .unwrap();

        assert_eq!(update.name.as_deref(), Some("Core plan"));
    }

    #[test]
    fn decimal_fields_remain_json_strings_on_create_input() {
        let encoded = serde_json::to_value(create_input()).unwrap();

        assert!(matches!(encoded["base_contribution"], Value::String(_)));
        assert!(matches!(encoded["max_single_execution"], Value::String(_)));
    }
}

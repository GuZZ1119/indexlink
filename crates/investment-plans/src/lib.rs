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

use std::sync::Arc;

use async_trait::async_trait;
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
    /// 只更新其中一项时，repository update 路径需结合当前计划再次校验最终状态。
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

/// 投资计划 repository port。
///
/// 这是应用层依赖的 outbound port；PostgreSQL adapter 将在后续 PR 中实现。
#[async_trait]
pub trait InvestmentPlanRepository: Send + Sync {
    /// 创建并持久化投资计划。
    async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError>;

    /// 按固定顺序列出投资计划。
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError>;

    /// 按 ID 查询投资计划。
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError>;

    /// 更新投资计划。
    ///
    /// 实现方必须在同一个原子写入路径中读取当前计划、合并已规范化的更新输入、
    /// 校验最终金额组合，并写入结果。
    async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError>;

    /// 设置投资计划启停状态。
    async fn set_active(
        &self,
        id: Uuid,
        is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError>;
}

/// Repository port 的安全错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanRepositoryError {
    /// 输入未通过领域校验。
    #[error(transparent)]
    Validation(#[from] PlanValidationError),
    /// 计划不存在。
    #[error("investment plan not found")]
    NotFound,
    /// 持久化后端暂不可用；不得包含数据库连接、SQL 或内部错误文本。
    #[error("investment plan persistence is unavailable")]
    Unavailable,
}

/// Investment Plan 应用服务。
#[derive(Clone)]
pub struct InvestmentPlanService {
    repository: Arc<dyn InvestmentPlanRepository>,
}

impl InvestmentPlanService {
    /// 创建服务。
    #[must_use]
    pub fn new(repository: Arc<dyn InvestmentPlanRepository>) -> Self {
        Self { repository }
    }

    /// 创建投资计划。
    ///
    /// 先执行领域规范化与校验，再调用 repository port；不计算执行金额。
    pub async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanApplicationError> {
        let input = input.normalize()?;
        self.repository.create(input).await.map_err(Into::into)
    }

    /// 列出投资计划。
    pub async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanApplicationError> {
        self.repository.list().await.map_err(Into::into)
    }

    /// 获取单个投资计划。
    pub async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanApplicationError> {
        self.repository.get(id).await.map_err(Into::into)
    }

    /// 更新投资计划。
    ///
    /// 先规范化输入；最终金额组合校验由 repository 在原子写入路径内完成。
    pub async fn update(
        &self,
        id: Uuid,
        input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanApplicationError> {
        let input = input.normalize()?;
        self.repository.update(id, input).await.map_err(Into::into)
    }

    /// 设置投资计划启停状态。
    pub async fn set_active(
        &self,
        id: Uuid,
        is_active: bool,
    ) -> Result<InvestmentPlan, PlanApplicationError> {
        self.repository
            .set_active(id, is_active)
            .await
            .map_err(Into::into)
    }
}

/// 应用服务错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanApplicationError {
    /// 输入未通过领域校验。
    #[error(transparent)]
    Validation(#[from] PlanValidationError),
    /// 计划不存在。
    #[error("investment plan not found")]
    NotFound,
    /// 持久化后端暂不可用。
    #[error("investment plan service is unavailable")]
    Unavailable,
}

impl From<PlanRepositoryError> for PlanApplicationError {
    fn from(error: PlanRepositoryError) -> Self {
        match error {
            PlanRepositoryError::Validation(error) => Self::Validation(error),
            PlanRepositoryError::NotFound => Self::NotFound,
            PlanRepositoryError::Unavailable => Self::Unavailable,
        }
    }
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
    use std::sync::Mutex;

    /// 构造测试用 Decimal，避免测试中出现浮点字面量。
    fn money(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    /// 构造一份尚未规范化的创建输入，供领域和服务测试复用。
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

    /// 将已规范化的创建输入转换成 fake repository 中的持久化计划。
    fn plan_from(id: Uuid, input: CreateInvestmentPlan) -> InvestmentPlan {
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        InvestmentPlan {
            id,
            name: input.name,
            symbol: input.symbol,
            base_contribution: input.base_contribution,
            currency: input.currency,
            schedule_kind: input.schedule_kind,
            schedule_day: input.schedule_day,
            max_single_execution: input.max_single_execution,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// 基于内存锁的 repository fake，用于验证应用服务与 repository port 契约。
    #[derive(Default)]
    struct FakeRepository {
        plans: Mutex<Vec<InvestmentPlan>>,
        fail: bool,
    }

    /// 在 fake repository 的锁保护数据中按 ID 找到可变计划。
    fn find_plan_mut(
        plans: &mut [InvestmentPlan],
        id: Uuid,
    ) -> Result<&mut InvestmentPlan, PlanRepositoryError> {
        plans
            .iter_mut()
            .find(|plan| plan.id == id)
            .ok_or(PlanRepositoryError::NotFound)
    }

    #[async_trait]
    impl InvestmentPlanRepository for FakeRepository {
        /// 创建计划并追加到内存集合。
        async fn create(
            &self,
            input: CreateInvestmentPlan,
        ) -> Result<InvestmentPlan, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            let id = Uuid::from_u128((self.plans.lock().unwrap().len() + 1) as u128);
            let plan = plan_from(id, input);
            self.plans.lock().unwrap().push(plan.clone());
            Ok(plan)
        }

        /// 返回当前内存集合快照。
        async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            Ok(self.plans.lock().unwrap().clone())
        }

        /// 按 ID 读取计划，失败开关用于覆盖 unavailable 映射。
        async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            self.plans
                .lock()
                .unwrap()
                .iter()
                .find(|plan| plan.id == id)
                .cloned()
                .ok_or(PlanRepositoryError::NotFound)
        }

        /// 在同一把锁内合并、校验并写入计划更新。
        async fn update(
            &self,
            id: Uuid,
            input: UpdateInvestmentPlan,
        ) -> Result<InvestmentPlan, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            let mut plans = self.plans.lock().unwrap();
            let plan = find_plan_mut(&mut plans, id)?;
            let base = input.base_contribution.unwrap_or(plan.base_contribution);
            let max = input
                .max_single_execution
                .unwrap_or(plan.max_single_execution);
            validate_amounts(base, max)?;

            if let Some(name) = input.name {
                plan.name = name;
            }
            if let Some(base) = input.base_contribution {
                plan.base_contribution = base;
            }
            if let Some(day) = input.schedule_day {
                plan.schedule_day = day;
            }
            if let Some(max) = input.max_single_execution {
                plan.max_single_execution = max;
            }
            if let Some(is_active) = input.is_active {
                plan.is_active = is_active;
            }
            plan.updated_at = OffsetDateTime::from_unix_timestamp(1_700_000_001).unwrap();
            Ok(plan.clone())
        }

        /// 在同一把锁内切换计划启停状态。
        async fn set_active(
            &self,
            id: Uuid,
            is_active: bool,
        ) -> Result<InvestmentPlan, PlanRepositoryError> {
            if self.fail {
                return Err(PlanRepositoryError::Unavailable);
            }
            let mut plans = self.plans.lock().unwrap();
            let plan = find_plan_mut(&mut plans, id)?;
            plan.is_active = is_active;
            plan.updated_at = OffsetDateTime::from_unix_timestamp(1_700_000_001).unwrap();
            Ok(plan.clone())
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct DecimalContract {
        #[serde(with = "rust_decimal::serde::str")]
        amount: Decimal,
    }

    /// 验证 JSON 字符串金额可以无浮点损失地反序列化。
    #[test]
    fn decimal_deserializes_from_json_string_without_float() {
        let payload = r#"{"amount":"1000.0001"}"#;

        let decoded: DecimalContract = serde_json::from_str(payload).unwrap();

        assert_eq!(decoded.amount.to_string(), "1000.0001");
    }

    /// 验证金额序列化到 JSON 时仍保持字符串形态。
    #[test]
    fn decimal_serializes_to_json_string_without_float() {
        let payload = DecimalContract {
            amount: "1500.25".parse().unwrap(),
        };

        let encoded = serde_json::to_value(payload).unwrap();

        assert_eq!(encoded, json!({"amount": "1500.25"}));
        assert!(matches!(encoded["amount"], Value::String(_)));
    }

    /// 验证 API 边界拒绝 JSON number，避免引入浮点精度风险。
    #[test]
    fn decimal_rejects_json_number_at_api_boundary() {
        let result = serde_json::from_value::<DecimalContract>(json!({"amount": 1000.00}));

        assert!(result.is_err());
    }

    /// 验证创建输入会裁剪文本并规范化 symbol/currency。
    #[test]
    fn create_plan_normalizes_text_fields() {
        let input = create_input().normalize().unwrap();

        assert_eq!(input.name, "VOO monthly DCA");
        assert_eq!(input.symbol, "VOO");
        assert_eq!(input.currency, "USD");
    }

    /// 验证创建输入拒绝低于基准金额的单次执行上限。
    #[test]
    fn create_plan_rejects_invalid_amount_relationship() {
        let mut input = create_input();
        input.max_single_execution = money("999.99");

        assert_eq!(
            input.normalize(),
            Err(PlanValidationError::MaxBelowBaseContribution)
        );
    }

    /// 验证创建输入拒绝非法执行日和币种。
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

    /// 验证 symbol 只能使用 ASCII 字符。
    #[test]
    fn create_plan_rejects_non_ascii_symbol() {
        let mut input = create_input();
        input.symbol = "åapl".to_owned();

        assert_eq!(input.normalize(), Err(PlanValidationError::InvalidSymbol));
    }

    /// 验证空 PATCH 不会进入更新流程。
    #[test]
    fn update_plan_rejects_empty_patch() {
        assert_eq!(
            UpdateInvestmentPlan::default().normalize(),
            Err(PlanValidationError::EmptyUpdate)
        );
    }

    /// 验证更新输入会规范化名称并校验同时提交的金额组合。
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

    /// 验证创建输入中的金额字段保持 JSON 字符串契约。
    #[test]
    fn decimal_fields_remain_json_strings_on_create_input() {
        let encoded = serde_json::to_value(create_input()).unwrap();

        assert!(matches!(encoded["base_contribution"], Value::String(_)));
        assert!(matches!(encoded["max_single_execution"], Value::String(_)));
    }

    /// 验证 service 在持久化前会规范化创建输入。
    #[tokio::test]
    async fn service_normalizes_create_input_before_persisting() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));

        let plan = service.create(create_input()).await.unwrap();

        assert_eq!(plan.name, "VOO monthly DCA");
        assert_eq!(plan.symbol, "VOO");
        assert_eq!(plan.currency, "USD");
    }

    /// 验证非法创建输入会在进入 repository 前被拒绝。
    #[tokio::test]
    async fn service_rejects_invalid_create_before_repository() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let mut input = create_input();
        input.schedule_day = 0;

        assert_eq!(
            service.create(input).await,
            Err(PlanApplicationError::Validation(
                PlanValidationError::InvalidScheduleDay
            ))
        );
    }

    /// 验证 service 通过 repository port 完成列表和单条读取。
    #[tokio::test]
    async fn service_lists_and_gets_plans_through_repository() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));

        let created = service.create(create_input()).await.unwrap();

        assert_eq!(service.list().await.unwrap(), vec![created.clone()]);
        assert_eq!(service.get(created.id).await.unwrap(), created);
    }

    /// 验证 repository 错误会映射为安全的 application 错误。
    #[tokio::test]
    async fn service_maps_repository_errors_to_safe_application_errors() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));

        assert_eq!(
            service.get(Uuid::from_u128(99)).await,
            Err(PlanApplicationError::NotFound)
        );

        let unavailable = InvestmentPlanService::new(Arc::new(FakeRepository {
            plans: Mutex::default(),
            fail: true,
        }));
        assert_eq!(
            unavailable.list().await,
            Err(PlanApplicationError::Unavailable)
        );
        assert_eq!(
            unavailable.get(Uuid::from_u128(1)).await,
            Err(PlanApplicationError::Unavailable)
        );
    }

    /// 验证 service update 能更新允许变更的计划字段。
    #[tokio::test]
    async fn service_updates_plan_fields() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let created = service.create(create_input()).await.unwrap();

        let updated = service
            .update(
                created.id,
                UpdateInvestmentPlan {
                    name: Some("  Core ETF  ".to_owned()),
                    schedule_day: Some(20),
                    max_single_execution: Some(money("2000.00")),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "Core ETF");
        assert_eq!(updated.schedule_day, 20);
        assert_eq!(updated.max_single_execution, money("2000.00"));
        assert!(updated.updated_at > created.updated_at);
    }

    /// 验证 repository update 路径会拒绝最终金额组合非法的更新。
    #[tokio::test]
    async fn service_rejects_update_that_breaks_final_amount_limit() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let created = service.create(create_input()).await.unwrap();

        assert_eq!(
            service
                .update(
                    created.id,
                    UpdateInvestmentPlan {
                        base_contribution: Some(money("2000.00")),
                        ..Default::default()
                    },
                )
                .await,
            Err(PlanApplicationError::Validation(
                PlanValidationError::MaxBelowBaseContribution
            ))
        );
        assert_eq!(
            service.get(created.id).await.unwrap().base_contribution,
            money("1000.00")
        );
    }

    /// 验证 service 能通过专用用例切换计划启停状态。
    #[tokio::test]
    async fn service_sets_active_state() {
        let service = InvestmentPlanService::new(Arc::new(FakeRepository::default()));
        let created = service.create(create_input()).await.unwrap();

        let disabled = service.set_active(created.id, false).await.unwrap();

        assert!(!disabled.is_active);
        assert!(disabled.updated_at > created.updated_at);
    }
}

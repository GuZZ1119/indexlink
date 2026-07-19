//! Investment Plan HTTP routes.

use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use investment_plans::{
    BucketAllocationRatio, CreateInvestmentPlan, InvestmentPlan, InvestmentPlanExecutionPreview,
    PreviewInvestmentPlanExecution, ScheduleKind, TwoBucketAllocationConfig, UpdateInvestmentPlan,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::{ApiError, ApiState};

/// 创建 investment plan 的入站 DTO。
#[derive(Debug, Deserialize)]
struct CreateInvestmentPlanRequest {
    /// 用户可读计划名称。
    name: String,
    /// 投资标的代码。
    symbol: String,
    /// 基准定投金额，JSON 中必须是字符串。
    #[serde(with = "rust_decimal::serde::str")]
    base_contribution: Decimal,
    /// 三位币种代码。
    currency: String,
    /// MVP 只接受 monthly。
    schedule_kind: ScheduleKindRequest,
    /// 每月执行日。
    schedule_day: i16,
    /// 单次执行金额硬上限，JSON 中必须是字符串。
    #[serde(with = "rust_decimal::serde::str")]
    max_single_execution: Decimal,
}

/// 更新 investment plan 的入站 DTO。
#[derive(Debug, Deserialize)]
struct UpdateInvestmentPlanRequest {
    /// 可选的新用户可读计划名称。
    name: Option<String>,
    /// 可选的新基准定投金额，JSON 中必须是字符串。
    #[serde(default, with = "rust_decimal::serde::str_option")]
    base_contribution: Option<Decimal>,
    /// 可选的新每月执行日。
    schedule_day: Option<i16>,
    /// 可选的新单次执行金额硬上限，JSON 中必须是字符串。
    #[serde(default, with = "rust_decimal::serde::str_option")]
    max_single_execution: Option<Decimal>,
    /// 可选启停状态。
    is_active: Option<bool>,
}

/// 执行预览的入站 DTO。
#[derive(Debug, Deserialize)]
struct PreviewInvestmentPlanExecutionRequest {
    /// 本次预览使用的月内日期。
    day_of_month: i16,
    /// 可选双桶分配配置；提供后仅在 due 时返回拆分金额。
    bucket_allocation: Option<TwoBucketAllocationRequest>,
}

/// 双桶分配配置的入站 DTO。
#[derive(Debug, Deserialize)]
struct TwoBucketAllocationRequest {
    /// 常规定投桶比例，JSON 中必须是字符串。
    #[serde(with = "rust_decimal::serde::str")]
    core_ratio: Decimal,
    /// 机会桶比例，JSON 中必须是字符串。
    #[serde(with = "rust_decimal::serde::str")]
    opportunity_ratio: Decimal,
}

/// API 边界支持的 schedule kind。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ScheduleKindRequest {
    /// 每月固定日期触发。
    Monthly,
}

impl From<ScheduleKindRequest> for ScheduleKind {
    /// Convert the API schedule value into the domain schedule kind.
    fn from(value: ScheduleKindRequest) -> Self {
        match value {
            ScheduleKindRequest::Monthly => Self::Monthly,
        }
    }
}

impl From<CreateInvestmentPlanRequest> for CreateInvestmentPlan {
    /// Convert a validated API DTO into the domain create input.
    fn from(value: CreateInvestmentPlanRequest) -> Self {
        Self {
            name: value.name,
            symbol: value.symbol,
            base_contribution: value.base_contribution,
            currency: value.currency,
            schedule_kind: value.schedule_kind.into(),
            schedule_day: value.schedule_day,
            max_single_execution: value.max_single_execution,
        }
    }
}

impl From<UpdateInvestmentPlanRequest> for UpdateInvestmentPlan {
    /// Convert the API update DTO into the domain update input.
    fn from(value: UpdateInvestmentPlanRequest) -> Self {
        Self {
            name: value.name,
            base_contribution: value.base_contribution,
            schedule_day: value.schedule_day,
            max_single_execution: value.max_single_execution,
            is_active: value.is_active,
        }
    }
}

impl PreviewInvestmentPlanExecutionRequest {
    /// Convert the API preview DTO into validated domain inputs.
    fn into_domain(
        self,
    ) -> Result<
        (
            PreviewInvestmentPlanExecution,
            Option<TwoBucketAllocationConfig>,
        ),
        ApiError,
    > {
        let input = PreviewInvestmentPlanExecution::new(self.day_of_month)?;
        let bucket_config = self
            .bucket_allocation
            .map(TwoBucketAllocationRequest::into_domain)
            .transpose()?;

        Ok((input, bucket_config))
    }
}

impl TwoBucketAllocationRequest {
    /// Convert API ratio strings into a validated domain bucket config.
    fn into_domain(self) -> Result<TwoBucketAllocationConfig, ApiError> {
        TwoBucketAllocationConfig::new(
            BucketAllocationRatio::new(self.core_ratio)?,
            BucketAllocationRatio::new(self.opportunity_ratio)?,
        )
        .map_err(Into::into)
    }
}

/// 构建 investment plan routes。
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/investment-plans", post(create_plan).get(list_plans))
        .route(
            "/investment-plans/:id",
            get(get_plan).patch(update_plan).delete(delete_plan),
        )
        .route(
            "/investment-plans/:id/execution-preview",
            post(preview_plan_execution),
        )
}

/// 创建 investment plan。
async fn create_plan(
    State(state): State<ApiState>,
    input: Result<Json<CreateInvestmentPlanRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<InvestmentPlan>), ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    Ok((
        StatusCode::CREATED,
        Json(state.plans().create(input.into()).await?),
    ))
}

/// 列出 investment plans。
async fn list_plans(State(state): State<ApiState>) -> Result<Json<Vec<InvestmentPlan>>, ApiError> {
    Ok(Json(state.plans().list().await?))
}

/// 按 ID 获取 investment plan。
async fn get_plan(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<InvestmentPlan>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    Ok(Json(state.plans().get(id).await?))
}

/// 更新 investment plan。
async fn update_plan(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<UpdateInvestmentPlanRequest>, JsonRejection>,
) -> Result<Json<InvestmentPlan>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    Ok(Json(state.plans().update(id, input.into()).await?))
}

/// 删除一个定投标的及其本地关联记录。
async fn delete_plan(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    state.plans().delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 预览 investment plan 在指定日期的执行状态。
async fn preview_plan_execution(
    State(state): State<ApiState>,
    id: Result<Path<Uuid>, PathRejection>,
    input: Result<Json<PreviewInvestmentPlanExecutionRequest>, JsonRejection>,
) -> Result<Json<InvestmentPlanExecutionPreview>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let (input, bucket_config) = input.into_domain()?;

    let preview = match bucket_config {
        Some(bucket_config) => {
            state
                .plans()
                .preview_execution_with_buckets(id, input, bucket_config)
                .await?
        }
        None => state.plans().preview_execution(id, input).await?,
    };

    Ok(Json(preview))
}

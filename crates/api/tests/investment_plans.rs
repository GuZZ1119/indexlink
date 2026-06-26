use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt;
use indexlink_api::{build_router, ApiState, ReadinessCheck, ReadinessError};
use investment_plans::{
    CreateInvestmentPlan, InvestmentPlan, InvestmentPlanRepository, InvestmentPlanService,
    PlanRepositoryError, ScheduleKind, UpdateInvestmentPlan,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

/// Readiness stub used by investment plan route tests.
struct Ready;

#[async_trait]
impl ReadinessCheck for Ready {
    /// Always report dependencies as available.
    async fn check(&self) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// In-memory repository fake for exercising HTTP routes through the service.
#[derive(Default)]
struct FakeRepository {
    plans: Mutex<Vec<InvestmentPlan>>,
}

#[async_trait]
impl InvestmentPlanRepository for FakeRepository {
    /// Store the normalized create input as a persisted plan.
    async fn create(
        &self,
        input: CreateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        let mut plans = self.plans.lock().unwrap();
        let plan = plan_from(Uuid::from_u128((plans.len() + 1) as u128), input);
        plans.push(plan.clone());
        Ok(plan)
    }

    /// Return a snapshot of stored plans.
    async fn list(&self) -> Result<Vec<InvestmentPlan>, PlanRepositoryError> {
        Ok(self.plans.lock().unwrap().clone())
    }

    /// Return one stored plan by ID.
    async fn get(&self, id: Uuid) -> Result<InvestmentPlan, PlanRepositoryError> {
        self.plans
            .lock()
            .unwrap()
            .iter()
            .find(|plan| plan.id == id)
            .cloned()
            .ok_or(PlanRepositoryError::NotFound)
    }

    /// Updates are outside this PR's route scope.
    async fn update(
        &self,
        _id: Uuid,
        _input: UpdateInvestmentPlan,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }

    /// Active-state toggles are outside this PR's route scope.
    async fn set_active(
        &self,
        _id: Uuid,
        _is_active: bool,
    ) -> Result<InvestmentPlan, PlanRepositoryError> {
        Err(PlanRepositoryError::Unavailable)
    }
}

/// Convert service input into a stored test plan.
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

/// Build an API app wired to the investment plan fake repository.
fn app(repository: Arc<FakeRepository>) -> axum::Router {
    build_router(ApiState::with_readiness_and_plans(
        Arc::new(Ready),
        InvestmentPlanService::new(repository),
        "0.1.0",
    ))
}

/// Parse an HTTP response body as JSON.
async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Build a normalized domain input for seeding the fake repository.
fn create_input() -> CreateInvestmentPlan {
    CreateInvestmentPlan {
        name: "Core ETF".to_owned(),
        symbol: "VOO".to_owned(),
        base_contribution: Decimal::new(1000, 0),
        currency: "USD".to_owned(),
        schedule_kind: ScheduleKind::Monthly,
        schedule_day: 15,
        max_single_execution: Decimal::new(1500, 0),
    }
}

/// Verify create route uses DTO conversion and returns string-encoded money.
#[tokio::test]
async fn create_plan_returns_normalized_plan_json() {
    let app = app(Arc::new(FakeRepository::default()));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "  Core ETF  ",
                        "symbol": " voo ",
                        "base_contribution": "1000.00",
                        "currency": " usd ",
                        "schedule_kind": "monthly",
                        "schedule_day": 15,
                        "max_single_execution": "1500.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["name"], json!("Core ETF"));
    assert_eq!(body["symbol"], json!("VOO"));
    assert!(body["base_contribution"].is_string());
}

/// Verify JSON extractor failures return the shared error envelope.
#[tokio::test]
async fn invalid_create_json_returns_safe_bad_request() {
    let response = app(Arc::new(FakeRepository::default()))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/investment-plans")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"base_contribution": 1000.0}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(response).await,
        json!({"error": {"code": "bad_request", "message": "invalid request"}})
    );
}

/// Verify list/get routes read plans and map bad IDs safely.
#[tokio::test]
async fn list_get_and_bad_id_routes_use_service() {
    let repository = Arc::new(FakeRepository::default());
    let created = repository.create(create_input()).await.unwrap();
    let app = app(repository);

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/investment-plans")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_json(list).await[0]["id"], json!(created.id));

    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/investment-plans/{}", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_json(get).await["name"], json!("Core ETF"));

    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/investment-plans/{}", Uuid::from_u128(99)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response_json(missing).await,
        json!({"error": {"code": "not_found", "message": "resource not found"}})
    );

    let bad_id = app
        .oneshot(
            Request::builder()
                .uri("/investment-plans/not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_id.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(bad_id).await,
        json!({"error": {"code": "bad_request", "message": "invalid request"}})
    );
}

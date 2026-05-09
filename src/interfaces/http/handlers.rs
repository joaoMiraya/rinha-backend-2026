use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};

use crate::application::services::fraud_score_service::score_transaction;
use crate::application::services::readiness_service::is_ready;
use crate::application::state::AppState;
use crate::domain::models::score::{FraudScoreRequest, FraudScoreResponse};
use crate::shared::error::AppError;

pub async fn ready_handler(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    if is_ready(state.as_ref()).await? {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::SERVICE_UNAVAILABLE)
    }
}

pub async fn fraud_score_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FraudScoreRequest>,
) -> Result<Json<FraudScoreResponse>, AppError> {
    let _ = &payload.id;
    let response = score_transaction(state.as_ref(), &payload).await?;
    Ok(Json(response))
}

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use crate::application::state::AppState;

use super::handlers::{fraud_score_handler, ready_handler};

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ready", get(ready_handler))
        .route("/fraud-score", post(fraud_score_handler))
        .with_state(state)
}

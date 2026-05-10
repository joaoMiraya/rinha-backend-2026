use crate::application::state::AppState;
use crate::domain::models::score::{FraudScoreRequest, FraudScoreResponse};
use crate::domain::services::vectorization::vectorize_transaction;
use crate::shared::error::AppError;

pub async fn score_transaction(
    state: &AppState,
    payload: &FraudScoreRequest,
) -> Result<FraudScoreResponse, AppError> {
    let vector = vectorize_transaction(payload, &state.normalization, &state.mcc_risk)?;
    let fraud_count = state.index.fraud_count_for(&vector)?;
    let fraud_score = fraud_count as f32 / 5.0;
    Ok(FraudScoreResponse {
        approved: fraud_score < 0.6,
        fraud_score,
    })
}

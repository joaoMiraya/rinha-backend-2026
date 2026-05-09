use crate::application::state::AppState;
use crate::domain::models::score::{FraudScoreRequest, FraudScoreResponse};
use crate::domain::services::vectorization::vectorize_transaction;
use crate::shared::error::AppError;

pub async fn score_transaction(
    state: &AppState,
    payload: &FraudScoreRequest,
) -> Result<FraudScoreResponse, AppError> {
    let vector = vectorize_transaction(payload, &state.normalization, &state.mcc_risk)?;
    let labels = state
        .qdrant
        .search_labels(&state.collection_name, &vector, 5)
        .await?;

    if labels.len() != 5 {
        return Err(AppError::Upstream(format!(
            "Expected 5 neighbors from Qdrant, got {}",
            labels.len()
        )));
    }

    let fraud_count = labels
        .iter()
        .filter(|label| label.as_str() == "fraud")
        .count();
    let fraud_score = fraud_count as f32 / 5.0;
    Ok(FraudScoreResponse {
        approved: fraud_score < 0.6,
        fraud_score,
    })
}

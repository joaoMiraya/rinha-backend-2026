use crate::application::state::AppState;
use crate::shared::error::AppError;

pub async fn is_ready(state: &AppState) -> Result<bool, AppError> {
    let _ = state;
    Ok(true)
}

use std::time::{Duration, Instant};

use crate::application::state::AppState;
use crate::infrastructure::config::app_config::AppConfig;
use crate::infrastructure::qdrant::client::QdrantClient;
use crate::shared::error::AppError;

pub async fn is_ready(state: &AppState) -> Result<bool, AppError> {
    state
        .qdrant
        .is_ready(
            &state.collection_name,
            state.readiness_require_points,
            state.ready_min_points,
        )
        .await
}

pub async fn wait_for_startup_readiness(
    config: &AppConfig,
    qdrant: &QdrantClient,
) -> Result<(), AppError> {
    let poll_interval = Duration::from_millis(config.health_poll_interval_ms);
    let timeout = Duration::from_secs(config.startup_timeout_secs);
    let started_at = Instant::now();

    loop {
        let ready = qdrant
            .is_ready(
                &config.collection_name,
                config.readiness_require_points,
                config.ready_min_points,
            )
            .await
            .unwrap_or(false);
        if ready {
            return Ok(());
        }
        if started_at.elapsed() >= timeout {
            return Err(AppError::Startup(format!(
                "Readiness timeout for collection '{}'",
                config.collection_name
            )));
        }
        tokio::time::sleep(poll_interval).await;
    }
}

pub async fn wait_for_qdrant_service(
    config: &AppConfig,
    qdrant: &QdrantClient,
) -> Result<(), AppError> {
    let poll_interval = Duration::from_millis(config.health_poll_interval_ms);
    let timeout = Duration::from_secs(config.startup_timeout_secs);
    let started_at = Instant::now();

    loop {
        let service_available = qdrant
            .is_ready(&config.collection_name, false, 0)
            .await
            .unwrap_or(false);
        if service_available {
            return Ok(());
        }
        if started_at.elapsed() >= timeout {
            return Err(AppError::Startup(format!(
                "Qdrant service timeout for collection '{}'",
                config.collection_name
            )));
        }
        tokio::time::sleep(poll_interval).await;
    }
}

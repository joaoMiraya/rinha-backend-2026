use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use tokio::net::TcpListener;

use crate::application::services::ingestion_service::run_ingestion;
use crate::application::services::readiness_service::{
    wait_for_qdrant_service, wait_for_startup_readiness,
};
use crate::application::state::AppState;
use crate::infrastructure::config::app_config::{AppConfig, AppMode};
use crate::infrastructure::qdrant::client::QdrantClient;
use crate::infrastructure::resources::loaders::{load_mcc_risk, load_normalization};
use crate::interfaces::http::router::build_router;
use crate::shared::error::AppError;

pub async fn run() -> Result<(), AppError> {
    let config = AppConfig::from_env()?;
    if config.mode == AppMode::Ingest {
        run_ingestion(config).await?;
        return Ok(());
    }
    run_server(config).await
}

async fn run_server(config: AppConfig) -> Result<(), AppError> {
    let normalization = load_normalization(&config.normalization_path)?;
    let mcc_risk = load_mcc_risk(&config.mcc_risk_path)?;

    let client = Client::builder()
        .timeout(Duration::from_millis(config.request_timeout_ms))
        .pool_max_idle_per_host(128)
        .build()?;

    let qdrant = QdrantClient::new(config.qdrant_url.clone(), client);
    wait_for_qdrant_service(&config, &qdrant).await?;

    if config.ingest_on_startup {
        run_ingestion(config.clone()).await?;
    }

    wait_for_startup_readiness(&config, &qdrant).await?;

    let state = Arc::new(AppState {
        qdrant,
        collection_name: config.collection_name.clone(),
        normalization,
        mcc_risk,
        readiness_require_points: config.readiness_require_points,
        ready_min_points: config.ready_min_points,
    });

    let app = build_router(state);
    let address = format!("{}:{}", config.host, config.port)
        .parse::<SocketAddr>()
        .map_err(|err| AppError::Config(format!("Invalid bind address: {}", err)))?;
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await.map_err(AppError::from)
}

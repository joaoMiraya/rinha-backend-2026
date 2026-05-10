use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;

use crate::application::state::AppState;
use crate::infrastructure::config::app_config::AppConfig;
use crate::infrastructure::resources::loaders::{
    load_mcc_risk, load_normalization, load_reference_index,
};
use crate::interfaces::http::router::build_router;
use crate::shared::error::AppError;

pub async fn run() -> Result<(), AppError> {
    run_server(AppConfig::from_env()?).await
}

async fn run_server(config: AppConfig) -> Result<(), AppError> {
    let normalization = load_normalization(&config.normalization_path)?;
    let mcc_risk = load_mcc_risk(&config.mcc_risk_path)?;
    let index = tokio::task::spawn_blocking({
        let index_path = config.index_path.clone();
        let references_path = config.references_path.clone();
        move || load_reference_index(&index_path, &references_path)
    })
    .await??;

    let state = Arc::new(AppState {
        index,
        normalization,
        mcc_risk,
    });

    let app = build_router(state);
    let address = format!("{}:{}", config.host, config.port)
        .parse::<SocketAddr>()
        .map_err(|err| AppError::Config(format!("Invalid bind address: {}", err)))?;
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await.map_err(AppError::from)
}

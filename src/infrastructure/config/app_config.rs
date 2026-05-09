use std::env;
use std::path::PathBuf;

use crate::shared::error::AppError;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AppMode {
    Serve,
    Ingest,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mode: AppMode,
    pub host: String,
    pub port: u16,
    pub qdrant_url: String,
    pub collection_name: String,
    pub normalization_path: PathBuf,
    pub mcc_risk_path: PathBuf,
    pub references_path: PathBuf,
    pub request_timeout_ms: u64,
    pub startup_timeout_secs: u64,
    pub health_poll_interval_ms: u64,
    pub readiness_require_points: bool,
    pub ready_min_points: u64,
    pub ingest_on_startup: bool,
    pub force_reingest: bool,
    pub ingest_batch_size: usize,
    pub ingest_wait: bool,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            mode: parse_mode_from_args(),
            host: env_var("APP_HOST", "0.0.0.0"),
            port: parse_env("APP_PORT", 3000_u16)?,
            qdrant_url: env_var("QDRANT_URL", "http://qdrant:6333"),
            collection_name: env_var("QDRANT_COLLECTION", "fraud_vectors"),
            normalization_path: PathBuf::from(env_var(
                "NORMALIZATION_FILE",
                "/app/resources/normalization.json",
            )),
            mcc_risk_path: PathBuf::from(env_var("MCC_RISK_FILE", "/app/resources/mcc_risk.json")),
            references_path: PathBuf::from(env_var(
                "REFERENCES_FILE",
                "/app/resources/references.json.gz",
            )),
            request_timeout_ms: parse_env("REQUEST_TIMEOUT_MS", 300_u64)?,
            startup_timeout_secs: parse_env("STARTUP_TIMEOUT_SECS", 900_u64)?,
            health_poll_interval_ms: parse_env("HEALTH_POLL_INTERVAL_MS", 2_000_u64)?,
            readiness_require_points: parse_env("READINESS_REQUIRE_POINTS", true)?,
            ready_min_points: parse_env("READY_MIN_POINTS", 1_u64)?,
            ingest_on_startup: parse_env("INGEST_ON_STARTUP", false)?,
            force_reingest: parse_env("FORCE_REINGEST", false)?,
            ingest_batch_size: parse_env("INGEST_BATCH_SIZE", 1_000_usize)?,
            ingest_wait: parse_env("INGEST_WAIT", false)?,
        })
    }
}

fn parse_mode_from_args() -> AppMode {
    let args: Vec<String> = env::args().skip(1).collect();
    for (index, arg) in args.iter().enumerate() {
        if arg == "--mode" {
            if let Some(mode) = args.get(index + 1) {
                if mode == "ingest" {
                    return AppMode::Ingest;
                }
            }
        } else if let Some(mode) = arg.strip_prefix("--mode=") {
            if mode == "ingest" {
                return AppMode::Ingest;
            }
        }
    }
    AppMode::Serve
}

fn env_var(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn parse_env<T>(key: &str, default: T) -> Result<T, AppError>
where
    T: std::str::FromStr + Copy,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(value) => value.parse::<T>().map_err(|error| {
            AppError::Config(format!(
                "Invalid value '{}' for env {}: {}",
                value, key, error
            ))
        }),
        Err(_) => Ok(default),
    }
}

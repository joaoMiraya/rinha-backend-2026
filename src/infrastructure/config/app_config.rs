use std::env;
use std::path::PathBuf;

use crate::shared::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub normalization_path: PathBuf,
    pub mcc_risk_path: PathBuf,
    pub index_path: PathBuf,
    pub references_path: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            host: env_var("APP_HOST", "0.0.0.0"),
            port: parse_env("APP_PORT", 3000_u16)?,
            normalization_path: PathBuf::from(env_var(
                "NORMALIZATION_FILE",
                "/app/resources/normalization.json",
            )),
            mcc_risk_path: PathBuf::from(env_var("MCC_RISK_FILE", "/app/resources/mcc_risk.json")),
            index_path: PathBuf::from(env_var("INDEX_FILE", "/app/resources/index.bin")),
            references_path: PathBuf::from(env_var(
                "REFERENCES_FILE",
                "/app/resources/references.json.gz",
            )),
        })
    }
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

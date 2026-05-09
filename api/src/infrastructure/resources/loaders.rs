use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::domain::models::score::NormalizationConfig;
use crate::shared::error::AppError;

pub fn load_normalization(path: &Path) -> Result<NormalizationConfig, AppError> {
    Ok(serde_json::from_reader(BufReader::new(File::open(path)?))?)
}

pub fn load_mcc_risk(path: &Path) -> Result<HashMap<String, f32>, AppError> {
    Ok(serde_json::from_reader(BufReader::new(File::open(path)?))?)
}

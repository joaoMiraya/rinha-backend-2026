use std::collections::HashMap;

use crate::domain::models::score::NormalizationConfig;
use crate::infrastructure::qdrant::client::QdrantClient;

#[derive(Debug, Clone)]
pub struct AppState {
    pub qdrant: QdrantClient,
    pub collection_name: String,
    pub normalization: NormalizationConfig,
    pub mcc_risk: HashMap<String, f32>,
    pub readiness_require_points: bool,
    pub ready_min_points: u64,
}

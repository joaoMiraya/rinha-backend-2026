use std::collections::HashMap;

use crate::domain::models::score::NormalizationConfig;
use crate::infrastructure::search::LocalReferenceIndex;

#[derive(Debug)]
pub struct AppState {
    pub index: LocalReferenceIndex,
    pub normalization: NormalizationConfig,
    pub mcc_risk: HashMap<String, f32>,
}

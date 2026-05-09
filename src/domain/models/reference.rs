use serde::Deserialize;

use crate::domain::VECTOR_SIZE;

#[derive(Debug, Deserialize)]
pub struct ReferenceEntry {
    pub vector: [f32; VECTOR_SIZE],
    pub label: String,
}

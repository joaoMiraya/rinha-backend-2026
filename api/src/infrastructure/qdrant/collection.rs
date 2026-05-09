use serde_json::{Value, json};

use crate::domain::VECTOR_SIZE;

pub(super) fn collection_config_body() -> Value {
    json!({
        "vectors": {
            "size": VECTOR_SIZE,
            "distance": "Euclid",
            "on_disk": true
        },
        "hnsw_config": {
            "m": 8,
            "ef_construct": 64,
            "full_scan_threshold": 20000,
            "on_disk": true
        },
        "optimizers_config": {
            "default_segment_number": 1,
            "memmap_threshold": 20000
        },
        "quantization_config": {
            "scalar": {
                "type": "int8",
                "quantile": 0.99,
                "always_ram": false
            }
        }
    })
}

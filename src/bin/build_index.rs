use std::path::PathBuf;

use api::infrastructure::resources::loaders::build_reference_index;
use api::shared::error::AppError;

fn main() -> Result<(), AppError> {
    let references_path =
        std::env::var("REFERENCES_FILE").unwrap_or_else(|_| "/app/resources/references.json.gz".into());
    let index_path =
        std::env::var("INDEX_FILE").unwrap_or_else(|_| "/app/resources/index.bin".into());

    build_reference_index(&PathBuf::from(references_path), &PathBuf::from(index_path))
}

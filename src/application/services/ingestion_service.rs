use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use flate2::read::GzDecoder;

use crate::infrastructure::config::app_config::AppConfig;
use crate::infrastructure::qdrant::blocking_client::{
    IngestPayload, IngestPoint, QdrantBlockingClient,
};
use crate::infrastructure::resources::reference_reader::for_each_reference_entry;
use crate::shared::error::AppError;

pub async fn run_ingestion(config: AppConfig) -> Result<(), AppError> {
    let timeout = Duration::from_secs(30);
    let config_clone = config.clone();
    tokio::task::spawn_blocking(move || run_ingestion_blocking(config_clone, timeout)).await?
}

fn run_ingestion_blocking(config: AppConfig, timeout: Duration) -> Result<(), AppError> {
    let qdrant = QdrantBlockingClient::new(config.qdrant_url.clone(), timeout)?;

    if config.force_reingest {
        qdrant.recreate_collection(&config.collection_name)?;
    } else {
        qdrant.ensure_collection(&config.collection_name)?;
        if qdrant.count_points(&config.collection_name)? > 0 {
            return Ok(());
        }
    }

    let file = File::open(&config.references_path)?;
    let reader = BufReader::new(GzDecoder::new(file));
    let mut next_id = 1_u64;
    let mut processed = 0_u64;
    let mut batch: Vec<IngestPoint> = Vec::with_capacity(config.ingest_batch_size);

    for_each_reference_entry(reader, |entry| {
        if entry.label != "fraud" && entry.label != "legit" {
            return Err(AppError::Startup(format!(
                "Invalid label '{}' in reference dataset",
                entry.label
            )));
        }

        batch.push(IngestPoint {
            id: next_id,
            vector: entry.vector,
            payload: IngestPayload { label: entry.label },
        });
        next_id += 1;

        if batch.len() >= config.ingest_batch_size {
            qdrant.upsert_points(&config.collection_name, &batch, config.ingest_wait)?;
            processed += batch.len() as u64;
            batch.clear();
        }
        Ok(())
    })?;

    if !batch.is_empty() {
        qdrant.upsert_points(&config.collection_name, &batch, config.ingest_wait)?;
        processed += batch.len() as u64;
    }

    if processed == 0 {
        return Err(AppError::Startup(
            "Reference dataset ingestion did not import any vectors".into(),
        ));
    }
    Ok(())
}

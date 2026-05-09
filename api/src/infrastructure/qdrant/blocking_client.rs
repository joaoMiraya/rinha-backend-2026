use std::time::Duration;

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::shared::error::AppError;

use super::collection::collection_config_body;

#[derive(Debug, Serialize)]
pub struct IngestPoint {
    pub id: u64,
    pub vector: [f32; crate::domain::VECTOR_SIZE],
    pub payload: IngestPayload,
}

#[derive(Debug, Serialize)]
pub struct IngestPayload {
    pub label: String,
}

#[derive(Debug)]
pub struct QdrantBlockingClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl QdrantBlockingClient {
    pub fn new(base_url: String, timeout: Duration) -> Result<Self, AppError> {
        Ok(Self {
            base_url,
            client: reqwest::blocking::Client::builder()
                .timeout(timeout)
                .build()?,
        })
    }

    pub fn recreate_collection(&self, collection_name: &str) -> Result<(), AppError> {
        let delete_url = format!("{}/collections/{}", self.base_url, collection_name);
        let _ = self.client.delete(delete_url).send()?;
        self.create_collection(collection_name)
    }

    pub fn ensure_collection(&self, collection_name: &str) -> Result<(), AppError> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);
        let response = self.client.get(url).send()?;
        if response.status().is_success() {
            return Ok(());
        }
        if response.status() != StatusCode::NOT_FOUND {
            return Err(AppError::Upstream(format!(
                "Qdrant collection check failed with status {}",
                response.status()
            )));
        }
        self.create_collection(collection_name)
    }

    pub fn count_points(&self, collection_name: &str) -> Result<u64, AppError> {
        let url = format!(
            "{}/collections/{}/points/count",
            self.base_url, collection_name
        );
        let response = self
            .client
            .post(url)
            .json(&json!({ "exact": false }))
            .send()?;
        if !response.status().is_success() {
            return Err(AppError::Upstream(format!(
                "Qdrant count failed with status {}",
                response.status()
            )));
        }
        let payload = response.json::<QdrantCountEnvelope>()?;
        Ok(payload.result.count)
    }

    pub fn upsert_points(
        &self,
        collection_name: &str,
        points: &[IngestPoint],
        wait: bool,
    ) -> Result<(), AppError> {
        let url = format!("{}/collections/{}/points", self.base_url, collection_name);
        let response = self
            .client
            .put(url)
            .query(&[("wait", wait)])
            .json(&json!({ "points": points }))
            .send()?;
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(AppError::Upstream(format!(
            "Qdrant upsert failed: status={}, body={}",
            status, body
        )))
    }

    fn create_collection(&self, collection_name: &str) -> Result<(), AppError> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);
        let response = self
            .client
            .put(url)
            .json(&collection_config_body())
            .send()?;
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(AppError::Upstream(format!(
            "Failed to create collection: status={}, body={}",
            status, body
        )))
    }
}

#[derive(Debug, Deserialize)]
struct QdrantCountEnvelope {
    result: QdrantCountResult,
}

#[derive(Debug, Deserialize)]
struct QdrantCountResult {
    count: u64,
}

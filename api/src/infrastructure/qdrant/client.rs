use std::collections::HashMap;

use axum::http::StatusCode;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::VECTOR_SIZE;
use crate::shared::error::AppError;

use super::collection::collection_config_body;

#[derive(Clone, Debug)]
pub struct QdrantClient {
    base_url: String,
    client: Client,
}

impl QdrantClient {
    pub fn new(base_url: String, client: Client) -> Self {
        Self { base_url, client }
    }

    pub async fn health(&self) -> Result<(), AppError> {
        let url = format!("{}/healthz", self.base_url);
        let response = self.client.get(url).send().await?;
        if response.status().is_success() {
            return Ok(());
        }
        Err(AppError::Upstream(format!(
            "Qdrant healthcheck failed with status {}",
            response.status()
        )))
    }

    pub async fn ensure_collection(&self, collection_name: &str) -> Result<(), AppError> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);
        let response = self.client.get(url).send().await?;
        if response.status().is_success() {
            return Ok(());
        }
        if response.status() != StatusCode::NOT_FOUND {
            return Err(AppError::Upstream(format!(
                "Qdrant collection check failed with status {}",
                response.status()
            )));
        }
        self.create_collection(collection_name).await
    }

    async fn create_collection(&self, collection_name: &str) -> Result<(), AppError> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);
        let response = self
            .client
            .put(url)
            .json(&collection_config_body())
            .send()
            .await?;
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(AppError::Upstream(format!(
            "Failed to create collection: status={}, body={}",
            status, body
        )))
    }

    pub async fn count_points(&self, collection_name: &str) -> Result<u64, AppError> {
        let url = format!(
            "{}/collections/{}/points/count",
            self.base_url, collection_name
        );
        let response = self
            .client
            .post(url)
            .json(&json!({ "exact": false }))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(AppError::Upstream(format!(
                "Qdrant count failed with status {}",
                response.status()
            )));
        }
        let payload = response.json::<QdrantCountEnvelope>().await?;
        Ok(payload.result.count)
    }

    pub async fn search_labels(
        &self,
        collection_name: &str,
        vector: &[f32; VECTOR_SIZE],
        limit: usize,
    ) -> Result<Vec<String>, AppError> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.base_url, collection_name
        );
        let response = self
            .client
            .post(url)
            .json(&json!({
                "vector": vector,
                "limit": limit,
                "with_payload": true,
                "with_vector": false
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "Qdrant search failed: status={}, body={}",
                status, body
            )));
        }

        let payload = response.json::<QdrantSearchEnvelope>().await?;
        let mut labels = Vec::with_capacity(payload.result.len());
        for point in payload.result {
            let label = point
                .payload
                .as_ref()
                .and_then(|p| p.get("label"))
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::Upstream("Qdrant point missing payload.label".into()))?;
            labels.push(label.to_string());
        }
        Ok(labels)
    }

    pub async fn is_ready(
        &self,
        collection_name: &str,
        require_points: bool,
        min_points: u64,
    ) -> Result<bool, AppError> {
        self.health().await?;
        self.ensure_collection(collection_name).await?;
        if !require_points {
            return Ok(true);
        }
        Ok(self.count_points(collection_name).await? >= min_points)
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

#[derive(Debug, Deserialize)]
struct QdrantSearchEnvelope {
    result: Vec<QdrantSearchPoint>,
}

#[derive(Debug, Deserialize)]
struct QdrantSearchPoint {
    payload: Option<HashMap<String, Value>>,
}

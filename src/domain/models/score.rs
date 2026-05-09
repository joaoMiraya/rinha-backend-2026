use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct NormalizationConfig {
    pub max_amount: f32,
    pub max_installments: f32,
    pub amount_vs_avg_ratio: f32,
    pub max_minutes: f32,
    pub max_km: f32,
    pub max_tx_count_24h: f32,
    pub max_merchant_avg_amount: f32,
}

#[derive(Debug, Deserialize)]
pub struct FraudScoreRequest {
    pub id: String,
    pub transaction: TransactionData,
    pub customer: CustomerData,
    pub merchant: MerchantData,
    pub terminal: TerminalData,
    pub last_transaction: Option<LastTransactionData>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionData {
    pub amount: f32,
    pub installments: u32,
    pub requested_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CustomerData {
    pub avg_amount: f32,
    pub tx_count_24h: u32,
    pub known_merchants: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MerchantData {
    pub id: String,
    pub mcc: String,
    pub avg_amount: f32,
}

#[derive(Debug, Deserialize)]
pub struct TerminalData {
    pub is_online: bool,
    pub card_present: bool,
    pub km_from_home: f32,
}

#[derive(Debug, Deserialize)]
pub struct LastTransactionData {
    pub timestamp: String,
    pub km_from_current: f32,
}

#[derive(Debug, Serialize)]
pub struct FraudScoreResponse {
    pub approved: bool,
    pub fraud_score: f32,
}

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Timelike, Utc};

use crate::domain::{
    VECTOR_SIZE,
    models::score::{FraudScoreRequest, NormalizationConfig},
};
use crate::shared::error::AppError;

pub fn vectorize_transaction(
    payload: &FraudScoreRequest,
    normalization: &NormalizationConfig,
    mcc_risk: &HashMap<String, f32>,
) -> Result<[f32; VECTOR_SIZE], AppError> {
    let requested_at = parse_utc(&payload.transaction.requested_at)?;

    let amount = clamp01(payload.transaction.amount / normalization.max_amount);
    let installments =
        clamp01(payload.transaction.installments as f32 / normalization.max_installments);
    let amount_vs_avg = if payload.customer.avg_amount <= 0.0 {
        1.0
    } else {
        clamp01(
            (payload.transaction.amount / payload.customer.avg_amount)
                / normalization.amount_vs_avg_ratio,
        )
    };
    let hour_of_day = requested_at.hour() as f32 / 23.0;
    let day_of_week = requested_at.weekday().num_days_from_monday() as f32 / 6.0;

    let (minutes_since_last_tx, km_from_last_tx) = match &payload.last_transaction {
        Some(last_tx) => {
            let last_timestamp = parse_utc(&last_tx.timestamp)?;
            let minutes = (requested_at - last_timestamp).num_seconds() as f32 / 60.0;
            (
                clamp01(minutes / normalization.max_minutes),
                clamp01(last_tx.km_from_current / normalization.max_km),
            )
        }
        None => (-1.0, -1.0),
    };

    let km_from_home = clamp01(payload.terminal.km_from_home / normalization.max_km);
    let tx_count_24h =
        clamp01(payload.customer.tx_count_24h as f32 / normalization.max_tx_count_24h);
    let is_online = if payload.terminal.is_online { 1.0 } else { 0.0 };
    let card_present = if payload.terminal.card_present {
        1.0
    } else {
        0.0
    };
    let unknown_merchant = if payload
        .customer
        .known_merchants
        .iter()
        .any(|merchant| merchant == &payload.merchant.id)
    {
        0.0
    } else {
        1.0
    };
    let merchant_mcc_risk = mcc_risk.get(&payload.merchant.mcc).copied().unwrap_or(0.5);
    let merchant_avg_amount =
        clamp01(payload.merchant.avg_amount / normalization.max_merchant_avg_amount);

    Ok([
        amount,
        installments,
        amount_vs_avg,
        hour_of_day,
        day_of_week,
        minutes_since_last_tx,
        km_from_last_tx,
        km_from_home,
        tx_count_24h,
        is_online,
        card_present,
        unknown_merchant,
        merchant_mcc_risk,
        merchant_avg_amount,
    ])
}

fn parse_utc(input: &str) -> Result<DateTime<Utc>, AppError> {
    Ok(DateTime::parse_from_rfc3339(input)?.with_timezone(&Utc))
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::domain::models::score::{
        CustomerData, FraudScoreRequest, MerchantData, NormalizationConfig, TerminalData,
        TransactionData,
    };

    use super::vectorize_transaction;

    fn default_norm() -> NormalizationConfig {
        NormalizationConfig {
            max_amount: 10_000.0,
            max_installments: 12.0,
            amount_vs_avg_ratio: 10.0,
            max_minutes: 1_440.0,
            max_km: 1_000.0,
            max_tx_count_24h: 20.0,
            max_merchant_avg_amount: 10_000.0,
        }
    }

    #[test]
    fn vectorizes_legit_sample_with_null_last_transaction() {
        let payload = FraudScoreRequest {
            id: "tx-1329056812".to_string(),
            transaction: TransactionData {
                amount: 41.12,
                installments: 2,
                requested_at: "2026-03-11T18:45:53Z".to_string(),
            },
            customer: CustomerData {
                avg_amount: 82.24,
                tx_count_24h: 3,
                known_merchants: vec!["MERC-003".to_string(), "MERC-016".to_string()],
            },
            merchant: MerchantData {
                id: "MERC-016".to_string(),
                mcc: "5411".to_string(),
                avg_amount: 60.25,
            },
            terminal: TerminalData {
                is_online: false,
                card_present: true,
                km_from_home: 29.23,
            },
            last_transaction: None,
        };
        let mut mcc_risk = HashMap::new();
        mcc_risk.insert("5411".to_string(), 0.15);

        let vector = vectorize_transaction(&payload, &default_norm(), &mcc_risk).unwrap();

        assert!((vector[0] - 0.0041).abs() < 0.0001);
        assert!((vector[1] - 0.1667).abs() < 0.0002);
        assert!((vector[2] - 0.05).abs() < 0.0001);
        assert!((vector[3] - 0.7826).abs() < 0.0002);
        assert!((vector[4] - 0.3333).abs() < 0.0002);
        assert_eq!(vector[5], -1.0);
        assert_eq!(vector[6], -1.0);
        assert!((vector[7] - 0.0292).abs() < 0.0002);
        assert!((vector[8] - 0.15).abs() < 0.0001);
        assert_eq!(vector[9], 0.0);
        assert_eq!(vector[10], 1.0);
        assert_eq!(vector[11], 0.0);
        assert!((vector[12] - 0.15).abs() < 0.0001);
        assert!((vector[13] - 0.0060).abs() < 0.0001);
    }

    #[test]
    fn defaults_mcc_risk_to_half_when_missing() {
        let payload = FraudScoreRequest {
            id: "tx-1".to_string(),
            transaction: TransactionData {
                amount: 100.0,
                installments: 1,
                requested_at: "2026-03-11T20:23:35Z".to_string(),
            },
            customer: CustomerData {
                avg_amount: 100.0,
                tx_count_24h: 1,
                known_merchants: vec![],
            },
            merchant: MerchantData {
                id: "MERC-X".to_string(),
                mcc: "0000".to_string(),
                avg_amount: 100.0,
            },
            terminal: TerminalData {
                is_online: true,
                card_present: false,
                km_from_home: 10.0,
            },
            last_transaction: None,
        };

        let vector = vectorize_transaction(&payload, &default_norm(), &HashMap::new()).unwrap();
        assert!((vector[12] - 0.5).abs() < f32::EPSILON);
    }
}

use async_trait::async_trait;
use common::{Chain, TxIntent, TxStatus};
use uuid::Uuid;
use crate::{error::TxError, traits::TransactionEngine, types::{BroadcastResult, Quote, RouteHop, SimulationResult}};

pub struct MockTransactionEngine;

impl MockTransactionEngine {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl TransactionEngine for MockTransactionEngine {
    async fn quote(&self, intent: TxIntent) -> Result<Quote, TxError> {
        let slippage_bps = intent.slippage_bps as u128;
        let expected = intent.amount.value;
        let min_out = expected * rust_decimal::Decimal::from(10_000u128 - slippage_bps) / rust_decimal::Decimal::from(10_000u128);

        Ok(Quote {
            quote_id: Uuid::new_v4(),
            intent: intent.clone(),
            expected_out: common::Amount { asset: intent.to.clone(), value: expected },
            minimum_out: common::Amount { asset: intent.to.clone(), value: min_out },
            route: vec![RouteHop { venue: "mock".into(), from: intent.from.clone(), to: intent.to.clone(), portion_bps: 10_000 }],
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            estimated_fee: common::Amount { asset: intent.from.clone(), value: rust_decimal::Decimal::ZERO },
            estimated_gas: 21_000,
        })
    }

    async fn build(&self, _quote: &Quote, _from_address: &str) -> Result<Vec<u8>, TxError> {
        let fake_tx = serde_json::json!({
            "to": "0x0000000000000000000000000000000000000001",
            "value": "0x1",
            "gas": "0x186a0",
            "maxFeePerGas": "0x24",
            "maxPriorityFeePerGas": "0x12",
            "nonce": 0,
            "chainId": 11155111
        });
        Ok(serde_json::to_vec(&fake_tx).unwrap())
    }

    async fn simulate(&self, _chain: Chain, _unsigned: &[u8]) -> Result<SimulationResult, TxError> {
        Ok(SimulationResult { success: true, gas_used: 21_000, warnings: vec![], revert_reason: None })
    }

    async fn broadcast(&self, chain: Chain, _signed_tx_hex: &str) -> Result<BroadcastResult, TxError> {
        Ok(BroadcastResult {
            tx_hash: format!("0xmock{}", Uuid::new_v4().simple()),
            chain,
            submitted_at: chrono::Utc::now(),
            status: TxStatus::Pending,
        })
    }

    async fn status(&self, chain: Chain, tx_hash: &str) -> Result<BroadcastResult, TxError> {
        Ok(BroadcastResult {
            tx_hash: tx_hash.to_string(),
            chain,
            submitted_at: chrono::Utc::now(),
            status: TxStatus::Confirmed { block: 1, confirmations: 1 },
        })
    }
}

//! Real EVM transaction engine backed by alloy.

use std::sync::Arc;
use alloy::{
    network::TransactionBuilder,
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::TransactionRequest,
    transports::http::{Client, Http},
};
use async_trait::async_trait;
use common::{Amount, Chain, TxIntent, TxStatus};
use tracing::{debug, info};

use crate::{
    error::TxError,
    traits::TransactionEngine,
    types::{BroadcastResult, Quote, RouteHop, SimulationResult},
};

pub struct AlloyTransactionEngine {
    provider: Arc<RootProvider<Http<Client>>>,
    chain_id: u64,
}

impl AlloyTransactionEngine {
    pub fn new(rpc_url: &str, chain_id: u64) -> Result<Self, TxError> {
        let url = rpc_url.parse().map_err(|e| TxError::Internal(format!("invalid RPC URL: {e}")))?;
        let provider = ProviderBuilder::new().on_http(url);
        info!(chain_id, "AlloyTransactionEngine initialized");
        Ok(Self { provider: Arc::new(provider), chain_id })
    }
}

#[async_trait]
impl TransactionEngine for AlloyTransactionEngine {
    async fn quote(&self, intent: TxIntent) -> Result<Quote, TxError> {
        let block = self.provider.get_block_number().await.map_err(|e| TxError::Internal(format!("rpc: {e}")))?;
        info!(block, "quote generated");

        let slippage_bps = intent.slippage_bps as u128;
        let expected = intent.amount.value;
        let min_out = expected * rust_decimal::Decimal::from(10_000u128 - slippage_bps) / rust_decimal::Decimal::from(10_000u128);

        Ok(Quote {
            quote_id: uuid::Uuid::new_v4(),
            intent: intent.clone(),
            expected_out: Amount { asset: intent.to.clone(), value: expected },
            minimum_out: Amount { asset: intent.to.clone(), value: min_out },
            route: vec![RouteHop {
                venue: "direct".into(),
                from: intent.from.clone(),
                to: intent.to.clone(),
                portion_bps: 10_000,
            }],
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(300), // 5 min for testing
            estimated_fee: Amount { asset: intent.from.clone(), value: rust_decimal::Decimal::ZERO },
            estimated_gas: 21_000,
        })
    }

    async fn build(&self, quote: &Quote, from_address: &str) -> Result<Vec<u8>, TxError> {
        if chrono::Utc::now() > quote.expires_at {
            return Err(TxError::QuoteExpired(quote.expires_at));
        }

        let to: Address = quote.intent.to.contract.as_deref().unwrap_or("0x0000000000000000000000000000000000000001").parse()
            .map_err(|e| TxError::BuildFailed(format!("invalid address: {e}")))?;
        let from: Address = from_address.parse().map_err(|e| TxError::BuildFailed(format!("invalid from: {e}")))?;
        let value = U256::from(quote.intent.amount.value.mantissa() as u128);

        let nonce = self.provider.get_transaction_count(from).await.map_err(|e| TxError::Internal(format!("rpc nonce: {e}")))?;
        let gas_price = self.provider.get_gas_price().await.map_err(|e| TxError::Internal(format!("rpc gas_price: {e}")))?;

        let tx = TransactionRequest::default()
            .with_from(from)
            .with_to(to)
            .with_nonce(nonce)
            .with_value(value)
            .with_chain_id(self.chain_id)
            .with_gas_limit(100_000)
            .with_max_fee_per_gas(gas_price * 2)
            .with_max_priority_fee_per_gas(gas_price);

        let bytes = serde_json::to_vec(&tx).map_err(|e| TxError::BuildFailed(format!("encode: {e}")))?;
        debug!(len = bytes.len(), nonce, "unsigned tx built");
        Ok(bytes)
    }

    async fn simulate(&self, _chain: Chain, unsigned: &[u8]) -> Result<SimulationResult, TxError> {
        let tx: TransactionRequest = serde_json::from_slice(unsigned).map_err(|e| TxError::SimulationFailed(format!("decode: {e}")))?;
        
        match self.provider.call(&tx).await {
            Ok(_) => Ok(SimulationResult { success: true, gas_used: 100_000, warnings: vec![], revert_reason: None }),
            Err(e) => Ok(SimulationResult { success: false, gas_used: 0, warnings: vec![], revert_reason: Some(e.to_string()) }),
        }
    }

    async fn broadcast(&self, chain: Chain, signed_tx_hex: &str) -> Result<BroadcastResult, TxError> {
        let hex_clean = signed_tx_hex.trim_start_matches("0x");
        let bytes = hex::decode(hex_clean).map_err(|e| TxError::Internal(format!("invalid hex: {e}")))?;
        let pending = self.provider.send_raw_transaction(&bytes).await.map_err(|e| TxError::Internal(format!("broadcast failed: {e}")))?;
        let tx_hash = pending.tx_hash();
        
        Ok(BroadcastResult {
            tx_hash: format!("{tx_hash}"),
            chain,
            submitted_at: chrono::Utc::now(),
            status: TxStatus::Pending,
        })
    }

    async fn status(&self, chain: Chain, tx_hash: &str) -> Result<BroadcastResult, TxError> {
        let hash = tx_hash.parse().map_err(|e| TxError::Internal(format!("invalid hash: {e}")))?;
        let receipt = self.provider.get_transaction_receipt(hash).await.map_err(|e| TxError::Internal(format!("rpc receipt: {e}")))?;
        
        let status = match receipt {
            Some(r) => {
                if r.status() {
                    TxStatus::Confirmed { block: r.block_number.unwrap_or_default(), confirmations: 1 }
                } else {
                    TxStatus::Failed { reason: "reverted".into() }
                }
            },
            None => TxStatus::Pending,
        };

        Ok(BroadcastResult {
            tx_hash: tx_hash.to_string(),
            chain,
            submitted_at: chrono::Utc::now(),
            status,
        })
    }
}

pub fn from_env() -> Result<AlloyTransactionEngine, TxError> {
    dotenvy::dotenv().ok();
    let url = std::env::var("SEPOLIA_RPC_URL").map_err(|_| TxError::Internal("SEPOLIA_RPC_URL not set".into()))?;
    let chain_id = std::env::var("EVM_CHAIN_ID").ok().and_then(|s| s.parse().ok()).unwrap_or(11155111u64);
    AlloyTransactionEngine::new(&url, chain_id)
}

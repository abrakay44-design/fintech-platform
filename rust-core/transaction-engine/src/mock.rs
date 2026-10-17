//! In-memory mock implementation of `TransactionEngine` for testing.

use crate::traits::TransactionEngine;
use crate::types::{BroadcastResult, Quote, RouteHop, SimulationResult};
use crate::TxError;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use common::{Amount, Chain, TxIntent, TxStatus};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory store of quotes and broadcast transactions.
#[derive(Default)]
struct Store {
    quotes: HashMap<Uuid, Quote>,
    broadcasts: HashMap<String, BroadcastResult>,
}

/// A deterministic in-memory implementation of `TransactionEngine`.
///
/// **Not for production.** For tests and local development only.
pub struct MockTransactionEngine {
    store: Arc<RwLock<Store>>,
}

impl MockTransactionEngine {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(Store::default())),
        }
    }
}

impl Default for MockTransactionEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_chain(chain: common::Chain) -> u64 {
    match chain {
        Chain::Ethereum => 1,
        Chain::Polygon => 137,
        Chain::Arbitrum => 42161,
        Chain::Solana => 0,
        Chain::Cosmos => 0,
        Chain::Near => 0,
    }
}

#[async_trait]
impl TransactionEngine for MockTransactionEngine {
    async fn quote(&self, intent: TxIntent) -> Result<Quote, TxError> {
        // Guard against same-asset swaps.
        if intent.from.symbol == intent.to.symbol {
            return Err(TxError::NoRoute {
                from: intent.from.symbol,
                to: intent.to.symbol,
            });
        }

        // Deterministic expected_out: 95% of input (5% slippage baked in).
        let expected_value = intent.amount.value * Decimal::new(95, 2);
        let minimum_value = expected_value * Decimal::new(98, 2); // 2% slippage

        let expected_out = Amount {
            asset: intent.to.clone(),
            value: expected_value,
        };
        let minimum_out = Amount {
            asset: intent.to.clone(),
            value: minimum_value,
        };
        let estimated_fee = Amount {
            asset: intent.from.clone(),
            value: Decimal::new(5, 3), // 0.005
        };

        let route = vec![RouteHop {
            venue: "mock-dex".to_string(),
            from: intent.from.clone(),
            to: intent.to.clone(),
            portion_bps: 10_000,
        }];

        let quote = Quote {
            quote_id: Uuid::new_v4(),
            intent,
            expected_out,
            minimum_out,
            route,
            expires_at: Utc::now() + Duration::minutes(2),
            estimated_fee,
            estimated_gas: 150_000,
        };

        let mut store = self.store.write().await;
        store.quotes.insert(quote.quote_id, quote.clone());

        Ok(quote)
    }

    async fn build(&self, quote: &Quote) -> Result<Vec<u8>, TxError> {
        if Utc::now() > quote.expires_at {
            return Err(TxError::QuoteExpired(quote.expires_at));
        }

        // In the mock, the unsigned payload is just the quote_id + chain_id.
        let chain_id = extract_chain(quote.intent.from.chain);
        let mut out = Vec::with_capacity(8 + 16);
        out.extend_from_slice(&chain_id.to_be_bytes());
        out.extend_from_slice(quote.quote_id.as_bytes());

        Ok(out)
    }

    async fn simulate(
        &self,
        _chain: Chain,
        _unsigned: &[u8],
    ) -> Result<SimulationResult, TxError> {
        Ok(SimulationResult {
            success: true,
            gas_used: 142_500,
            warnings: vec![],
            revert_reason: None,
        })
    }

    async fn broadcast(
        &self,
        chain: Chain,
        signed: &[u8],
    ) -> Result<BroadcastResult, TxError> {
        if signed.is_empty() {
            return Err(TxError::BroadcastFailed("empty payload".into()));
        }

        // Deterministic tx hash: SHA-256 of payload.
        use k256::sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(signed);
        let hash = hasher.finalize();
        let tx_hash = format!("0x{}", hex::encode(hash));

        let result = BroadcastResult {
            tx_hash: tx_hash.clone(),
            chain,
            submitted_at: Utc::now(),
            status: TxStatus::Pending,
        };

        let mut store = self.store.write().await;
        store.broadcasts.insert(tx_hash, result.clone());

        Ok(result)
    }

    async fn status(&self, _chain: Chain, tx_hash: &str) -> Result<BroadcastResult, TxError> {
        let store = self.store.read().await;
        store
            .broadcasts
            .get(tx_hash)
            .cloned()
            .ok_or_else(|| TxError::Internal(format!("tx not found: {tx_hash}")))
    }
}

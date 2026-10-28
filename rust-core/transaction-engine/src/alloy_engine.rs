//! Real EVM transaction engine backed by alloy and Sepolia (or any EVM RPC).

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
use tracing::{debug, info, warn};

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
        let url = rpc_url
            .parse()
            .map_err(|e| TxError::Internal(format!("invalid RPC URL: {e}")))?;
        let provider = ProviderBuilder::new().on_http(url);
        info!(chain_id, "AlloyTransactionEngine initialized");
        Ok(Self {
            provider: Arc::new(provider),
            chain_id,
        })
    }

    fn chain_to_evm(chain: Chain) -> Result<u64, TxError> {
        match chain {
            Chain::Ethereum => Ok(11155111), // Sepolia
            Chain::Polygon => Ok(80002),     // Amoy
            Chain::Arbitrum => Ok(421614),   // Arbitrum Sepolia
            _ => Err(TxError::UnsupportedChain(0)),
        }
    }
}

#[async_trait]
impl TransactionEngine for AlloyTransactionEngine {
    async fn quote(&self, intent: TxIntent) -> Result<Quote, TxError> {
        debug!(?intent, "fetching quote");
        let block = self
            .provider
            .get_block_number()
            .await
            .map_err(|e| TxError::Internal(format!("rpc: {e}")))?;
        info!(block, "quote generated");

        let slippage_bps = intent.slippage_bps as u128;
        let expected = intent.amount.value;
        let min_out = expected * rust_decimal::Decimal::from(10_000u128 - slippage_bps) / rust_decimal::Decimal::from(10_000u128);

        Ok(Quote {
            quote_id: uuid::Uuid::new_v4(),
            intent: intent.clone(),
            expected_out: Amount {
                asset: intent.to.clone(),
                value: expected,
            },
            minimum_out: Amount {
                asset: intent.to.clone(),
                value: min_out,
            },
            route: vec![RouteHop {
                venue: "direct".into(),
                from: intent.from.clone(),
                to: intent.to.clone(),
                portion_bps: 10_000,
            }],
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(30),
            estimated_fee: Amount {
                asset: intent.from.clone(),
                value: rust_decimal::Decimal::ZERO, // TODO: calculate using gas_price
            },
            estimated_gas: 21_000,
        })
    }

    async fn build(&self, quote: &Quote) -> Result<Vec<u8>, TxError> {
        if chrono::Utc::now() > quote.expires_at {
            return Err(TxError::QuoteExpired(quote.expires_at));
        }

                // For a native transfer or simple test, we use the target contract if it exists.
        // Otherwise, fall back to a dummy address (or the user's address if we passed it in).
        let to_str = quote
            .intent
            .to
            .contract
            .as_deref()
            .unwrap_or("0x0000000000000000000000000000000000000001");
            
        let to: Address = to_str
            .parse()
            .map_err(|e| TxError::BuildFailed(format!("invalid address: {e}")))?;
        let value = U256::from(quote.intent.amount.value.mantissa() as u128);

        // 1. Fetch live gas price from the network
        let gas_price = self
            .provider
            .get_gas_price()
            .await
            .map_err(|e| TxError::Internal(format!("rpc gas_price: {e}")))?;

        // 2. Construct EIP-1559 transaction
        let tx = TransactionRequest::default()
            .with_to(to)
            .with_value(value)
            .with_chain_id(self.chain_id)
            .with_gas_limit(100_000) // Placeholder; estimate_gas needed for real accuracy
            .with_max_fee_per_gas(gas_price * 2)
            .with_max_priority_fee_per_gas(gas_price);

        // 3. Serialize to JSON (the frontend will deserialize this and sign it)
        let bytes = serde_json::to_vec(&tx)
            .map_err(|e| TxError::BuildFailed(format!("encode: {e}")))?;
        
        debug!(len = bytes.len(), gas_price, "unsigned tx built with live gas price");
        Ok(bytes)
    }

    async fn simulate(
        &self,
        _chain: Chain,
        unsigned: &[u8],
    ) -> Result<SimulationResult, TxError> {
        // 1. Deserialize the transaction JSON back into a request
        let tx: TransactionRequest = serde_json::from_slice(unsigned)
            .map_err(|e| TxError::SimulationFailed(format!("decode: {e}")))?;

        // 2. Call eth_call on the network
        match self.provider.call(&tx).await {
            Ok(_) => Ok(SimulationResult {
                success: true,
                gas_used: 100_000,
                warnings: vec![],
                revert_reason: None,
            }),
            Err(e) => Ok(SimulationResult {
                success: false,
                gas_used: 0,
                warnings: vec!["execution reverted".into()],
                revert_reason: Some(e.to_string()),
            }),
        }
    }

    async fn broadcast(
        &self,
        chain: Chain,
        _signed: &[u8],
    ) -> Result<BroadcastResult, TxError> {
        let _chain_id = Self::chain_to_evm(chain)?;
        // Will be implemented once we wire up the frontend's signed payload
        Err(TxError::BroadcastFailed(
            "broadcast not yet implemented for alloy engine".into(),
        ))
    }

    async fn status(&self, _chain: Chain, tx_hash: &str) -> Result<BroadcastResult, TxError> {
        warn!(tx_hash, "status stub");
        Ok(BroadcastResult {
            tx_hash: tx_hash.to_string(),
            chain: Chain::Ethereum,
            submitted_at: chrono::Utc::now(),
            status: TxStatus::Pending,
        })
    }
}

pub fn from_env() -> Result<AlloyTransactionEngine, TxError> {
    dotenvy::dotenv().ok();
    let url = std::env::var("SEPOLIA_RPC_URL")
        .map_err(|_| TxError::Internal("SEPOLIA_RPC_URL not set".into()))?;
    let chain_id = std::env::var("EVM_CHAIN_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(11155111u64);
    AlloyTransactionEngine::new(&url, chain_id)
}

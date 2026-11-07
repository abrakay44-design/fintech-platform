//! The `TransactionEngine` trait — central abstraction for DEX interaction.

use crate::types::{BroadcastResult, Quote, SimulationResult};
use crate::TxError;
use async_trait::async_trait;
use common::{Chain, TxIntent};

#[async_trait]
pub trait TransactionEngine: Send + Sync + 'static {
    async fn quote(&self, intent: TxIntent) -> Result<Quote, TxError>;

    async fn build(&self, quote: &Quote, from_address: &str) -> Result<Vec<u8>, TxError>;

    async fn simulate(&self, chain: Chain, unsigned: &[u8]) -> Result<SimulationResult, TxError>;

    async fn broadcast(&self, chain: Chain, signed_tx_hex: &str) -> Result<BroadcastResult, TxError>;

    async fn status(&self, chain: Chain, tx_hash: &str) -> Result<BroadcastResult, TxError>;
}

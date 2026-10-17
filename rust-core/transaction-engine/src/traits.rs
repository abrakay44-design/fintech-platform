//! The `TransactionEngine` trait — central abstraction for DEX interaction.

use crate::types::{BroadcastResult, Quote, SimulationResult};
use crate::TxError;
use async_trait::async_trait;
use common::{Chain, TxIntent};

/// The central trait for transaction operations.
///
/// Implementations must uphold these invariants:
///
/// 1. **Quotes are self-verifying**: rebuilding a quote must produce the same
///    unsigned transaction payload.
/// 2. **Expired quotes are rejected**: `build` must reject quotes past their
///    `expires_at` timestamp.
/// 3. **Slippage is enforced**: the final amount must not violate the quote's
///    minimum_out.
/// 4. **Broadcasting is idempotent**: submitting the same signed payload twice
///    must not result in two separate transactions.
#[async_trait]
pub trait TransactionEngine: Send + Sync + 'static {
    /// Fetch a quote for a given intent.
    async fn quote(&self, intent: TxIntent) -> Result<Quote, TxError>;

    /// Build an unsigned transaction from a quote.
    ///
    /// Returns the raw transaction bytes that must be signed externally.
    /// Rejects expired quotes with `TxError::QuoteExpired`.
    async fn build(&self, quote: &Quote) -> Result<Vec<u8>, TxError>;

    /// Simulate a transaction before it's signed.
    ///
    /// Uses `eth_call` against the target chain.
    async fn simulate(&self, chain: Chain, unsigned: &[u8]) -> Result<SimulationResult, TxError>;

    /// Broadcast a signed transaction to the network.
    ///
    /// Must reject transactions that fail simulation (defense in depth).
    async fn broadcast(&self, chain: Chain, signed: &[u8]) -> Result<BroadcastResult, TxError>;

    /// Poll the status of a previously broadcast transaction.
    async fn status(&self, chain: Chain, tx_hash: &str) -> Result<BroadcastResult, TxError>;
}

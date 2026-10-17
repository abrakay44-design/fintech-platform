//! Domain types for the transaction engine.

use common::{Amount, AssetId, Chain, TxIntent, TxStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single hop in a swap route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHop {
    /// Venue name (e.g., "uniswap-v3", "sushiswap", "curve").
    pub venue: String,
    /// Input asset for this hop.
    pub from: AssetId,
    /// Output asset for this hop.
    pub to: AssetId,
    /// Portion of the total input allocated to this hop, in basis points.
    pub portion_bps: u16,
}

/// A quote from a DEX aggregator (or mocked source).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub quote_id: Uuid,
    pub intent: TxIntent,
    /// Amount the user will receive in the best case.
    pub expected_out: Amount,
    /// Minimum amount user will accept (applies slippage).
    pub minimum_out: Amount,
    /// Route used to fulfill the intent.
    pub route: Vec<RouteHop>,
    /// When this quote stops being valid.
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Estimated transaction fee.
    pub estimated_fee: Amount,
    /// Estimated gas units.
    pub estimated_gas: u64,
}

/// Result of simulating a transaction on-chain before signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub warnings: Vec<String>,
    pub revert_reason: Option<String>,
}

/// A transaction that has been broadcast to the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastResult {
    pub tx_hash: String,
    pub chain: Chain,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub status: TxStatus,
}

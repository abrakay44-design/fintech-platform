use crate::{Amount, AssetId, Chain};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxIntent {
    pub intent_id: Uuid,
    pub user_id: Uuid,
    pub from: AssetId,
    pub to: AssetId,
    pub amount: Amount,
    pub slippage_bps: u16,
    pub deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct SignedTx {
    pub chain: Chain,
    pub raw_bytes: Vec<u8>,
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxStatus {
    Pending,
    Confirmed { block: u64, confirmations: u32 },
    Failed { reason: String },
}


//! Types specific to the orchestration layer.

use chrono::{DateTime, Utc};
use ledger::AccountId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SwapRequest {
    pub from_account: AccountId,
    pub to_account: AccountId,
    pub wallet_handle: wallet_service::PublicKeyHandle,
    pub signature: wallet_service::Signature,
    pub intent: common::TxIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapReceipt {
    pub tx_hash: String,
    pub chain: common::Chain,
    pub ledger_post_id: Uuid,
    pub settled_at: DateTime<Utc>,
}

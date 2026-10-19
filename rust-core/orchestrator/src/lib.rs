//! # orchestrator
//!
//! Wires the four Rust core crates into a single atomic transaction flow.

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod error;
pub mod types;

pub use error::OrchestratorError;
pub use types::{SwapReceipt, SwapRequest};

use chrono::Utc;
use common::TxIntent;
use ledger::{Entry, Post};
use policy_engine::{PolicyDecision, PolicyEngine, TxRecord};
use std::sync::Arc;
use transaction_engine::TransactionEngine;
use uuid::Uuid;
use wallet_service::WalletService;

pub struct Dependencies {
    pub tx_engine: Arc<dyn TransactionEngine>,
    pub policy: Arc<dyn PolicyEngine>,
    pub wallet: Arc<dyn WalletService>,
    pub ledger: Arc<dyn ledger::Ledger>,
}

pub async fn execute_swap(
    deps: &Dependencies,
    req: SwapRequest,
) -> Result<SwapReceipt, OrchestratorError> {
    let intent = req.intent.clone();

    let quote = deps.tx_engine.quote(intent.clone()).await?;

    match deps.policy.evaluate(&intent).await? {
        PolicyDecision::Allow => {}
        PolicyDecision::Deny { reason } => {
            return Err(OrchestratorError::Denied { reason });
        }
        PolicyDecision::RequireStepUp { challenge } => {
            return Err(OrchestratorError::StepUpRequired {
                reason: challenge.reason,
                challenge_id: challenge.challenge_id,
            });
        }
    }

    let unsigned = deps.tx_engine.build(&quote).await?;
    let signed = deps
        .wallet
        .attach_signature(&req.wallet_handle, &unsigned, &req.signature)
        .await?;

    let chain = quote.intent.from.chain;
    let broadcast = deps.tx_engine.broadcast(chain, &signed.raw_bytes).await?;

    let usd_value = estimate_usd_value(&intent);
    let record = TxRecord {
        tx_id: Uuid::new_v4(),
        user_id: intent.user_id,
        intent: intent.clone(),
        usd_value,
        timestamp: Utc::now(),
    };
    if let Err(e) = deps.policy.record(record).await {
        tracing::warn!("policy record failed after broadcast: {e}");
    }

    let post = Post::new(vec![
        Entry::debit(req.from_account, intent.amount.clone()),
        Entry::credit(req.to_account, intent.amount),
    ])
    .with_metadata("tx_hash", broadcast.tx_hash.clone())
    .with_metadata("user_id", intent.user_id.to_string())
    .with_metadata("quote_id", quote.quote_id.to_string());

    let post_id = post.post_id;
    deps.ledger.post(post).await?;

    Ok(SwapReceipt {
        tx_hash: broadcast.tx_hash,
        chain,
        ledger_post_id: post_id,
        settled_at: Utc::now(),
    })
}

fn estimate_usd_value(intent: &TxIntent) -> rust_decimal::Decimal {
    intent.amount.value
}

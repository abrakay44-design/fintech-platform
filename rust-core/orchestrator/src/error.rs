//! Errors produced by the orchestrator.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("transaction engine error: {0}")]
    Tx(#[from] transaction_engine::TxError),

    #[error("policy engine error: {0}")]
    Policy(#[from] policy_engine::PolicyError),

    #[error("wallet error: {0}")]
    Wallet(#[from] wallet_service::WalletError),

    #[error("ledger error: {0}")]
    Ledger(#[from] ledger::LedgerError),

    #[error("transaction denied by policy: {reason}")]
    Denied { reason: String },

    #[error("step-up required: {reason}")]
    StepUpRequired {
        reason: String,
        challenge_id: uuid::Uuid,
    },

    #[error("internal orchestrator error: {0}")]
    Internal(String),
}

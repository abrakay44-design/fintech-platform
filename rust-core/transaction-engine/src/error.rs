//! Errors specific to the transaction engine.

use common::CoreError;

#[derive(Debug, thiserror::Error)]
pub enum TxError {
    #[error("no route found from {from} to {to}")]
    NoRoute { from: String, to: String },

    #[error("quote expired at {0}")]
    QuoteExpired(chrono::DateTime<chrono::Utc>),

    #[error("quote not found: {0}")]
    QuoteNotFound(String),

    #[error("slippage exceeded: expected {expected}, got {actual}")]
    SlippageExceeded { expected: String, actual: String },

    #[error("insufficient balance for {0}")]
    InsufficientBalance(String),

    #[error("unsupported chain: {0}")]
    UnsupportedChain(u64),

    #[error("build failed: {0}")]
    BuildFailed(String),

    #[error("simulation failed: {0}")]
    SimulationFailed(String),

    #[error("broadcast failed: {0}")]
    BroadcastFailed(String),

    #[error("wallet error: {0}")]
    Wallet(#[from] wallet_service::WalletError),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<TxError> for CoreError {
    fn from(err: TxError) -> Self {
        match err {
            TxError::InsufficientBalance(_) => CoreError::InsufficientBalance,
            TxError::Wallet(w) => CoreError::SigningFailure(w.to_string()),
            TxError::NoRoute { .. } => CoreError::InvalidInput("no route".into()),
            TxError::QuoteExpired(_) => CoreError::InvalidInput("quote expired".into()),
            TxError::QuoteNotFound(_) => CoreError::InvalidInput("quote not found".into()),
            TxError::SlippageExceeded { .. } => CoreError::InvalidInput("slippage".into()),
            TxError::UnsupportedChain(_) => CoreError::InvalidInput("unsupported chain".into()),
            TxError::BuildFailed(_) | TxError::SimulationFailed(_) => {
                CoreError::Internal("build/sim".into())
            }
            TxError::BroadcastFailed(_) => CoreError::Network("broadcast".into()),
            TxError::Internal(_) => CoreError::Internal("internal".into()),
        }
    }
}

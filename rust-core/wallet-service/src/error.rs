//! Error types specific to wallet operations.

use common::CoreError;

/// Errors that can occur during wallet operations.
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("key not found for handle {0}")]
    KeyNotFound(String),

    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("signature verification failed")]
    SignatureVerificationFailed,

    #[error("user rejected the signing request")]
    UserRejected,

    #[error("address mismatch: expected {expected}, got {actual}")]
    AddressMismatch { expected: String, actual: String },

    #[error("signing operation timed out")]
    Timeout,

    #[error("unsupported chain: {0}")]
    UnsupportedChain(String),

    #[error("derivation index overflow")]
    DerivationOverflow,

    #[error("cryptographic error: {0}")]
    Crypto(String),

    #[error("storage error: {0}")]
    Storage(String),
}

impl From<WalletError> for CoreError {
    fn from(err: WalletError) -> Self {
        match err {
            WalletError::UserRejected => CoreError::SigningFailure("user rejected".into()),
            WalletError::SignatureVerificationFailed => {
                CoreError::SigningFailure("verification failed".into())
            }
            WalletError::Timeout => CoreError::Network("timeout".into()),
            WalletError::KeyNotFound(_) => CoreError::InvalidInput("key not found".into()),
            WalletError::InvalidAddress(msg) => CoreError::InvalidInput(msg),
            WalletError::AddressMismatch { .. } => CoreError::InvalidInput("address mismatch".into()),
            WalletError::UnsupportedChain(chain) => CoreError::InvalidInput(chain),
            WalletError::DerivationOverflow => CoreError::InvalidInput("derivation overflow".into()),
            WalletError::Crypto(_) | WalletError::Storage(_) => CoreError::Internal("crypto/storage".into()),
        }
    }
}

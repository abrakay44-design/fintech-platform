//! Wallet-specific error types.
//!
//! These are deliberately distinct from `common::CoreError` so the wallet
//! service can express failure modes that only make sense for key management.

use thiserror::Error;

/// Errors that can occur during wallet operations.
///
/// Every variant is designed to be safe to log — no secret material is ever
/// included in the `Display` output.
#[derive(Debug, Error)]
pub enum WalletError {
    /// The provided key handle does not correspond to any stored key.
    #[error("unknown key handle")]
    UnknownKeyHandle,

    /// The user is not authorized to use this key handle.
    #[error("unauthorized key access")]
    Unauthorized,

    /// The signing operation failed (cryptographic error).
    /// Message is always a generic reason — never includes key material.
    #[error("signing failed: {0}")]
    SigningFailed(String),

    /// The provided payload is not valid for the requested chain.
    #[error("invalid payload for chain: {0}")]
    InvalidPayload(String),

    /// Key derivation index is out of range.
    #[error("derivation index {0} out of range")]
    DerivationIndexOutOfRange(u32),

    /// The requested chain is not supported by this wallet backend.
    #[error("unsupported chain: {0}")]
    UnsupportedChain(String),

    /// Backend is temporarily unavailable (network, HSM, MPC coordinator).
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),

    /// Any internal invariant violation. Should never be user-visible.
    #[error("internal wallet error")]
    Internal,
}

/// Convenience alias for wallet operations.
pub type WalletResult<T> = Result<T, WalletError>;

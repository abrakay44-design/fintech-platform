//! Wallet service — manages public keys, addresses, and signature verification.
//!
//! **Non-custodial model.** This crate never touches private keys directly.
//! It handles:
//!
//! - Registration of user-controlled addresses (via signed challenge)
//! - Verification of signatures
//! - Preparation of unsigned transactions for external signing
//! - Attachment of external signatures to complete a signed transaction
//!
//! For a production implementation, see the ADRs in `docs/adr/`.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod error;
pub mod mock;
pub mod traits;
pub mod types;

pub use error::WalletError;
pub use mock::MockWalletService;
pub use traits::{AddressInfo, WalletService};
pub use types::{
    Address, KeyOrigin, PublicKey, PublicKeyHandle, Signature, SignedPayload, SigningRequest,
};

// Re-export hex for test convenience.
pub use hex;

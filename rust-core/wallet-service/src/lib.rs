//! # wallet-service
//!
//! The most security-critical crate in the platform.
//!
//! ## Two layers
//!
//! - **`SigningBackend`** — the low-level primitive that holds keys and
//!   produces signatures. Used by the mock (tests) and, later, an MPC or HSM
//!   backend. **Not used in the MVP.**
//!
//! - **`WalletService`** — the non-custodial external-signer model. The platform
//!   never holds keys. Users sign in their own wallet; this service verifies
//!   signatures and assembles signed transactions.
//!
//! ## Custody model — MVP
//!
//! The MVP ships `NonCustodialService`, which:
//!
//! 1. Registers addresses only after a signature challenge.
//! 2. Verifies ECDSA signatures locally (via `k256`).
//! 3. Builds unsigned transactions for the user's wallet to sign.
//! 4. Assembles signed envelopes ready for broadcast.
//!
//! Private keys are never stored, transmitted, or accepted by this crate.

#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod mock;
pub mod noncustodial;
pub mod traits;
pub mod types;

pub use error::{WalletError, WalletResult};

// The non-custodial (external-signer) trait and its implementation.
pub use noncustodial::NonCustodialService;
pub use traits::{AddressInfo, WalletService};

// The low-level key-holding trait (mock backend only for now).
pub use types::{
    Address, KeyHandle, KeyOrigin, PublicKey, PublicKeyHandle, PublicKeyInfo, Signature,
    SignedPayload, SignedTransaction, SigningRequest,
};

use async_trait::async_trait;
use common::Chain;
use uuid::Uuid;

/// Low-level signing backend.
///
/// This is the primitive that actually holds keys and produces signatures.
/// In the MVP, the only implementation is `MockWalletService` (tests only).
/// In production, this would be an MPC or HSM backend.
///
/// For the user-facing non-custodial flow, see `NonCustodialService`.
#[async_trait]
pub trait SigningBackend: Send + Sync {
    /// Derive a new address for a user on a given chain at a given index.
    async fn derive_address(
        &self,
        user_id: Uuid,
        chain: Chain,
        derivation_index: u32,
    ) -> WalletResult<PublicKeyInfo>;

    /// Fetch the public key associated with a stored handle.
    async fn get_public_key(&self, handle: KeyHandle) -> WalletResult<PublicKeyInfo>;

    /// Sign an arbitrary payload with the key behind `handle`.
    async fn sign(&self, handle: KeyHandle, payload: &[u8]) -> WalletResult<Vec<u8>>;

    /// Revoke a key handle. Subsequent `sign()` calls must fail.
    async fn revoke(&self, handle: KeyHandle) -> WalletResult<()>;
}

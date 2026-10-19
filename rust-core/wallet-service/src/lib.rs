//! # wallet-service
//!
//! The most security-critical crate in the platform.
//!
//! ## Design principles
//!
//! 1. **Key material never leaves this crate.** Even in tests, secrets are
//!    wrapped in `secrecy::Secret` and zeroized on drop.
//! 2. **Opaque handles.** Callers pass a `KeyHandle` (a UUID) — never a raw key.
//! 3. **Chain-aware.** Each operation is parameterised by `Chain` so the same
//!    trait works for EVM, Solana, Cosmos.
//! 4. **Backend-agnostic.** The `WalletService` trait is implemented by
//!    *multiple* backends: a mock for tests, a non-custodial "user signs"
//!    backend for MVP, and (later) an MPC or HSM backend.
//!
//! ## Custody model — MVP
//!
//! The MVP is **non-custodial**: the platform never holds private keys.
//! Instead, this crate:
//!
//! - Derives public addresses (so the UI can show balances).
//! - Builds unsigned transactions (so the user can review and sign).
//! - Verifies signatures returned by the user's wallet (MetaMask, Phantom).
//!
//! The `WalletService` trait still has a `sign()` method — it is used only by
//! the `MockWalletService` in tests, and by a future MPC backend. In production
//! MVP, the Go gateway calls the user's wallet directly; this crate verifies.

#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    missing_docs,
    rust_2018_idioms
)]
#![allow(clippy::missing_errors_doc)]

pub mod error;
pub mod mock;
pub mod types;

pub use error::{WalletError, WalletResult};
pub use types::{KeyHandle, PublicKeyInfo, SignedTransaction};

use async_trait::async_trait;
use common::Chain;
use uuid::Uuid;

/// The wallet backend interface.
///
/// Every implementation must uphold these invariants:
///
/// - `sign()` never logs, prints, or serialises the raw signature.
/// - `derive_address()` is deterministic: same `(user_id, chain, index)` always
///   yields the same address.
/// - `get_public_key()` never returns secret material.
/// - All async methods are cancellation-safe.
#[async_trait]
pub trait WalletService: Send + Sync {
    /// Derive a new address for a user on a given chain at a given index.
    ///
    /// The derivation is deterministic — the same inputs always produce the
    /// same address. This is essential for wallet recovery.
    async fn derive_address(
        &self,
        user_id: Uuid,
        chain: Chain,
        derivation_index: u32,
    ) -> WalletResult<PublicKeyInfo>;

    /// Fetch the public key associated with a stored handle.
    ///
    /// Returns an error if the handle is unknown or has been revoked.
    async fn get_public_key(&self, handle: KeyHandle) -> WalletResult<PublicKeyInfo>;

    /// Sign an arbitrary payload with the key behind `handle`.
    ///
    /// ## Security notes
    ///
    /// - The raw private key is never exposed, even to the caller.
    /// - The returned bytes are the cryptographic signature only — not a
    ///   signed transaction envelope.
    /// - Callers must supply a payload that is at most 32 bytes for secp256k1
    ///   (the hash of the real message). This prevents preimage attacks.
    async fn sign(&self, handle: KeyHandle, payload: &[u8]) -> WalletResult<Vec<u8>>;

    /// Revoke a key handle. Subsequent `sign()` calls must fail.
    async fn revoke(&self, handle: KeyHandle) -> WalletResult<()>;
}

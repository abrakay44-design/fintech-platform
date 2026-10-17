//! The `WalletService` trait — the central abstraction for wallet operations.
//!
//! Every implementation of this trait must uphold these invariants:
//!
//! 1. **No private keys stored server-side** in the non-custodial model.
//! 2. **Addresses are verified before registration.** A user cannot register
//!    an address they don't control.
//! 3. **Signatures are always verified locally** before being broadcast.
//! 4. **Handles are opaque.** The consumer never sees cryptographic details.

use crate::{
    types::{Address, KeyOrigin, PublicKey, PublicKeyHandle, Signature, SignedPayload, SigningRequest},
    WalletError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Public information about a registered address.
#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub address: Address,
    pub public_key: PublicKey,
    pub chain_id: u64,
    pub handle: PublicKeyHandle,
    pub origin: KeyOrigin,
    pub registered_at: DateTime<Utc>,
}

/// The central trait for wallet operations.
///
/// Implementations must be thread-safe (`Send + Sync`) and support
/// concurrent access from multiple async tasks.
#[async_trait]
pub trait WalletService: Send + Sync + 'static {
    /// Register a user's external wallet address.
    ///
    /// The caller must provide a *signed challenge* that proves the user
    /// controls the private key for the given address.
    ///
    /// `signature` must be over `challenge_bytes` (e.g., a SiWE message).
    /// The implementation MUST verify this signature before storing anything.
    async fn register_address(
        &self,
        chain_id: u64,
        address: Address,
        challenge_bytes: &[u8],
        signature: &Signature,
    ) -> Result<AddressInfo, WalletError>;

    /// Look up a previously-registered address.
    async fn lookup_address(
        &self,
        handle: &PublicKeyHandle,
    ) -> Result<AddressInfo, WalletError>;

    /// List all registered addresses for a user.
    async fn list_addresses(
        &self,
        user_id: &str,
    ) -> Result<Vec<AddressInfo>, WalletError>;

    /// Verify that a signature matches the expected signer and payload.
    ///
    /// Returns `Ok(())` if the signature is valid for the given address.
    /// Returns `Err(WalletError::SignatureVerificationFailed)` otherwise.
    async fn verify_signature(
        &self,
        address: &Address,
        payload: &[u8],
        signature: &Signature,
    ) -> Result<(), WalletError>;

    /// Prepare an unsigned transaction for external signing.
    ///
    /// In the non-custodial model, this method constructs the raw transaction
    /// bytes that the user's external wallet will sign.
    ///
    /// Returns the unsigned payload the frontend should pass to the wallet.
    async fn prepare_unsigned(
        &self,
        handle: &PublicKeyHandle,
        request: &SigningRequest,
    ) -> Result<Vec<u8>, WalletError>;

    /// Attach a signature to a previously-prepared unsigned transaction.
    ///
    /// Verifies the signature locally before accepting it, and returns
    /// the fully-signed payload ready for broadcast.
    async fn attach_signature(
        &self,
        handle: &PublicKeyHandle,
        unsigned_payload: &[u8],
        signature: &Signature,
    ) -> Result<SignedPayload, WalletError>;

    /// Recover the address that produced a given signature over a payload.
    ///
    /// Useful for SiWE-style flows where the backend receives a signature
    /// and needs to know which address signed.
    async fn recover_signer(
        &self,
        payload: &[u8],
        signature: &Signature,
    ) -> Result<Address, WalletError>;
}

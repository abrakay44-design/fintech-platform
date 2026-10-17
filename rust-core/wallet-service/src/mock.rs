//! In-memory mock implementation for testing.
//!
//! This is NOT for production. It stores everything in memory and uses
//! the `k256` crate directly for signature operations.

use crate::{
    traits::{AddressInfo, WalletService},
    types::{Address, KeyOrigin, PublicKey, PublicKeyHandle, Signature, SignedPayload, SigningRequest},
    WalletError,
};
use async_trait::async_trait;
use chrono::Utc;
use k256::ecdsa::{
    Signature as K256Signature, VerifyingKey,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory store used by the mock.
#[derive(Default)]
struct Store {
    by_handle: HashMap<PublicKeyHandle, AddressInfo>,
    by_user: HashMap<String, Vec<PublicKeyHandle>>,
}

/// A simple in-memory implementation of `WalletService`.
///
/// **Not for production.** Test use only.
pub struct MockWalletService {
    store: Arc<RwLock<Store>>,
}

impl MockWalletService {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(Store::default())),
        }
    }
}

impl Default for MockWalletService {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a secp256k1 signature (65 bytes: r || s || v) to the k256 type.
fn sig_to_k256(sig: &Signature) -> Result<K256Signature, WalletError> {
    let bytes = sig.as_bytes();
    // k256::ecdsa::Signature::from_slice expects 64 bytes (r || s), no v
    K256Signature::from_slice(&bytes[..64])
        .map_err(|e| WalletError::Crypto(format!("signature parse: {e}")))
}

/// Derive an Ethereum address from a secp256k1 public key.
///
/// Proper Ethereum: keccak256(uncompressed_pubkey_without_prefix)[12..]
/// Simplified here to a deterministic hash so the mock works without keccak.
fn derive_address(pubkey: &PublicKey) -> Address {
    // Use SHA-256 for mock purposes (real Ethereum uses keccak256).
    use k256::sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(pubkey.0);
    let hash = hasher.finalize();
    let hex = hex::encode(&hash[12..32]);
    Address::from_hex(&hex).expect("derived address is valid")
}

#[async_trait]
impl WalletService for MockWalletService {
    async fn register_address(
        &self,
        chain_id: u64,
        address: Address,
        challenge_bytes: &[u8],
        signature: &Signature,
    ) -> Result<AddressInfo, WalletError> {
        // Recover the signer.
        let recovered = self.recover_signer(challenge_bytes, signature).await?;

        if recovered != address {
            return Err(WalletError::AddressMismatch {
                expected: address.to_hex().to_string(),
                actual: recovered.to_hex().to_string(),
            });
        }

        // Extract public key from signature (recoverable).
        let sig = sig_to_k256(signature)?;
        let rec_id = k256::ecdsa::RecoveryId::from_byte(signature.0[64])
            .ok_or_else(|| WalletError::Crypto("invalid recovery id".into()))?;
        use k256::sha2::{Digest, Sha256};
let mut hasher = Sha256::new();
hasher.update(challenge_bytes);
let hash = hasher.finalize();
let verifying_key = VerifyingKey::recover_from_prehash(
    &hash,
    &sig,
    rec_id,
)
.map_err(|e| WalletError::Crypto(format!("recover: {e}")))?;

        // Convert to uncompressed x || y (64 bytes).
        let point = verifying_key.to_encoded_point(false);
        let mut pub_bytes = [0u8; 64];
        pub_bytes.copy_from_slice(&point.as_bytes()[1..65]);
        let public_key = PublicKey(pub_bytes);

        let handle = PublicKeyHandle::new(uuid::Uuid::new_v4().to_string());
        let info = AddressInfo {
            address: address.clone(),
            public_key,
            chain_id,
            handle: handle.clone(),
            origin: KeyOrigin::Registered,
            registered_at: Utc::now(),
        };

        let mut store = self.store.write().await;
        store.by_handle.insert(handle, info.clone());
        // Note: caller should supply a user_id; for the mock, use a default.
        store
            .by_user
            .entry("default-user".to_string())
            .or_default()
            .push(info.handle.clone());

        Ok(info)
    }

    async fn lookup_address(
        &self,
        handle: &PublicKeyHandle,
    ) -> Result<AddressInfo, WalletError> {
        let store = self.store.read().await;
        store
            .by_handle
            .get(handle)
            .cloned()
            .ok_or_else(|| WalletError::KeyNotFound(handle.as_str().to_string()))
    }

    async fn list_addresses(
        &self,
        user_id: &str,
    ) -> Result<Vec<AddressInfo>, WalletError> {
        let store = self.store.read().await;
        let handles = store.by_user.get(user_id).cloned().unwrap_or_default();
        Ok(handles
            .into_iter()
            .filter_map(|h| store.by_handle.get(&h).cloned())
            .collect())
    }

    async fn verify_signature(
        &self,
        address: &Address,
        payload: &[u8],
        signature: &Signature,
    ) -> Result<(), WalletError> {
        let recovered = self.recover_signer(payload, signature).await?;
        if recovered != *address {
            return Err(WalletError::AddressMismatch {
                expected: address.to_hex().to_string(),
                actual: recovered.to_hex().to_string(),
            });
        }
        Ok(())
    }

    async fn prepare_unsigned(
        &self,
        _handle: &PublicKeyHandle,
        request: &SigningRequest,
    ) -> Result<Vec<u8>, WalletError> {
        // In the mock, we just return the payload with the chain_id prefixed.
        // A real implementation would build a proper transaction.
        let mut out = Vec::with_capacity(8 + request.unsigned_payload.len());
        out.extend_from_slice(&request.chain_id.to_be_bytes());
        out.extend_from_slice(&request.unsigned_payload);
        Ok(out)
    }

    async fn attach_signature(
        &self,
        handle: &PublicKeyHandle,
        unsigned_payload: &[u8],
        signature: &Signature,
    ) -> Result<SignedPayload, WalletError> {
        let info = self.lookup_address(handle).await?;
        self.verify_signature(&info.address, unsigned_payload, signature).await?;

        let mut signed_bytes = unsigned_payload.to_vec();
        signed_bytes.extend_from_slice(signature.as_bytes());

        Ok(SignedPayload {
            signature: signature.clone(),
            signed_bytes,
        })
    }
    async fn recover_signer(
    &self,
    payload: &[u8],
    signature: &Signature,
) -> Result<Address, WalletError> {
    use k256::sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let hash = hasher.finalize();

    let sig = sig_to_k256(signature)?;
    let rec_id = k256::ecdsa::RecoveryId::from_byte(signature.0[64])
        .ok_or_else(|| WalletError::Crypto("invalid recovery id".into()))?;
    let verifying_key = VerifyingKey::recover_from_prehash(&hash, &sig, rec_id)
        .map_err(|e| WalletError::Crypto(format!("recover: {e}")))?;

    let point = verifying_key.to_encoded_point(false);
    let mut pub_bytes = [0u8; 64];
    pub_bytes.copy_from_slice(&point.as_bytes()[1..65]);
    Ok(derive_address(&PublicKey(pub_bytes)))
}
}

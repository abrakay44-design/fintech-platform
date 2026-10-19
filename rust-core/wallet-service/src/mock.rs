//! In-memory mock backend for tests and local development.
//!
//! **Never use in production.** Keys are generated fresh per-process and
//! stored in a `HashMap`. All `sign()` operations are real secp256k1
//! signatures, but the keys themselves are ephemeral.

use crate::error::{WalletError, WalletResult};
use crate::types::{KeyHandle, PublicKeyInfo, SignedTransaction};
use crate::WalletService;

use async_trait::async_trait;
use common::Chain;
use k256::ecdsa::{Signature, SigningKey, VerifyingKey};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory wallet service.
///
/// - Derives deterministic addresses from `(user_id, chain, index)` via a hash.
/// - Uses real secp256k1 signing (`k256`).
/// - Zeroizes keys on drop.
pub struct MockWalletService {
    // Map from key handle to signing key.
    // In a real MPC/HSM backend, this map lives in a hardware module.
    keys: Arc<RwLock<HashMap<KeyHandle, SigningKey>>>,
    // Map from (user_id, chain, index) to key handle, so derivation is deterministic.
    addresses: Arc<RwLock<HashMap<(Uuid, Chain, u32), KeyHandle>>>,
}

impl MockWalletService {
    /// Create a new, empty mock.
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            addresses: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MockWalletService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WalletService for MockWalletService {
    async fn derive_address(
        &self,
        user_id: Uuid,
        chain: Chain,
        derivation_index: u32,
    ) -> WalletResult<PublicKeyInfo> {
        if derivation_index > 0x7FFF_FFFF {
            return Err(WalletError::DerivationIndexOutOfRange(derivation_index));
        }

        // Deterministic key derivation for tests: seed from the tuple.
        // In production, this would be BIP-32/BIP-44 derivation from a seed.
        let seed = format!("{user_id}:{chain:?}:{derivation_index}");
        let mut hasher = <sha3::Sha3_256 as sha3::Digest>::new();
        use sha3::Digest;
        hasher.update(seed.as_bytes());
        let hash = hasher.finalize();
        let signing_key = SigningKey::from_bytes(&hash).map_err(|e| {
            WalletError::SigningFailed(format!("invalid derived key: {e}"))
        })?;

        let verifying_key: VerifyingKey = *signing_key.verifying_key();
        let public_key = verifying_key.to_encoded_point(true).as_bytes().to_vec();

        let handle = KeyHandle(Uuid::new_v4());

        self.keys.write().await.insert(handle, signing_key.clone());
        self.addresses
            .write()
            .await
            .insert((user_id, chain, derivation_index), handle);

        Ok(PublicKeyInfo {
            chain,
            address: hex_address(&public_key),
            public_key,
            derivation_index,
        })
    }

    async fn get_public_key(&self, handle: KeyHandle) -> WalletResult<PublicKeyInfo> {
        let keys = self.keys.read().await;
        let sk = keys.get(&handle).ok_or(WalletError::UnknownKeyHandle)?;
        let vk = sk.verifying_key();
        let public_key = vk.to_encoded_point(true).as_bytes().to_vec();

        Ok(PublicKeyInfo {
            chain: Chain::Ethereum, // mock default
            address: hex_address(&public_key),
            public_key,
            derivation_index: 0,
        })
    }

    async fn sign(&self, handle: KeyHandle, payload: &[u8]) -> WalletResult<Vec<u8>> {
        if payload.len() > 32 {
            return Err(WalletError::InvalidPayload(
                "payload exceeds 32 bytes".into(),
            ));
        }

        let keys = self.keys.read().await;
        let sk = keys.get(&handle).ok_or(WalletError::UnknownKeyHandle)?;

        // Pad the payload to 32 bytes (secp256k1 expects a 32-byte digest).
        let mut digest = [0u8; 32];
        digest[..payload.len()].copy_from_slice(payload);

        let (signature, _recovery_id): (Signature, _) = sk
            .sign_prehash_recoverable(&digest)
            .map_err(|e| WalletError::SigningFailed(e.to_string()))?;

        Ok(signature.to_bytes().to_vec())
    }

    async fn revoke(&self, handle: KeyHandle) -> WalletResult<()> {
        let removed = self.keys.write().await.remove(&handle);
        if removed.is_none() {
            return Err(WalletError::UnknownKeyHandle);
        }

        // Also remove from address index.
        let mut addresses = self.addresses.write().await;
        addresses.retain(|_, h| *h != handle);
        Ok(())
    }
}

/// Encode a public key as a hex address for testing purposes.
fn hex_address(public_key: &[u8]) -> String {
    let mut s = String::from("0x");
    for b in public_key {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Validate that a signature verifies against a public key and payload.
/// Useful in the test suite.
pub fn verify(
    public_key: &[u8],
    payload: &[u8],
    signature_bytes: &[u8],
) -> bool {
    use k256::ecdsa::signature::Verifier;

    let vk = match VerifyingKey::from_sec1_bytes(public_key) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let sig = match Signature::from_slice(signature_bytes) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let mut digest = [0u8; 32];
    digest[..payload.len().min(32)].copy_from_slice(&payload[..payload.len().min(32)]);

    vk.verify(&digest, &sig).is_ok()
}

// Suppress unused import warning for `SignedTransaction` (used in future work).
#[allow(dead_code)]
fn _keep_signed_tx_import(_: SignedTransaction) {}

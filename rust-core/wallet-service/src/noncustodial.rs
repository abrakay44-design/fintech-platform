//! Non-custodial wallet service — the MVP backend.
//!
//! ## Design
//!
//! The platform never holds private keys. All signature production happens in
//! the user's own wallet (MetaMask, Phantom, Ledger). This service:
//!
//! - **Verifies** signatures supplied by the user's wallet (ECDSA recovery).
//! - **Registers** addresses only after a successful challenge-response.
//! - **Builds unsigned transactions** for the user to review and sign.
//! - **Attaches** user-supplied signatures to produce a broadcastable envelope.
//!
//! ## Stub mode
//!
//! `prepare_unsigned` returns a placeholder payload and `attach_signature`
//! assembles a synthetic envelope. This is enough to wire the gateway and
//! frontend. Real EVM transaction encoding (RLP, EIP-1559, EIP-712) will be
//! added in a later phase using `alloy`.

use crate::error::{WalletError, WalletResult};
use crate::traits::{AddressInfo, WalletService};
use crate::types::{
    Address, KeyOrigin, PublicKey, PublicKeyHandle, Signature, SignedPayload, SigningRequest,
};

use async_trait::async_trait;
use chrono::Utc;
use k256::ecdsa::{RecoveryId, Signature as EcdsaSignature, VerifyingKey};
use sha3::{Digest, Keccak256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Non-custodial wallet service.
///
/// Stores the *public* metadata for registered addresses. Never stores private
/// keys — the platform cannot move user funds.
pub struct NonCustodialService {
    // Map from handle → AddressInfo.
    addresses: Arc<RwLock<HashMap<PublicKeyHandle, AddressInfo>>>,
    // Map from (user_id, address string) → handle, for deduplication.
    index: Arc<RwLock<HashMap<(String, String), PublicKeyHandle>>>,
}

impl NonCustodialService {
    /// Create a new, empty service.
    pub fn new() -> Self {
        Self {
            addresses: Arc::new(RwLock::new(HashMap::new())),
            index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for NonCustodialService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WalletService for NonCustodialService {
    async fn register_address(
        &self,
        chain_id: u64,
        address: Address,
        challenge_bytes: &[u8],
        signature: &Signature,
    ) -> WalletResult<AddressInfo> {
        // 1. Verify the challenge signature proves ownership.
        self.verify_signature(&address, challenge_bytes, signature)
            .await?;

        // 2. Check if already registered.
        let user_key = format!("{chain_id}:{}", address.as_str());
        {
            let index = self.index.read().await;
            for ((_uid, addr), handle) in index.iter() {
                if addr == &user_key {
                    return self
                        .lookup_address(handle)
                        .await
                        .map_err(|_| WalletError::Internal);
                }
            }
        }

        // 3. Extract the public key from the signature (recovery).
        let public_key_bytes = recover_public_key(challenge_bytes, signature)?;

        // 4. Create the handle and store the metadata.
        let handle = PublicKeyHandle(Uuid::new_v4());
        let info = AddressInfo {
            address: address.clone(),
            public_key: PublicKey(public_key_bytes),
            chain_id,
            handle,
            origin: KeyOrigin::ExternalWallet,
            registered_at: Utc::now(),
        };

        self.addresses.write().await.insert(handle, info.clone());
        self.index
            .write()
            .await
            .insert((String::new(), user_key), handle);

        Ok(info)
    }

    async fn lookup_address(
        &self,
        handle: &PublicKeyHandle,
    ) -> WalletResult<AddressInfo> {
        self.addresses
            .read()
            .await
            .get(handle)
            .cloned()
            .ok_or(WalletError::UnknownKeyHandle)
    }

    async fn list_addresses(&self, _user_id: &str) -> WalletResult<Vec<AddressInfo>> {
        // In the stub, return everything (single-user dev mode).
        Ok(self.addresses.read().await.values().cloned().collect())
    }

    async fn verify_signature(
        &self,
        address: &Address,
        payload: &[u8],
        signature: &Signature,
    ) -> WalletResult<()> {
        let recovered = self.recover_signer(payload, signature).await?;
        if recovered.as_str().eq_ignore_ascii_case(address.as_str()) {
            Ok(())
        } else {
            Err(WalletError::SigningFailed(format!(
                "signature does not match address (recovered: {})",
                recovered.as_str()
            )))
        }
    }

    async fn prepare_unsigned(
        &self,
        handle: &PublicKeyHandle,
        request: &SigningRequest,
    ) -> WalletResult<Vec<u8>> {
        // Confirm the handle exists (auth check).
        let _ = self.lookup_address(handle).await?;

        // Stub: return the request payload as the unsigned body.
        // A real implementation would RLP-encode an EIP-1559 transaction here.
        Ok(request.payload.clone())
    }

    async fn attach_signature(
        &self,
        handle: &PublicKeyHandle,
        unsigned_payload: &[u8],
        signature: &Signature,
    ) -> WalletResult<SignedPayload> {
        let info = self.lookup_address(handle).await?;

        // Verify signature against the payload and the registered address.
        self.verify_signature(&info.address, unsigned_payload, signature)
            .await?;

        // Stub: concatenate payload + signature as the "signed transaction".
        // A real implementation would produce a proper signed EIP-1559 envelope.
        let mut raw_bytes = unsigned_payload.to_vec();
        raw_bytes.extend_from_slice(signature.as_bytes());

        // Synthesize a hash (keccak256 of the raw bytes).
        let mut hasher = Keccak256::new();
        hasher.update(&raw_bytes);
        let hash = hasher.finalize();
        let tx_hash = format!("0x{}", hex_encode(&hash));

        Ok(SignedPayload {
            chain: crate::noncustodial::chain_for_id(info.chain_id),
            signer: info.address,
            raw_bytes,
            tx_hash,
        })
    }

    async fn recover_signer(
        &self,
        payload: &[u8],
        signature: &Signature,
    ) -> WalletResult<Address> {
        let pubkey = recover_public_key(payload, signature)?;
        let addr = public_key_to_address(&pubkey)?;
        Ok(Address(addr))
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Recover the secp256k1 public key from a 65-byte signature and a message.
fn recover_public_key(payload: &[u8], signature: &Signature) -> WalletResult<Vec<u8>> {
    let sig_bytes = signature.as_bytes();
    if sig_bytes.len() != 65 {
        return Err(WalletError::SigningFailed(format!(
            "expected 65-byte signature, got {}",
            sig_bytes.len()
        )));
    }

    // r || s || v — v is the last byte (0 or 1 for k256).
    let sig_rs = EcdsaSignature::from_slice(&sig_bytes[..64])
        .map_err(|e| WalletError::SigningFailed(format!("invalid signature: {e}")))?;
    let recovery_byte = sig_bytes[64];
    let recovery_id = RecoveryId::from_byte(recovery_byte)
        .ok_or_else(|| WalletError::SigningFailed("invalid recovery id".into()))?;

    // Hash the payload with keccak256 — EVM-style message hashing.
    let mut hasher = Keccak256::new();
    hasher.update(payload);
    let digest = hasher.finalize();

    let recovered = VerifyingKey::recover_from_prehash(&digest, &sig_rs, recovery_id)
        .map_err(|e| WalletError::SigningFailed(format!("recovery failed: {e}")))?;

    let encoded = recovered.to_encoded_point(true);
    Ok(encoded.as_bytes().to_vec())
}

/// Convert a secp256k1 public key to an EVM address (last 20 bytes of keccak256).
fn public_key_to_address(public_key: &[u8]) -> WalletResult<String> {
    // Decompress — the encoded point is compressed (33 bytes), but EVM addresses
    // hash the *uncompressed* form (64 bytes without the 0x04 prefix).
    let vk = VerifyingKey::from_sec1_bytes(public_key)
        .map_err(|e| WalletError::SigningFailed(format!("invalid public key: {e}")))?;

    let uncompressed = vk.to_encoded_point(false);
    let bytes = uncompressed.as_bytes();
    // bytes[0] == 0x04, then 32 bytes X, then 32 bytes Y. Hash X || Y.
    let mut hasher = Keccak256::new();
    hasher.update(&bytes[1..]);
    let hash = hasher.finalize();

    let addr_bytes = &hash[12..];
    Ok(format!("0x{}", hex_encode(addr_bytes)))
}

/// Hex-encode bytes (lowercase, no 0x prefix).
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Map an EVM chain ID to a `common::Chain` for display purposes.
pub fn chain_for_id(chain_id: u64) -> common::Chain {
    match chain_id {
        1 => common::Chain::Ethereum,
        137 => common::Chain::Polygon,
        42161 => common::Chain::Arbitrum,
        _ => common::Chain::Ethereum,
    }
}

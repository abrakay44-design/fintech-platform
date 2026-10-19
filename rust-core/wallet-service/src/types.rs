//! Public wallet types — everything that can safely cross a service boundary.

use common::Chain;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque identifier for a key stored by the wallet backend.
///
/// **Never** contains raw secret material. The backend maps this to an HSM
/// slot, MPC share set, or (in dev) an in-memory key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyHandle(pub Uuid);

/// Public key material — safe to log, transmit, and store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyInfo {
    /// Which chain this address belongs to.
    pub chain: Chain,

    /// Human-readable address (hex for EVM, base58 for Solana).
    pub address: String,

    /// Raw public key bytes (33 bytes compressed for secp256k1, 32 for Ed25519).
    pub public_key: Vec<u8>,

    /// Derivation index used to derive this key (HD wallets only).
    pub derivation_index: u32,
}

/// A signed transaction ready for broadcast.
///
/// Contains no secrets — safe to pass to the transaction engine and log the hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// Which chain this transaction targets.
    pub chain: Chain,

    /// Serialized, signed transaction bytes ready to broadcast.
    pub raw_bytes: Vec<u8>,

    /// Transaction hash (hex string, 0x-prefixed).
    pub tx_hash: String,
}

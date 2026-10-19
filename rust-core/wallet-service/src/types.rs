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

// ============================================================================
// Non-custodial wallet types
//
// These types support the external-signer model where the user's own wallet
// (MetaMask, Phantom) signs transactions. The platform never holds private keys.
// ============================================================================


/// A blockchain address (hex for EVM, base58 for Solana).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub String);

impl Address {
    /// Create an address from a string. Does not validate the format.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A public key in compressed byte form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey(pub Vec<u8>);

/// An opaque handle to a registered address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKeyHandle(pub Uuid);

/// How the address was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyOrigin {
    /// User connected via WalletConnect / MetaMask and signed a challenge.
    ExternalWallet,
    /// Address derived from a server-managed HD wallet (future).
    ManagedHd,
    /// Address from an MPC share set (future).
    Mpc,
}

/// An ECDSA signature in raw bytes (65-byte r||s||v for EVM).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub Vec<u8>);

impl Signature {
    /// Wrap raw signature bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Return the raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// A request to build an unsigned transaction for external signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningRequest {
    /// The chain the transaction targets.
    pub chain: Chain,
    /// The account that will sign.
    pub from: Address,
    /// Raw transaction body to be signed (RLP, protobuf, etc.).
    /// In the stub implementation this is a placeholder.
    pub payload: Vec<u8>,
    /// Optional human-readable summary for the UI.
    pub summary: Option<String>,
}

/// A signed payload ready for broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPayload {
    /// The chain the signed payload targets.
    pub chain: Chain,
    /// The account that produced the signature.
    pub signer: Address,
    /// The complete signed transaction bytes.
    pub raw_bytes: Vec<u8>,
    /// The transaction hash (hex, 0x-prefixed).
    pub tx_hash: String,
}

//! Domain types for wallet operations.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A blockchain address (hex string, with or without 0x prefix).
///
/// We store the address in canonical lowercase hex form without `0x` prefix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(String);

impl Address {
    /// Parse an address from a hex string.
    pub fn from_hex(hex: &str) -> Result<Self, crate::WalletError> {
        let cleaned = hex.trim_start_matches("0x").trim_start_matches("0X").to_lowercase();
        if cleaned.len() != 40 || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(crate::WalletError::InvalidAddress(hex.to_string()));
        }
        Ok(Address(cleaned))
    }

    /// Return the address as lowercase hex (no 0x).
    pub fn to_hex(&self) -> &str {
        &self.0
    }

    /// Return the address in checksummed/EIP-55 display form (with 0x).
    pub fn display_checksummed(&self) -> String {
        // Simplified: we return 0x + hex here; proper EIP-55 checksum requires keccak
        format!("0x{}", self.0)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", self.0)
    }
}

/// A 65-byte secp256k1 signature (r || s || recovery_id).
#[derive(Debug, Clone)]
pub struct Signature(pub(crate) [u8; 65]);

impl Signature {
    pub fn as_bytes(&self) -> &[u8; 65] {
        &self.0
    }

    pub fn from_bytes(bytes: [u8; 65]) -> Self {
        Signature(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// A public key for a secp256k1 keypair.
#[derive(Debug, Clone)]
pub struct PublicKey(pub(crate) [u8; 64]); // uncompressed x||y without 0x04 prefix

impl PublicKey {
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}

/// A handle identifying a public key stored server-side.
///
/// With the non-custodial model, this is an opaque reference that
/// ties a user to a known public key and address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKeyHandle(pub String);

impl PublicKeyHandle {
    pub fn new(s: impl Into<String>) -> Self {
        PublicKeyHandle(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A request to sign a transaction.
#[derive(Debug, Clone)]
pub struct SigningRequest {
    /// The chain the transaction belongs to (e.g., "ethereum", "polygon").
    pub chain_id: u64,
    /// The unsigned transaction payload (chain-specific encoding).
    pub unsigned_payload: Vec<u8>,
    /// Optional human-readable description for display to user.
    pub description: Option<String>,
}

/// A signed transaction ready for broadcast.
#[derive(Debug, Clone)]
pub struct SignedPayload {
    pub signature: Signature,
    pub signed_bytes: Vec<u8>,
}

/// Information about a public key's provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOrigin {
    /// Created via `derive` in this service (deterministic).
    Derived,
    /// Imported by user from an external wallet.
    Imported,
    /// Registered by external signer (e.g., MetaMask, hardware wallet).
    Registered,
}

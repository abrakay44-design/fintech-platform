//! Integration tests for the non-custodial wallet flow.
//!
//! These tests use a locally-generated secp256k1 key to simulate a user's
//! wallet signing a challenge and a transaction payload.

use k256::ecdsa::signature::hazmat::PrehashSigner;
use k256::ecdsa::{RecoveryId, Signature as EcdsaSignature, SigningKey, VerifyingKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use sha3::{Digest, Keccak256};

use wallet_service::{
    noncustodial::NonCustodialService, Address, Signature, SigningRequest, WalletService,
};

/// Helper: produce a 65-byte EVM-style signature from a message and a key.
fn sign_message(sk: &SigningKey, message: &[u8]) -> Vec<u8> {
    let mut hasher = Keccak256::new();
    hasher.update(message);
    let digest = hasher.finalize();

    let (sig, recovery_id): (EcdsaSignature, RecoveryId) = sk
        .sign_prehash_recoverable(&digest)
        .expect("signing failed");

    let mut bytes = Vec::with_capacity(65);
    bytes.extend_from_slice(&sig.to_bytes());
    bytes.push(recovery_id.to_byte());
    bytes
}

/// Helper: derive the EVM address from a signing key.
fn address_for(sk: &SigningKey) -> String {
    let vk: VerifyingKey = *sk.verifying_key();
    let uncompressed = vk.to_encoded_point(false);
    let bytes = uncompressed.as_bytes();

    let mut hasher = Keccak256::new();
    hasher.update(&bytes[1..]);
    let hash = hasher.finalize();

    let mut s = String::from("0x");
    for b in &hash[12..] {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[tokio::test]
async fn register_address_with_valid_signature() {
    let svc = NonCustodialService::new();
    let sk = SigningKey::random(&mut rand::thread_rng());
    let addr = Address(address_for(&sk));

    let challenge = b"Sign in to fintech-platform: nonce=abc123";
    let sig = Signature(sign_message(&sk, challenge));

    let info = svc
        .register_address(1, addr.clone(), challenge, &sig)
        .await
        .expect("registration should succeed");

    assert_eq!(info.address, addr);
    assert_eq!(info.chain_id, 1);
    assert!(!info.public_key.0.is_empty());
}

#[tokio::test]
async fn register_address_with_wrong_signature_fails() {
    let svc = NonCustodialService::new();
    let sk = SigningKey::random(&mut rand::thread_rng());
    let wrong_addr = Address("0x000000000000000000000000000000000000dead".into());

    let challenge = b"Sign in to fintech-platform";
    let sig = Signature(sign_message(&sk, challenge));

    // The address doesn't match the signer → verification should fail.
    let result = svc
        .register_address(1, wrong_addr, challenge, &sig)
        .await;

    assert!(result.is_err(), "expected registration to fail");
}

#[tokio::test]
async fn verify_signature_succeeds_for_correct_signer() {
    let svc = NonCustodialService::new();
    let sk = SigningKey::random(&mut rand::thread_rng());
    let addr = Address(address_for(&sk));

    let payload = b"transaction payload";
    let sig = Signature(sign_message(&sk, payload));

    svc.verify_signature(&addr, payload, &sig)
        .await
        .expect("verification should succeed");
}

#[tokio::test]
async fn verify_signature_fails_for_wrong_payload() {
    let svc = NonCustodialService::new();
    let sk = SigningKey::random(&mut rand::thread_rng());
    let addr = Address(address_for(&sk));

    let sig = Signature(sign_message(&sk, b"original payload"));
    let tampered = b"tampered payload";

    let result = svc.verify_signature(&addr, tampered, &sig).await;
    assert!(result.is_err(), "verification should fail for wrong payload");
}

#[tokio::test]
async fn recover_signer_matches_expected_address() {
    let svc = NonCustodialService::new();
    let sk = SigningKey::random(&mut rand::thread_rng());
    let expected = address_for(&sk);

    let payload = b"hello";
    let sig = Signature(sign_message(&sk, payload));

    let recovered = svc.recover_signer(payload, &sig).await.expect("recover");
    assert_eq!(recovered.as_str(), expected);
}

#[tokio::test]
async fn lookup_unknown_handle_fails() {
    let svc = NonCustodialService::new();
    let handle = wallet_service::PublicKeyHandle(uuid::Uuid::new_v4());

    let result = svc.lookup_address(&handle).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn prepare_unsigned_returns_payload() {
    let svc = NonCustodialService::new();
    let sk = SigningKey::random(&mut rand::thread_rng());
    let addr = Address(address_for(&sk));

    let challenge = b"register";
    let sig = Signature(sign_message(&sk, challenge));
    let info = svc
        .register_address(1, addr, challenge, &sig)
        .await
        .unwrap();

    let request = SigningRequest {
        chain: common::Chain::Ethereum,
        from: info.address.clone(),
        payload: b"raw tx body".to_vec(),
        summary: Some("test".into()),
    };

    let unsigned = svc
        .prepare_unsigned(&info.handle, &request)
        .await
        .expect("prepare");
    assert_eq!(unsigned, b"raw tx body");
}

#[tokio::test]
async fn attach_signature_produces_signed_payload() {
    let svc = NonCustodialService::new();
    let sk = SigningKey::random(&mut rand::thread_rng());
    let addr = Address(address_for(&sk));

    let challenge = b"register";
    let sig = Signature(sign_message(&sk, challenge));
    let info = svc
        .register_address(1, addr, challenge, &sig)
        .await
        .unwrap();

    // User signs the unsigned payload.
    let unsigned = b"raw tx body";
    let tx_sig = Signature(sign_message(&sk, unsigned));

    let signed = svc
        .attach_signature(&info.handle, unsigned, &tx_sig)
        .await
        .expect("attach");

    assert_eq!(signed.signer, info.address);
    assert!(signed.tx_hash.starts_with("0x"));
    assert_eq!(signed.raw_bytes.len(), unsigned.len() + 65);
}

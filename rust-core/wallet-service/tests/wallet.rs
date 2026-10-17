//! Integration tests for the wallet service.

use k256::ecdsa::{
    signature::hazmat::PrehashSigner,
    SigningKey,
};
use wallet_service::{
    Address, KeyOrigin, MockWalletService, Signature, SigningRequest, WalletService,
};

fn test_signing_key() -> SigningKey {
    let bytes = [0x42u8; 32];
    SigningKey::from_slice(&bytes).expect("valid key")
}

fn address_for(key: &SigningKey) -> Address {
    use k256::sha2::{Digest, Sha256};
    let point = key.verifying_key().to_encoded_point(false);
    let mut hasher = Sha256::new();
    hasher.update(&point.as_bytes()[1..]);
    let hash = hasher.finalize();
    Address::from_hex(&hex::encode(&hash[12..32])).unwrap()
}

fn sign(key: &SigningKey, payload: &[u8]) -> Signature {
    use k256::sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let hash = hasher.finalize();
    let (sig, rec_id) = key.sign_prehash(&hash).expect("sign");
    let mut bytes = [0u8; 65];
    bytes[..64].copy_from_slice(&sig.to_bytes());
    bytes[64] = rec_id.to_byte();
    Signature::from_bytes(bytes)
}

#[tokio::test]
async fn register_address_succeeds_with_valid_signature() {
    let svc = MockWalletService::new();
    let key = test_signing_key();
    let addr = address_for(&key);
    let challenge = b"Sign this message to register: nonce-12345";
    let sig = sign(&key, challenge);

    let info = svc
        .register_address(1, addr.clone(), challenge, &sig)
        .await
        .expect("registration should succeed");

    assert_eq!(info.address, addr);
    assert_eq!(info.chain_id, 1);
    assert_eq!(info.origin, KeyOrigin::Registered);
    assert!(!info.handle.as_str().is_empty());
}

#[tokio::test]
async fn register_address_fails_with_wrong_signer() {
    let svc = MockWalletService::new();
    let key = test_signing_key();
    let wrong_key = SigningKey::from_slice(&[0x11u8; 32]).unwrap();
    let wrong_addr = address_for(&wrong_key);
    let challenge = b"Sign this message";
    let sig = sign(&key, challenge);

    let result = svc.register_address(1, wrong_addr, challenge, &sig).await;
    assert!(matches!(result, Err(wallet_service::WalletError::AddressMismatch { .. })));
}

#[tokio::test]
async fn verify_signature_accepts_valid() {
    let svc = MockWalletService::new();
    let key = test_signing_key();
    let addr = address_for(&key);
    let payload = b"hello world";
    let sig = sign(&key, payload);

    svc.verify_signature(&addr, payload, &sig).await.unwrap();
}

#[tokio::test]
async fn verify_signature_rejects_wrong_payload() {
    let svc = MockWalletService::new();
    let key = test_signing_key();
    let addr = address_for(&key);
    let sig = sign(&key, b"original");

    let result = svc.verify_signature(&addr, b"tampered", &sig).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn prepare_unsigned_prepends_chain_id() {
    let svc = MockWalletService::new();
    let key = test_signing_key();
    let addr = address_for(&key);
    let challenge = b"register";
    let sig = sign(&key, challenge);
    let info = svc.register_address(137, addr, challenge, &sig).await.unwrap();

    let req = SigningRequest {
        chain_id: 137,
        unsigned_payload: b"tx-body".to_vec(),
        description: None,
    };

    let unsigned = svc.prepare_unsigned(&info.handle, &req).await.unwrap();

    assert_eq!(&unsigned[..8], &[0u8, 0, 0, 0, 0, 0, 0, 137u8]);
    assert_eq!(&unsigned[8..], b"tx-body");
}

#[tokio::test]
async fn recover_signer_matches_expected_address() {
    let svc = MockWalletService::new();
    let key = test_signing_key();
    let expected = address_for(&key);
    let payload = b"recover me";
    let sig = sign(&key, payload);

    let recovered = svc.recover_signer(payload, &sig).await.unwrap();
    assert_eq!(recovered, expected);
}

#[tokio::test]
async fn attach_signature_returns_full_payload() {
    let svc = MockWalletService::new();
    let key = test_signing_key();
    let addr = address_for(&key);
    let challenge = b"register";
    let sig = sign(&key, challenge);
    let info = svc.register_address(1, addr, challenge, &sig).await.unwrap();

    let unsigned = b"unsigned-tx";
    let tx_sig = sign(&key, unsigned);

    let signed = svc
        .attach_signature(&info.handle, unsigned, &tx_sig)
        .await
        .unwrap();

    assert_eq!(signed.signed_bytes.len(), unsigned.len() + 65);
}

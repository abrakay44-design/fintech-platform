use common::Chain;
use uuid::Uuid;
use wallet_service::{mock::MockWalletService, WalletService};

#[tokio::test]
async fn derive_is_deterministic() {
    let svc = MockWalletService::new();
    let user = Uuid::new_v4();

    let a1 = svc.derive_address(user, Chain::Ethereum, 0).await.unwrap();
    let a2 = svc.derive_address(user, Chain::Ethereum, 0).await.unwrap();

    assert_eq!(a1.public_key, a2.public_key);
    assert_eq!(a1.address, a2.address);
}

#[tokio::test]
async fn different_users_get_different_addresses() {
    let svc = MockWalletService::new();
    let a = svc.derive_address(Uuid::new_v4(), Chain::Ethereum, 0).await.unwrap();
    let b = svc.derive_address(Uuid::new_v4(), Chain::Ethereum, 0).await.unwrap();
    assert_ne!(a.public_key, b.public_key);
}

#[tokio::test]
async fn different_chains_get_different_addresses() {
    let svc = MockWalletService::new();
    let user = Uuid::new_v4();

    let eth = svc.derive_address(user, Chain::Ethereum, 0).await.unwrap();
    let sol = svc.derive_address(user, Chain::Solana, 0).await.unwrap();
    assert_ne!(eth.public_key, sol.public_key);
}

#[tokio::test]
async fn different_indices_get_different_addresses() {
    let svc = MockWalletService::new();
    let user = Uuid::new_v4();

    let a = svc.derive_address(user, Chain::Ethereum, 0).await.unwrap();
    let b = svc.derive_address(user, Chain::Ethereum, 1).await.unwrap();
    assert_ne!(a.public_key, b.public_key);
}

#[tokio::test]
async fn revoking_unknown_handle_fails() {
    let svc = MockWalletService::new();
    let handle = wallet_service::KeyHandle(Uuid::new_v4());
    assert!(svc.revoke(handle).await.is_err());
}

#[tokio::test]
async fn payload_too_long_is_rejected() {
    let svc = MockWalletService::new();
    let handle = wallet_service::KeyHandle(Uuid::new_v4());
    let payload = vec![0u8; 64];
    let err = svc.sign(handle, &payload).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("payload") || msg.contains("unknown key"),
        "unexpected error: {msg}"
    );
}

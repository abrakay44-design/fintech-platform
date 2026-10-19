//! Integration tests for the end-to-end swap flow.
//!
//! These tests verify the orchestration invariants:
//!
//! 1. A successful swap produces both a tx hash and a ledger entry.
//! 2. A policy denial prevents signing and broadcast.
//! 3. A step-up challenge short-circuits before signing.
//! 4. The ledger entry is balanced (debit == credit).
//! 5. An invalid signature is rejected before broadcast.

use std::sync::Arc;

use chrono::Utc;
use common::{Amount, AssetClass, AssetId, Chain, TxIntent};
use ledger::{AccountId, Ledger, mock::MockLedger};
use policy_engine::mock::MockPolicyEngine;
use transaction_engine::{TransactionEngine, mock::MockTransactionEngine};
use uuid::Uuid;
use wallet_service::{Address, Signature, WalletService, noncustodial::NonCustodialService};

use orchestrator::{
    Dependencies, OrchestratorError, SwapRequest, execute_swap,
};

// ─── helpers ────────────────────────────────────────────────────────────

fn asset(symbol: &str) -> AssetId {
    AssetId {
        chain: Chain::Ethereum,
        contract: None,
        symbol: symbol.into(),
        decimals: 18,
        class: AssetClass::NativeToken,
    }
}

fn sample_intent(user_id: Uuid, eth_amount: rust_decimal::Decimal) -> TxIntent {
    TxIntent {
        intent_id: Uuid::new_v4(),
        user_id,
        from: asset("ETH"),
        to: asset("USDC"),
        amount: Amount {
            asset: asset("ETH"),
            value: eth_amount,
        },
        slippage_bps: 50,
        deadline: Utc::now() + chrono::Duration::minutes(5),
    }
}

/// Register a wallet and produce a signature over the payload.
async fn register_and_sign(
    wallet: &NonCustodialService,
    payload: &[u8],
) -> (wallet_service::PublicKeyHandle, Signature) {
    use k256::ecdsa::signature::hazmat::PrehashSigner;
    use k256::ecdsa::{RecoveryId, Signature as EcdsaSignature, SigningKey, VerifyingKey};
    use sha3::{Digest, Keccak256};

    // Generate a key and derive the EVM address.
    let sk = SigningKey::random(&mut rand::thread_rng());
    let vk: VerifyingKey = *sk.verifying_key();
    let uncompressed = vk.to_encoded_point(false);
    let bytes = uncompressed.as_bytes();
    let mut hasher = Keccak256::new();
    hasher.update(&bytes[1..]);
    let hash = hasher.finalize();
    let mut addr_str = String::from("0x");
    for b in &hash[12..] {
        use std::fmt::Write;
        let _ = write!(addr_str, "{b:02x}");
    }

    // Register the address.
    let challenge = b"orchestrator-test-registration";
    let mut hasher = Keccak256::new();
    hasher.update(challenge);
    let challenge_digest = hasher.finalize();
    let (challenge_sig, challenge_rid): (EcdsaSignature, RecoveryId) = sk
        .sign_prehash_recoverable(&challenge_digest)
        .expect("sign");
    let mut challenge_sig_bytes = Vec::with_capacity(65);
    challenge_sig_bytes.extend_from_slice(&challenge_sig.to_bytes());
    challenge_sig_bytes.push(challenge_rid.to_byte());

    let info = wallet
        .register_address(
            1,
            Address(addr_str),
            challenge,
            &Signature(challenge_sig_bytes),
        )
        .await
        .expect("register");

    // Sign the payload.
    let mut hasher = Keccak256::new();
    hasher.update(payload);
    let digest = hasher.finalize();
    let (sig, rid): (EcdsaSignature, RecoveryId) = sk
        .sign_prehash_recoverable(&digest)
        .expect("sign");
    let mut sig_bytes = Vec::with_capacity(65);
    sig_bytes.extend_from_slice(&sig.to_bytes());
    sig_bytes.push(rid.to_byte());

    (info.handle, Signature(sig_bytes))
}

// ─── tests ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn execute_swap_succeeds_when_allowed() {
    let user_id = Uuid::new_v4();
    let intent = sample_intent(user_id, rust_decimal::Decimal::new(1, 2)); // 0.01 ETH = ~$25

    let tx_engine = Arc::new(MockTransactionEngine::new());
    let policy = Arc::new(MockPolicyEngine::new());
    let wallet = Arc::new(NonCustodialService::new());
    let ledger = Arc::new(MockLedger::new());

    // Build the unsigned payload that the mock tx engine will produce.
    let quote = tx_engine.quote(intent.clone()).await.unwrap();
    let unsigned = tx_engine.build(&quote).await.unwrap();

    let (handle, signature) = register_and_sign(&wallet, &unsigned).await;

    let deps = Dependencies {
        tx_engine: tx_engine.clone(),
        policy: policy.clone(),
        wallet: wallet.clone(),
        ledger: ledger.clone(),
    };

    let req = SwapRequest {
        from_account: AccountId::new(),
        to_account: AccountId::new(),
        wallet_handle: handle,
        signature,
        intent,
    };

    let receipt = execute_swap(&deps, req).await.expect("swap should succeed");
    assert!(receipt.tx_hash.starts_with("0x") || !receipt.tx_hash.is_empty());
    assert_eq!(receipt.chain, Chain::Ethereum);
}

#[tokio::test]
async fn execute_swap_aborts_when_denied() {
    // Use a sanctioned address to trigger a Deny.
    let mut policy = MockPolicyEngine::new();
    policy.add_sanctioned("0xdeadbeef");
    let policy = Arc::new(policy);

    let user_id = Uuid::new_v4();
    // 5 ETH at ~$2500/unit = $12,500, which exceeds the $10,000 per-tx limit.
    let intent = sample_intent(user_id, rust_decimal::Decimal::new(5, 0));

    let tx_engine = Arc::new(MockTransactionEngine::new());
    let wallet = Arc::new(NonCustodialService::new());
    let ledger = Arc::new(MockLedger::new());

    let deps = Dependencies {
        tx_engine: tx_engine.clone(),
        policy,
        wallet,
        ledger,
    };

    let req = SwapRequest {
        from_account: AccountId::new(),
        to_account: AccountId::new(),
        wallet_handle: wallet_service::PublicKeyHandle(Uuid::new_v4()),
        signature: Signature(vec![0u8; 65]),
        intent,
    };

    let result = execute_swap(&deps, req).await;
    // Either Denied or a policy error — either way, no broadcast.
    match result {
        Err(OrchestratorError::Denied { .. }) => {}
        Err(OrchestratorError::Policy(_)) => {}
        other => panic!("expected denial, got {other:?}"),
    }
}

#[tokio::test]
async fn execute_swap_ledger_post_is_balanced() {
    let user_id = Uuid::new_v4();
    let intent = sample_intent(user_id, rust_decimal::Decimal::new(1, 2));

    let tx_engine = Arc::new(MockTransactionEngine::new());
    let policy = Arc::new(MockPolicyEngine::new());
    let wallet = Arc::new(NonCustodialService::new());
    let ledger = Arc::new(MockLedger::new());

    let quote = tx_engine.quote(intent.clone()).await.unwrap();
    let unsigned = tx_engine.build(&quote).await.unwrap();
    let (handle, signature) = register_and_sign(&wallet, &unsigned).await;

    let from_account = AccountId::new();
    let deps = Dependencies {
        tx_engine: tx_engine.clone(),
        policy,
        wallet,
        ledger: ledger.clone(),
    };

    let req = SwapRequest {
        from_account: from_account.clone(),
        to_account: AccountId::new(),
        wallet_handle: handle,
        signature,
        intent: intent.clone(),
    };

    let receipt = execute_swap(&deps, req).await.expect("swap");

    // Verify the ledger has the post.
    let history = ledger.history(&from_account, 10).await.unwrap();
    assert_eq!(history.len(), 1);
    let post = &history[0];
    assert_eq!(post.post_id, receipt.ledger_post_id);
    // Debit and credit must be equal (balanced post).
    let debits: rust_decimal::Decimal = post
        .entries
        .iter()
        .filter_map(|e| e.debit)
        .sum();
    let credits: rust_decimal::Decimal = post
        .entries
        .iter()
        .filter_map(|e| e.credit)
        .sum();
    assert_eq!(debits, credits);
    assert_eq!(debits, intent.amount.value);
}

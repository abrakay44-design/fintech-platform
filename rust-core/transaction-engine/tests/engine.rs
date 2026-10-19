//! Integration tests for the transaction engine.

use common::{Amount, AssetClass, AssetId, Chain, TxIntent};
use chrono::Duration;
use rust_decimal::Decimal;
use transaction_engine::{MockTransactionEngine, TransactionEngine};
use uuid::Uuid;

fn eth() -> AssetId {
    AssetId {
        chain: Chain::Ethereum,
        contract: None,
        symbol: "ETH".to_string(),
        decimals: 18,
        class: AssetClass::NativeToken,
    }
}

fn usdc() -> AssetId {
    AssetId {
        chain: Chain::Ethereum,
        contract: Some("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string()),
        symbol: "USDC".to_string(),
        decimals: 6,
        class: AssetClass::Erc20,
    }
}

fn make_intent(from: AssetId, to: AssetId, value: Decimal) -> TxIntent {
    TxIntent {
        intent_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        from: from.clone(),
        to: to.clone(),
        amount: Amount { asset: from, value },
        slippage_bps: 50,
        deadline: chrono::Utc::now() + Duration::minutes(5),
    }
}

#[tokio::test]
async fn quote_returns_valid_quote() {
    let engine = MockTransactionEngine::new();
    let intent = make_intent(eth(), usdc(), Decimal::new(1, 0));

    let quote = engine.quote(intent.clone()).await.unwrap();

    assert_eq!(quote.intent.from.symbol, "ETH");
    assert_eq!(quote.intent.to.symbol, "USDC");
    assert_eq!(quote.expected_out.value, Decimal::new(95, 2));
    assert_eq!(quote.minimum_out.value, Decimal::new(931, 3));
    assert_eq!(quote.route.len(), 1);
    assert!(quote.expires_at > chrono::Utc::now());
}

#[tokio::test]
async fn quote_rejects_same_asset() {
    let engine = MockTransactionEngine::new();
    let intent = make_intent(eth(), eth(), Decimal::new(1, 0));

    let result = engine.quote(intent).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn build_produces_deterministic_payload() {
    let engine = MockTransactionEngine::new();
    let intent = make_intent(eth(), usdc(), Decimal::new(1, 0));
    let intent_id = intent.intent_id;
    let quote = engine.quote(intent).await.unwrap();

    let unsigned = engine.build(&quote).await.unwrap();

    // Layout: chain_id (8) | intent_id (16) | amount_str (variable)
    assert_eq!(&unsigned[..8], &[0u8, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(&unsigned[8..24], intent_id.as_bytes());
    // Amount "1" is 1 byte, so total is 8 + 16 + 1 = 25
    assert_eq!(unsigned.len(), 25);

    // Rebuilding the same quote must produce identical bytes.
    let unsigned2 = engine.build(&quote).await.unwrap();
    assert_eq!(unsigned, unsigned2);
}

#[tokio::test]
async fn build_rejects_expired_quote() {
    let engine = MockTransactionEngine::new();
    let intent = make_intent(eth(), usdc(), Decimal::new(1, 0));
    let mut quote = engine.quote(intent).await.unwrap();

    quote.expires_at = chrono::Utc::now() - Duration::seconds(1);

    let result = engine.build(&quote).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn simulate_returns_success() {
    let engine = MockTransactionEngine::new();
    let result = engine.simulate(Chain::Ethereum, b"unsigned-tx").await.unwrap();

    assert!(result.success);
    assert!(result.gas_used > 0);
    assert!(result.revert_reason.is_none());
}

#[tokio::test]
async fn broadcast_returns_tx_hash() {
    let engine = MockTransactionEngine::new();
    let result = engine.broadcast(Chain::Ethereum, b"signed-tx").await.unwrap();

    assert!(result.tx_hash.starts_with("0x"));
    assert_eq!(result.tx_hash.len(), 66);
    assert_eq!(result.chain, Chain::Ethereum);
}

#[tokio::test]
async fn status_returns_broadcast_result() {
    let engine = MockTransactionEngine::new();
    let broadcast = engine.broadcast(Chain::Ethereum, b"signed-tx").await.unwrap();

    let status = engine.status(Chain::Ethereum, &broadcast.tx_hash).await.unwrap();
    assert_eq!(status.tx_hash, broadcast.tx_hash);
}

#[tokio::test]
async fn status_returns_error_for_unknown_hash() {
    let engine = MockTransactionEngine::new();
    let result = engine.status(Chain::Ethereum, "0xdeadbeef").await;
    assert!(result.is_err());
}

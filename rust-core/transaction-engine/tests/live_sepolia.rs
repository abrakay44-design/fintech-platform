//! Live Sepolia integration test.
//!
//! Run with:
//!   cargo test -p transaction-engine --test live_sepolia -- --ignored --nocapture
//!
//! Requires `.env` at repo root with `SEPOLIA_RPC_URL` set.

use common::{Amount, AssetClass, AssetId, Chain, TxIntent};
use rust_decimal::Decimal;
use transaction_engine::TransactionEngine;
use uuid::Uuid;

fn eth() -> AssetId {
    AssetId {
        chain: Chain::Ethereum,
        contract: None,
        symbol: "ETH".into(),
        decimals: 18,
        class: AssetClass::NativeToken,
    }
}

#[tokio::test]
#[ignore]
async fn quote_hits_live_sepolia() {
    let engine = transaction_engine::alloy_engine::from_env()
        .expect("SEPOLIA_RPC_URL must be set in .env");

    let intent = TxIntent {
        intent_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        from: eth(),
        to: eth(),
        amount: Amount {
            asset: eth(),
            value: Decimal::new(1, 3),
        },
        slippage_bps: 50,
        deadline: chrono::Utc::now() + chrono::Duration::seconds(30),
    };

    let quote = engine.quote(intent).await.expect("quote failed");
    println!("quote expires at {}", quote.expires_at);
    println!("expected out: {} {}", quote.expected_out.value, quote.expected_out.asset.symbol);
    println!("route: {:?}", quote.route.iter().map(|h| &h.venue).collect::<Vec<_>>());
    assert!(quote.expires_at > chrono::Utc::now());
}

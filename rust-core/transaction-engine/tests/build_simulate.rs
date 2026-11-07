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
async fn build_and_simulate() {
    let engine = transaction_engine::alloy_engine::from_env().expect("RPC URL missing");

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
        deadline: chrono::Utc::now() + chrono::Duration::minutes(1),
    };

    println!("1. Fetching quote...");
    let quote = engine.quote(intent).await.expect("quote failed");

    // Build requires a contract address for `to`, but our dummy intent has `None`.
    // Let's inject a known Sepolia address (e.g., a random EOA) just for the test.
    let mut test_quote = quote.clone();
    test_quote.intent.to.contract = Some("0x0000000000000000000000000000000000000001".into());

    println!("2. Building real EIP-1559 transaction...");
    let unsigned = engine.build(&test_quote, "0x0000000000000000000000000000000000000001").await.expect("build failed");
    println!("Built JSON: {}", String::from_utf8_lossy(&unsigned));

    println!("3. Simulating transaction via eth_call...");
    let sim = engine.simulate(Chain::Ethereum, &unsigned).await.expect("simulate failed");
    
    println!("Simulation success: {}", sim.success);
    println!("Revert reason: {:?}", sim.revert_reason);
    
    assert!(sim.success, "Simulation should succeed for a simple transfer");
}

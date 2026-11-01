//! Live Postgres integration test.
//!
//! Run with:
//!   cargo test -p ledger --test postgres_live -- --ignored --nocapture
//!
//! Requires the docker-compose postgres container to be running
//! (it exposes port 5432 on localhost).

use common::{Amount, AssetClass, AssetId, Chain};
use ledger::{
    types::{AccountId, Entry, Post},
    Ledger, PostgresLedger,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

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
async fn post_and_read_balance() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://fintech:dev_password@localhost:5432/fintech".into()
    });

    let pool = PgPool::connect(&db_url).await.expect("connect to postgres");
    let ledger = PostgresLedger::new(pool);

    let from = AccountId::new();
    let to = AccountId::new();

    // Transfer 0.001 ETH from `from` to `to`.
    // Debit increases balance, credit decreases.
    let post = Post::new(vec![
        Entry::credit(from.clone(), Amount { asset: eth(), value: Decimal::new(1, 3) }),
        Entry::debit(to.clone(),     Amount { asset: eth(), value: Decimal::new(1, 3) }),
    ]);
    let post_id = post.post_id;

    ledger.post(post.clone()).await.expect("first post should succeed");
    println!("✅ Posted: {}", post_id);

    // Duplicate post_id must be rejected
    let dup = ledger.post(post.clone()).await;
    assert!(dup.is_err(), "duplicate post_id should be rejected");
    println!("✅ Duplicate rejected correctly");

    // Balance checks
    let bal_from = ledger.balance(&from, &eth()).await.expect("balance from");
    let bal_to = ledger.balance(&to, &eth()).await.expect("balance to");
    println!("   from balance: {}", bal_from);
    println!("   to   balance: {}", bal_to);
    assert_eq!(bal_from, Decimal::new(-1, 3), "sender should be down 0.001");
    assert_eq!(bal_to, Decimal::new(1, 3), "receiver should be up 0.001");

    // Balances (multi-asset list) should include this one
    let all = ledger.balances(&from).await.expect("balances");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].asset.symbol, "ETH");

    // History should include this post
    let hist = ledger.history(&from, 10).await.expect("history");
    assert_eq!(hist.len(), 1);
    assert_eq!(hist[0].post_id, post_id);
    assert_eq!(hist[0].entries.len(), 2);

    println!("✅ All assertions passed");
    println!("   post_id to check after restart: {}", post_id);
}

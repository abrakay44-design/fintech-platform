//! Integration tests for the ledger.

use common::{Amount, AssetClass, AssetId, Chain};
use ledger::{AccountId, Balance, Entry, Ledger, MockLedger, Post};
use rust_decimal::Decimal;

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

fn amount(asset: AssetId, value: i64) -> Amount {
    Amount {
        asset,
        value: Decimal::from(value),
    }
}

#[tokio::test]
async fn balanced_post_updates_balances() {
    let ledger = MockLedger::new();
    let alice = AccountId::new();
    let bob = AccountId::new();

    let post = Post::new(vec![
        Entry::debit(alice.clone(), amount(eth(), 100)),
        Entry::credit(bob.clone(), amount(eth(), 100)),
    ]);

    ledger.post(post).await.unwrap();

    assert_eq!(
        ledger.balance(&alice, &eth()).await.unwrap(),
        Decimal::from(100)
    );
    assert_eq!(
        ledger.balance(&bob, &eth()).await.unwrap(),
        Decimal::from(-100)
    );
}

#[tokio::test]
async fn unbalanced_post_is_rejected() {
    let ledger = MockLedger::new();
    let alice = AccountId::new();
    let bob = AccountId::new();

    let post = Post::new(vec![
        Entry::debit(alice.clone(), amount(eth(), 100)),
        Entry::credit(bob.clone(), amount(eth(), 50)), // wrong!
    ]);

    let result = ledger.post(post).await;
    assert!(matches!(
        result,
        Err(ledger::LedgerError::Unbalanced { .. })
    ));

    // Balances must be unchanged.
    assert_eq!(
        ledger.balance(&alice, &eth()).await.unwrap(),
        Decimal::ZERO
    );
}

#[tokio::test]
async fn empty_post_is_rejected() {
    let ledger = MockLedger::new();
    let post = Post::new(vec![]);

    let result = ledger.post(post).await;
    assert!(matches!(result, Err(ledger::LedgerError::Empty)));
}

#[tokio::test]
async fn entry_with_both_debit_and_credit_is_rejected() {
    let ledger = MockLedger::new();
    let alice = AccountId::new();
    let bob = AccountId::new();

    let bad_entry = Entry {
        account_id: alice.clone(),
        asset: eth(),
        debit: Some(Decimal::from(100)),
        credit: Some(Decimal::from(100)),
    };

    let post = Post::new(vec![
        bad_entry,
        Entry::credit(bob.clone(), amount(eth(), 0)),
    ]);

    let result = ledger.post(post).await;
    assert!(matches!(result, Err(ledger::LedgerError::InvalidEntry)));
}

#[tokio::test]
async fn multi_asset_swap_balances_per_asset() {
    let ledger = MockLedger::new();
    let alice = AccountId::new();
    let system_eth = AccountId::new();
    let system_usdc = AccountId::new();

    // Alice gives 1 ETH, receives 2500 USDC
    let post = Post::new(vec![
        Entry::debit(system_eth.clone(), amount(eth(), 1)),
        Entry::credit(alice.clone(), amount(eth(), 1)),
        Entry::debit(alice.clone(), amount(usdc(), 2500)),
        Entry::credit(system_usdc.clone(), amount(usdc(), 2500)),
    ]);

    ledger.post(post).await.unwrap();

    assert_eq!(
        ledger.balance(&alice, &eth()).await.unwrap(),
        Decimal::from(-1)
    );
    assert_eq!(
        ledger.balance(&alice, &usdc()).await.unwrap(),
        Decimal::from(2500)
    );
    assert_eq!(
        ledger.balance(&system_eth, &eth()).await.unwrap(),
        Decimal::from(1)
    );
}

#[tokio::test]
async fn duplicate_post_id_is_rejected() {
    let ledger = MockLedger::new();
    let alice = AccountId::new();
    let bob = AccountId::new();

    let post = Post::new(vec![
        Entry::debit(alice.clone(), amount(eth(), 100)),
        Entry::credit(bob.clone(), amount(eth(), 100)),
    ]);
    let post_id = post.post_id;

    ledger.post(post).await.unwrap();

    // Try to post again with the same ID
    let mut dup = Post::new(vec![
        Entry::debit(alice.clone(), amount(eth(), 50)),
        Entry::credit(bob.clone(), amount(eth(), 50)),
    ]);
    dup.post_id = post_id;

    let result = ledger.post(dup).await;
    assert!(matches!(
        result,
        Err(ledger::LedgerError::DuplicatePostId(_))
    ));
}

#[tokio::test]
async fn history_returns_recent_posts() {
    let ledger = MockLedger::new();
    let alice = AccountId::new();
    let bob = AccountId::new();

    for i in 1..=3 {
        let post = Post::new(vec![
            Entry::debit(alice.clone(), amount(eth(), i * 10)),
            Entry::credit(bob.clone(), amount(eth(), i * 10)),
        ]);
        ledger.post(post).await.unwrap();
    }

    let history = ledger.history(&alice, 10).await.unwrap();
    assert_eq!(history.len(), 3);
}

#[tokio::test]
async fn reconciliation_reports_no_discrepancies_when_clean() {
    let ledger = MockLedger::new();
    let alice = AccountId::new();
    let bob = AccountId::new();

    ledger
        .post(Post::new(vec![
            Entry::debit(alice.clone(), amount(eth(), 100)),
            Entry::credit(bob.clone(), amount(eth(), 100)),
        ]))
        .await
        .unwrap();

    let on_chain = vec![Balance {
        account_id: alice.clone(),
        asset: eth(),
        amount: Decimal::from(100),
    }];

    let report = ledger.reconcile(&alice, on_chain).await.unwrap();
    assert!(report.is_clean());
}

#[tokio::test]
async fn reconciliation_detects_discrepancies() {
    let ledger = MockLedger::new();
    let alice = AccountId::new();
    let bob = AccountId::new();

    ledger
        .post(Post::new(vec![
            Entry::debit(alice.clone(), amount(eth(), 100)),
            Entry::credit(bob.clone(), amount(eth(), 100)),
        ]))
        .await
        .unwrap();

    // On-chain shows 90 instead of 100
    let on_chain = vec![Balance {
        account_id: alice.clone(),
        asset: eth(),
        amount: Decimal::from(90),
    }];

    let report = ledger.reconcile(&alice, on_chain).await.unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.discrepancies.len(), 1);
    assert_eq!(report.discrepancies[0].delta, Decimal::from(10));
}

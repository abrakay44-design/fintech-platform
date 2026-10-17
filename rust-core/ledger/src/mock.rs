//! In-memory mock implementation of `Ledger`.

use crate::traits::Ledger;
use crate::types::{AccountId, Balance, Discrepancy, Entry, Post, ReconciliationReport};
use crate::LedgerError;
use async_trait::async_trait;
use chrono::Utc;
use common::AssetId;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Key for looking up a balance: (account, asset_symbol).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BalanceKey {
    account_id: AccountId,
    asset_symbol: String,
}

#[derive(Default)]
struct Store {
    posts: Vec<Post>,
    post_ids: HashSet<uuid::Uuid>,
    balances: HashMap<BalanceKey, Decimal>,
}

pub struct MockLedger {
    store: Arc<RwLock<Store>>,
}

impl MockLedger {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(Store::default())),
        }
    }

    fn balance_key(account: &AccountId, asset: &AssetId) -> BalanceKey {
        BalanceKey {
            account_id: account.clone(),
            asset_symbol: asset.symbol.clone(),
        }
    }
}

impl Default for MockLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate that the post balances per asset.
fn validate_balance(entries: &[Entry]) -> Result<(), LedgerError> {
    let mut debits: HashMap<String, Decimal> = HashMap::new();
    let mut credits: HashMap<String, Decimal> = HashMap::new();

    for entry in entries {
        let symbol = entry.asset.symbol.clone();
        match (entry.debit, entry.credit) {
            (Some(d), None) => {
                *debits.entry(symbol).or_default() += d;
            }
            (None, Some(c)) => {
                *credits.entry(symbol).or_default() += c;
            }
            _ => return Err(LedgerError::InvalidEntry),
        }
    }

    let all_symbols: HashSet<_> = debits.keys().chain(credits.keys()).collect();
    for symbol in all_symbols {
        let d = debits.get(symbol).copied().unwrap_or(Decimal::ZERO);
        let c = credits.get(symbol).copied().unwrap_or(Decimal::ZERO);
        if d != c {
            return Err(LedgerError::Unbalanced {
                debits: d.to_string(),
                credits: c.to_string(),
                asset: symbol.clone(),
            });
        }
    }

    Ok(())
}

#[async_trait]
impl Ledger for MockLedger {
    async fn post(&self, post: Post) -> Result<(), LedgerError> {
        if post.entries.is_empty() {
            return Err(LedgerError::Empty);
        }

        const MAX_ENTRIES: usize = 1_000;
        if post.entries.len() > MAX_ENTRIES {
            return Err(LedgerError::TooManyEntries { max: MAX_ENTRIES });
        }

        validate_balance(&post.entries)?;

        let mut store = self.store.write().await;

        if store.post_ids.contains(&post.post_id) {
            return Err(LedgerError::DuplicatePostId(post.post_id.to_string()));
        }

        // Apply the entries to running balances.
        for entry in &post.entries {
            let key = Self::balance_key(&entry.account_id, &entry.asset);
            let delta = entry.signed_amount();
            *store.balances.entry(key).or_insert(Decimal::ZERO) += delta;
        }

        store.post_ids.insert(post.post_id);
        store.posts.push(post);

        Ok(())
    }

    async fn balance(
        &self,
        account_id: &AccountId,
        asset: &AssetId,
    ) -> Result<Decimal, LedgerError> {
        let store = self.store.read().await;
        let key = Self::balance_key(account_id, asset);
        Ok(store.balances.get(&key).copied().unwrap_or(Decimal::ZERO))
    }

    async fn balances(&self, account_id: &AccountId) -> Result<Vec<Balance>, LedgerError> {
        let store = self.store.read().await;
        Ok(store
            .balances
            .iter()
            .filter(|(k, _)| &k.account_id == account_id)
            .map(|(k, v)| Balance {
                account_id: account_id.clone(),
                asset: AssetId {
                    chain: common::Chain::Ethereum,
                    contract: None,
                    symbol: k.asset_symbol.clone(),
                    decimals: 18,
                    class: common::AssetClass::NativeToken,
                },
                amount: *v,
            })
            .collect())
    }

    async fn history(
        &self,
        account_id: &AccountId,
        limit: usize,
    ) -> Result<Vec<Post>, LedgerError> {
        let store = self.store.read().await;
        Ok(store
            .posts
            .iter()
            .rev()
            .filter(|p| p.entries.iter().any(|e| &e.account_id == account_id))
            .take(limit)
            .cloned()
            .collect())
    }

    async fn reconcile(
        &self,
        account_id: &AccountId,
        on_chain: Vec<Balance>,
    ) -> Result<ReconciliationReport, LedgerError> {
        let mut discrepancies = Vec::new();

        for oc in &on_chain {
            let internal = self.balance(account_id, &oc.asset).await?;
            if internal != oc.amount {
                discrepancies.push(Discrepancy {
                    account_id: account_id.clone(),
                    asset: oc.asset.clone(),
                    internal,
                    on_chain: oc.amount,
                    delta: internal - oc.amount,
                });
            }
        }

        Ok(ReconciliationReport {
            account_id: account_id.clone(),
            checked_at: Utc::now(),
            discrepancies,
        })
    }
}

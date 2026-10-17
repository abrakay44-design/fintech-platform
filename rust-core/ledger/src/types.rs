//! Domain types for the ledger.

use common::{Amount, AssetId};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A single account in the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub Uuid);

impl AccountId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for AccountId {
    fn default() -> Self {
        Self::new()
    }
}

/// A single line in a post. Each line is either a debit OR a credit
/// (never both, never neither).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub account_id: AccountId,
    pub asset: AssetId,
    /// Amount debited from the account (if Some, credit is None).
    pub debit: Option<Decimal>,
    /// Amount credited to the account (if Some, debit is None).
    pub credit: Option<Decimal>,
}

impl Entry {
    pub fn debit(account: AccountId, amount: Amount) -> Self {
        Self {
            account_id: account,
            asset: amount.asset,
            debit: Some(amount.value),
            credit: None,
        }
    }

    pub fn credit(account: AccountId, amount: Amount) -> Self {
        Self {
            account_id: account,
            asset: amount.asset,
            debit: None,
            credit: Some(amount.value),
        }
    }

    /// Net signed amount for this entry (debit positive, credit negative).
    pub fn signed_amount(&self) -> Decimal {
        match (self.debit, self.credit) {
            (Some(d), None) => d,
            (None, Some(c)) => -c,
            _ => Decimal::ZERO,
        }
    }
}

/// A balanced set of entries representing one atomic financial event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub post_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub entries: Vec<Entry>,
    /// Free-form metadata (e.g., original tx hash, user ID, reasoning).
    pub metadata: HashMap<String, String>,
}

impl Post {
    pub fn new(entries: Vec<Entry>) -> Self {
        Self {
            post_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            entries,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Current balance of an account for a specific asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub account_id: AccountId,
    pub asset: AssetId,
    pub amount: Decimal,
}

/// A discrepancy detected during reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discrepancy {
    pub account_id: AccountId,
    pub asset: AssetId,
    pub internal: Decimal,
    pub on_chain: Decimal,
    pub delta: Decimal,
}

/// Result of reconciling an account against on-chain balances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub account_id: AccountId,
    pub checked_at: DateTime<Utc>,
    pub discrepancies: Vec<Discrepancy>,
}

impl ReconciliationReport {
    pub fn is_clean(&self) -> bool {
        self.discrepancies.is_empty()
    }
}

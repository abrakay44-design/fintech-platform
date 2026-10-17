//! The `Ledger` trait — the financial source of truth.

use crate::types::{AccountId, Balance, Post, ReconciliationReport};
use crate::LedgerError;
use async_trait::async_trait;
use common::AssetId;
use rust_decimal::Decimal;

/// The central trait for the double-entry ledger.
///
/// Invariants that implementations MUST uphold:
///
/// 1. **Balance invariant**: `sum(debits) == sum(credits)` per asset
///    within any single post. Unbalanced posts are rejected.
///
/// 2. **Immutability**: Once a post is written, it cannot be modified.
///    Corrections require a new compensating post.
///
/// 3. **Atomicity**: A post either succeeds entirely or fails entirely.
///
/// 4. **Idempotency**: Posting the same `post_id` twice is a no-op
///    (returns `DuplicatePostId` if it's a duplicate, but doesn't
///    corrupt the ledger).
#[async_trait]
pub trait Ledger: Send + Sync + 'static {
    /// Post a balanced set of entries.
    ///
    /// Returns `Err(LedgerError::Unbalanced { .. })` if the post doesn't balance.
    /// Returns `Err(LedgerError::Empty)` if the post has no entries.
    /// Returns `Err(LedgerError::DuplicatePostId(..))` if the post ID already exists.
    async fn post(&self, post: Post) -> Result<(), LedgerError>;

    /// Fetch the balance of an account for a specific asset.
    ///
    /// If the account has never been touched for this asset, returns 0.
    async fn balance(
        &self,
        account_id: &AccountId,
        asset: &AssetId,
    ) -> Result<Decimal, LedgerError>;

    /// Fetch all balances for an account across all assets it touches.
    async fn balances(&self, account_id: &AccountId) -> Result<Vec<Balance>, LedgerError>;

    /// Fetch the full post history for an account (most recent first).
    async fn history(
        &self,
        account_id: &AccountId,
        limit: usize,
    ) -> Result<Vec<Post>, LedgerError>;

    /// Reconcile an account against provided on-chain balances.
    ///
    /// The `on_chain` parameter is expected to come from a blockchain
    /// RPC call. This method compares internal balances to on-chain
    /// and reports any discrepancies.
    async fn reconcile(
        &self,
        account_id: &AccountId,
        on_chain: Vec<Balance>,
    ) -> Result<ReconciliationReport, LedgerError>;
}

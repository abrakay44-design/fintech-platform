//! PostgreSQL-backed double-entry ledger.

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use common::AssetId;
use rust_decimal::Decimal;

use crate::{
    error::LedgerError,
    traits::Ledger,
    types::{AccountId, Balance, Discrepancy, Entry, Post, ReconciliationReport},
};

pub struct PostgresLedger {
    pool: PgPool,
}

impl PostgresLedger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Ledger for PostgresLedger {
    async fn post(&self, post: Post) -> Result<(), LedgerError> {
        if post.entries.is_empty() {
            return Err(LedgerError::Empty);
        }

        let exists: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM posts WHERE post_id = $1)",
        )
        .bind(post.post_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| LedgerError::Internal(e.to_string()))?;

        if exists {
            return Err(LedgerError::DuplicatePostId(post.post_id.to_string()));
        }

        // Enforce balance invariant per asset
        let mut net: HashMap<String, Decimal> = HashMap::new();
        let mut debits_by_asset: HashMap<String, Decimal> = HashMap::new();
        let mut credits_by_asset: HashMap<String, Decimal> = HashMap::new();
        for entry in &post.entries {
            let symbol = entry.asset.symbol.clone();
            if let Some(d) = entry.debit {
                *debits_by_asset.entry(symbol.clone()).or_default() += d;
            }
            if let Some(c) = entry.credit {
                *credits_by_asset.entry(symbol.clone()).or_default() += c;
            }
            *net.entry(symbol).or_default() += entry.signed_amount();
        }
        for (symbol, sum) in &net {
            if *sum != Decimal::ZERO {
                let debits = debits_by_asset.get(symbol).copied().unwrap_or(Decimal::ZERO);
                let credits = credits_by_asset.get(symbol).copied().unwrap_or(Decimal::ZERO);
                return Err(LedgerError::Unbalanced {
                    debits: debits.to_string(),
                    credits: credits.to_string(),
                    asset: symbol.clone(),
                });
            }
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| LedgerError::Internal(e.to_string()))?;

        let metadata_json =
            serde_json::to_value(&post.metadata).map_err(|e| LedgerError::Internal(e.to_string()))?;

        sqlx::query("INSERT INTO posts (post_id, timestamp, metadata) VALUES ($1, $2, $3)")
            .bind(post.post_id)
            .bind(post.timestamp)
            .bind(metadata_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| LedgerError::Internal(e.to_string()))?;

        for entry in &post.entries {
            let asset_json = serde_json::to_value(&entry.asset)
                .map_err(|e| LedgerError::Internal(e.to_string()))?;

            sqlx::query(
                "INSERT INTO entries (entry_id, post_id, account_id, asset, debit, credit)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(post.post_id)
            .bind(entry.account_id.as_uuid())
            .bind(asset_json)
            .bind(entry.debit)
            .bind(entry.credit)
            .execute(&mut *tx)
            .await
            .map_err(|e| LedgerError::Internal(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| LedgerError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn balance(
        &self,
        account_id: &AccountId,
        asset: &AssetId,
    ) -> Result<Decimal, LedgerError> {
        let asset_json =
            serde_json::to_value(asset).map_err(|e| LedgerError::Internal(e.to_string()))?;

        let row: Option<(Decimal,)> = sqlx::query_as::<_, (Decimal,)>(
            "SELECT COALESCE(SUM(COALESCE(debit, 0)), 0) - COALESCE(SUM(COALESCE(credit, 0)), 0)
             FROM entries WHERE account_id = $1 AND asset = $2",
        )
        .bind(account_id.as_uuid())
        .bind(asset_json)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| LedgerError::Internal(e.to_string()))?;

        Ok(row.map(|r| r.0).unwrap_or(Decimal::ZERO))
    }

    async fn balances(&self, account_id: &AccountId) -> Result<Vec<Balance>, LedgerError> {
        let rows: Vec<(serde_json::Value, Decimal)> =
            sqlx::query_as::<_, (serde_json::Value, Decimal)>(
                "SELECT asset, SUM(COALESCE(debit, 0)) - SUM(COALESCE(credit, 0)) AS amount
                 FROM entries WHERE account_id = $1 GROUP BY asset",
            )
            .bind(account_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| LedgerError::Internal(e.to_string()))?;

        let mut out = Vec::new();
        for (asset_json, amount) in rows {
            let asset: AssetId = serde_json::from_value(asset_json)
                .map_err(|e| LedgerError::Internal(e.to_string()))?;
            out.push(Balance {
                account_id: account_id.clone(),
                asset,
                amount,
            });
        }
        Ok(out)
    }

    async fn history(
        &self,
        account_id: &AccountId,
        limit: usize,
    ) -> Result<Vec<Post>, LedgerError> {
        let post_rows: Vec<(Uuid,)> = sqlx::query_as::<_, (Uuid,)>(
            "SELECT p.post_id FROM posts p
             JOIN entries e ON p.post_id = e.post_id
             WHERE e.account_id = $1
             GROUP BY p.post_id, p.timestamp
             ORDER BY p.timestamp DESC
             LIMIT $2",
        )
        .bind(account_id.as_uuid())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| LedgerError::Internal(e.to_string()))?;

        let mut out = Vec::new();
        for (post_id,) in post_rows {
            let post_row: Option<(Uuid, chrono::DateTime<chrono::Utc>, serde_json::Value)> =
                sqlx::query_as::<_, (Uuid, chrono::DateTime<chrono::Utc>, serde_json::Value)>(
                    "SELECT post_id, timestamp, metadata FROM posts WHERE post_id = $1",
                )
                .bind(post_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| LedgerError::Internal(e.to_string()))?;

            let Some((post_id, timestamp, metadata_json)) = post_row else {
                continue;
            };

            let entry_rows: Vec<(Uuid, serde_json::Value, Option<Decimal>, Option<Decimal>)> =
                sqlx::query_as::<_, (Uuid, serde_json::Value, Option<Decimal>, Option<Decimal>)>(
                    "SELECT account_id, asset, debit, credit FROM entries WHERE post_id = $1",
                )
                .bind(post_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| LedgerError::Internal(e.to_string()))?;

            let mut entries = Vec::new();
            for (acc_uuid, asset_json, debit, credit) in entry_rows {
                let asset: AssetId = serde_json::from_value(asset_json)
                    .map_err(|e| LedgerError::Internal(e.to_string()))?;
                entries.push(Entry {
                    account_id: AccountId::from_uuid(acc_uuid),
                    asset,
                    debit,
                    credit,
                });
            }

            let metadata: HashMap<String, String> =
                serde_json::from_value(metadata_json).unwrap_or_default();

            out.push(Post {
                post_id,
                timestamp,
                entries,
                metadata,
            });
        }
        Ok(out)
    }

    async fn reconcile(
        &self,
        account_id: &AccountId,
        on_chain: Vec<Balance>,
    ) -> Result<ReconciliationReport, LedgerError> {
        let internal = self.balances(account_id).await?;
        let mut discrepancies = Vec::new();

        for oc in &on_chain {
            let internal_amount = internal
                .iter()
                .find(|b| b.asset.symbol == oc.asset.symbol)
                .map(|b| b.amount)
                .unwrap_or(Decimal::ZERO);

            if internal_amount != oc.amount {
                discrepancies.push(Discrepancy {
                    account_id: account_id.clone(),
                    asset: oc.asset.clone(),
                    internal: internal_amount,
                    on_chain: oc.amount,
                    delta: internal_amount - oc.amount,
                });
            }
        }

        Ok(ReconciliationReport {
            account_id: account_id.clone(),
            checked_at: chrono::Utc::now(),
            discrepancies,
        })
    }
}

//! In-memory mock implementation of `PolicyEngine`.

use crate::traits::PolicyEngine;
use crate::types::{Limits, PolicyDecision, ScreenResult, StepUpChallenge, StepUpMethod, TxRecord};
use crate::PolicyError;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use common::{Chain, TxIntent};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Threshold above which transactions require step-up auth.
/// In practice this would be per-user, but for the mock we use a global.
const STEP_UP_THRESHOLD_USD: i64 = 1_000;

#[derive(Default)]
struct Store {
    limits: HashMap<Uuid, Limits>,
    records: HashMap<Uuid, Vec<TxRecord>>,
}

pub struct MockPolicyEngine {
    store: Arc<RwLock<Store>>,
    /// Addresses that will be treated as sanctioned (for tests).
    sanctioned: Vec<String>,
}

impl MockPolicyEngine {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(Store::default())),
            sanctioned: vec![
                "0xsanctioned-address-do-not-use".to_string(),
                "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string(),
            ],
        }
    }

    /// Add an address to the mock sanctions list (for tests).
    pub fn add_sanctioned(&mut self, address: impl Into<String>) {
        self.sanctioned.push(address.into());
    }

    /// Estimate USD value of an amount. For the mock we assume 1:1
    /// conversion for USDC and a fixed rate for everything else.
    fn estimate_usd(&self, intent: &TxIntent) -> Decimal {
        let rate = match intent.from.symbol.as_str() {
            "USDC" | "USDT" | "DAI" => Decimal::ONE,
            "ETH" => Decimal::new(2_500, 0),   // $2500/ETH
            "BTC" => Decimal::new(60_000, 0),  // $60,000/BTC
            _ => Decimal::new(1, 0),           // assume $1
        };
        intent.amount.value * rate
    }
}

impl Default for MockPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolicyEngine for MockPolicyEngine {
    async fn evaluate(&self, intent: &TxIntent) -> Result<PolicyDecision, PolicyError> {
        // 1. Reject zero or negative amounts.
        if intent.amount.value <= Decimal::ZERO {
            return Err(PolicyError::InvalidAmount {
                amount: intent.amount.value.to_string(),
                symbol: intent.amount.asset.symbol.clone(),
            });
        }

        // 2. Load limits (default if none set).
        let limits = {
            let store = self.store.read().await;
            store
                .limits
                .get(&intent.user_id)
                .cloned()
                .unwrap_or_default()
        };

        let usd_value = self.estimate_usd(intent);

        // 3. Per-transaction limit.
        if usd_value > limits.per_tx_usd {
            return Err(PolicyError::PerTxLimitExceeded {
                limit: limits.per_tx_usd.to_string(),
                amount: usd_value.to_string(),
            });
        }

        // 4. Daily limit.
        let daily = self.daily_total_usd(intent.user_id).await?;
        let would_be = daily + usd_value;
        if would_be > limits.daily_withdrawal_usd {
            return Err(PolicyError::DailyLimitExceeded {
                limit: limits.daily_withdrawal_usd.to_string(),
                would_be: would_be.to_string(),
            });
        }

        // 5. Velocity.
        let count = {
            let store = self.store.read().await;
            let cutoff = Utc::now() - Duration::hours(1);
            store
                .records
                .get(&intent.user_id)
                .map(|v| v.iter().filter(|r| r.timestamp > cutoff).count() as u32)
                .unwrap_or(0)
        };
        if count >= limits.velocity_per_hour {
            return Err(PolicyError::VelocityExceeded {
                count,
                max: limits.velocity_per_hour,
            });
        }

        // 6. Sanctions screening on the destination.
        let dest = intent
            .to
            .contract
            .as_deref()
            .unwrap_or("0x0000000000000000000000000000000000000000");
        if self.sanctioned.iter().any(|s| s.eq_ignore_ascii_case(dest)) {
            return Err(PolicyError::SanctionedAddress {
                address: dest.to_string(),
            });
        }

        // 7. Step-up required for high-value transactions.
        if usd_value >= Decimal::from(STEP_UP_THRESHOLD_USD) {
            return Ok(PolicyDecision::RequireStepUp {
                challenge: StepUpChallenge {
                    challenge_id: Uuid::new_v4(),
                    method: StepUpMethod::Passkey,
                    expires_at: Utc::now() + Duration::minutes(5),
                    reason: format!(
                        "Transaction of ${usd_value} USD exceeds the ${STEP_UP_THRESHOLD_USD} step-up threshold"
                    ),
                },
            });
        }

        Ok(PolicyDecision::Allow)
    }

    async fn screen_address(
        &self,
        address: &str,
        _chain: Chain,
    ) -> Result<ScreenResult, PolicyError> {
        if self.sanctioned.iter().any(|s| s.eq_ignore_ascii_case(address)) {
            return Ok(ScreenResult::Sanctioned {
                list: "mock-sanctions".to_string(),
            });
        }
        Ok(ScreenResult::Clean)
    }

    async fn get_limits(&self, user_id: Uuid) -> Result<Limits, PolicyError> {
        let store = self.store.read().await;
        Ok(store.limits.get(&user_id).cloned().unwrap_or_default())
    }

    async fn set_limits(&self, user_id: Uuid, limits: Limits) -> Result<(), PolicyError> {
        let mut store = self.store.write().await;
        store.limits.insert(user_id, limits);
        Ok(())
    }

    async fn record(&self, record: TxRecord) -> Result<(), PolicyError> {
        let mut store = self.store.write().await;
        store
            .records
            .entry(record.user_id)
            .or_default()
            .push(record);
        Ok(())
    }

    async fn daily_total_usd(&self, user_id: Uuid) -> Result<Decimal, PolicyError> {
        let store = self.store.read().await;
        let cutoff = Utc::now() - Duration::hours(24);
        let total: Decimal = store
            .records
            .get(&user_id)
            .map(|v| {
                v.iter()
                    .filter(|r| r.timestamp > cutoff)
                    .map(|r| r.usd_value)
                    .sum()
            })
            .unwrap_or(Decimal::ZERO);
        Ok(total)
    }
}

//! Domain types for the policy engine.

use common::TxIntent;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The decision returned by the policy engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PolicyDecision {
    /// Transaction is allowed.
    Allow,
    /// Transaction is denied — hard block.
    Deny { reason: String },
    /// Transaction requires step-up authentication.
    RequireStepUp { challenge: StepUpChallenge },
}

/// A challenge the user must complete before the transaction can proceed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepUpChallenge {
    pub challenge_id: Uuid,
    pub method: StepUpMethod,
    pub expires_at: DateTime<Utc>,
    /// Human-readable reason shown to the user.
    pub reason: String,
}

/// Available step-up methods.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepUpMethod {
    /// Re-authenticate with the user's passkey.
    Passkey,
    /// 6-digit TOTP code from authenticator app.
    Totp,
    /// Email confirmation with a time-limited link.
    EmailConfirmation,
    /// Manual review by support team.
    ManualReview,
}

/// User-defined and platform-enforced limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    /// Maximum USD-equivalent per single transaction.
    pub per_tx_usd: Decimal,
    /// Maximum USD-equivalent over rolling 24 hours.
    pub daily_withdrawal_usd: Decimal,
    /// Maximum transactions per rolling hour.
    pub velocity_per_hour: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            per_tx_usd: Decimal::new(10_000, 0),          // $10,000
            daily_withdrawal_usd: Decimal::new(50_000, 0), // $50,000
            velocity_per_hour: 20,
        }
    }
}

/// A record of a completed transaction, used for velocity tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRecord {
    pub tx_id: Uuid,
    pub user_id: Uuid,
    pub intent: TxIntent,
    pub usd_value: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Result of screening an address against sanctions lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenResult {
    /// Address is not on any list.
    Clean,
    /// Address is on the sanctions list (OFAC, EU, UN).
    Sanctioned { list: String },
    /// Address is flagged for suspicious activity (not necessarily sanctioned).
    Flagged { reason: String },
}

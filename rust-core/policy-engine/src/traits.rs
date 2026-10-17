//! The `PolicyEngine` trait — enforces safety before any signing happens.

use crate::types::{Limits, PolicyDecision, ScreenResult, TxRecord};
use crate::PolicyError;
use async_trait::async_trait;
use common::{Chain, TxIntent};
use uuid::Uuid;

/// The central trait for policy enforcement.
///
/// Invariants that implementations must uphold:
///
/// 1. **`evaluate` is called BEFORE every signing operation.** It must
///    return `Allow`, `Deny`, or `RequireStepUp` — never "skip".
/// 2. **Velocity tracking is atomic.** Two transactions submitted
///    simultaneously must not both count "0 prior transactions".
/// 3. **Sanctions screening is mandatory** for all outbound transfers.
/// 4. **`record` must be called after a successful broadcast** to keep
///    velocity and limits accurate.
#[async_trait]
pub trait PolicyEngine: Send + Sync + 'static {
    /// Evaluate an intent against all policies before signing.
    async fn evaluate(&self, intent: &TxIntent) -> Result<PolicyDecision, PolicyError>;

    /// Screen an address against sanctions and AML lists.
    async fn screen_address(
        &self,
        address: &str,
        chain: Chain,
    ) -> Result<ScreenResult, PolicyError>;

    /// Fetch the current limits for a user.
    async fn get_limits(&self, user_id: Uuid) -> Result<Limits, PolicyError>;

    /// Set new limits for a user.
    async fn set_limits(&self, user_id: Uuid, limits: Limits) -> Result<(), PolicyError>;

    /// Record a completed transaction (for velocity and daily totals).
    async fn record(&self, record: TxRecord) -> Result<(), PolicyError>;

    /// Sum of USD-equivalent transactions for a user in the last 24 hours.
    async fn daily_total_usd(&self, user_id: Uuid) -> Result<rust_decimal::Decimal, PolicyError>;
}

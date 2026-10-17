//! Errors specific to the policy engine.

use common::CoreError;

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("daily limit exceeded: {limit} USD, would reach {would_be} USD")]
    DailyLimitExceeded { limit: String, would_be: String },

    #[error("per-transaction limit exceeded: {limit} USD, got {amount} USD")]
    PerTxLimitExceeded { limit: String, amount: String },

    #[error("velocity limit exceeded: {count} in last hour, max {max}")]
    VelocityExceeded { count: u32, max: u32 },

    #[error("address {address} is on the sanctions list")]
    SanctionedAddress { address: String },

    #[error("address {address} is flagged for {reason}")]
    FlaggedAddress { address: String, reason: String },

    #[error("amount {amount} {symbol} is invalid or zero")]
    InvalidAmount { amount: String, symbol: String },

    #[error("unsupported chain: {0}")]
    UnsupportedChain(u64),

    #[error("user {0} not found")]
    UserNotFound(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<PolicyError> for CoreError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::DailyLimitExceeded { .. }
            | PolicyError::PerTxLimitExceeded { .. }
            | PolicyError::VelocityExceeded { .. }
            | PolicyError::SanctionedAddress { .. }
            | PolicyError::FlaggedAddress { .. }
            | PolicyError::InvalidAmount { .. }
            | PolicyError::UnsupportedChain(_)
            | PolicyError::UserNotFound(_) => {
                CoreError::PolicyViolation(err.to_string())
            }
            PolicyError::Storage(_) | PolicyError::Internal(_) => {
                CoreError::Internal("policy engine".into())
            }
        }
    }
}

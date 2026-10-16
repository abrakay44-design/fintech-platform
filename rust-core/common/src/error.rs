#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("policy violation: {0}")]
    PolicyViolation(String),
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("signing failure: {0}")]
    SigningFailure(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

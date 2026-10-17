//! Errors specific to the ledger.

use common::CoreError;

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("post is unbalanced: debits {debits} != credits {credits} for asset {asset}")]
    Unbalanced {
        debits: String,
        credits: String,
        asset: String,
    },

    #[error("post is empty")]
    Empty,

    #[error("post contains more than {max} entries")]
    TooManyEntries { max: usize },

    #[error("entry has both debit and credit, or neither")]
    InvalidEntry,

    #[error("account {0} not found")]
    AccountNotFound(String),

    #[error("duplicate post id: {0}")]
    DuplicatePostId(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<LedgerError> for CoreError {
    fn from(err: LedgerError) -> Self {
        match err {
            LedgerError::Unbalanced { .. }
            | LedgerError::Empty
            | LedgerError::InvalidEntry
            | LedgerError::TooManyEntries { .. } => CoreError::InvalidInput(err.to_string()),
            LedgerError::AccountNotFound(_) | LedgerError::DuplicatePostId(_) => {
                CoreError::InvalidInput(err.to_string())
            }
            LedgerError::Storage(_) | LedgerError::Internal(_) => {
                CoreError::Internal("ledger".into())
            }
        }
    }
}

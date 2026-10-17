//! Ledger — the double-entry accounting system.
//!
//! Every value movement in the platform MUST be recorded here.
//! The ledger is the financial source of truth.
//!
//! Invariants:
//! - Every post balances (sum of debits == sum of credits per asset)
//! - Posts are immutable (corrections = compensating posts)
//! - Atomicity: all entries in a post succeed or fail together

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod error;
pub mod mock;
pub mod traits;
pub mod types;

pub use error::LedgerError;
pub use mock::MockLedger;
pub use traits::Ledger;
pub use types::{
    AccountId, Balance, Discrepancy, Entry, Post, ReconciliationReport,
};

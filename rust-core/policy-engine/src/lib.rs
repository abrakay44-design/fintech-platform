//! Policy engine — the security layer between intent and signing.
//!
//! Every transaction MUST pass through `PolicyEngine::evaluate` before
//! it is signed. The decision can be:
//!
//! - `Allow` — proceed to sign
//! - `Deny` — hard block (limits, sanctions, invalid input)
//! - `RequireStepUp` — user must re-authenticate before signing

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod error;
pub mod mock;
pub mod traits;
pub mod types;

pub use error::PolicyError;
pub use mock::MockPolicyEngine;
pub use traits::PolicyEngine;
pub use types::{
    Limits, PolicyDecision, ScreenResult, StepUpChallenge, StepUpMethod, TxRecord,
};

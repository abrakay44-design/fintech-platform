//! Transaction engine — converts intents into signed, broadcastable transactions.
//!
//! Architecture:
//!
//! ```text
//! TxIntent  --quote--> Quote
//! Quote     --build--> unsigned bytes
//! unsigned  --wallet.sign--> SignedPayload
//! signed    --broadcast--> BroadcastResult
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod error;
pub mod mock;
pub mod traits;
pub mod types;

pub use error::TxError;
pub use mock::MockTransactionEngine;
pub use traits::TransactionEngine;
pub use types::{BroadcastResult, Quote, RouteHop, SimulationResult};

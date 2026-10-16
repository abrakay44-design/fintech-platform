//! Shared types, errors, and domain primitives.

pub mod asset;
pub mod error;
pub mod tx;

pub use asset::{Amount, AssetClass, AssetId, Chain};
pub use error::{CoreError, CoreResult};
pub use tx::{SignedTx, TxIntent, TxStatus};

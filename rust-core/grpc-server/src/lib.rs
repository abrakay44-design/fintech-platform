//! gRPC server that exposes the Rust core to external clients.
//!
//! ## Design
//!
//! - One `tonic` service: `SwapService` (defined in `proto/fintech.proto`).
//! - Every RPC delegates to the orchestrator or wallet service.
//! - The service holds references to the dependency traits (via `Arc<dyn _>`),
//!   so mock implementations can be injected for tests.

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

pub mod convert;
pub mod service;

pub mod proto {
    tonic::include_proto!("fintech.v1");
}

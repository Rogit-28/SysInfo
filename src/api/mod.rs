//! HTTP API module for the SysInfo telemetry system.
//!
//! Provides a REST API for accessing hardware telemetry data using Axum.

pub mod handlers;
mod routes;
mod server;

pub use routes::create_router;
pub use server::{start_server, AppState};

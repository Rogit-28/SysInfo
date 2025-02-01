//! Data persistence module.
//!
//! Handles storing telemetry data to external systems like InfluxDB.

pub mod influxdb;

pub use influxdb::{InfluxDbClient, InfluxDbWriter};

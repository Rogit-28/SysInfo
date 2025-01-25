//! Platform-specific implementations.
//!
//! Contains platform abstraction layers for different operating systems.
//! Currently only Windows is supported.

#[cfg(windows)]
pub mod windows;

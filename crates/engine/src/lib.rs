//! Arbitrage detection engine.
//!
//! This crate contains the core logic for detecting arbitrage opportunities
//! across multiple exchanges and calculating optimal routes.

pub mod detector;
pub mod premium;
pub mod route;

pub use detector::*;
pub use premium::*;
pub use route::*;

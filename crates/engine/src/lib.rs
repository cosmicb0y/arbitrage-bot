//! Arbitrage detection engine.
//!
//! This crate contains the core logic for detecting arbitrage opportunities
//! across multiple exchanges and calculating optimal routes.

pub mod depth;
pub mod detector;
pub mod fee;
pub mod orderbook;
pub mod premium;
pub mod route;

pub use depth::*;
pub use detector::*;
pub use fee::*;
pub use orderbook::*;
pub use premium::*;
pub use route::*;

//! Trade execution for arbitrage opportunities.
//!
//! This crate handles order submission, status tracking, and execution
//! across CEX and DEX platforms.

pub mod error;
pub mod order;
pub mod cex;
pub mod dex;

pub use error::*;
pub use order::*;
pub use cex::*;
pub use dex::*;

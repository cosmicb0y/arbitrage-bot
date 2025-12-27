//! High-performance serialization for arbitrage data.
//!
//! Uses efficient binary format for fast serialization/deserialization.

pub mod opportunity;
pub mod price;

pub use opportunity::*;
pub use price::*;

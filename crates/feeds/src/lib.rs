//! Real-time price feed collection from exchanges.
//!
//! This crate provides WebSocket connections to various exchanges
//! for collecting real-time price data.

pub mod adapter;
pub mod aggregator;
pub mod discovery;
pub mod error;
pub mod feed;
pub mod manager;
pub mod websocket;

pub use adapter::*;
pub use aggregator::*;
pub use discovery::*;
pub use error::*;
pub use feed::*;
pub use manager::*;
pub use websocket::*;

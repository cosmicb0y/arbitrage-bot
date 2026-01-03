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
pub mod rest;
pub mod symbol_mapping;
pub mod websocket;

pub use adapter::{
    BinanceAdapter, BithumbAdapter, BybitAdapter, CoinbaseAdapter, CoinbaseCredentials,
    CoinbaseL2Event, GateIOAdapter, UpbitAdapter,
};
pub use aggregator::*;
pub use discovery::*;
pub use error::*;
pub use feed::*;
pub use manager::*;
pub use rest::*;
pub use symbol_mapping::*;
pub use websocket::*;

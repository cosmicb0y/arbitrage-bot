//! Real-time price feed collection from exchanges.
//!
//! This crate provides WebSocket connections to various exchanges
//! for collecting real-time price data.
//!
//! ## Architecture
//!
//! - `adapter/` - Exchange-specific message parsing
//! - `runner/` - Feed runners that process WebSocket messages and emit `FeedMessage`
//! - `message` - Channel message types (`FeedMessage`, `ParsedTick`, `ConnectionEvent`)

pub mod adapter;
pub mod aggregator;
pub mod discovery;
pub mod error;
pub mod feed;
pub mod manager;
pub mod message;
pub mod rest;
pub mod runner;
pub mod subscription;
pub mod symbol_mapping;
pub mod websocket;

pub use adapter::{
    BinanceAdapter, BithumbAdapter, BithumbMessage, BybitAdapter, CoinbaseAdapter,
    CoinbaseCredentials, CoinbaseL2Event, ExchangeAdapter, GateIOAdapter, KoreanExchangeAdapter,
    UpbitAdapter, UpbitMessage,
};
pub use aggregator::*;
pub use discovery::*;
pub use error::*;
pub use feed::*;
pub use manager::*;
pub use message::{ConnectionEvent, FeedMessage, Orderbook, ParsedTick};
pub use rest::*;
pub use runner::*;
pub use subscription::{
    BatchSubscriptionConfig, BatchSubscriptionResult, BinanceSubscriptionBuilder,
    BithumbSubscriptionBuilder, BybitSubscriptionBuilder, CoinbaseSubscriptionBuilder,
    ExchangeRateLimit, ExchangeSubscriptionTracker, GateIOSubscriptionBuilder,
    NewMarketSubscriptionHandler, SubscriptionChange, SubscriptionError, SubscriptionEvent,
    SubscriptionEventType, SubscriptionLogger, SubscriptionManager, SubscriptionRateLimiter,
    SubscriptionRetryPolicy, SubscriptionRetryState, SubscriptionStatus, UpbitSubscriptionBuilder,
    SUBSCRIPTION_CHANNEL_BUFFER,
};
pub use symbol_mapping::*;
pub use websocket::*;

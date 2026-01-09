//! Feed handlers for cryptocurrency exchange WebSocket connections.
//!
//! This module provides feed handlers for processing real-time price data
//! from various cryptocurrency exchanges.

pub mod common;
pub mod connection;
pub mod korean;
pub mod overseas;

use crate::state::SharedState;
use crate::status_notifier::StatusNotifierHandle;
use crate::ws_server::BroadcastSender;
use arbitrage_feeds::SymbolMappings;
use std::sync::Arc;

/// Shared context passed to all feed handlers.
///
/// Contains all the dependencies needed by feed handlers to process
/// WebSocket messages and update application state.
#[derive(Clone)]
pub struct FeedContext {
    pub state: SharedState,
    pub broadcast_tx: BroadcastSender,
    pub symbol_mappings: Arc<SymbolMappings>,
    pub status_notifier: Option<StatusNotifierHandle>,
}

impl FeedContext {
    /// Create a new FeedContext with all required dependencies.
    pub fn new(
        state: SharedState,
        broadcast_tx: BroadcastSender,
        symbol_mappings: Arc<SymbolMappings>,
        status_notifier: Option<StatusNotifierHandle>,
    ) -> Self {
        Self {
            state,
            broadcast_tx,
            symbol_mappings,
            status_notifier,
        }
    }
}

// Re-export feed handlers for convenient access
pub use korean::bithumb::run_bithumb_feed;
pub use korean::upbit::run_upbit_feed;
pub use overseas::binance::run_binance_feed;
pub use overseas::bybit::run_bybit_feed;
pub use overseas::coinbase::run_coinbase_feed;
pub use overseas::gateio::run_gateio_feed;

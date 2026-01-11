//! Feed handlers for cryptocurrency exchange WebSocket connections.
//!
//! This module provides feed handlers for processing real-time price data
//! from various cryptocurrency exchanges.
//!
//! ## Architecture
//!
//! Feed processing is split between `crates/feeds` (runners) and this module (handler):
//! - **Runners** (in `arbitrage_feeds::runner`): Parse exchange-specific messages, emit `FeedMessage`
//! - **Handler** (`handler::run_feed_handler`): Process `FeedMessage`, update state, broadcast to clients
//!
//! ## Data Flow
//!
//! ```text
//! WsClient (WebSocket) → Runner (parsing) → FeedMessage → Handler (state update, broadcast)
//! ```

pub mod common;
pub mod handler;

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

// Re-export handler
pub use handler::run_feed_handler;

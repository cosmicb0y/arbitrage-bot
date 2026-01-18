//! Feed runners that process WebSocket messages and emit FeedMessage.
//!
//! Each runner:
//! - Receives `WsMessage` from a WebSocket connection
//! - Parses exchange-specific formats using adapters
//! - Emits `FeedMessage` (containing `ParsedTick` or `ConnectionEvent`)
//! - Has no application-level dependencies (no SharedState, no broadcast)
//!
//! The application handler receives `FeedMessage` and handles:
//! - State updates
//! - Currency conversions
//! - Broadcasting to clients

mod binance;
mod bithumb;
mod bybit;
mod coinbase;
mod gateio;
mod upbit;

pub use binance::run_binance;
pub use bithumb::run_bithumb;
pub use bybit::run_bybit;
pub use coinbase::run_coinbase;
pub use gateio::run_gateio;
pub use upbit::run_upbit;

use crate::message::{ConnectionEvent, FeedMessage};
use crate::WsMessage;
use arbitrage_core::Exchange;
use tokio::sync::mpsc;

/// Sender type for feed messages.
pub type FeedSender = mpsc::Sender<FeedMessage>;

/// Handle WebSocket connection lifecycle events.
///
/// Returns `true` if the message was a connection event (caller should continue to next message).
/// Returns `false` if the message is data that should be processed.
pub fn handle_connection_event(msg: &WsMessage, exchange: Exchange, tx: &FeedSender) -> bool {
    match msg {
        WsMessage::Connected => {
            let _ = tx.try_send(ConnectionEvent::Connected(exchange).into());
            true
        }
        WsMessage::Reconnected => {
            let _ = tx.try_send(ConnectionEvent::Reconnected(exchange).into());
            true
        }
        WsMessage::Disconnected => {
            let _ = tx.try_send(ConnectionEvent::Disconnected(exchange).into());
            true
        }
        WsMessage::Error(e) => {
            let _ = tx.try_send(ConnectionEvent::Error(exchange, e.clone()).into());
            true
        }
        WsMessage::CircuitBreakerOpen(duration) => {
            let _ = tx.try_send(ConnectionEvent::CircuitBreakerOpen(exchange, *duration).into());
            true
        }
        WsMessage::Text(_) | WsMessage::Binary(_) => false,
    }
}

/// Drain stale messages from the receiver channel.
///
/// Call this on disconnect to clear any buffered messages that are
/// now invalid due to the connection being lost.
pub fn drain_channel(rx: &mut mpsc::Receiver<WsMessage>) {
    while rx.try_recv().is_ok() {}
}

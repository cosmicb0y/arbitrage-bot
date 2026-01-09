//! WebSocket connection lifecycle handling.
//!
//! This module provides a common handler for WebSocket connection events
//! that is shared across all exchange feed handlers.

use crate::status_notifier::{StatusEvent, StatusNotifierHandle};
use arbitrage_core::Exchange;
use arbitrage_feeds::WsMessage;
use tracing::{debug, info, warn};

/// Action to take after handling a connection event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionAction {
    /// Skip to next message (connection event was handled)
    Continue,
    /// Process this message normally (it's a data message)
    ProcessMessage,
}

/// Handle WebSocket connection lifecycle events.
///
/// This function handles the common connection events (Connected, Reconnected,
/// Disconnected, Error, CircuitBreakerOpen) that are identical across all exchanges.
///
/// # Arguments
/// * `msg` - The WebSocket message to check
/// * `exchange` - The exchange this message is from
/// * `status_notifier` - Optional notifier for connection status updates
/// * `on_reconnect` - Callback invoked on reconnection for cache clearing
///
/// # Returns
/// * `ConnectionAction::Continue` - The message was a connection event, skip to next
/// * `ConnectionAction::ProcessMessage` - The message is data, process it normally
pub fn handle_connection_event<F>(
    msg: &WsMessage,
    exchange: Exchange,
    status_notifier: &Option<StatusNotifierHandle>,
    on_reconnect: F,
) -> ConnectionAction
where
    F: FnOnce(),
{
    match msg {
        WsMessage::Connected => {
            debug!("{:?}: Connected to WebSocket", exchange);
            if let Some(ref notifier) = status_notifier {
                notifier.try_send(StatusEvent::Connected(exchange));
            }
            ConnectionAction::Continue
        }
        WsMessage::Reconnected => {
            info!("{:?}: Reconnected - clearing all cached data", exchange);
            on_reconnect();
            if let Some(ref notifier) = status_notifier {
                notifier.try_send(StatusEvent::Reconnected(exchange));
            }
            ConnectionAction::Continue
        }
        WsMessage::Disconnected => {
            warn!("{:?}: Disconnected from WebSocket", exchange);
            if let Some(ref notifier) = status_notifier {
                notifier.try_send(StatusEvent::Disconnected(exchange));
            }
            ConnectionAction::Continue
        }
        WsMessage::Error(e) => {
            warn!("{:?}: Error - {}", exchange, e);
            ConnectionAction::Continue
        }
        WsMessage::CircuitBreakerOpen(wait_time) => {
            warn!(
                "{:?}: Circuit breaker OPEN - connection blocked for {:?}",
                exchange, wait_time
            );
            if let Some(ref notifier) = status_notifier {
                notifier.try_send(StatusEvent::CircuitBreakerOpen(exchange, *wait_time));
            }
            ConnectionAction::Continue
        }
        // Text and Binary messages should be processed by the specific feed handler
        WsMessage::Text(_) | WsMessage::Binary(_) => ConnectionAction::ProcessMessage,
    }
}

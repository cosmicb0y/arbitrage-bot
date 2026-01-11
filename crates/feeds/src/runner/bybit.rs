//! Bybit feed runner.
//!
//! Processes WebSocket messages from Bybit and emits ParsedTick messages.
//! Handles both snapshot and delta orderbook updates.

use super::{drain_channel, handle_connection_event, FeedSender};
use crate::adapter::BybitAdapter;
use crate::message::{Orderbook, ParsedTick};
use crate::WsMessage;
use arbitrage_core::Exchange;
use tokio::sync::mpsc;
use tracing::debug;

/// Run the Bybit feed processor.
///
/// Receives WebSocket messages, parses them using BybitAdapter,
/// and sends ParsedTick messages to the handler.
pub async fn run_bybit(mut rx: mpsc::Receiver<WsMessage>, tx: FeedSender) {
    debug!("Starting Bybit feed runner");

    while let Some(msg) = rx.recv().await {
        // Handle connection lifecycle events
        if handle_connection_event(&msg, Exchange::Bybit, &tx) {
            // On disconnect, drain stale messages
            if matches!(msg, WsMessage::Disconnected) {
                drain_channel(&mut rx);
            }
            continue;
        }

        // Process Bybit-specific messages (text only)
        if let WsMessage::Text(text) = msg {
            process_text_message(&text, &tx);
        }
    }

    debug!("Bybit feed runner stopped");
}

/// Process a text (JSON) message from Bybit.
fn process_text_message(text: &str, tx: &FeedSender) {
    // Process orderbook messages (both snapshots and deltas)
    if BybitAdapter::is_orderbook_message(text) {
        if let Ok((_tick, symbol, quote, bids, asks, is_snapshot)) =
            BybitAdapter::parse_orderbook_full(text)
        {
            // Check if this is a stablecoin rate update
            if symbol == "USDT" || symbol == "USDC" {
                // Get mid price from best bid/ask
                let mid = if let (Some((bid, _)), Some((ask, _))) = (bids.first(), asks.first()) {
                    (bid + ask) / 2.0
                } else {
                    return;
                };

                let rate_tick = ParsedTick::stablecoin_rate(
                    Exchange::Bybit,
                    &symbol,
                    &quote,
                    arbitrage_core::FixedPoint::from_f64(mid),
                );
                let _ = tx.try_send(rate_tick.into());
            }

            // Create orderbook (snapshot or delta)
            let orderbook = if is_snapshot {
                Orderbook::new(bids, asks)
            } else {
                Orderbook::delta(bids, asks)
            };

            // Get best bid/ask for the tick
            let (bid, bid_size) = orderbook.bids.first().copied().unwrap_or((0.0, 0.0));
            let (ask, ask_size) = orderbook.asks.first().copied().unwrap_or((0.0, 0.0));

            if bid > 0.0 && ask > 0.0 {
                let mid = (bid + ask) / 2.0;

                let parsed = ParsedTick::price_with_orderbook(
                    Exchange::Bybit,
                    symbol,
                    quote,
                    arbitrage_core::FixedPoint::from_f64(mid),
                    arbitrage_core::FixedPoint::from_f64(bid),
                    arbitrage_core::FixedPoint::from_f64(ask),
                    arbitrage_core::FixedPoint::from_f64(bid_size),
                    arbitrage_core::FixedPoint::from_f64(ask_size),
                    orderbook,
                );

                let _ = tx.try_send(parsed.into());
            }
        }
    }
}

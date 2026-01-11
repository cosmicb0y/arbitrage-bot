//! Gate.io feed runner.
//!
//! Processes WebSocket messages from Gate.io and emits ParsedTick messages.
//! Handles both snapshot and delta orderbook updates.

use super::{drain_channel, handle_connection_event, FeedSender};
use crate::adapter::{ExchangeAdapter, GateIOAdapter};
use crate::message::{Orderbook, ParsedTick};
use crate::WsMessage;
use arbitrage_core::{Exchange, FixedPoint};
use tokio::sync::mpsc;
use tracing::debug;

/// Run the Gate.io feed processor.
///
/// Receives WebSocket messages, parses them using GateIOAdapter,
/// and sends ParsedTick messages to the handler.
pub async fn run_gateio(mut rx: mpsc::Receiver<WsMessage>, tx: FeedSender) {
    debug!("Starting Gate.io feed runner");

    while let Some(msg) = rx.recv().await {
        // Handle connection lifecycle events
        if handle_connection_event(&msg, Exchange::GateIO, &tx) {
            // On disconnect, drain stale messages
            if matches!(msg, WsMessage::Disconnected) {
                drain_channel(&mut rx);
            }
            continue;
        }

        // Process Gate.io-specific messages (text only)
        if let WsMessage::Text(text) = msg {
            process_text_message(&text, &tx);
        }
    }

    debug!("Gate.io feed runner stopped");
}

/// Process a text (JSON) message from Gate.io.
fn process_text_message(text: &str, tx: &FeedSender) {
    // Process orderbook messages (both snapshots and deltas)
    if GateIOAdapter::is_orderbook_message(text) {
        match GateIOAdapter::parse_orderbook_full(text) {
            Ok((currency_pair, _bid, _ask, _bid_size, _ask_size, bids, asks, is_snapshot)) => {
                // Extract symbol and quote from currency pair
                if let Some((symbol, quote)) = GateIOAdapter::extract_base_quote(&currency_pair) {
                    // Get best bid/ask
                    let (bid, bid_size) = bids.first().copied().unwrap_or((0.0, 0.0));
                    let (ask, ask_size) = asks.first().copied().unwrap_or((0.0, 0.0));

                    // Skip if no valid prices
                    if bid == 0.0 || ask == 0.0 {
                        return;
                    }

                    let mid = (bid + ask) / 2.0;

                    // Check if this is a stablecoin rate update
                    if symbol == "USDT" || symbol == "USDC" {
                        let rate_tick = ParsedTick::stablecoin_rate(
                            Exchange::GateIO,
                            &symbol,
                            &quote,
                            FixedPoint::from_f64(mid),
                        );
                        let _ = tx.try_send(rate_tick.into());
                    }

                    // Create orderbook (snapshot or delta)
                    let orderbook = if is_snapshot {
                        Orderbook::new(bids, asks)
                    } else {
                        Orderbook::delta(bids, asks)
                    };

                    let parsed = ParsedTick::price_with_orderbook(
                        Exchange::GateIO,
                        symbol,
                        quote,
                        FixedPoint::from_f64(mid),
                        FixedPoint::from_f64(bid),
                        FixedPoint::from_f64(ask),
                        FixedPoint::from_f64(bid_size),
                        FixedPoint::from_f64(ask_size),
                        orderbook,
                    );

                    let _ = tx.try_send(parsed.into());
                }
            }
            Err(_) => {
                // Orderbook parse failed - ignore silently
                // (detailed logging handled by adapter if needed)
            }
        }
    }
}

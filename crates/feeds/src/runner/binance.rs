//! Binance feed runner.
//!
//! Processes WebSocket messages from Binance and emits ParsedTick messages.

use super::{drain_channel, handle_connection_event, FeedSender};
use crate::adapter::BinanceAdapter;
use crate::message::{Orderbook, ParsedTick};
use crate::WsMessage;
use arbitrage_core::Exchange;
use tokio::sync::mpsc;
use tracing::debug;

/// Run the Binance feed processor.
///
/// Receives WebSocket messages, parses them using BinanceAdapter,
/// and sends ParsedTick messages to the handler.
pub async fn run_binance(mut rx: mpsc::Receiver<WsMessage>, tx: FeedSender) {
    debug!("Starting Binance feed runner");

    while let Some(msg) = rx.recv().await {
        // Handle connection lifecycle events
        if handle_connection_event(&msg, Exchange::Binance, &tx) {
            // On disconnect, drain stale messages
            if matches!(msg, WsMessage::Disconnected) {
                drain_channel(&mut rx);
            }
            continue;
        }

        // Process Binance-specific messages (text only)
        if let WsMessage::Text(text) = msg {
            process_text_message(&text, &tx);
        }
    }

    debug!("Binance feed runner stopped");
}

/// Process a text (JSON) message from Binance.
fn process_text_message(text: &str, tx: &FeedSender) {
    // Process partial depth stream (20 levels orderbook snapshot)
    if BinanceAdapter::is_partial_depth_message(text) {
        if let Ok((tick, symbol, quote, bids, asks)) =
            BinanceAdapter::parse_partial_depth_with_base_quote(text)
        {
            // Check if this is a stablecoin rate update
            if symbol == "USDT" || symbol == "USDC" {
                let rate_tick =
                    ParsedTick::stablecoin_rate(Exchange::Binance, &symbol, &quote, tick.price());
                let _ = tx.try_send(rate_tick.into());
            }

            // Create price tick with orderbook
            let orderbook = if !bids.is_empty() && !asks.is_empty() {
                Some(Orderbook::new(bids, asks))
            } else {
                None
            };

            let parsed = if let Some(ob) = orderbook {
                ParsedTick::price_with_orderbook(
                    Exchange::Binance,
                    symbol,
                    quote,
                    tick.price(),
                    tick.bid(),
                    tick.ask(),
                    tick.bid_size(),
                    tick.ask_size(),
                    ob,
                )
            } else {
                ParsedTick::price(
                    Exchange::Binance,
                    symbol,
                    quote,
                    tick.price(),
                    tick.bid(),
                    tick.ask(),
                    tick.bid_size(),
                    tick.ask_size(),
                )
            };

            let _ = tx.try_send(parsed.into());
        }
    }
}

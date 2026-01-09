//! Gate.io exchange feed handler.
//!
//! Processes WebSocket messages from Gate.io, including:
//! - Orderbook messages with full depth
//! - Stablecoin price tracking

use super::process_overseas_price_update;
use crate::feeds::connection::{handle_connection_event, ConnectionAction};
use crate::feeds::FeedContext;
use arbitrage_core::Exchange;
use arbitrage_feeds::{ExchangeAdapter, GateIOAdapter, WsMessage};
use tokio::sync::mpsc;
use tracing::debug;

/// Run the Gate.io feed processor.
///
/// Processes WebSocket messages from Gate.io and updates application state.
/// Only processes JSON text messages.
pub async fn run_gateio_feed(ctx: FeedContext, mut rx: mpsc::Receiver<WsMessage>) {
    debug!("Starting Gate.io live feed processor");

    while let Some(msg) = rx.recv().await {
        if !ctx.state.is_running() {
            break;
        }

        // Handle connection lifecycle events
        let state_ref = &ctx.state;
        match handle_connection_event(&msg, Exchange::GateIO, &ctx.status_notifier, || {
            state_ref.clear_exchange_caches(Exchange::GateIO);
        }) {
            ConnectionAction::Continue => continue,
            ConnectionAction::ProcessMessage => {}
        }

        // Process Gate.io-specific messages (text only)
        if let WsMessage::Text(text) = msg {
            process_text_message(&text, &ctx).await;
        }
    }

    debug!("Gate.io feed processor stopped");
}

/// Process a text (JSON) message from Gate.io.
async fn process_text_message(text: &str, ctx: &FeedContext) {
    // Process orderbook message only (depth with bid/ask sizes)
    if GateIOAdapter::is_orderbook_message(text) {
        match GateIOAdapter::parse_orderbook_full(text) {
            Ok((currency_pair, bid, ask, bid_size, ask_size, bids, asks)) => {
                // Extract base symbol and quote from currency_pair (e.g., BTC_USDT -> BTC, USDT)
                if let Some((symbol, quote)) = GateIOAdapter::extract_base_quote(&currency_pair) {
                    // Calculate mid price from bid/ask
                    let mid_price = arbitrage_core::FixedPoint::from_f64(
                        (bid.to_f64() + ask.to_f64()) / 2.0,
                    );

                    // Use canonical name if mapping exists
                    let display_symbol = ctx.symbol_mappings.canonical_name("GateIO", &symbol);
                    let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

                    // Store full orderbook for depth walking calculation
                    if !bids.is_empty() && !asks.is_empty() {
                        ctx.state
                            .update_orderbook_snapshot(Exchange::GateIO, pair_id, &bids, &asks);
                        tracing::debug!(
                            "GateIO orderbook stored: {} pair_id={} bids={} asks={}",
                            display_symbol,
                            pair_id,
                            bids.len(),
                            asks.len()
                        );
                    } else {
                        tracing::warn!(
                            "GateIO empty orderbook: {} bids={} asks={}",
                            currency_pair,
                            bids.len(),
                            asks.len()
                        );
                    }

                    // Process price update with stablecoin conversion
                    process_overseas_price_update(
                        &symbol, &quote, mid_price, bid, ask, bid_size, ask_size,
                        Exchange::GateIO, ctx,
                    )
                    .await;
                }
            }
            Err(e) => {
                // Orderbook parse failed - log first few failures for debugging
                static LOGGED: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(0);
                if LOGGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 5 {
                    tracing::warn!("GateIO parse error: {}", e);
                }
            }
        }
    }
}

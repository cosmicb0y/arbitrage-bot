//! Binance exchange feed handler.
//!
//! Processes WebSocket messages from Binance, including:
//! - Partial depth stream (20 levels orderbook snapshot)
//! - Stablecoin rate updates (USDT/USD, USDC/USDT, USDC/USD)

use super::process_overseas_price_update;
use crate::feeds::connection::{handle_connection_event, ConnectionAction};
use crate::feeds::FeedContext;
use arbitrage_core::Exchange;
use arbitrage_feeds::{BinanceAdapter, WsMessage};
use tokio::sync::mpsc;
use tracing::debug;

/// Run the Binance feed processor.
///
/// Processes WebSocket messages from Binance and updates application state.
/// Only processes JSON text messages (Binance doesn't use binary).
pub async fn run_binance_feed(ctx: FeedContext, mut rx: mpsc::Receiver<WsMessage>) {
    debug!("Starting Binance live feed processor");

    while let Some(msg) = rx.recv().await {
        if !ctx.state.is_running() {
            break;
        }

        // Handle connection lifecycle events
        let state_ref = &ctx.state;
        match handle_connection_event(&msg, Exchange::Binance, &ctx.status_notifier, || {
            state_ref.clear_exchange_caches(Exchange::Binance);
        }) {
            ConnectionAction::Continue => continue,
            ConnectionAction::ProcessMessage => {}
        }

        // Process Binance-specific messages (text only)
        if let WsMessage::Text(text) = msg {
            process_text_message(&text, &ctx).await;
        }
    }

    debug!("Binance feed processor stopped");
}

/// Process a text (JSON) message from Binance.
async fn process_text_message(text: &str, ctx: &FeedContext) {
    // Process partial depth stream (20 levels orderbook snapshot)
    if BinanceAdapter::is_partial_depth_message(text) {
        if let Ok((tick, symbol, quote, bids, asks)) =
            BinanceAdapter::parse_partial_depth_with_base_quote(text)
        {
            // Update stablecoin prices for this exchange
            if symbol == "USDT" || symbol == "USDC" {
                ctx.state.update_exchange_stablecoin_price(
                    Exchange::Binance,
                    &symbol,
                    &quote,
                    tick.price().to_f64(),
                );
                // Also update global USDT/USD if this is a direct pair
                if symbol == "USDT" && quote == "USD" {
                    ctx.state.update_usdt_usd_price(tick.price());
                }
            }

            // Use canonical name if mapping exists
            let display_symbol = ctx.symbol_mappings.canonical_name("Binance", &symbol);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

            // Store full orderbook for depth walking calculation
            if !bids.is_empty() && !asks.is_empty() {
                ctx.state
                    .update_orderbook_snapshot(Exchange::Binance, pair_id, &bids, &asks);
            }

            // Process price update with stablecoin conversion
            process_overseas_price_update(
                &symbol,
                &quote,
                tick.price(),
                tick.bid(),
                tick.ask(),
                tick.bid_size(),
                tick.ask_size(),
                Exchange::Binance,
                ctx,
            )
            .await;
        }
    }
}

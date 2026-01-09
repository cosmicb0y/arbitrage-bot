//! Bybit exchange feed handler.
//!
//! Processes WebSocket messages from Bybit, including:
//! - Orderbook snapshots (full replacement)
//! - Orderbook deltas (incremental updates)
//! - Stablecoin and reference crypto price tracking

use crate::feeds::common::convert_stablecoin_to_usd_for_exchange;
use crate::feeds::connection::{handle_connection_event, ConnectionAction};
use crate::feeds::FeedContext;
use crate::ws_server;
use arbitrage_core::{Exchange, FixedPoint, PriceTick, QuoteCurrency};
use arbitrage_feeds::{BybitAdapter, WsMessage};
use tokio::sync::mpsc;
use tracing::debug;

/// Run the Bybit feed processor.
///
/// Processes WebSocket messages from Bybit and updates application state.
/// Handles both snapshot and delta orderbook updates.
pub async fn run_bybit_feed(ctx: FeedContext, mut rx: mpsc::Receiver<WsMessage>) {
    debug!("Starting Bybit live feed processor");

    while let Some(msg) = rx.recv().await {
        if !ctx.state.is_running() {
            break;
        }

        // Handle connection lifecycle events
        let state_ref = &ctx.state;
        match handle_connection_event(&msg, Exchange::Bybit, &ctx.status_notifier, || {
            state_ref.clear_exchange_caches(Exchange::Bybit);
        }) {
            ConnectionAction::Continue => continue,
            ConnectionAction::ProcessMessage => {}
        }

        // Process Bybit-specific messages (text only)
        if let WsMessage::Text(text) = msg {
            process_text_message(&text, &ctx).await;
        }
    }

    debug!("Bybit feed processor stopped");
}

/// Process a text (JSON) message from Bybit.
async fn process_text_message(text: &str, ctx: &FeedContext) {
    // Process orderbook only (accurate bid/ask with depth)
    if BybitAdapter::is_orderbook_message(text) {
        if let Ok((_tick, symbol, quote, bids, asks, is_snapshot)) =
            BybitAdapter::parse_orderbook_full(text)
        {
            // Use canonical name if mapping exists
            let display_symbol = ctx.symbol_mappings.canonical_name("Bybit", &symbol);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

            // Handle snapshot vs delta
            if is_snapshot {
                // Full orderbook replacement
                if !bids.is_empty() && !asks.is_empty() {
                    ctx.state
                        .update_orderbook_snapshot(Exchange::Bybit, pair_id, &bids, &asks);
                }
            } else {
                // Delta update - apply changes to existing orderbook
                ctx.state
                    .apply_orderbook_delta(Exchange::Bybit, pair_id, &bids, &asks);
            }

            // Get best bid/ask from orderbook cache (always accurate after delta applied)
            if let Some((best_bid, best_ask, bid_size, ask_size)) =
                ctx.state.get_best_bid_ask(Exchange::Bybit, pair_id)
            {
                let mid = (best_bid + best_ask) / 2.0;
                let mid_fp = FixedPoint::from_f64(mid);
                let bid_fp = FixedPoint::from_f64(best_bid);
                let ask_fp = FixedPoint::from_f64(best_ask);
                let bid_size_fp = FixedPoint::from_f64(bid_size);
                let ask_size_fp = FixedPoint::from_f64(ask_size);

                // Update stablecoin prices for this exchange
                if symbol == "USDT" || symbol == "USDC" {
                    ctx.state
                        .update_exchange_stablecoin_price(Exchange::Bybit, &symbol, &quote, mid);
                }

                // Use BTC as reference crypto for deriving stablecoin rates
                if symbol == "BTC" && (quote == "USD" || quote == "USDT" || quote == "USDC") {
                    ctx.state
                        .update_exchange_ref_crypto_price(Exchange::Bybit, &quote, mid);
                }

                // Convert stablecoin prices to USD
                let mid_usd = convert_stablecoin_to_usd_for_exchange(
                    mid_fp,
                    quote_currency,
                    Exchange::Bybit,
                    &ctx.state,
                );
                let bid_usd = convert_stablecoin_to_usd_for_exchange(
                    bid_fp,
                    quote_currency,
                    Exchange::Bybit,
                    &ctx.state,
                );
                let ask_usd = convert_stablecoin_to_usd_for_exchange(
                    ask_fp,
                    quote_currency,
                    Exchange::Bybit,
                    &ctx.state,
                );

                // Update state with both USD-converted and raw prices
                ctx.state
                    .update_price_with_bid_ask_and_raw(
                        Exchange::Bybit,
                        pair_id,
                        &display_symbol,
                        mid_usd,
                        bid_usd,
                        ask_usd, // USD-normalized
                        bid_fp,
                        ask_fp, // Original USDT/USDC
                        bid_size_fp,
                        ask_size_fp,
                        quote_currency,
                    )
                    .await;

                // Broadcast to clients
                let tick = PriceTick::with_depth(
                    Exchange::Bybit,
                    pair_id,
                    mid_fp,
                    bid_fp,
                    ask_fp,
                    bid_size_fp,
                    ask_size_fp,
                    quote_currency,
                );
                ws_server::broadcast_price_with_quote(
                    &ctx.broadcast_tx,
                    Exchange::Bybit,
                    pair_id,
                    &display_symbol,
                    Some(&quote),
                    &tick,
                );
            }
        }
    }
}

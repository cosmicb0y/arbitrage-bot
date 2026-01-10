//! Gate.io exchange feed handler.
//!
//! Processes WebSocket messages from Gate.io, including:
//! - Orderbook snapshots (full replacement)
//! - Orderbook deltas (incremental updates)
//! - Stablecoin price tracking

use crate::feeds::common::convert_stablecoin_to_usd_for_exchange;
use crate::feeds::connection::{handle_connection_event, ConnectionAction};
use crate::feeds::FeedContext;
use crate::ws_server;
use arbitrage_core::{Exchange, FixedPoint, PriceTick, QuoteCurrency};
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
    // Process orderbook messages (both snapshots and deltas)
    if GateIOAdapter::is_orderbook_message(text) {
        match GateIOAdapter::parse_orderbook_full(text) {
            Ok((currency_pair, _bid, _ask, _bid_size, _ask_size, bids, asks, is_snapshot)) => {
                // Extract base symbol and quote from currency_pair (e.g., BTC_USDT -> BTC, USDT)
                if let Some((symbol, quote)) = GateIOAdapter::extract_base_quote(&currency_pair) {
                    // Use canonical name if mapping exists
                    let display_symbol = ctx.symbol_mappings.canonical_name("GateIO", &symbol);
                    let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
                    let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

                    // Handle snapshot vs delta
                    if is_snapshot {
                        // Full orderbook replacement
                        if !bids.is_empty() && !asks.is_empty() {
                            ctx.state
                                .update_orderbook_snapshot(Exchange::GateIO, pair_id, &bids, &asks);
                        }
                    } else {
                        // Delta update - apply changes to existing orderbook
                        ctx.state
                            .apply_orderbook_delta(Exchange::GateIO, pair_id, &bids, &asks);
                    }

                    // Get best bid/ask from orderbook cache (always accurate after delta applied)
                    if let Some((best_bid, best_ask, bid_size, ask_size)) =
                        ctx.state.get_best_bid_ask(Exchange::GateIO, pair_id)
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
                                .update_exchange_stablecoin_price(Exchange::GateIO, &symbol, &quote, mid);
                        }

                        // Use BTC as reference crypto for deriving stablecoin rates
                        if symbol == "BTC" && (quote == "USD" || quote == "USDT" || quote == "USDC") {
                            ctx.state
                                .update_exchange_ref_crypto_price(Exchange::GateIO, &quote, mid);
                        }

                        // Convert stablecoin prices to USD
                        let mid_usd = convert_stablecoin_to_usd_for_exchange(
                            mid_fp,
                            quote_currency,
                            Exchange::GateIO,
                            &ctx.state,
                        );
                        let bid_usd = convert_stablecoin_to_usd_for_exchange(
                            bid_fp,
                            quote_currency,
                            Exchange::GateIO,
                            &ctx.state,
                        );
                        let ask_usd = convert_stablecoin_to_usd_for_exchange(
                            ask_fp,
                            quote_currency,
                            Exchange::GateIO,
                            &ctx.state,
                        );

                        // Update state with both USD-converted and raw prices
                        ctx.state
                            .update_price_with_bid_ask_and_raw(
                                Exchange::GateIO,
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
                            Exchange::GateIO,
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
                            Exchange::GateIO,
                            pair_id,
                            &display_symbol,
                            Some(&quote),
                            &tick,
                        );
                    }
                }
            }
            Err(e) => {
                // Orderbook parse failed - log first few failures for debugging
                static LOGGED: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(0);
                if LOGGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 5 {
                    // Log truncated message for debugging
                    let msg_preview: String = text.chars().take(300).collect();
                    tracing::warn!("GateIO parse error: {} | msg: {}", e, msg_preview);
                }
            }
        }
    }
}

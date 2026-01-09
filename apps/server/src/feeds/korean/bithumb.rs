//! Bithumb exchange feed handler.
//!
//! Processes WebSocket messages from Bithumb, including:
//! - Ticker messages (price updates)
//! - Orderbook messages (bid/ask with depth)
//! - Both text (JSON) and binary formats

use super::{process_korean_orderbook, process_korean_ticker};
use crate::feeds::common::OrderbookCache;
use crate::feeds::connection::{handle_connection_event, ConnectionAction};
use crate::feeds::FeedContext;
use arbitrage_core::Exchange;
use arbitrage_feeds::{BithumbAdapter, BithumbMessage, ExchangeAdapter, WsMessage};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

/// Run the Bithumb feed processor.
///
/// Processes WebSocket messages from Bithumb and updates application state.
/// Handles both text (JSON) and binary message formats.
pub async fn run_bithumb_feed(ctx: FeedContext, mut rx: mpsc::Receiver<WsMessage>) {
    debug!("Starting Bithumb live feed processor");
    let orderbook_cache: OrderbookCache = Arc::new(dashmap::DashMap::new());

    while let Some(msg) = rx.recv().await {
        if !ctx.state.is_running() {
            break;
        }

        // Handle connection lifecycle events
        let cache_ref = &orderbook_cache;
        let state_ref = &ctx.state;
        match handle_connection_event(&msg, Exchange::Bithumb, &ctx.status_notifier, || {
            cache_ref.clear();
            state_ref.clear_exchange_caches(Exchange::Bithumb);
        }) {
            ConnectionAction::Continue => continue,
            ConnectionAction::ProcessMessage => {}
        }

        // Process Bithumb-specific messages
        match msg {
            WsMessage::Text(text) => {
                process_text_message(&text, &ctx, &orderbook_cache);
            }
            WsMessage::Binary(data) => {
                process_binary_message(&data, &ctx, &orderbook_cache);
            }
            _ => {}
        }
    }

    debug!("Bithumb feed processor stopped");
}

/// Process a text (JSON) message from Bithumb.
fn process_text_message(text: &str, ctx: &FeedContext, orderbook_cache: &OrderbookCache) {
    // Try full orderbook parse first for depth walking
    if let Ok(snapshot) = BithumbAdapter::parse_orderbook_full(text) {
        process_korean_orderbook::<BithumbAdapter>(
            &snapshot.code,
            snapshot.best_bid,
            snapshot.best_ask,
            snapshot.best_bid_size,
            snapshot.best_ask_size,
            Exchange::Bithumb,
            ctx,
            orderbook_cache,
        );

        // Store full orderbook for depth walking
        if let Some(symbol) = BithumbAdapter::extract_base_symbol(&snapshot.code) {
            let display_symbol = ctx.symbol_mappings.canonical_name("Bithumb", &symbol);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            ctx.state
                .update_orderbook_snapshot(Exchange::Bithumb, pair_id, &snapshot.bids, &snapshot.asks);
        }
    } else if let Ok(bithumb_msg) = BithumbAdapter::parse_message(text) {
        match bithumb_msg {
            BithumbMessage::Orderbook {
                code,
                bid,
                ask,
                bid_size,
                ask_size,
            } => {
                process_korean_orderbook::<BithumbAdapter>(
                    &code,
                    bid,
                    ask,
                    bid_size,
                    ask_size,
                    Exchange::Bithumb,
                    ctx,
                    orderbook_cache,
                );
            }
            BithumbMessage::Ticker { code, price } => {
                process_korean_ticker::<BithumbAdapter>(
                    &code,
                    price,
                    Exchange::Bithumb,
                    ctx,
                    orderbook_cache,
                );
            }
        }
    }
}

/// Process a binary message from Bithumb.
fn process_binary_message(data: &[u8], ctx: &FeedContext, orderbook_cache: &OrderbookCache) {
    // Try full orderbook parse first for depth walking
    match BithumbAdapter::parse_orderbook_full_binary(data) {
        Ok(snapshot) => {
            process_korean_orderbook::<BithumbAdapter>(
                &snapshot.code,
                snapshot.best_bid,
                snapshot.best_ask,
                snapshot.best_bid_size,
                snapshot.best_ask_size,
                Exchange::Bithumb,
                ctx,
                orderbook_cache,
            );

            // Store full orderbook for depth walking
            if let Some(symbol) = BithumbAdapter::extract_base_symbol(&snapshot.code) {
                let display_symbol = ctx.symbol_mappings.canonical_name("Bithumb", &symbol);
                let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
                ctx.state
                    .update_orderbook_snapshot(Exchange::Bithumb, pair_id, &snapshot.bids, &snapshot.asks);
            }
        }
        Err(_) => {
            // Not a full orderbook message - try ticker/orderbook parsing
            if let Ok(bithumb_msg) = BithumbAdapter::parse_message_binary(data) {
                match bithumb_msg {
                    BithumbMessage::Orderbook {
                        code,
                        bid,
                        ask,
                        bid_size,
                        ask_size,
                    } => {
                        process_korean_orderbook::<BithumbAdapter>(
                            &code,
                            bid,
                            ask,
                            bid_size,
                            ask_size,
                            Exchange::Bithumb,
                            ctx,
                            orderbook_cache,
                        );
                    }
                    BithumbMessage::Ticker { code, price } => {
                        process_korean_ticker::<BithumbAdapter>(
                            &code,
                            price,
                            Exchange::Bithumb,
                            ctx,
                            orderbook_cache,
                        );
                    }
                }
            }
        }
    }
}

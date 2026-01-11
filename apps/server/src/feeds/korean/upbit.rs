//! Upbit exchange feed handler.
//!
//! Processes WebSocket messages from Upbit, including:
//! - Ticker messages (price updates)
//! - Orderbook messages (bid/ask with depth)
//! - Both text (JSON) and binary (MessagePack) formats

use super::{process_korean_orderbook, process_korean_ticker};
use crate::feeds::common::OrderbookCache;
use crate::feeds::connection::{handle_connection_event, ConnectionAction};
use crate::feeds::FeedContext;
use arbitrage_core::Exchange;
use arbitrage_feeds::{ExchangeAdapter, UpbitAdapter, UpbitMessage, WsMessage};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

/// Run the Upbit feed processor.
///
/// Processes WebSocket messages from Upbit and updates application state.
/// Handles both text (JSON) and binary (MessagePack) message formats.
pub async fn run_upbit_feed(ctx: FeedContext, mut rx: mpsc::Receiver<WsMessage>) {
    debug!("Starting Upbit live feed processor");
    let orderbook_cache: OrderbookCache = Arc::new(dashmap::DashMap::new());

    while let Some(msg) = rx.recv().await {
        if !ctx.state.is_running() {
            break;
        }

        // Handle connection lifecycle events
        let cache_ref = &orderbook_cache;
        let state_ref = &ctx.state;
        match handle_connection_event(
            &msg,
            Exchange::Upbit,
            &ctx.status_notifier,
            || {
                cache_ref.clear();
                state_ref.clear_exchange_caches(Exchange::Upbit);
            },
            || {
                // On disconnect: drain stale messages and clear caches
                while rx.try_recv().is_ok() {}
                cache_ref.clear();
                state_ref.clear_exchange_caches(Exchange::Upbit);
            },
        ) {
            ConnectionAction::Continue => continue,
            ConnectionAction::ProcessMessage => {}
        }

        // Process Upbit-specific messages
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

    debug!("Upbit feed processor stopped");
}

/// Process a text (JSON) message from Upbit.
fn process_text_message(text: &str, ctx: &FeedContext, orderbook_cache: &OrderbookCache) {
    // Try full orderbook parse first for depth walking
    if let Ok((code, bid, ask, bid_size, ask_size, bids, asks)) =
        UpbitAdapter::parse_orderbook_full(text)
    {
        process_korean_orderbook::<UpbitAdapter>(
            &code,
            bid,
            ask,
            bid_size,
            ask_size,
            Exchange::Upbit,
            ctx,
            orderbook_cache,
        );

        // Store full orderbook for depth walking
        if let Some(symbol) = UpbitAdapter::extract_base_symbol(&code) {
            let display_symbol = ctx.symbol_mappings.canonical_name("Upbit", &symbol);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            ctx.state
                .update_orderbook_snapshot(Exchange::Upbit, pair_id, &bids, &asks);
        }
    } else if let Ok(upbit_msg) = UpbitAdapter::parse_message(text) {
        match upbit_msg {
            UpbitMessage::Orderbook {
                code,
                bid,
                ask,
                bid_size,
                ask_size,
            } => {
                process_korean_orderbook::<UpbitAdapter>(
                    &code,
                    bid,
                    ask,
                    bid_size,
                    ask_size,
                    Exchange::Upbit,
                    ctx,
                    orderbook_cache,
                );
            }
            UpbitMessage::Ticker { code, price } => {
                process_korean_ticker::<UpbitAdapter>(
                    &code,
                    price,
                    Exchange::Upbit,
                    ctx,
                    orderbook_cache,
                );
            }
        }
    }
}

/// Process a binary (MessagePack) message from Upbit.
fn process_binary_message(data: &[u8], ctx: &FeedContext, orderbook_cache: &OrderbookCache) {
    // Try full orderbook parse first for depth walking
    match UpbitAdapter::parse_orderbook_full_binary(data) {
        Ok((code, bid, ask, bid_size, ask_size, bids, asks)) => {
            process_korean_orderbook::<UpbitAdapter>(
                &code,
                bid,
                ask,
                bid_size,
                ask_size,
                Exchange::Upbit,
                ctx,
                orderbook_cache,
            );

            // Store full orderbook for depth walking
            if let Some(symbol) = UpbitAdapter::extract_base_symbol(&code) {
                let display_symbol = ctx.symbol_mappings.canonical_name("Upbit", &symbol);
                let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
                ctx.state
                    .update_orderbook_snapshot(Exchange::Upbit, pair_id, &bids, &asks);
            }
        }
        Err(_) => {
            // Not a full orderbook - try ticker/orderbook parsing
            if let Ok(upbit_msg) = UpbitAdapter::parse_message_binary(data) {
                match upbit_msg {
                    UpbitMessage::Orderbook {
                        code,
                        bid,
                        ask,
                        bid_size,
                        ask_size,
                    } => {
                        process_korean_orderbook::<UpbitAdapter>(
                            &code,
                            bid,
                            ask,
                            bid_size,
                            ask_size,
                            Exchange::Upbit,
                            ctx,
                            orderbook_cache,
                        );
                    }
                    UpbitMessage::Ticker { code, price } => {
                        process_korean_ticker::<UpbitAdapter>(
                            &code,
                            price,
                            Exchange::Upbit,
                            ctx,
                            orderbook_cache,
                        );
                    }
                }
            }
        }
    }
}

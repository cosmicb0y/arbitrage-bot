//! Upbit feed runner.
//!
//! Processes WebSocket messages from Upbit and emits ParsedTick messages.
//! Handles both text (JSON) and binary (MessagePack) message formats.
//! Maintains orderbook cache for ticker correlation.

use super::{drain_channel, handle_connection_event, FeedSender};
use crate::adapter::{ExchangeAdapter, KoreanExchangeAdapter, UpbitAdapter, UpbitMessage};
use crate::message::{Orderbook, ParsedTick};
use crate::WsMessage;
use arbitrage_core::{Exchange, FixedPoint};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

/// Orderbook cache: code -> (bid, ask, bid_size, ask_size)
type OrderbookCache = Arc<DashMap<String, (FixedPoint, FixedPoint, FixedPoint, FixedPoint)>>;

/// Run the Upbit feed processor.
///
/// Receives WebSocket messages, parses them using UpbitAdapter,
/// and sends ParsedTick messages to the handler.
pub async fn run_upbit(mut rx: mpsc::Receiver<WsMessage>, tx: FeedSender) {
    debug!("Starting Upbit feed runner");

    let orderbook_cache: OrderbookCache = Arc::new(DashMap::new());

    while let Some(msg) = rx.recv().await {
        // Handle connection lifecycle events
        if handle_connection_event(&msg, Exchange::Upbit, &tx) {
            // On disconnect or reconnect, clear orderbook cache
            if matches!(msg, WsMessage::Disconnected | WsMessage::Reconnected) {
                orderbook_cache.clear();
                if matches!(msg, WsMessage::Disconnected) {
                    drain_channel(&mut rx);
                }
            }
            continue;
        }

        // Process Upbit-specific messages
        match msg {
            WsMessage::Text(text) => {
                process_text_message(&text, &tx, &orderbook_cache);
            }
            WsMessage::Binary(data) => {
                process_binary_message(&data, &tx, &orderbook_cache);
            }
            _ => {}
        }
    }

    debug!("Upbit feed runner stopped");
}

/// Process a text (JSON) message from Upbit.
fn process_text_message(text: &str, tx: &FeedSender, orderbook_cache: &OrderbookCache) {
    // Try full orderbook parse first for depth walking
    if let Ok((code, bid, ask, bid_size, ask_size, bids, asks)) =
        UpbitAdapter::parse_orderbook_full(text)
    {
        process_orderbook(
            &code,
            bid,
            ask,
            bid_size,
            ask_size,
            Some((bids, asks)),
            tx,
            orderbook_cache,
        );
    } else if let Ok(upbit_msg) = UpbitAdapter::parse_message(text) {
        match upbit_msg {
            UpbitMessage::Orderbook {
                code,
                bid,
                ask,
                bid_size,
                ask_size,
            } => {
                process_orderbook(
                    &code,
                    bid,
                    ask,
                    bid_size,
                    ask_size,
                    None,
                    tx,
                    orderbook_cache,
                );
            }
            UpbitMessage::Ticker { code, price } => {
                process_ticker(&code, price, tx, orderbook_cache);
            }
        }
    }
}

/// Process a binary (MessagePack) message from Upbit.
fn process_binary_message(data: &[u8], tx: &FeedSender, orderbook_cache: &OrderbookCache) {
    // Try full orderbook parse first for depth walking
    match UpbitAdapter::parse_orderbook_full_binary(data) {
        Ok((code, bid, ask, bid_size, ask_size, bids, asks)) => {
            process_orderbook(
                &code,
                bid,
                ask,
                bid_size,
                ask_size,
                Some((bids, asks)),
                tx,
                orderbook_cache,
            );
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
                        process_orderbook(
                            &code,
                            bid,
                            ask,
                            bid_size,
                            ask_size,
                            None,
                            tx,
                            orderbook_cache,
                        );
                    }
                    UpbitMessage::Ticker { code, price } => {
                        process_ticker(&code, price, tx, orderbook_cache);
                    }
                }
            }
        }
    }
}

/// Process orderbook message.
fn process_orderbook(
    code: &str,
    bid: FixedPoint,
    ask: FixedPoint,
    bid_size: FixedPoint,
    ask_size: FixedPoint,
    full_orderbook: Option<(Vec<(f64, f64)>, Vec<(f64, f64)>)>,
    tx: &FeedSender,
    orderbook_cache: &OrderbookCache,
) {
    // Handle USDT/KRW rate from orderbook (mid price)
    if UpbitAdapter::is_usdt_market(code) {
        let mid = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);
        let rate_tick = ParsedTick::stablecoin_rate(Exchange::Upbit, "USDT", "KRW", mid);
        let _ = tx.try_send(rate_tick.into());
        return;
    }

    // Handle USDC/KRW rate from orderbook (mid price)
    if UpbitAdapter::is_usdc_market(code) {
        let mid = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);
        let rate_tick = ParsedTick::stablecoin_rate(Exchange::Upbit, "USDC", "KRW", mid);
        let _ = tx.try_send(rate_tick.into());
        return;
    }

    // Store in cache for ticker handler
    orderbook_cache.insert(code.to_string(), (bid, ask, bid_size, ask_size));

    // Extract symbol from code
    if let Some(symbol) = UpbitAdapter::extract_base_symbol(code) {
        let mid = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

        // Create orderbook if full data available
        let orderbook = full_orderbook.map(|(bids, asks)| Orderbook::new(bids, asks));

        let parsed = if let Some(ob) = orderbook {
            ParsedTick::price_with_orderbook(
                Exchange::Upbit,
                symbol,
                "KRW",
                mid,
                bid,
                ask,
                bid_size,
                ask_size,
                ob,
            )
        } else {
            ParsedTick::price(
                Exchange::Upbit,
                symbol,
                "KRW",
                mid,
                bid,
                ask,
                bid_size,
                ask_size,
            )
        };

        let _ = tx.try_send(parsed.into());
    }
}

/// Process ticker message.
fn process_ticker(
    code: &str,
    price: FixedPoint,
    tx: &FeedSender,
    orderbook_cache: &OrderbookCache,
) {
    // Handle USDT/KRW for exchange rate
    if UpbitAdapter::is_usdt_market(code) {
        let rate_tick = ParsedTick::stablecoin_rate(Exchange::Upbit, "USDT", "KRW", price);
        let _ = tx.try_send(rate_tick.into());
        return;
    }

    // Handle USDC/KRW for exchange rate
    if UpbitAdapter::is_usdc_market(code) {
        let rate_tick = ParsedTick::stablecoin_rate(Exchange::Upbit, "USDC", "KRW", price);
        let _ = tx.try_send(rate_tick.into());
        return;
    }

    // Extract symbol from code
    if let Some(symbol) = UpbitAdapter::extract_base_symbol(code) {
        // Get bid/ask from orderbook cache, default to price if not available
        let (bid, ask, bid_size, ask_size) =
            orderbook_cache.get(code).map(|r| *r.value()).unwrap_or((
                price,
                price,
                FixedPoint::from_f64(0.0),
                FixedPoint::from_f64(0.0),
            ));

        let parsed = ParsedTick::price(
            Exchange::Upbit,
            symbol,
            "KRW",
            price,
            bid,
            ask,
            bid_size,
            ask_size,
        );

        let _ = tx.try_send(parsed.into());
    }
}

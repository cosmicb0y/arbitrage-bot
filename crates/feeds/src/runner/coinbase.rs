//! Coinbase feed runner.
//!
//! Processes WebSocket messages from Coinbase and emits ParsedTick messages.
//! Maintains full orderbook state using BTreeMap for efficient best bid/ask lookup.

use super::{drain_channel, handle_connection_event, FeedSender};
use crate::adapter::{CoinbaseAdapter, CoinbaseL2Event, ExchangeAdapter};
use crate::message::{Orderbook, ParsedTick};
use crate::WsMessage;
use arbitrage_core::{Exchange, FixedPoint};
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::mpsc;
use tracing::debug;

/// Coinbase orderbook structure.
/// Uses BTreeMap for sorted price levels:
/// - Bids: Reverse<i64> key to get highest price first
/// - Asks: i64 key to get lowest price first
type CoinbaseOrderbook = (BTreeMap<Reverse<i64>, f64>, BTreeMap<i64, f64>);

/// Convert price to i64 key (multiply by 1e8 for precision)
fn price_to_key(price: f64) -> i64 {
    (price * 100_000_000.0) as i64
}

/// Run the Coinbase feed processor.
///
/// Receives WebSocket messages, parses them using CoinbaseAdapter,
/// and sends ParsedTick messages to the handler.
pub async fn run_coinbase(mut rx: mpsc::Receiver<WsMessage>, tx: FeedSender) {
    debug!("Starting Coinbase feed runner");

    // Full orderbook cache for Coinbase
    let mut orderbook_cache: HashMap<String, CoinbaseOrderbook> = HashMap::new();

    while let Some(msg) = rx.recv().await {
        // Handle connection lifecycle events
        if handle_connection_event(&msg, Exchange::Coinbase, &tx) {
            // On disconnect or reconnect, clear orderbook cache
            if matches!(msg, WsMessage::Disconnected | WsMessage::Reconnected) {
                orderbook_cache.clear();
                if matches!(msg, WsMessage::Disconnected) {
                    drain_channel(&mut rx);
                }
            }
            continue;
        }

        // Process Coinbase-specific messages (text only)
        if let WsMessage::Text(text) = msg {
            process_text_message(&text, &tx, &mut orderbook_cache);
        }
    }

    debug!("Coinbase feed runner stopped");
}

/// Process a text (JSON) message from Coinbase.
fn process_text_message(
    text: &str,
    tx: &FeedSender,
    orderbook_cache: &mut HashMap<String, CoinbaseOrderbook>,
) {
    // Process level2 messages
    if let Ok(l2_event) = CoinbaseAdapter::parse_l2_event(text) {
        match l2_event {
            CoinbaseL2Event::Snapshot {
                product_id,
                bids,
                asks,
            } => {
                process_snapshot(&product_id, bids, asks, tx, orderbook_cache);
            }
            CoinbaseL2Event::Update {
                product_id,
                changes,
            } => {
                process_update(&product_id, changes, tx, orderbook_cache);
            }
        }
    }
}

/// Process a Level 2 snapshot (full orderbook replacement).
fn process_snapshot(
    product_id: &str,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
    tx: &FeedSender,
    orderbook_cache: &mut HashMap<String, CoinbaseOrderbook>,
) {
    // Build full orderbook from snapshot
    let mut bid_map: BTreeMap<Reverse<i64>, f64> = BTreeMap::new();
    let mut ask_map: BTreeMap<i64, f64> = BTreeMap::new();

    for (price, size) in &bids {
        if *size > 0.0 {
            bid_map.insert(Reverse(price_to_key(*price)), *size);
        }
    }
    for (price, size) in &asks {
        if *size > 0.0 {
            ask_map.insert(price_to_key(*price), *size);
        }
    }

    // Store in cache
    orderbook_cache.insert(product_id.to_string(), (bid_map.clone(), ask_map.clone()));

    // Get best bid/ask
    let (best_bid, bid_size) = bid_map
        .iter()
        .next()
        .map(|(Reverse(k), v)| (*k as f64 / 100_000_000.0, *v))
        .unwrap_or((0.0, 0.0));
    let (best_ask, ask_size) = ask_map
        .iter()
        .next()
        .map(|(k, v)| (*k as f64 / 100_000_000.0, *v))
        .unwrap_or((0.0, 0.0));

    if best_bid > 0.0 && best_ask > 0.0 {
        // Convert BTreeMap to Vec for orderbook
        let bids_vec: Vec<(f64, f64)> = bid_map
            .iter()
            .map(|(Reverse(k), v)| (*k as f64 / 100_000_000.0, *v))
            .collect();
        let asks_vec: Vec<(f64, f64)> = ask_map
            .iter()
            .map(|(k, v)| (*k as f64 / 100_000_000.0, *v))
            .collect();

        emit_price_tick(
            product_id, best_bid, best_ask, bid_size, ask_size, bids_vec, asks_vec,
            true, // is_snapshot
            tx,
        );
    }
}

/// Process a Level 2 update (incremental changes).
fn process_update(
    product_id: &str,
    changes: Vec<(String, f64, f64)>,
    tx: &FeedSender,
    orderbook_cache: &mut HashMap<String, CoinbaseOrderbook>,
) {
    if let Some((bid_map, ask_map)) = orderbook_cache.get_mut(product_id) {
        // Apply incremental updates
        for (side, price, size) in changes {
            let key = price_to_key(price);
            if side == "buy" {
                if size > 0.0 {
                    bid_map.insert(Reverse(key), size);
                } else {
                    bid_map.remove(&Reverse(key));
                }
            } else if side == "sell" {
                if size > 0.0 {
                    ask_map.insert(key, size);
                } else {
                    ask_map.remove(&key);
                }
            }
        }

        // Get updated best bid/ask
        let (best_bid, bid_size) = bid_map
            .iter()
            .next()
            .map(|(Reverse(k), v)| (*k as f64 / 100_000_000.0, *v))
            .unwrap_or((0.0, 0.0));
        let (best_ask, ask_size) = ask_map
            .iter()
            .next()
            .map(|(k, v)| (*k as f64 / 100_000_000.0, *v))
            .unwrap_or((0.0, 0.0));

        if best_bid > 0.0 && best_ask > 0.0 {
            // Convert BTreeMap to Vec for orderbook
            let bids_vec: Vec<(f64, f64)> = bid_map
                .iter()
                .map(|(Reverse(k), v)| (*k as f64 / 100_000_000.0, *v))
                .collect();
            let asks_vec: Vec<(f64, f64)> = ask_map
                .iter()
                .map(|(k, v)| (*k as f64 / 100_000_000.0, *v))
                .collect();

            emit_price_tick(
                product_id, best_bid, best_ask, bid_size, ask_size, bids_vec, asks_vec,
                false, // is_snapshot (this is a delta-applied update)
                tx,
            );
        }
    }
}

/// Emit a price tick to the handler.
#[allow(clippy::too_many_arguments)]
fn emit_price_tick(
    product_id: &str,
    bid: f64,
    ask: f64,
    bid_size: f64,
    ask_size: f64,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
    is_snapshot: bool,
    tx: &FeedSender,
) {
    if let Some((symbol, quote)) = CoinbaseAdapter::extract_base_quote(product_id) {
        let mid = (bid + ask) / 2.0;

        // Check if this is a stablecoin rate update
        if symbol == "USDT" || symbol == "USDC" {
            let rate_tick = ParsedTick::stablecoin_rate(
                Exchange::Coinbase,
                &symbol,
                &quote,
                FixedPoint::from_f64(mid),
            );
            let _ = tx.try_send(rate_tick.into());
        }

        // Create orderbook
        let orderbook = if is_snapshot {
            Orderbook::new(bids, asks)
        } else {
            // For updates, we send the full current orderbook as snapshot
            // since we've already applied the delta
            Orderbook::new(bids, asks)
        };

        let parsed = ParsedTick::price_with_orderbook(
            Exchange::Coinbase,
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

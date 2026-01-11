//! Coinbase exchange feed handler.
//!
//! Processes WebSocket messages from Coinbase, including:
//! - Level 2 snapshots (full orderbook)
//! - Level 2 updates (incremental changes)
//! - Uses BTreeMap for efficient sorted orderbook maintenance

use crate::feeds::common::convert_stablecoin_to_usd_for_exchange;
use crate::feeds::connection::{handle_connection_event, ConnectionAction};
use crate::feeds::FeedContext;
use crate::ws_server;
use arbitrage_core::{Exchange, FixedPoint, PriceTick, QuoteCurrency};
use arbitrage_feeds::{CoinbaseAdapter, CoinbaseL2Event, ExchangeAdapter, WsMessage};
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
/// Processes WebSocket messages from Coinbase and updates application state.
/// Maintains a full orderbook using BTreeMap for efficient best bid/ask lookup.
pub async fn run_coinbase_feed(ctx: FeedContext, mut rx: mpsc::Receiver<WsMessage>) {
    debug!("Starting Coinbase live feed processor");

    // Full orderbook cache for Coinbase
    let mut orderbook_cache: HashMap<String, CoinbaseOrderbook> = HashMap::new();

    while let Some(msg) = rx.recv().await {
        if !ctx.state.is_running() {
            break;
        }

        // Handle connection lifecycle events
        let state_ref = &ctx.state;
        let action = handle_connection_event(
            &msg,
            Exchange::Coinbase,
            &ctx.status_notifier,
            || state_ref.clear_exchange_caches(Exchange::Coinbase),
            || {
                // On disconnect: clear caches (drain happens after)
                state_ref.clear_exchange_caches(Exchange::Coinbase);
            },
        );

        // Handle cache clearing and message drain for Coinbase (uses local HashMap)
        match &msg {
            WsMessage::Reconnected | WsMessage::Disconnected => {
                orderbook_cache.clear();
                if matches!(msg, WsMessage::Disconnected) {
                    while rx.try_recv().is_ok() {}
                }
            }
            _ => {}
        }

        match action {
            ConnectionAction::Continue => continue,
            ConnectionAction::ProcessMessage => {}
        }

        // Process Coinbase-specific messages (text only)
        if let WsMessage::Text(text) = msg {
            process_text_message(&text, &ctx, &mut orderbook_cache).await;
        }
    }

    debug!("Coinbase feed processor stopped");
}

/// Process a text (JSON) message from Coinbase.
async fn process_text_message(
    text: &str,
    ctx: &FeedContext,
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
                process_snapshot(&product_id, bids, asks, ctx, orderbook_cache).await;
            }
            CoinbaseL2Event::Update {
                product_id,
                changes,
            } => {
                process_update(&product_id, changes, ctx, orderbook_cache).await;
            }
        }
    }
}

/// Process a Level 2 snapshot (full orderbook replacement).
async fn process_snapshot(
    product_id: &str,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
    ctx: &FeedContext,
    orderbook_cache: &mut HashMap<String, CoinbaseOrderbook>,
) {
    // Build full orderbook from snapshot
    let mut bid_map: BTreeMap<Reverse<i64>, f64> = BTreeMap::new();
    let mut ask_map: BTreeMap<i64, f64> = BTreeMap::new();

    for (price, size) in bids {
        if size > 0.0 {
            bid_map.insert(Reverse(price_to_key(price)), size);
        }
    }
    for (price, size) in asks {
        if size > 0.0 {
            ask_map.insert(price_to_key(price), size);
        }
    }

    // Get best bid (first in Reverse-sorted map) and best ask (first in sorted map)
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

    debug!(
        "Coinbase snapshot: {} bid={:.4} ask={:.4} (levels: {} bids, {} asks)",
        product_id,
        best_bid,
        best_ask,
        bid_map.len(),
        ask_map.len()
    );

    // Store in cache
    orderbook_cache.insert(product_id.to_string(), (bid_map.clone(), ask_map.clone()));

    // Store full orderbook for depth walking calculation
    let bids_vec: Vec<(f64, f64)> = bid_map
        .iter()
        .map(|(Reverse(k), v)| (*k as f64 / 100_000_000.0, *v))
        .collect();
    let asks_vec: Vec<(f64, f64)> = ask_map
        .iter()
        .map(|(k, v)| (*k as f64 / 100_000_000.0, *v))
        .collect();

    if let Some(base) = CoinbaseAdapter::extract_base_symbol(product_id) {
        let pair_id = arbitrage_core::symbol_to_pair_id(&base);
        ctx.state
            .update_orderbook_snapshot(Exchange::Coinbase, pair_id, &bids_vec, &asks_vec);
    }

    // Process orderbook update
    let bid = FixedPoint::from_f64(best_bid);
    let ask = FixedPoint::from_f64(best_ask);
    let bid_sz = FixedPoint::from_f64(bid_size);
    let ask_sz = FixedPoint::from_f64(ask_size);
    process_coinbase_orderbook(product_id, bid, ask, bid_sz, ask_sz, ctx).await;
}

/// Process a Level 2 update (incremental changes).
async fn process_update(
    product_id: &str,
    changes: Vec<(String, f64, f64)>,
    ctx: &FeedContext,
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
                    // size == 0 means remove this level
                    bid_map.remove(&Reverse(key));
                }
            } else if side == "sell" {
                if size > 0.0 {
                    ask_map.insert(key, size);
                } else {
                    // size == 0 means remove this level
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
            // Update full orderbook for depth walking
            let bids_vec: Vec<(f64, f64)> = bid_map
                .iter()
                .map(|(Reverse(k), v)| (*k as f64 / 100_000_000.0, *v))
                .collect();
            let asks_vec: Vec<(f64, f64)> = ask_map
                .iter()
                .map(|(k, v)| (*k as f64 / 100_000_000.0, *v))
                .collect();

            if let Some(base) = CoinbaseAdapter::extract_base_symbol(product_id) {
                let pair_id = arbitrage_core::symbol_to_pair_id(&base);
                ctx.state
                    .update_orderbook_snapshot(Exchange::Coinbase, pair_id, &bids_vec, &asks_vec);
            }

            let bid = FixedPoint::from_f64(best_bid);
            let ask = FixedPoint::from_f64(best_ask);
            let bid_sz = FixedPoint::from_f64(bid_size);
            let ask_sz = FixedPoint::from_f64(ask_size);
            process_coinbase_orderbook(product_id, bid, ask, bid_sz, ask_sz, ctx).await;
        }
    }
}

/// Process Coinbase orderbook update and broadcast.
async fn process_coinbase_orderbook(
    product_id: &str,
    bid: FixedPoint,
    ask: FixedPoint,
    bid_size: FixedPoint,
    ask_size: FixedPoint,
    ctx: &FeedContext,
) {
    // Extract base symbol and quote from product_id (e.g., BTC-USD -> BTC, USD)
    if let Some((symbol, quote)) = CoinbaseAdapter::extract_base_quote(product_id) {
        // Calculate mid price from bid/ask
        let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

        // Update stablecoin prices for this exchange
        if symbol == "USDT" || symbol == "USDC" {
            ctx.state.update_exchange_stablecoin_price(
                Exchange::Coinbase,
                &symbol,
                &quote,
                mid_price.to_f64(),
            );
        }

        // Use canonical name if mapping exists
        let display_symbol = ctx.symbol_mappings.canonical_name("Coinbase", &symbol);
        let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

        // Treat Coinbase USD as USDC for crypto assets (native USD markets behave like USDC)
        // But keep USD for stablecoin pairs like USDT-USD
        let normalized_quote = if quote == "USD" && symbol != "USDT" && symbol != "USDC" {
            "USDC".to_string()
        } else {
            quote
        };
        let quote_currency =
            QuoteCurrency::from_str(&normalized_quote).unwrap_or(QuoteCurrency::USDC);

        // Convert stablecoin prices to USD (USDC quote needs conversion)
        let mid_usd = convert_stablecoin_to_usd_for_exchange(
            mid_price,
            quote_currency,
            Exchange::Coinbase,
            &ctx.state,
        );
        let bid_usd =
            convert_stablecoin_to_usd_for_exchange(bid, quote_currency, Exchange::Coinbase, &ctx.state);
        let ask_usd =
            convert_stablecoin_to_usd_for_exchange(ask, quote_currency, Exchange::Coinbase, &ctx.state);

        let tick = PriceTick::new(Exchange::Coinbase, pair_id, mid_price, bid, ask)
            .with_sizes(bid_size, ask_size);

        // Update state with both USD-converted and raw prices
        ctx.state
            .update_price_with_bid_ask_and_raw(
                Exchange::Coinbase,
                pair_id,
                &display_symbol,
                mid_usd,
                bid_usd,
                ask_usd, // USD-normalized
                bid,
                ask, // Original USD/USDC
                bid_size,
                ask_size,
                quote_currency,
            )
            .await;

        ws_server::broadcast_price_with_quote(
            &ctx.broadcast_tx,
            Exchange::Coinbase,
            pair_id,
            &display_symbol,
            Some(&normalized_quote),
            &tick,
        );
    }
}

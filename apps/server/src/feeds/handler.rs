//! Common feed handler that processes FeedMessage from all exchanges.
//!
//! This handler receives ParsedTick and ConnectionEvent from feed runners
//! and performs application-level operations:
//! - Currency conversions (KRW→USD, stablecoin normalization)
//! - State updates
//! - Broadcasting to WebSocket clients
//! - Status notifications

use super::common::{convert_krw_to_usd_for_exchange, convert_stablecoin_to_usd_for_exchange};
use super::FeedContext;
use crate::status_notifier::StatusEvent;
use crate::ws_server;
use arbitrage_core::{symbol_to_pair_id, Exchange, FixedPoint, PriceTick, QuoteCurrency};
use arbitrage_feeds::{ConnectionEvent, FeedMessage, ParsedTick};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Run the common feed handler.
///
/// Receives FeedMessage from all exchange runners and processes them.
pub async fn run_feed_handler(mut rx: mpsc::Receiver<FeedMessage>, ctx: FeedContext) {
    debug!("Starting common feed handler");

    while let Some(msg) = rx.recv().await {
        if !ctx.state.is_running() {
            break;
        }

        match msg {
            FeedMessage::Tick(tick) => {
                process_tick(tick, &ctx).await;
            }
            FeedMessage::Event(event) => {
                process_event(event, &ctx);
            }
        }
    }

    debug!("Common feed handler stopped");
}

/// Process a parsed tick.
async fn process_tick(tick: ParsedTick, ctx: &FeedContext) {
    match tick {
        ParsedTick::Price {
            exchange,
            symbol,
            quote,
            mid,
            bid,
            ask,
            bid_size,
            ask_size,
            orderbook,
        } => {
            process_price_tick(
                exchange, &symbol, &quote, mid, bid, ask, bid_size, ask_size, orderbook, ctx,
            )
            .await;
        }
        ParsedTick::StablecoinRate {
            exchange,
            stablecoin,
            quote,
            rate,
        } => {
            process_stablecoin_rate(exchange, &stablecoin, &quote, rate, ctx);
        }
    }
}

/// Process a price tick update.
#[allow(clippy::too_many_arguments)]
async fn process_price_tick(
    exchange: Exchange,
    symbol: &str,
    quote: &str,
    mid: FixedPoint,
    bid: FixedPoint,
    ask: FixedPoint,
    bid_size: FixedPoint,
    ask_size: FixedPoint,
    orderbook: Option<arbitrage_feeds::Orderbook>,
    ctx: &FeedContext,
) {
    // Get exchange name for symbol mapping
    let exchange_name = match exchange {
        Exchange::Binance => "Binance",
        Exchange::Coinbase => "Coinbase",
        Exchange::Bybit => "Bybit",
        Exchange::GateIO => "GateIO",
        Exchange::Upbit => "Upbit",
        Exchange::Bithumb => "Bithumb",
        _ => return, // Unsupported exchange
    };

    // Use canonical name if mapping exists
    let display_symbol = ctx.symbol_mappings.canonical_name(exchange_name, symbol);
    let pair_id = symbol_to_pair_id(&display_symbol);
    let quote_currency = QuoteCurrency::from_str(quote).unwrap_or(QuoteCurrency::USD);

    // Convert prices to USD based on quote currency
    let (mid_usd, bid_usd, ask_usd) = if quote == "KRW" {
        // Korean exchanges: KRW → USD conversion
        match convert_krw_to_usd_for_exchange(mid, exchange, &ctx.state) {
            Some(mid_converted) => {
                let bid_converted =
                    convert_krw_to_usd_for_exchange(bid, exchange, &ctx.state).unwrap_or(mid_converted);
                let ask_converted =
                    convert_krw_to_usd_for_exchange(ask, exchange, &ctx.state).unwrap_or(mid_converted);
                (mid_converted, bid_converted, ask_converted)
            }
            None => {
                // No exchange rate yet - skip update
                debug!(
                    "{:?}: Skipping {} - no KRW exchange rate yet",
                    exchange, display_symbol
                );
                return;
            }
        }
    } else {
        // Overseas exchanges: stablecoin → USD conversion
        let mid_usd = convert_stablecoin_to_usd_for_exchange(mid, quote_currency, exchange, &ctx.state);
        let bid_usd = convert_stablecoin_to_usd_for_exchange(bid, quote_currency, exchange, &ctx.state);
        let ask_usd = convert_stablecoin_to_usd_for_exchange(ask, quote_currency, exchange, &ctx.state);
        (mid_usd, bid_usd, ask_usd)
    };

    // Update orderbook snapshot if available
    if let Some(ref ob) = orderbook {
        if ob.is_snapshot {
            // Full snapshot - replace orderbook
            if !ob.bids.is_empty() && !ob.asks.is_empty() {
                ctx.state
                    .update_orderbook_snapshot(exchange, pair_id, &ob.bids, &ob.asks);
            }
        } else {
            // Delta update - apply changes
            ctx.state
                .apply_orderbook_delta(exchange, pair_id, &ob.bids, &ob.asks);
        }
    }

    // Update state with both USD-converted and raw prices
    ctx.state
        .update_price_with_bid_ask_and_raw(
            exchange,
            pair_id,
            &display_symbol,
            mid_usd,
            bid_usd,
            ask_usd, // USD-normalized
            bid,
            ask, // Original quote currency
            bid_size,
            ask_size,
            quote_currency,
        )
        .await;

    // Broadcast to connected clients
    // Use original quote currency prices (KRW, USDT, USDC, USD)
    // Include USD-converted prices for cross-currency comparison
    let tick = PriceTick::with_depth(
        exchange,
        pair_id,
        mid,
        bid,
        ask,
        bid_size,
        ask_size,
        quote_currency,
    );
    ws_server::broadcast_price_with_quote_and_usd(
        &ctx.broadcast_tx,
        exchange,
        pair_id,
        &display_symbol,
        Some(quote),
        &tick,
        Some(mid_usd.to_f64()),
        Some(bid_usd.to_f64()),
        Some(ask_usd.to_f64()),
    );
}

/// Process a stablecoin rate update.
fn process_stablecoin_rate(
    exchange: Exchange,
    stablecoin: &str,
    quote: &str,
    rate: FixedPoint,
    ctx: &FeedContext,
) {
    let rate_f64 = rate.to_f64();

    // Update exchange-specific stablecoin price
    ctx.state
        .update_exchange_stablecoin_price(exchange, stablecoin, quote, rate_f64);

    // Handle specific rate updates
    match (stablecoin, quote) {
        ("USDT", "USD") => {
            ctx.state.update_usdt_usd_price(rate);
        }
        ("USDT", "KRW") => {
            match exchange {
                Exchange::Upbit => ctx.state.update_upbit_usdt_krw(rate),
                Exchange::Bithumb => ctx.state.update_bithumb_usdt_krw(rate),
                _ => {}
            }
            // Broadcast exchange rate
            ws_server::broadcast_exchange_rate(&ctx.broadcast_tx, &ctx.state, rate_f64);
            debug!("Updated {:?} USDT/KRW rate: {:.2}", exchange, rate_f64);
        }
        ("USDC", "KRW") => {
            match exchange {
                Exchange::Upbit => ctx.state.update_upbit_usdc_krw(rate),
                Exchange::Bithumb => ctx.state.update_bithumb_usdc_krw(rate),
                _ => {}
            }
            debug!("Updated {:?} USDC/KRW rate: {:.2}", exchange, rate_f64);
        }
        _ => {}
    }
}

/// Process a connection event.
fn process_event(event: ConnectionEvent, ctx: &FeedContext) {
    match event {
        ConnectionEvent::Connected(exchange) => {
            debug!("{:?}: Connected to WebSocket", exchange);
            if let Some(ref notifier) = ctx.status_notifier {
                notifier.try_send(StatusEvent::Connected(exchange));
            }
        }
        ConnectionEvent::Reconnected(exchange) => {
            info!("{:?}: Reconnected - clearing cached data", exchange);
            ctx.state.clear_exchange_caches(exchange);
            if let Some(ref notifier) = ctx.status_notifier {
                notifier.try_send(StatusEvent::Reconnected(exchange));
            }
        }
        ConnectionEvent::Disconnected(exchange) => {
            warn!("{:?}: Disconnected - clearing caches", exchange);
            ctx.state.clear_exchange_caches(exchange);
            if let Some(ref notifier) = ctx.status_notifier {
                notifier.try_send(StatusEvent::Disconnected(exchange));
            }
        }
        ConnectionEvent::CircuitBreakerOpen(exchange, wait_time) => {
            warn!(
                "{:?}: Circuit breaker OPEN - connection blocked for {:?}",
                exchange, wait_time
            );
            if let Some(ref notifier) = ctx.status_notifier {
                notifier.try_send(StatusEvent::CircuitBreakerOpen(exchange, wait_time));
            }
        }
        ConnectionEvent::Error(exchange, error) => {
            warn!("{:?}: Error - {}", exchange, error);
        }
    }
}

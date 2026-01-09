//! Korean exchange feed handlers (Upbit, Bithumb).
//!
//! Korean exchanges share common characteristics:
//! - Prices are denominated in KRW
//! - Use USDT/KRW rate for USD conversion
//! - Support both ticker and orderbook messages
//! - Handle both text and binary WebSocket messages

pub mod bithumb;
pub mod upbit;

use super::common::{convert_krw_to_usd_for_exchange, OrderbookCache};
use super::FeedContext;
use crate::ws_server;
use arbitrage_core::{Exchange, FixedPoint, PriceTick, QuoteCurrency};
use arbitrage_feeds::KoreanExchangeAdapter;

/// Process ticker message for Korean exchanges (Upbit, Bithumb).
///
/// Handles:
/// - USDT/KRW rate updates for exchange rate calculation
/// - USDC/KRW rate updates
/// - Regular trading pairs with KRW->USD conversion
pub fn process_korean_ticker<A: KoreanExchangeAdapter>(
    code: &str,
    price: FixedPoint,
    exchange: Exchange,
    ctx: &FeedContext,
    orderbook_cache: &OrderbookCache,
) {
    // Handle USDT/KRW for exchange rate
    if A::is_usdt_market(code) {
        match exchange {
            Exchange::Upbit => ctx.state.update_upbit_usdt_krw(price),
            Exchange::Bithumb => ctx.state.update_bithumb_usdt_krw(price),
            _ => {}
        }
        let rate = price.to_f64();
        ws_server::broadcast_exchange_rate(&ctx.broadcast_tx, &ctx.state, rate);
        tracing::debug!("Updated {:?} USDT/KRW rate: {:.2}", exchange, rate);
        return;
    }

    // Handle USDC/KRW for exchange rate
    if A::is_usdc_market(code) {
        match exchange {
            Exchange::Upbit => ctx.state.update_upbit_usdc_krw(price),
            Exchange::Bithumb => ctx.state.update_bithumb_usdc_krw(price),
            _ => {}
        }
        tracing::debug!(
            "Updated {:?} USDC/KRW rate: {:.2}",
            exchange,
            price.to_f64()
        );
        return;
    }

    // Handle trading pairs - extract symbol from code (e.g., "KRW-BTC" -> "BTC")
    if let Some(symbol) = A::extract_base_symbol(code) {
        // Use canonical name if mapping exists
        let exchange_name = match exchange {
            Exchange::Upbit => "Upbit",
            Exchange::Bithumb => "Bithumb",
            _ => return,
        };
        let display_symbol = ctx.symbol_mappings.canonical_name(exchange_name, &symbol);
        let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

        // Get bid/ask and sizes from orderbook cache (in KRW), default to price if not available
        let (bid_krw, ask_krw, bid_size, ask_size) = orderbook_cache
            .get(code)
            .map(|r| *r.value())
            .unwrap_or((
                price,
                price,
                FixedPoint::from_f64(0.0),
                FixedPoint::from_f64(0.0),
            ));

        // Convert KRW to USD using exchange's USDT/KRW rate
        // Skip if exchange rate is not available yet
        if let Some(price_usd) = convert_krw_to_usd_for_exchange(price, exchange, &ctx.state) {
            let bid_usd =
                convert_krw_to_usd_for_exchange(bid_krw, exchange, &ctx.state).unwrap_or(price_usd);
            let ask_usd =
                convert_krw_to_usd_for_exchange(ask_krw, exchange, &ctx.state).unwrap_or(price_usd);
            let tick_usd = PriceTick::new(exchange, pair_id, price_usd, bid_usd, ask_usd)
                .with_sizes(bid_size, ask_size);

            // Update state asynchronously with KRW quote, bid/ask, and original KRW prices
            let state_clone = ctx.state.clone();
            let display_symbol_clone = display_symbol.clone();
            tokio::spawn(async move {
                state_clone
                    .update_price_with_bid_ask_and_raw(
                        exchange,
                        pair_id,
                        &display_symbol_clone,
                        price_usd,
                        bid_usd,
                        ask_usd, // USD-normalized
                        bid_krw,
                        ask_krw, // Original KRW
                        bid_size,
                        ask_size,
                        QuoteCurrency::KRW,
                    )
                    .await;
            });

            ws_server::broadcast_price_with_quote(
                &ctx.broadcast_tx,
                exchange,
                pair_id,
                &display_symbol,
                Some("KRW"),
                &tick_usd,
            );
        }
    }
}

/// Process orderbook message for Korean exchanges (Upbit, Bithumb).
///
/// Updates both the local cache (for ticker handler lookup) and state directly
/// for real-time price accuracy.
pub fn process_korean_orderbook<A: KoreanExchangeAdapter>(
    code: &str,
    bid: FixedPoint,
    ask: FixedPoint,
    bid_size: FixedPoint,
    ask_size: FixedPoint,
    exchange: Exchange,
    ctx: &FeedContext,
    orderbook_cache: &OrderbookCache,
) {
    // Skip stablecoin markets (USDT, USDC)
    if A::is_stablecoin_market(code) {
        return;
    }

    // Store bid/ask and sizes in cache for use by ticker handler
    orderbook_cache.insert(code.to_string(), (bid, ask, bid_size, ask_size));

    // Also update state directly from orderbook for real-time accuracy
    // This ensures prices reflect current orderbook even without trades
    if let Some(symbol) = A::extract_base_symbol(code) {
        let exchange_name = match exchange {
            Exchange::Upbit => "Upbit",
            Exchange::Bithumb => "Bithumb",
            _ => return,
        };
        let display_symbol = ctx.symbol_mappings.canonical_name(exchange_name, &symbol);
        let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

        // Convert KRW to USD using exchange's USDT/KRW rate
        if let Some(bid_usd) = convert_krw_to_usd_for_exchange(bid, exchange, &ctx.state) {
            let ask_usd =
                convert_krw_to_usd_for_exchange(ask, exchange, &ctx.state).unwrap_or(bid_usd);
            let mid_price_usd =
                FixedPoint::from_f64((bid_usd.to_f64() + ask_usd.to_f64()) / 2.0);
            let tick_usd = PriceTick::new(exchange, pair_id, mid_price_usd, bid_usd, ask_usd)
                .with_sizes(bid_size, ask_size);

            // Update state asynchronously
            let state_clone = ctx.state.clone();
            let display_symbol_clone = display_symbol.clone();
            tokio::spawn(async move {
                state_clone
                    .update_price_with_bid_ask_and_raw(
                        exchange,
                        pair_id,
                        &display_symbol_clone,
                        mid_price_usd,
                        bid_usd,
                        ask_usd, // USD-normalized
                        bid,
                        ask, // Original KRW
                        bid_size,
                        ask_size,
                        QuoteCurrency::KRW,
                    )
                    .await;
            });

            ws_server::broadcast_price_with_quote(
                &ctx.broadcast_tx,
                exchange,
                pair_id,
                &display_symbol,
                Some("KRW"),
                &tick_usd,
            );
        }
    }
}

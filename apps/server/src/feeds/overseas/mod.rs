//! Overseas exchange feed handlers (Binance, Coinbase, Bybit, GateIO).
//!
//! Overseas exchanges share common characteristics:
//! - Prices are denominated in USDT, USDC, or USD
//! - Use stablecoin/USD conversion rates
//! - Generally use JSON text WebSocket messages only

pub mod binance;
pub mod bybit;
pub mod coinbase;
pub mod gateio;

use super::common::convert_stablecoin_to_usd_for_exchange;
use super::FeedContext;
use crate::ws_server;
use arbitrage_core::{Exchange, FixedPoint, PriceTick, QuoteCurrency};

/// Process price update for overseas exchanges with stablecoin conversion.
///
/// Common flow for Binance, Bybit, GateIO:
/// 1. Update stablecoin prices if this is a stablecoin pair
/// 2. Convert prices to USD
/// 3. Update state and broadcast
pub async fn process_overseas_price_update(
    symbol: &str,
    quote: &str,
    mid_price: FixedPoint,
    bid: FixedPoint,
    ask: FixedPoint,
    bid_size: FixedPoint,
    ask_size: FixedPoint,
    exchange: Exchange,
    ctx: &FeedContext,
) {
    let exchange_name = match exchange {
        Exchange::Binance => "Binance",
        Exchange::Coinbase => "Coinbase",
        Exchange::Bybit => "Bybit",
        Exchange::GateIO => "GateIO",
        _ => return,
    };

    // Update stablecoin prices for this exchange
    if symbol == "USDT" || symbol == "USDC" {
        ctx.state
            .update_exchange_stablecoin_price(exchange, symbol, quote, mid_price.to_f64());
    }

    // Use canonical name if mapping exists
    let display_symbol = ctx.symbol_mappings.canonical_name(exchange_name, symbol);
    let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
    let quote_currency = QuoteCurrency::from_str(quote).unwrap_or(QuoteCurrency::USD);

    // Convert stablecoin prices to USD
    let mid_usd =
        convert_stablecoin_to_usd_for_exchange(mid_price, quote_currency, exchange, &ctx.state);
    let bid_usd = convert_stablecoin_to_usd_for_exchange(bid, quote_currency, exchange, &ctx.state);
    let ask_usd = convert_stablecoin_to_usd_for_exchange(ask, quote_currency, exchange, &ctx.state);

    let tick =
        PriceTick::new(exchange, pair_id, mid_price, bid, ask).with_sizes(bid_size, ask_size);

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
            ask, // Original USDT/USDC
            bid_size,
            ask_size,
            quote_currency,
        )
        .await;

    ws_server::broadcast_price_with_quote(
        &ctx.broadcast_tx,
        exchange,
        pair_id,
        &display_symbol,
        Some(quote),
        &tick,
    );
}

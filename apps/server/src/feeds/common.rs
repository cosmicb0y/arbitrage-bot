//! Common utilities shared across feed handlers.
//!
//! This module provides currency conversion functions and type aliases
//! used by both Korean and overseas exchange feed handlers.

use crate::state::SharedState;
use arbitrage_core::{Exchange, FixedPoint, QuoteCurrency};
use std::sync::Arc;

/// Cache for orderbook bid/ask and sizes (code -> (bid, ask, bid_size, ask_size))
/// Used by Korean exchanges (Upbit, Bithumb) for ticker+orderbook correlation.
pub type OrderbookCache =
    Arc<dashmap::DashMap<String, (FixedPoint, FixedPoint, FixedPoint, FixedPoint)>>;

/// Convert KRW price to USD using exchange-specific USDT/KRW rate.
///
/// Uses USDT/KRW from the same exchange, then converts via USDT/USD.
/// Returns None if exchange rate is not available yet.
pub fn convert_krw_to_usd_for_exchange(
    krw_price: FixedPoint,
    exchange: Exchange,
    state: &SharedState,
) -> Option<FixedPoint> {
    // Get exchange-specific USDT/KRW rate
    let usdt_krw = state.get_usdt_krw_for_exchange(exchange)?;
    let usdt_krw_f64 = usdt_krw.to_f64();

    // Validate USDT/KRW rate is reasonable (should be around 1300-1600)
    if usdt_krw_f64 < 1000.0 || usdt_krw_f64 > 2000.0 {
        tracing::warn!(
            "{:?}: Invalid USDT/KRW rate {:.2} - skipping price conversion",
            exchange,
            usdt_krw_f64
        );
        return None;
    }

    let usdt_usd = state.get_usdt_usd_price();

    // KRW -> USDT -> USD
    // price_usdt = krw_price / usdt_krw
    // price_usd = price_usdt * usdt_usd
    let price_usdt = krw_price.to_f64() / usdt_krw_f64;
    let price_usd = price_usdt * usdt_usd.to_f64();

    // Sanity check: converted price should be positive and reasonable
    if price_usd <= 0.0 || !price_usd.is_finite() {
        tracing::warn!(
            "{:?}: Invalid converted price {:.8} from KRW {:.2}",
            exchange,
            price_usd,
            krw_price.to_f64()
        );
        return None;
    }

    Some(FixedPoint::from_f64(price_usd))
}

/// Convert stablecoin (USDT/USDC) price to USD using exchange-specific rates.
///
/// Returns the original price if quote is USD or rate is unavailable.
pub fn convert_stablecoin_to_usd_for_exchange(
    price: FixedPoint,
    quote: QuoteCurrency,
    exchange: Exchange,
    state: &SharedState,
) -> FixedPoint {
    let rate = match quote {
        QuoteCurrency::USDT => state.get_usdt_usd_for_exchange(exchange),
        QuoteCurrency::USDC => state.get_usdc_usd_for_exchange(exchange),
        QuoteCurrency::USD => return price, // Already USD
        _ => return price,                  // KRW or other - handled separately
    };

    // Validate rate is reasonable (0.90 - 1.10 for stablecoins)
    if rate < 0.90 || rate > 1.10 {
        tracing::warn!(
            "{:?}: Unusual {:?}/USD rate {:.4} - using 1:1 fallback",
            exchange,
            quote,
            rate
        );
        return price;
    }

    // Log significant depegging events
    if rate < 0.98 || rate > 1.02 {
        tracing::info!(
            "{:?}: Stablecoin deviation {:?}/USD = {:.4}",
            exchange,
            quote,
            rate
        );
    }

    FixedPoint::from_f64(price.to_f64() * rate)
}

/// Extract base and quote from Binance symbol (e.g., btcusdt -> BTC, USDT)
pub fn extract_binance_base_quote(symbol: &str) -> Option<(String, String)> {
    let s = symbol.to_uppercase();
    // Try known quote currencies in order of length (longer first)
    for quote in &["USDT", "USDC", "BUSD", "USD", "BTC", "ETH", "BNB"] {
        if s.ends_with(quote) {
            let base = &s[..s.len() - quote.len()];
            if !base.is_empty() {
                return Some((base.to_string(), quote.to_string()));
            }
        }
    }
    None
}

/// Extract base and quote from Bybit symbol (e.g., BTCUSDT -> BTC, USDT)
pub fn extract_bybit_base_quote(symbol: &str) -> Option<(String, String)> {
    // Bybit uses same format as Binance
    extract_binance_base_quote(symbol)
}

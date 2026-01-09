//! Exchange adapter trait and implementations.
//!
//! Each exchange has its own WebSocket message format.
//! Adapters normalize these into our internal PriceTick format.

mod binance;
mod bithumb;
mod bybit;
mod coinbase;
mod gateio;
mod upbit;

pub use binance::BinanceAdapter;
pub use bithumb::{BithumbAdapter, BithumbMessage, OrderbookSnapshot as BithumbOrderbookSnapshot};
pub use bybit::BybitAdapter;
pub use coinbase::{CoinbaseAdapter, CoinbaseCredentials, CoinbaseL2Event};
pub use gateio::GateIOAdapter;
pub use upbit::{UpbitAdapter, UpbitMessage};

use arbitrage_core::{Exchange, FixedPoint};

/// Common message types returned by adapters after parsing.
#[derive(Debug, Clone)]
pub enum ExchangeMessage {
    /// Ticker message with trade price
    Ticker { symbol: String, price: FixedPoint },
    /// Orderbook message with best bid/ask and optional full depth
    Orderbook {
        symbol: String,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        /// Full orderbook bids (price, qty) - optional
        bids: Option<Vec<(f64, f64)>>,
        /// Full orderbook asks (price, qty) - optional
        asks: Option<Vec<(f64, f64)>>,
    },
}

/// Trait for exchange-specific WebSocket adapters.
///
/// All adapters share common patterns for:
/// - Extracting base/quote symbols from exchange-specific formats
/// - Parsing WebSocket messages into normalized types
/// - Generating subscription messages
pub trait ExchangeAdapter {
    /// Get the exchange identifier
    fn exchange() -> Exchange;

    /// Get WebSocket URL for this exchange
    fn ws_url() -> &'static str;

    /// Extract base and quote symbols from exchange-specific format.
    /// Returns (base, quote) tuple, e.g., ("BTC", "USDT")
    fn extract_base_quote(symbol: &str) -> Option<(String, String)>;

    /// Extract just the base symbol (convenience method)
    fn extract_base_symbol(symbol: &str) -> Option<String> {
        Self::extract_base_quote(symbol).map(|(base, _)| base)
    }

    /// Extract just the quote currency (convenience method)
    fn extract_quote_currency(symbol: &str) -> Option<String> {
        Self::extract_base_quote(symbol).map(|(_, quote)| quote)
    }

    /// Generate subscription messages for the given symbols.
    /// Returns one or more messages to send over WebSocket.
    fn subscribe_messages(symbols: &[String]) -> Vec<String>;
}

/// Helper trait for Korean exchange adapters (Upbit, Bithumb).
/// These exchanges share the same WebSocket protocol format.
pub trait KoreanExchangeAdapter: ExchangeAdapter {
    /// Check if the market code is for USDT (exchange rate)
    fn is_usdt_market(code: &str) -> bool;

    /// Check if the market code is for USDC (exchange rate)
    fn is_usdc_market(code: &str) -> bool;

    /// Check if the market code is a stablecoin market (USDT or USDC)
    fn is_stablecoin_market(code: &str) -> bool {
        Self::is_usdt_market(code) || Self::is_usdc_market(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_message_types() {
        let ticker = ExchangeMessage::Ticker {
            symbol: "BTC".to_string(),
            price: FixedPoint::from_f64(50000.0),
        };

        let orderbook = ExchangeMessage::Orderbook {
            symbol: "BTC".to_string(),
            bid: FixedPoint::from_f64(49999.0),
            ask: FixedPoint::from_f64(50001.0),
            bid_size: FixedPoint::from_f64(1.0),
            ask_size: FixedPoint::from_f64(1.0),
            bids: None,
            asks: None,
        };

        // Just verify they can be created
        match ticker {
            ExchangeMessage::Ticker { symbol, .. } => assert_eq!(symbol, "BTC"),
            _ => panic!("Expected Ticker"),
        }

        match orderbook {
            ExchangeMessage::Orderbook { symbol, .. } => assert_eq!(symbol, "BTC"),
            _ => panic!("Expected Orderbook"),
        }
    }
}

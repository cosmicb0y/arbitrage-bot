//! Feed message types for communication between runners and handlers.
//!
//! This module defines the channel-based message types that runners send
//! to handlers, enabling clean separation between parsing logic (in crates/feeds)
//! and application logic (in apps/server).

use arbitrage_core::{Exchange, FixedPoint};
use std::time::Duration;

/// Message sent from feed runners to the application handler.
#[derive(Debug, Clone)]
pub enum FeedMessage {
    /// Parsed price tick data
    Tick(ParsedTick),
    /// Connection lifecycle event
    Event(ConnectionEvent),
}

/// Parsed price data from an exchange.
///
/// This is the normalized output from feed runners after parsing
/// exchange-specific WebSocket messages.
#[derive(Debug, Clone)]
pub enum ParsedTick {
    /// Regular price update with bid/ask
    Price {
        exchange: Exchange,
        /// Original symbol from exchange (e.g., "BTCUSDT", "KRW-BTC")
        symbol: String,
        /// Quote currency (e.g., "USDT", "USD", "KRW")
        quote: String,
        /// Mid price
        mid: FixedPoint,
        /// Best bid price
        bid: FixedPoint,
        /// Best ask price
        ask: FixedPoint,
        /// Best bid size
        bid_size: FixedPoint,
        /// Best ask size
        ask_size: FixedPoint,
        /// Full orderbook for depth walking (optional)
        orderbook: Option<Orderbook>,
    },
    /// Stablecoin exchange rate update (USDT/USD, USDC/USD, USDT/KRW, etc.)
    StablecoinRate {
        exchange: Exchange,
        /// Stablecoin symbol ("USDT" or "USDC")
        stablecoin: String,
        /// Quote currency ("USD" or "KRW")
        quote: String,
        /// Exchange rate
        rate: FixedPoint,
    },
}

/// Full orderbook snapshot for depth walking calculations.
#[derive(Debug, Clone)]
pub struct Orderbook {
    /// Bid levels: (price, quantity)
    pub bids: Vec<(f64, f64)>,
    /// Ask levels: (price, quantity)
    pub asks: Vec<(f64, f64)>,
    /// Whether this is a full snapshot (true) or delta update (false)
    pub is_snapshot: bool,
}

impl Orderbook {
    /// Create a new orderbook snapshot from bid/ask vectors.
    pub fn new(bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> Self {
        Self {
            bids,
            asks,
            is_snapshot: true,
        }
    }

    /// Create a delta orderbook update.
    pub fn delta(bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> Self {
        Self {
            bids,
            asks,
            is_snapshot: false,
        }
    }

    /// Check if the orderbook is empty.
    pub fn is_empty(&self) -> bool {
        self.bids.is_empty() && self.asks.is_empty()
    }
}

/// WebSocket connection lifecycle events.
///
/// These events are sent by runners to notify the handler about
/// connection state changes.
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// Initial connection established
    Connected(Exchange),
    /// Connection lost
    Disconnected(Exchange),
    /// Reconnected after disconnection (caches should be cleared)
    Reconnected(Exchange),
    /// Circuit breaker opened due to repeated failures
    CircuitBreakerOpen(Exchange, Duration),
    /// Error occurred (non-fatal)
    Error(Exchange, String),
}

impl ParsedTick {
    /// Get the exchange this tick is from.
    pub fn exchange(&self) -> Exchange {
        match self {
            ParsedTick::Price { exchange, .. } => *exchange,
            ParsedTick::StablecoinRate { exchange, .. } => *exchange,
        }
    }

    /// Create a new price tick.
    pub fn price(
        exchange: Exchange,
        symbol: impl Into<String>,
        quote: impl Into<String>,
        mid: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
    ) -> Self {
        ParsedTick::Price {
            exchange,
            symbol: symbol.into(),
            quote: quote.into(),
            mid,
            bid,
            ask,
            bid_size,
            ask_size,
            orderbook: None,
        }
    }

    /// Create a new price tick with orderbook.
    pub fn price_with_orderbook(
        exchange: Exchange,
        symbol: impl Into<String>,
        quote: impl Into<String>,
        mid: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        orderbook: Orderbook,
    ) -> Self {
        ParsedTick::Price {
            exchange,
            symbol: symbol.into(),
            quote: quote.into(),
            mid,
            bid,
            ask,
            bid_size,
            ask_size,
            orderbook: Some(orderbook),
        }
    }

    /// Create a new stablecoin rate tick.
    pub fn stablecoin_rate(
        exchange: Exchange,
        stablecoin: impl Into<String>,
        quote: impl Into<String>,
        rate: FixedPoint,
    ) -> Self {
        ParsedTick::StablecoinRate {
            exchange,
            stablecoin: stablecoin.into(),
            quote: quote.into(),
            rate,
        }
    }
}

impl From<ParsedTick> for FeedMessage {
    fn from(tick: ParsedTick) -> Self {
        FeedMessage::Tick(tick)
    }
}

impl From<ConnectionEvent> for FeedMessage {
    fn from(event: ConnectionEvent) -> Self {
        FeedMessage::Event(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_tick_price() {
        let tick = ParsedTick::price(
            Exchange::Binance,
            "BTC",
            "USDT",
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50001.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.5),
        );

        assert_eq!(tick.exchange(), Exchange::Binance);
        if let ParsedTick::Price { symbol, quote, .. } = tick {
            assert_eq!(symbol, "BTC");
            assert_eq!(quote, "USDT");
        } else {
            panic!("Expected Price variant");
        }
    }

    #[test]
    fn test_parsed_tick_stablecoin() {
        let tick = ParsedTick::stablecoin_rate(
            Exchange::Upbit,
            "USDT",
            "KRW",
            FixedPoint::from_f64(1350.0),
        );

        assert_eq!(tick.exchange(), Exchange::Upbit);
        if let ParsedTick::StablecoinRate {
            stablecoin, quote, ..
        } = tick
        {
            assert_eq!(stablecoin, "USDT");
            assert_eq!(quote, "KRW");
        } else {
            panic!("Expected StablecoinRate variant");
        }
    }

    #[test]
    fn test_feed_message_from() {
        let tick = ParsedTick::stablecoin_rate(
            Exchange::Binance,
            "USDT",
            "USD",
            FixedPoint::from_f64(1.0),
        );
        let msg: FeedMessage = tick.into();
        assert!(matches!(msg, FeedMessage::Tick(_)));

        let event = ConnectionEvent::Connected(Exchange::Binance);
        let msg: FeedMessage = event.into();
        assert!(matches!(msg, FeedMessage::Event(_)));
    }

    #[test]
    fn test_orderbook() {
        let orderbook = Orderbook::new(
            vec![(49999.0, 1.0), (49998.0, 2.0)],
            vec![(50001.0, 1.5), (50002.0, 0.5)],
        );

        assert!(!orderbook.is_empty());
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);

        let empty = Orderbook::new(vec![], vec![]);
        assert!(empty.is_empty());
    }
}

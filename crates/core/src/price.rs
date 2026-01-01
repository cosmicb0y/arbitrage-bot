//! Price data structures for real-time market data.

use crate::{Exchange, QuoteCurrency};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// Fixed-point number with 18 decimal places.
/// Used for precise price representation without floating-point errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FixedPoint(pub u64);

impl FixedPoint {
    /// Number of decimal places (8 for price precision)
    pub const DECIMALS: u32 = 8;
    /// Scale factor: 10^8 (fits comfortably in u64 for most prices)
    pub const SCALE: u64 = 100_000_000;

    /// Create from f64 (for testing/convenience, not recommended for production)
    pub fn from_f64(value: f64) -> Self {
        Self((value * Self::SCALE as f64) as u64)
    }

    /// Convert to f64 (for display/debugging)
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }

    /// Calculate premium in basis points: (sell - buy) / buy * 10000
    pub fn premium_bps(buy: FixedPoint, sell: FixedPoint) -> i32 {
        if buy.0 == 0 {
            return 0;
        }
        let diff = sell.0 as i128 - buy.0 as i128;
        ((diff * 10000) / buy.0 as i128) as i32
    }
}

impl Add for FixedPoint {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for FixedPoint {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

/// Real-time price tick data.
/// Packed for minimal memory footprint in high-throughput scenarios.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C, packed)]
pub struct PriceTick {
    /// Exchange where this price was observed
    pub exchange: Exchange,      // 2 bytes
    /// Internal pair ID for fast lookup
    pub pair_id: u32,            // 4 bytes
    /// Quote currency (USD, USDT, USDC, KRW, etc.)
    quote_currency: u8,          // 1 byte
    /// Current price (fixed-point 18 decimals)
    price: u64,                  // 8 bytes
    /// 24h trading volume
    pub volume_24h: u64,         // 8 bytes
    /// Best bid price
    bid: u64,                    // 8 bytes
    /// Best ask price
    ask: u64,                    // 8 bytes
    /// Best bid size (quantity available at best bid)
    bid_size: u64,               // 8 bytes
    /// Best ask size (quantity available at best ask)
    ask_size: u64,               // 8 bytes
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,       // 8 bytes
    /// Liquidity (TVL for DEX, depth for CEX)
    pub liquidity: u64,          // 8 bytes
}
// Total: 71 bytes

impl PriceTick {
    /// Create a new price tick with default quote currency (USD).
    pub fn new(
        exchange: Exchange,
        pair_id: u32,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
    ) -> Self {
        Self::with_quote(exchange, pair_id, price, bid, ask, QuoteCurrency::USD)
    }

    /// Create a new price tick with specified quote currency.
    pub fn with_quote(
        exchange: Exchange,
        pair_id: u32,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        quote_currency: QuoteCurrency,
    ) -> Self {
        Self {
            exchange,
            pair_id,
            quote_currency: quote_currency as u8,
            price: price.0,
            volume_24h: 0,
            bid: bid.0,
            ask: ask.0,
            bid_size: 0,
            ask_size: 0,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            liquidity: 0,
        }
    }

    /// Create a new price tick with bid/ask sizes (orderbook depth).
    pub fn with_depth(
        exchange: Exchange,
        pair_id: u32,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        quote_currency: QuoteCurrency,
    ) -> Self {
        Self {
            exchange,
            pair_id,
            quote_currency: quote_currency as u8,
            price: price.0,
            volume_24h: 0,
            bid: bid.0,
            ask: ask.0,
            bid_size: bid_size.0,
            ask_size: ask_size.0,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            liquidity: 0,
        }
    }

    /// Get price as FixedPoint.
    #[inline]
    pub fn price(&self) -> FixedPoint {
        FixedPoint(self.price)
    }

    /// Get bid as FixedPoint.
    #[inline]
    pub fn bid(&self) -> FixedPoint {
        FixedPoint(self.bid)
    }

    /// Get ask as FixedPoint.
    #[inline]
    pub fn ask(&self) -> FixedPoint {
        FixedPoint(self.ask)
    }

    /// Get bid size as FixedPoint.
    #[inline]
    pub fn bid_size(&self) -> FixedPoint {
        FixedPoint(self.bid_size)
    }

    /// Get ask size as FixedPoint.
    #[inline]
    pub fn ask_size(&self) -> FixedPoint {
        FixedPoint(self.ask_size)
    }

    /// Set bid/ask sizes (builder pattern).
    #[inline]
    pub fn with_sizes(mut self, bid_size: FixedPoint, ask_size: FixedPoint) -> Self {
        self.bid_size = bid_size.0;
        self.ask_size = ask_size.0;
        self
    }

    /// Calculate bid-ask spread in basis points.
    pub fn spread_bps(&self) -> i32 {
        FixedPoint::premium_bps(self.bid(), self.ask())
    }

    /// Get exchange (safe accessor for packed struct).
    #[inline]
    pub fn exchange(&self) -> Exchange {
        self.exchange
    }

    /// Get pair_id (safe accessor for packed struct).
    #[inline]
    pub fn pair_id(&self) -> u32 {
        self.pair_id
    }

    /// Get volume_24h (safe accessor for packed struct).
    #[inline]
    pub fn volume_24h(&self) -> FixedPoint {
        FixedPoint(self.volume_24h)
    }

    /// Set volume_24h.
    #[inline]
    pub fn set_volume_24h(&mut self, volume: FixedPoint) {
        self.volume_24h = volume.0;
    }

    /// Builder pattern: set volume_24h and return self.
    #[inline]
    pub fn with_volume_24h(mut self, volume: FixedPoint) -> Self {
        self.volume_24h = volume.0;
        self
    }

    /// Get liquidity (safe accessor for packed struct).
    #[inline]
    pub fn liquidity(&self) -> FixedPoint {
        FixedPoint(self.liquidity)
    }

    /// Get timestamp_ms (safe accessor for packed struct).
    #[inline]
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }

    /// Get quote currency (safe accessor for packed struct).
    #[inline]
    pub fn quote_currency(&self) -> QuoteCurrency {
        QuoteCurrency::from_id(self.quote_currency).unwrap_or(QuoteCurrency::USD)
    }

    /// Check if this price is in a USD-equivalent currency.
    #[inline]
    pub fn is_usd_equivalent(&self) -> bool {
        self.quote_currency().is_usd_equivalent()
    }
}

/// Orderbook snapshot with multiple price levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub exchange: Exchange,
    pub pair_id: u32,
    pub timestamp_ms: u64,
    /// Bids: (price, quantity) sorted descending by price
    pub bids: Vec<(u64, u64)>,
    /// Asks: (price, quantity) sorted ascending by price
    pub asks: Vec<(u64, u64)>,
}

impl OrderbookSnapshot {
    /// Get best bid price.
    pub fn best_bid(&self) -> Option<FixedPoint> {
        self.bids.first().map(|(p, _)| FixedPoint(*p))
    }

    /// Get best ask price.
    pub fn best_ask(&self) -> Option<FixedPoint> {
        self.asks.first().map(|(p, _)| FixedPoint(*p))
    }

    /// Calculate mid price.
    pub fn mid_price(&self) -> Option<FixedPoint> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(FixedPoint((bid.0 + ask.0) / 2)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Fixed-point arithmetic tests ===

    #[test]
    fn test_fixed_point_conversion() {
        // 1.0 in 8 decimals
        let one = FixedPoint::from_f64(1.0);
        assert_eq!(one.0, 100_000_000u64);

        // 50000.50 in 8 decimals
        let price = FixedPoint::from_f64(50000.5);
        assert_eq!(price.to_f64(), 50000.5);
    }

    #[test]
    fn test_fixed_point_arithmetic() {
        let a = FixedPoint::from_f64(100.0);
        let b = FixedPoint::from_f64(50.0);

        // Addition
        assert_eq!((a + b).to_f64(), 150.0);

        // Subtraction
        assert_eq!((a - b).to_f64(), 50.0);
    }

    #[test]
    fn test_fixed_point_bps_calculation() {
        let buy = FixedPoint::from_f64(100.0);
        let sell = FixedPoint::from_f64(101.0);

        // (101 - 100) / 100 * 10000 = 100 bps (1%)
        let bps = FixedPoint::premium_bps(buy, sell);
        assert_eq!(bps, 100);
    }

    // === PriceTick tests ===

    #[test]
    fn test_price_tick_size() {
        // Verify packed struct size for performance
        // exchange(2) + pair_id(4) + quote_currency(1) + price(8) + volume_24h(8) + bid(8) + ask(8) + bid_size(8) + ask_size(8) + timestamp_ms(8) + liquidity(8) = 71
        assert_eq!(std::mem::size_of::<PriceTick>(), 71);
    }

    #[test]
    fn test_price_tick_new() {
        let tick = PriceTick::new(
            Exchange::Binance,
            12345,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50001.0),
        );

        assert_eq!(tick.exchange(), Exchange::Binance);
        assert_eq!(tick.pair_id(), 12345);
        assert_eq!(tick.price().to_f64(), 50000.0);
        assert_eq!(tick.bid().to_f64(), 49999.0);
        assert_eq!(tick.ask().to_f64(), 50001.0);
    }

    #[test]
    fn test_price_tick_spread_bps() {
        let tick = PriceTick::new(
            Exchange::Binance,
            1,
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(99.9),  // bid
            FixedPoint::from_f64(100.1), // ask
        );

        // Spread = (100.1 - 99.9) / 99.9 * 10000 ≈ 20 bps
        let spread = tick.spread_bps();
        assert!(spread >= 19 && spread <= 21); // Allow for rounding
    }

    #[test]
    fn test_price_tick_quote_currency() {
        use crate::QuoteCurrency;

        // Default constructor uses USD
        let tick_usd = PriceTick::new(
            Exchange::Binance,
            1,
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(99.9),
            FixedPoint::from_f64(100.1),
        );
        assert_eq!(tick_usd.quote_currency(), QuoteCurrency::USD);
        assert!(tick_usd.is_usd_equivalent());

        // with_quote constructor with USDT
        let tick_usdt = PriceTick::with_quote(
            Exchange::Binance,
            1,
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(99.9),
            FixedPoint::from_f64(100.1),
            QuoteCurrency::USDT,
        );
        assert_eq!(tick_usdt.quote_currency(), QuoteCurrency::USDT);
        assert!(tick_usdt.is_usd_equivalent());

        // with_quote constructor with KRW
        let tick_krw = PriceTick::with_quote(
            Exchange::Upbit,
            1,
            FixedPoint::from_f64(50000000.0),
            FixedPoint::from_f64(49999000.0),
            FixedPoint::from_f64(50001000.0),
            QuoteCurrency::KRW,
        );
        assert_eq!(tick_krw.quote_currency(), QuoteCurrency::KRW);
        assert!(!tick_krw.is_usd_equivalent());
    }

    // === OrderbookSnapshot tests ===

    #[test]
    fn test_orderbook_snapshot() {
        let snapshot = OrderbookSnapshot {
            exchange: Exchange::Binance,
            pair_id: 1,
            timestamp_ms: 1700000000000,
            bids: vec![
                (FixedPoint::from_f64(100.0).0, FixedPoint::from_f64(10.0).0),
                (FixedPoint::from_f64(99.0).0, FixedPoint::from_f64(20.0).0),
            ],
            asks: vec![
                (FixedPoint::from_f64(101.0).0, FixedPoint::from_f64(5.0).0),
                (FixedPoint::from_f64(102.0).0, FixedPoint::from_f64(15.0).0),
            ],
        };

        assert_eq!(snapshot.bids.len(), 2);
        assert_eq!(snapshot.asks.len(), 2);
        assert_eq!(snapshot.best_bid().unwrap().to_f64(), 100.0);
        assert_eq!(snapshot.best_ask().unwrap().to_f64(), 101.0);
    }

    #[test]
    fn test_orderbook_mid_price() {
        let snapshot = OrderbookSnapshot {
            exchange: Exchange::Binance,
            pair_id: 1,
            timestamp_ms: 1700000000000,
            bids: vec![(FixedPoint::from_f64(100.0).0, 1)],
            asks: vec![(FixedPoint::from_f64(102.0).0, 1)],
        };

        // Mid = (100 + 102) / 2 = 101
        assert_eq!(snapshot.mid_price().unwrap().to_f64(), 101.0);
    }
}

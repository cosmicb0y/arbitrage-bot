//! Orderbook cache for storing full orderbook depth.
//!
//! This module provides efficient storage for orderbook snapshots
//! with support for both full snapshots and incremental updates.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use arbitrage_core::FixedPoint;

/// Default maximum levels to store per side.
pub const DEFAULT_MAX_LEVELS: usize = 20;

/// Side of the orderbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

/// Orderbook cache with configurable depth levels.
///
/// Stores bids sorted in descending order by price (best bid first)
/// and asks sorted in ascending order by price (best ask first).
#[derive(Debug, Clone)]
pub struct OrderbookCache {
    /// Maximum levels to store per side.
    max_levels: usize,
    /// Bids: price (descending) -> quantity
    bids: BTreeMap<Reverse<u64>, u64>,
    /// Asks: price (ascending) -> quantity
    asks: BTreeMap<u64, u64>,
    /// Last update timestamp in milliseconds.
    timestamp_ms: u64,
}

impl Default for OrderbookCache {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_LEVELS)
    }
}

impl OrderbookCache {
    /// Create a new orderbook cache with the specified max levels.
    pub fn new(max_levels: usize) -> Self {
        Self {
            max_levels,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            timestamp_ms: 0,
        }
    }

    /// Update from a full snapshot, replacing all existing data.
    ///
    /// # Arguments
    /// * `bids` - Bids as (price, quantity) pairs, sorted descending by price
    /// * `asks` - Asks as (price, quantity) pairs, sorted ascending by price
    pub fn update_snapshot(&mut self, bids: &[(u64, u64)], asks: &[(u64, u64)]) {
        self.bids.clear();
        self.asks.clear();

        // Insert bids (take top N levels)
        for (price, qty) in bids.iter().take(self.max_levels) {
            if *qty > 0 {
                self.bids.insert(Reverse(*price), *qty);
            }
        }

        // Insert asks (take top N levels)
        for (price, qty) in asks.iter().take(self.max_levels) {
            if *qty > 0 {
                self.asks.insert(*price, *qty);
            }
        }

        self.timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Update from a full snapshot with f64 prices.
    pub fn update_snapshot_f64(&mut self, bids: &[(f64, f64)], asks: &[(f64, f64)]) {
        let bids_u64: Vec<(u64, u64)> = bids
            .iter()
            .map(|(p, q)| (FixedPoint::from_f64(*p).0, FixedPoint::from_f64(*q).0))
            .collect();
        let asks_u64: Vec<(u64, u64)> = asks
            .iter()
            .map(|(p, q)| (FixedPoint::from_f64(*p).0, FixedPoint::from_f64(*q).0))
            .collect();
        self.update_snapshot(&bids_u64, &asks_u64);
    }

    /// Apply an incremental delta update.
    ///
    /// # Arguments
    /// * `side` - Bid or Ask side
    /// * `price` - Price level
    /// * `qty` - New quantity (0 to remove the level)
    pub fn apply_delta(&mut self, side: Side, price: u64, qty: u64) {
        match side {
            Side::Bid => {
                if qty == 0 {
                    self.bids.remove(&Reverse(price));
                } else {
                    self.bids.insert(Reverse(price), qty);
                    // Trim if over max levels
                    while self.bids.len() > self.max_levels {
                        // Remove worst bid (lowest price = last in descending order)
                        self.bids.pop_last();
                    }
                }
            }
            Side::Ask => {
                if qty == 0 {
                    self.asks.remove(&price);
                } else {
                    self.asks.insert(price, qty);
                    // Trim if over max levels
                    while self.asks.len() > self.max_levels {
                        // Remove worst ask (highest price = last in ascending order)
                        self.asks.pop_last();
                    }
                }
            }
        }

        self.timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Apply an incremental delta update with f64 values.
    pub fn apply_delta_f64(&mut self, side: Side, price: f64, qty: f64) {
        self.apply_delta(
            side,
            FixedPoint::from_f64(price).0,
            FixedPoint::from_f64(qty).0,
        );
    }

    /// Get the best bid (highest price).
    pub fn best_bid(&self) -> Option<(FixedPoint, FixedPoint)> {
        self.bids
            .first_key_value()
            .map(|(Reverse(p), q)| (FixedPoint(*p), FixedPoint(*q)))
    }

    /// Get the best ask (lowest price).
    pub fn best_ask(&self) -> Option<(FixedPoint, FixedPoint)> {
        self.asks
            .first_key_value()
            .map(|(p, q)| (FixedPoint(*p), FixedPoint(*q)))
    }

    /// Get the bid-ask spread in basis points.
    pub fn spread_bps(&self) -> Option<i32> {
        match (self.best_bid(), self.best_ask()) {
            (Some((bid, _)), Some((ask, _))) if bid.0 > 0 => {
                Some(((ask.0 as i64 - bid.0 as i64) * 10000 / bid.0 as i64) as i32)
            }
            _ => None,
        }
    }

    /// Iterate over bids in descending price order (best first).
    /// Returns (price, quantity) as u64 pairs.
    pub fn bids_iter(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.bids.iter().map(|(Reverse(p), q)| (*p, *q))
    }

    /// Iterate over asks in ascending price order (best first).
    /// Returns (price, quantity) as u64 pairs.
    pub fn asks_iter(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.asks.iter().map(|(p, q)| (*p, *q))
    }

    /// Get bids as a vector of (price, quantity) pairs in descending price order.
    pub fn bids_vec(&self) -> Vec<(u64, u64)> {
        self.bids_iter().collect()
    }

    /// Get asks as a vector of (price, quantity) pairs in ascending price order.
    pub fn asks_vec(&self) -> Vec<(u64, u64)> {
        self.asks_iter().collect()
    }

    /// Get the timestamp of the last update.
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }

    /// Check if the orderbook has any data.
    pub fn is_empty(&self) -> bool {
        self.bids.is_empty() && self.asks.is_empty()
    }

    /// Get the number of bid levels.
    pub fn bid_levels(&self) -> usize {
        self.bids.len()
    }

    /// Get the number of ask levels.
    pub fn ask_levels(&self) -> usize {
        self.asks.len()
    }

    /// Calculate total bid depth (sum of all bid quantities).
    pub fn total_bid_depth(&self) -> FixedPoint {
        FixedPoint(self.bids.values().sum())
    }

    /// Calculate total ask depth (sum of all ask quantities).
    pub fn total_ask_depth(&self) -> FixedPoint {
        FixedPoint(self.asks.values().sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_snapshot() {
        let mut ob = OrderbookCache::new(10);

        let bids = vec![
            (50000_00000000u64, 1_00000000u64), // 50000.0 price, 1.0 qty
            (49990_00000000u64, 2_00000000u64),
            (49980_00000000u64, 3_00000000u64),
        ];

        let asks = vec![
            (50010_00000000u64, 1_50000000u64),
            (50020_00000000u64, 2_50000000u64),
            (50030_00000000u64, 3_50000000u64),
        ];

        ob.update_snapshot(&bids, &asks);

        assert_eq!(ob.bid_levels(), 3);
        assert_eq!(ob.ask_levels(), 3);

        let best_bid = ob.best_bid().unwrap();
        assert_eq!(best_bid.0 .0, 50000_00000000u64);
        assert_eq!(best_bid.1 .0, 1_00000000u64);

        let best_ask = ob.best_ask().unwrap();
        assert_eq!(best_ask.0 .0, 50010_00000000u64);
        assert_eq!(best_ask.1 .0, 1_50000000u64);
    }

    #[test]
    fn test_orderbook_delta() {
        let mut ob = OrderbookCache::new(10);

        // Add levels via delta
        ob.apply_delta(Side::Bid, 50000_00000000, 1_00000000);
        ob.apply_delta(Side::Bid, 49990_00000000, 2_00000000);
        ob.apply_delta(Side::Ask, 50010_00000000, 1_50000000);

        assert_eq!(ob.bid_levels(), 2);
        assert_eq!(ob.ask_levels(), 1);

        // Update existing level
        ob.apply_delta(Side::Bid, 50000_00000000, 5_00000000);
        let best_bid = ob.best_bid().unwrap();
        assert_eq!(best_bid.1 .0, 5_00000000u64);

        // Remove level
        ob.apply_delta(Side::Bid, 50000_00000000, 0);
        assert_eq!(ob.bid_levels(), 1);
        let best_bid = ob.best_bid().unwrap();
        assert_eq!(best_bid.0 .0, 49990_00000000u64);
    }

    #[test]
    fn test_orderbook_max_levels() {
        let mut ob = OrderbookCache::new(3);

        // Add more than max levels
        for i in 0..5 {
            ob.apply_delta(Side::Bid, (50000 - i) * 100000000, 1_00000000);
        }

        // Should only keep top 3 (highest prices for bids)
        assert_eq!(ob.bid_levels(), 3);

        let bids: Vec<_> = ob.bids_iter().collect();
        assert_eq!(bids[0].0, 50000_00000000); // Best bid
        assert_eq!(bids[2].0, 49998_00000000); // Third best
    }

    #[test]
    fn test_orderbook_spread() {
        let mut ob = OrderbookCache::new(10);

        ob.apply_delta(Side::Bid, 50000_00000000, 1_00000000);
        ob.apply_delta(Side::Ask, 50050_00000000, 1_00000000);

        // Spread = (50050 - 50000) / 50000 * 10000 = 10 bps
        let spread = ob.spread_bps().unwrap();
        assert_eq!(spread, 10);
    }

    #[test]
    fn test_orderbook_f64() {
        let mut ob = OrderbookCache::new(10);

        let bids = vec![(50000.0, 1.0), (49990.0, 2.0)];
        let asks = vec![(50010.0, 1.5), (50020.0, 2.5)];

        ob.update_snapshot_f64(&bids, &asks);

        let best_bid = ob.best_bid().unwrap();
        assert!((best_bid.0.to_f64() - 50000.0).abs() < 0.01);

        let best_ask = ob.best_ask().unwrap();
        assert!((best_ask.0.to_f64() - 50010.0).abs() < 0.01);
    }
}

//! Price aggregator for collecting and distributing price updates.
//!
//! Aggregates prices from multiple exchanges and provides a unified view.

use arbitrage_core::{Exchange, FixedPoint, PriceTick};
use dashmap::DashMap;
use std::sync::Arc;

/// Key for price storage: (exchange, pair_id)
type PriceKey = (u16, u32);

/// Thread-safe price aggregator for multiple exchanges.
#[derive(Debug, Clone)]
pub struct PriceAggregator {
    /// Prices indexed by (exchange_id, pair_id)
    prices: Arc<DashMap<PriceKey, PriceTick>>,
}

impl PriceAggregator {
    /// Create a new price aggregator.
    pub fn new() -> Self {
        Self {
            prices: Arc::new(DashMap::new()),
        }
    }

    /// Update price for an exchange/pair.
    pub fn update(&self, tick: PriceTick) {
        let key = (tick.exchange() as u16, tick.pair_id());
        self.prices.insert(key, tick);
    }

    /// Get the latest price for an exchange/pair.
    pub fn get_price(&self, exchange: Exchange, pair_id: u32) -> Option<PriceTick> {
        let key = (exchange as u16, pair_id);
        self.prices.get(&key).map(|r| *r)
    }

    /// Get all prices for a specific pair across all exchanges.
    pub fn get_all_prices_for_pair(&self, pair_id: u32) -> Vec<PriceTick> {
        self.prices
            .iter()
            .filter(|r| r.key().1 == pair_id)
            .map(|r| *r.value())
            .collect()
    }

    /// Get all prices across all exchanges and pairs.
    pub fn get_all_prices(&self) -> Vec<PriceTick> {
        self.prices.iter().map(|r| *r.value()).collect()
    }

    /// Calculate premium between two exchanges for a pair.
    /// Returns basis points (bps): (sell - buy) / buy * 10000
    pub fn calculate_premium(
        &self,
        buy_exchange: Exchange,
        sell_exchange: Exchange,
        pair_id: u32,
    ) -> Option<i32> {
        let buy_tick = self.get_price(buy_exchange, pair_id)?;
        let sell_tick = self.get_price(sell_exchange, pair_id)?;

        Some(FixedPoint::premium_bps(buy_tick.price(), sell_tick.price()))
    }

    /// Find the best arbitrage opportunity for a pair.
    /// Returns (buy_exchange, sell_exchange, premium_bps).
    pub fn find_best_opportunity(&self, pair_id: u32) -> Option<(Exchange, Exchange, i32)> {
        let prices = self.get_all_prices_for_pair(pair_id);
        if prices.len() < 2 {
            return None;
        }

        // Find min and max prices
        let mut min_price_tick: Option<&PriceTick> = None;
        let mut max_price_tick: Option<&PriceTick> = None;

        for tick in &prices {
            if min_price_tick.is_none() || tick.price().0 < min_price_tick.unwrap().price().0 {
                min_price_tick = Some(tick);
            }
            if max_price_tick.is_none() || tick.price().0 > max_price_tick.unwrap().price().0 {
                max_price_tick = Some(tick);
            }
        }

        let min = min_price_tick?;
        let max = max_price_tick?;

        // Buy at min, sell at max
        let premium = FixedPoint::premium_bps(min.price(), max.price());

        Some((min.exchange(), max.exchange(), premium))
    }

    /// Check if a price is stale (older than max_age_ms).
    pub fn is_stale(&self, exchange: Exchange, pair_id: u32, max_age_ms: u64) -> bool {
        let Some(tick) = self.get_price(exchange, pair_id) else {
            return true;
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        now.saturating_sub(tick.timestamp_ms()) > max_age_ms
    }

    /// Get the number of stored prices.
    pub fn len(&self) -> usize {
        self.prices.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }

    /// Clear all prices.
    pub fn clear(&self) {
        self.prices.clear();
    }
}

impl Default for PriceAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tick(exchange: Exchange, pair_id: u32, price: f64) -> PriceTick {
        PriceTick::new(
            exchange,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(price - 1.0),
            FixedPoint::from_f64(price + 1.0),
        )
    }

    #[test]
    fn test_aggregator_update_price() {
        let aggregator = PriceAggregator::new();

        let tick = create_test_tick(Exchange::Binance, 1, 50000.0);
        aggregator.update(tick);

        let price = aggregator.get_price(Exchange::Binance, 1);
        assert!(price.is_some());
        assert!((price.unwrap().price().to_f64() - 50000.0).abs() < 0.01);
    }

    #[test]
    fn test_aggregator_get_all_prices() {
        let aggregator = PriceAggregator::new();

        aggregator.update(create_test_tick(Exchange::Binance, 1, 50000.0));
        aggregator.update(create_test_tick(Exchange::Coinbase, 1, 50100.0));
        aggregator.update(create_test_tick(Exchange::Kraken, 1, 50050.0));

        let prices = aggregator.get_all_prices_for_pair(1);
        assert_eq!(prices.len(), 3);
    }

    #[test]
    fn test_aggregator_calculate_premium() {
        let aggregator = PriceAggregator::new();

        // Binance: $50,000 (buy here)
        // Coinbase: $50,500 (sell here)
        aggregator.update(create_test_tick(Exchange::Binance, 1, 50000.0));
        aggregator.update(create_test_tick(Exchange::Coinbase, 1, 50500.0));

        let premium = aggregator.calculate_premium(Exchange::Binance, Exchange::Coinbase, 1);

        // Premium = (50500 - 50000) / 50000 * 10000 = 100 bps (1%)
        assert!(premium.is_some());
        assert_eq!(premium.unwrap(), 100);
    }

    #[test]
    fn test_aggregator_best_opportunity() {
        let aggregator = PriceAggregator::new();

        aggregator.update(create_test_tick(Exchange::Binance, 1, 50000.0));
        aggregator.update(create_test_tick(Exchange::Coinbase, 1, 50500.0));
        aggregator.update(create_test_tick(Exchange::Kraken, 1, 49900.0));

        let (buy_ex, sell_ex, premium) = aggregator.find_best_opportunity(1).unwrap();

        // Best: Buy at Kraken ($49,900), Sell at Coinbase ($50,500)
        // Premium = (50500 - 49900) / 49900 * 10000 ≈ 120 bps
        assert_eq!(buy_ex, Exchange::Kraken);
        assert_eq!(sell_ex, Exchange::Coinbase);
        assert!(premium > 100);
    }

    #[test]
    fn test_aggregator_stale_price_detection() {
        let aggregator = PriceAggregator::new();

        let tick = create_test_tick(Exchange::Binance, 1, 50000.0);
        aggregator.update(tick);

        // Fresh price should not be stale
        assert!(!aggregator.is_stale(Exchange::Binance, 1, 5000));
    }

    // Story 5.1: Dynamic market price storage tests (AC: #2)
    #[test]
    fn test_dynamic_market_price_storage() {
        use arbitrage_core::symbol_to_pair_id;

        let aggregator = PriceAggregator::new();

        // Test storing a dynamically subscribed market (DOGE)
        let doge_pair_id = symbol_to_pair_id("DOGE");
        let tick = create_test_tick(Exchange::Binance, doge_pair_id, 0.12345);
        aggregator.update(tick);

        // Verify price is stored and retrievable
        let stored = aggregator.get_price(Exchange::Binance, doge_pair_id);
        assert!(stored.is_some());
        assert!((stored.unwrap().price().to_f64() - 0.12345).abs() < 0.00001);
    }

    #[test]
    fn test_dynamic_market_price_retrieval_by_pair_id() {
        use arbitrage_core::symbol_to_pair_id;

        let aggregator = PriceAggregator::new();

        // Store prices for multiple dynamic markets
        let xrp_pair_id = symbol_to_pair_id("XRP");
        let shib_pair_id = symbol_to_pair_id("SHIB");

        aggregator.update(create_test_tick(Exchange::Binance, xrp_pair_id, 0.55));
        aggregator.update(create_test_tick(Exchange::Upbit, xrp_pair_id, 0.56));
        aggregator.update(create_test_tick(Exchange::Binance, shib_pair_id, 0.00001));

        // get_price with correct pair_id works
        let xrp_binance = aggregator.get_price(Exchange::Binance, xrp_pair_id);
        assert!(xrp_binance.is_some());
        assert!((xrp_binance.unwrap().price().to_f64() - 0.55).abs() < 0.001);

        let xrp_upbit = aggregator.get_price(Exchange::Upbit, xrp_pair_id);
        assert!(xrp_upbit.is_some());
        assert!((xrp_upbit.unwrap().price().to_f64() - 0.56).abs() < 0.001);

        // Different pair_id returns different prices
        let shib = aggregator.get_price(Exchange::Binance, shib_pair_id);
        assert!(shib.is_some());
        assert!((shib.unwrap().price().to_f64() - 0.00001).abs() < 0.000001);
    }

    #[test]
    fn test_dynamic_market_concurrent_multi_exchange_update() {
        use arbitrage_core::symbol_to_pair_id;
        use std::sync::Arc;
        use std::thread;

        let aggregator = Arc::new(PriceAggregator::new());
        let doge_pair_id = symbol_to_pair_id("DOGE");

        // Simulate concurrent updates from multiple exchanges
        let handles: Vec<_> = vec![
            (Exchange::Binance, 0.123),
            (Exchange::Coinbase, 0.124),
            (Exchange::Upbit, 0.125),
            (Exchange::Bybit, 0.126),
        ]
        .into_iter()
        .map(|(exchange, price)| {
            let agg = Arc::clone(&aggregator);
            thread::spawn(move || {
                let tick = PriceTick::new(
                    exchange,
                    doge_pair_id,
                    FixedPoint::from_f64(price),
                    FixedPoint::from_f64(price - 0.001),
                    FixedPoint::from_f64(price + 0.001),
                );
                agg.update(tick);
            })
        })
        .collect();

        for h in handles {
            h.join().unwrap();
        }

        // Verify all exchanges have their prices stored correctly
        let all_prices = aggregator.get_all_prices_for_pair(doge_pair_id);
        assert_eq!(all_prices.len(), 4);

        // Verify each exchange has correct price
        for tick in &all_prices {
            match tick.exchange() {
                Exchange::Binance => assert!((tick.price().to_f64() - 0.123).abs() < 0.001),
                Exchange::Coinbase => assert!((tick.price().to_f64() - 0.124).abs() < 0.001),
                Exchange::Upbit => assert!((tick.price().to_f64() - 0.125).abs() < 0.001),
                Exchange::Bybit => assert!((tick.price().to_f64() - 0.126).abs() < 0.001),
                _ => panic!("Unexpected exchange"),
            }
        }
    }

    #[test]
    fn test_dynamic_market_price_update_overwrites() {
        use arbitrage_core::symbol_to_pair_id;

        let aggregator = PriceAggregator::new();
        let pair_id = symbol_to_pair_id("DOGE");

        // First update
        aggregator.update(create_test_tick(Exchange::Binance, pair_id, 0.10));
        let first = aggregator.get_price(Exchange::Binance, pair_id).unwrap();
        assert!((first.price().to_f64() - 0.10).abs() < 0.001);

        // Second update should overwrite
        aggregator.update(create_test_tick(Exchange::Binance, pair_id, 0.15));
        let second = aggregator.get_price(Exchange::Binance, pair_id).unwrap();
        assert!((second.price().to_f64() - 0.15).abs() < 0.001);

        // Should still only have 1 entry for this pair/exchange
        assert_eq!(aggregator.len(), 1);
    }
}

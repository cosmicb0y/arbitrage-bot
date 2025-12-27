//! Premium calculation across exchange pairs.
//!
//! Calculates and tracks arbitrage premiums between all exchange pairs.

use arbitrage_core::{Exchange, FixedPoint};
use std::collections::HashMap;

/// Premium calculation configuration.
#[derive(Debug, Clone)]
pub struct PremiumConfig {
    /// Minimum premium in basis points to consider profitable.
    pub min_premium_bps: i32,
    /// Maximum age of price data before considering it stale (ms).
    pub max_staleness_ms: u64,
    /// Trading fee in basis points per trade.
    pub trading_fee_bps: i32,
    /// Estimated gas cost in basis points.
    pub gas_cost_bps: i32,
}

impl Default for PremiumConfig {
    fn default() -> Self {
        Self {
            min_premium_bps: 30,     // 0.3%
            max_staleness_ms: 5000,  // 5 seconds
            trading_fee_bps: 10,     // 0.1% per trade
            gas_cost_bps: 5,         // 0.05%
        }
    }
}

impl PremiumConfig {
    /// Check if a premium is profitable after costs.
    pub fn is_profitable(&self, premium_bps: i32) -> bool {
        premium_bps >= self.min_premium_bps
    }

    /// Calculate net profit after fees.
    pub fn net_profit_bps(&self, gross_premium_bps: i32) -> i32 {
        gross_premium_bps - (2 * self.trading_fee_bps) - self.gas_cost_bps
    }
}

/// Premium entry for a single exchange.
#[derive(Debug, Clone, Copy)]
struct PriceEntry {
    price: FixedPoint,
    timestamp_ms: u64,
}

/// Premium matrix for calculating arbitrage between exchanges.
#[derive(Debug, Clone)]
pub struct PremiumMatrix {
    pair_id: u32,
    prices: HashMap<u16, PriceEntry>, // exchange_id -> price
}

impl PremiumMatrix {
    /// Create a new premium matrix for a trading pair.
    pub fn new(pair_id: u32) -> Self {
        Self {
            pair_id,
            prices: HashMap::new(),
        }
    }

    /// Get the pair ID.
    pub fn pair_id(&self) -> u32 {
        self.pair_id
    }

    /// Check if the matrix has no prices.
    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }

    /// Get the number of exchanges with prices.
    pub fn exchange_count(&self) -> usize {
        self.prices.len()
    }

    /// Update price for an exchange.
    pub fn update_price(&mut self, exchange: Exchange, price: FixedPoint) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.prices.insert(
            exchange as u16,
            PriceEntry {
                price,
                timestamp_ms: now,
            },
        );
    }

    /// Get price for an exchange.
    pub fn get_price(&self, exchange: Exchange) -> Option<FixedPoint> {
        self.prices.get(&(exchange as u16)).map(|e| e.price)
    }

    /// Calculate premium between buy and sell exchanges.
    /// Returns basis points: (sell - buy) / buy * 10000
    pub fn get_premium(&self, buy_exchange: Exchange, sell_exchange: Exchange) -> Option<i32> {
        let buy_price = self.get_price(buy_exchange)?;
        let sell_price = self.get_price(sell_exchange)?;
        Some(FixedPoint::premium_bps(buy_price, sell_price))
    }

    /// Find the best arbitrage opportunity.
    /// Returns (buy_exchange, sell_exchange, premium_bps).
    pub fn best_opportunity(&self) -> Option<(Exchange, Exchange, i32)> {
        if self.prices.len() < 2 {
            return None;
        }

        let mut best: Option<(Exchange, Exchange, i32)> = None;

        for (&buy_id, buy_entry) in &self.prices {
            for (&sell_id, sell_entry) in &self.prices {
                if buy_id == sell_id {
                    continue;
                }

                let premium = FixedPoint::premium_bps(buy_entry.price, sell_entry.price);

                if best.is_none() || premium > best.as_ref().unwrap().2 {
                    let buy_ex = Exchange::from_id(buy_id)?;
                    let sell_ex = Exchange::from_id(sell_id)?;
                    best = Some((buy_ex, sell_ex, premium));
                }
            }
        }

        best
    }

    /// Get all premium pairs.
    /// Returns Vec<(buy_exchange, sell_exchange, premium_bps)>.
    pub fn all_premiums(&self) -> Vec<(Exchange, Exchange, i32)> {
        let mut result = Vec::new();

        for (&buy_id, buy_entry) in &self.prices {
            for (&sell_id, sell_entry) in &self.prices {
                if buy_id == sell_id {
                    continue;
                }

                if let (Some(buy_ex), Some(sell_ex)) = (
                    Exchange::from_id(buy_id),
                    Exchange::from_id(sell_id),
                ) {
                    let premium = FixedPoint::premium_bps(buy_entry.price, sell_entry.price);
                    result.push((buy_ex, sell_ex, premium));
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrage_core::PriceTick;

    fn create_tick(exchange: Exchange, price: f64) -> PriceTick {
        PriceTick::new(
            exchange,
            1, // pair_id
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(price - 1.0),
            FixedPoint::from_f64(price + 1.0),
        )
    }

    #[test]
    fn test_premium_matrix_new() {
        let matrix = PremiumMatrix::new(1); // pair_id = 1
        assert_eq!(matrix.pair_id(), 1);
        assert!(matrix.is_empty());
    }

    #[test]
    fn test_premium_matrix_update() {
        let mut matrix = PremiumMatrix::new(1);

        matrix.update_price(Exchange::Binance, FixedPoint::from_f64(50000.0));
        matrix.update_price(Exchange::Coinbase, FixedPoint::from_f64(50500.0));

        assert!(!matrix.is_empty());
        assert_eq!(matrix.exchange_count(), 2);
    }

    #[test]
    fn test_premium_matrix_get_premium() {
        let mut matrix = PremiumMatrix::new(1);

        matrix.update_price(Exchange::Binance, FixedPoint::from_f64(50000.0));
        matrix.update_price(Exchange::Coinbase, FixedPoint::from_f64(50500.0));

        // Buy at Binance ($50,000), sell at Coinbase ($50,500)
        // Premium = (50500 - 50000) / 50000 * 10000 = 100 bps
        let premium = matrix.get_premium(Exchange::Binance, Exchange::Coinbase);
        assert_eq!(premium, Some(100));

        // Reverse: negative premium
        let premium = matrix.get_premium(Exchange::Coinbase, Exchange::Binance);
        assert!(premium.unwrap() < 0);
    }

    #[test]
    fn test_premium_matrix_best_pair() {
        let mut matrix = PremiumMatrix::new(1);

        matrix.update_price(Exchange::Binance, FixedPoint::from_f64(50000.0));
        matrix.update_price(Exchange::Coinbase, FixedPoint::from_f64(50500.0));
        matrix.update_price(Exchange::Kraken, FixedPoint::from_f64(49800.0));

        let (buy, sell, premium) = matrix.best_opportunity().unwrap();

        // Best: Buy at Kraken ($49,800), sell at Coinbase ($50,500)
        assert_eq!(buy, Exchange::Kraken);
        assert_eq!(sell, Exchange::Coinbase);
        assert!(premium > 100); // > 1%
    }

    #[test]
    fn test_premium_matrix_all_premiums() {
        let mut matrix = PremiumMatrix::new(1);

        matrix.update_price(Exchange::Binance, FixedPoint::from_f64(50000.0));
        matrix.update_price(Exchange::Coinbase, FixedPoint::from_f64(50500.0));
        matrix.update_price(Exchange::Kraken, FixedPoint::from_f64(49800.0));

        let premiums = matrix.all_premiums();
        // 3 exchanges = 3 * 2 = 6 pairs (buy/sell combinations)
        assert_eq!(premiums.len(), 6);
    }

    #[test]
    fn test_premium_config() {
        let config = PremiumConfig::default();
        assert!(config.min_premium_bps > 0);
        assert!(config.max_staleness_ms > 0);
    }

    #[test]
    fn test_premium_config_is_profitable() {
        let config = PremiumConfig {
            min_premium_bps: 50,
            ..Default::default()
        };

        assert!(config.is_profitable(100)); // 100 bps > 50 bps
        assert!(!config.is_profitable(30)); // 30 bps < 50 bps
    }
}

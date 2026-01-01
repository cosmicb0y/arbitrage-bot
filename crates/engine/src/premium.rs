//! Premium calculation across exchange pairs.
//!
//! Calculates and tracks arbitrage premiums between all exchange pairs.

use arbitrage_core::{Exchange, FixedPoint, QuoteCurrency};
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
    /// Best bid price (highest buy order)
    bid: FixedPoint,
    /// Best ask price (lowest sell order)
    ask: FixedPoint,
    /// Best bid size (quantity available at best bid)
    bid_size: FixedPoint,
    /// Best ask size (quantity available at best ask)
    ask_size: FixedPoint,
    timestamp_ms: u64,
    quote: QuoteCurrency,
}

/// Premium matrix for calculating arbitrage between exchanges.
#[derive(Debug, Clone)]
pub struct PremiumMatrix {
    pair_id: u32,
    /// Key: (exchange_id, quote_currency_id) to differentiate USDT vs USDC markets
    prices: HashMap<(u16, u8), PriceEntry>,
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

    /// Update price for an exchange with default quote currency (USD).
    pub fn update_price(&mut self, exchange: Exchange, price: FixedPoint) {
        self.update_price_with_quote(exchange, price, QuoteCurrency::USD);
    }

    /// Update price for an exchange with specified quote currency.
    /// Uses mid price as bid/ask when only price is available.
    pub fn update_price_with_quote(&mut self, exchange: Exchange, price: FixedPoint, quote: QuoteCurrency) {
        self.update_price_with_bid_ask(exchange, price, price, price, FixedPoint::from_f64(0.0), FixedPoint::from_f64(0.0), quote);
    }

    /// Update price for an exchange with bid/ask from orderbook (no size info).
    /// This enables accurate premium calculation using best bid/ask.
    pub fn update_price_with_bid_ask(
        &mut self,
        exchange: Exchange,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        quote: QuoteCurrency,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Use (exchange_id, quote_id) as key to differentiate USDT vs USDC markets
        self.prices.insert(
            (exchange as u16, quote as u8),
            PriceEntry {
                price,
                bid,
                ask,
                bid_size,
                ask_size,
                timestamp_ms: now,
                quote,
            },
        );
    }

    /// Get price for an exchange (returns first matching price regardless of quote).
    /// For multi-quote support, use get_price_with_quote instead.
    pub fn get_price(&self, exchange: Exchange) -> Option<FixedPoint> {
        self.prices.iter()
            .find(|(&(ex_id, _), _)| ex_id == exchange as u16)
            .map(|(_, entry)| entry.price)
    }

    /// Get price for an exchange with specific quote currency.
    pub fn get_price_with_quote(&self, exchange: Exchange, quote: QuoteCurrency) -> Option<FixedPoint> {
        self.prices.get(&(exchange as u16, quote as u8)).map(|e| e.price)
    }

    /// Get bid price for an exchange with specific quote currency.
    pub fn get_bid_with_quote(&self, exchange: Exchange, quote: QuoteCurrency) -> Option<FixedPoint> {
        self.prices.get(&(exchange as u16, quote as u8)).map(|e| e.bid)
    }

    /// Get ask price for an exchange with specific quote currency.
    pub fn get_ask_with_quote(&self, exchange: Exchange, quote: QuoteCurrency) -> Option<FixedPoint> {
        self.prices.get(&(exchange as u16, quote as u8)).map(|e| e.ask)
    }

    /// Get quote currency for an exchange (returns first matching quote).
    pub fn get_quote(&self, exchange: Exchange) -> Option<QuoteCurrency> {
        self.prices.iter()
            .find(|(&(ex_id, _), _)| ex_id == exchange as u16)
            .map(|(_, entry)| entry.quote)
    }

    /// Calculate premium between buy and sell exchanges using orderbook prices.
    /// Uses ask price for buying (lowest sell order) and bid price for selling (highest buy order).
    /// Returns basis points: (sell_bid - buy_ask) / buy_ask * 10000
    pub fn get_premium(&self, buy_exchange: Exchange, sell_exchange: Exchange) -> Option<i32> {
        // For buying: use ask price (lowest price someone is willing to sell)
        // For selling: use bid price (highest price someone is willing to buy)
        let buy_entry = self.prices.iter()
            .find(|(&(ex_id, _), _)| ex_id == buy_exchange as u16)
            .map(|(_, e)| e)?;
        let sell_entry = self.prices.iter()
            .find(|(&(ex_id, _), _)| ex_id == sell_exchange as u16)
            .map(|(_, e)| e)?;

        // Premium = (sell_bid - buy_ask) / buy_ask * 10000
        Some(FixedPoint::premium_bps(buy_entry.ask, sell_entry.bid))
    }

    /// Find the best arbitrage opportunity using orderbook prices.
    /// Uses ask price for buying and bid price for selling.
    /// Returns (buy_exchange, sell_exchange, premium_bps).
    pub fn best_opportunity(&self) -> Option<(Exchange, Exchange, i32)> {
        if self.prices.len() < 2 {
            return None;
        }

        let mut best: Option<(Exchange, Exchange, i32)> = None;

        for (&(buy_ex_id, _buy_quote_id), buy_entry) in &self.prices {
            for (&(sell_ex_id, _sell_quote_id), sell_entry) in &self.prices {
                // Skip same exchange AND same quote (same market)
                if buy_ex_id == sell_ex_id && buy_entry.quote == sell_entry.quote {
                    continue;
                }

                // Premium using orderbook: buy at ask, sell at bid
                let premium = FixedPoint::premium_bps(buy_entry.ask, sell_entry.bid);

                if best.is_none() || premium > best.as_ref().unwrap().2 {
                    let buy_ex = Exchange::from_id(buy_ex_id)?;
                    let sell_ex = Exchange::from_id(sell_ex_id)?;
                    best = Some((buy_ex, sell_ex, premium));
                }
            }
        }

        best
    }

    /// Get all premium pairs.
    /// Returns Vec<(buy_exchange, sell_exchange, premium_bps)>.
    pub fn all_premiums(&self) -> Vec<(Exchange, Exchange, i32)> {
        self.all_premiums_with_quotes()
            .into_iter()
            .map(|(buy_ex, sell_ex, _, _, premium)| (buy_ex, sell_ex, premium))
            .collect()
    }

    /// Get all premium pairs with quote currencies using orderbook prices.
    /// Uses ask price for buying and bid price for selling.
    /// Returns Vec<(buy_exchange, sell_exchange, buy_quote, sell_quote, premium_bps)>.
    pub fn all_premiums_with_quotes(&self) -> Vec<(Exchange, Exchange, QuoteCurrency, QuoteCurrency, i32)> {
        let mut result = Vec::new();

        for (&(buy_ex_id, _buy_quote_id), buy_entry) in &self.prices {
            for (&(sell_ex_id, _sell_quote_id), sell_entry) in &self.prices {
                // Skip same exchange AND same quote (same market)
                if buy_ex_id == sell_ex_id && buy_entry.quote == sell_entry.quote {
                    continue;
                }

                if let (Some(buy_ex), Some(sell_ex)) = (
                    Exchange::from_id(buy_ex_id),
                    Exchange::from_id(sell_ex_id),
                ) {
                    // Premium using orderbook: buy at ask, sell at bid
                    let premium = FixedPoint::premium_bps(buy_entry.ask, sell_entry.bid);
                    result.push((buy_ex, sell_ex, buy_entry.quote, sell_entry.quote, premium));
                }
            }
        }

        result
    }

    /// Get all premium pairs with quote currencies and bid/ask prices.
    /// Returns Vec<(buy_exchange, sell_exchange, buy_quote, sell_quote, buy_ask, sell_bid, premium_bps)>.
    pub fn all_premiums_with_bid_ask(&self) -> Vec<(Exchange, Exchange, QuoteCurrency, QuoteCurrency, FixedPoint, FixedPoint, i32)> {
        self.all_premiums_with_depth()
            .into_iter()
            .map(|(buy_ex, sell_ex, buy_quote, sell_quote, buy_ask, sell_bid, _, _, premium)| {
                (buy_ex, sell_ex, buy_quote, sell_quote, buy_ask, sell_bid, premium)
            })
            .collect()
    }

    /// Get all premium pairs with quote currencies, bid/ask prices, and orderbook depth.
    /// Returns Vec<(buy_exchange, sell_exchange, buy_quote, sell_quote, buy_ask, sell_bid, buy_ask_size, sell_bid_size, premium_bps)>.
    pub fn all_premiums_with_depth(&self) -> Vec<(Exchange, Exchange, QuoteCurrency, QuoteCurrency, FixedPoint, FixedPoint, FixedPoint, FixedPoint, i32)> {
        let mut result = Vec::new();

        for (&(buy_ex_id, _buy_quote_id), buy_entry) in &self.prices {
            for (&(sell_ex_id, _sell_quote_id), sell_entry) in &self.prices {
                // Skip same exchange AND same quote (same market)
                if buy_ex_id == sell_ex_id && buy_entry.quote == sell_entry.quote {
                    continue;
                }

                if let (Some(buy_ex), Some(sell_ex)) = (
                    Exchange::from_id(buy_ex_id),
                    Exchange::from_id(sell_ex_id),
                ) {
                    // Premium using orderbook: buy at ask, sell at bid
                    let premium = FixedPoint::premium_bps(buy_entry.ask, sell_entry.bid);
                    result.push((
                        buy_ex, sell_ex,
                        buy_entry.quote, sell_entry.quote,
                        buy_entry.ask, sell_entry.bid,
                        buy_entry.ask_size, sell_entry.bid_size,
                        premium
                    ));
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

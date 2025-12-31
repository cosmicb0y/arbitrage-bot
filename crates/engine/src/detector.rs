//! Arbitrage opportunity detector.
//!
//! Monitors price feeds and detects profitable arbitrage opportunities.

use arbitrage_core::{ArbitrageOpportunity, Asset, Chain, Exchange, FixedPoint, QuoteCurrency};
use crate::PremiumMatrix;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static OPPORTUNITY_ID: AtomicU64 = AtomicU64::new(1);

/// Configuration for the opportunity detector.
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Minimum premium in basis points to detect.
    pub min_premium_bps: i32,
    /// Maximum age of price data (ms).
    pub max_staleness_ms: u64,
    /// Enabled exchanges.
    pub enabled_exchanges: Vec<Exchange>,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            min_premium_bps: 30,
            max_staleness_ms: 5000,
            enabled_exchanges: vec![
                Exchange::Binance,
                Exchange::Coinbase,
                Exchange::Kraken,
                Exchange::Okx,
                Exchange::Bybit,
            ],
        }
    }
}

/// Get asset for a pair_id using the symbol registry.
fn asset_for_pair_id(pair_id: u32, symbol_registry: &HashMap<u32, String>) -> Asset {
    // First check the dynamic symbol registry
    if let Some(symbol) = symbol_registry.get(&pair_id) {
        return Asset::from_symbol(symbol);
    }

    // Fallback to legacy hardcoded pair_ids for backwards compatibility
    match pair_id {
        1 => Asset::btc(),
        2 => Asset::eth(),
        3 => Asset::sol(),
        _ => Asset::native("UNKNOWN", Chain::Ethereum, 18),
    }
}

/// Opportunity detector that monitors prices and detects arbitrage.
#[derive(Debug)]
pub struct OpportunityDetector {
    config: DetectorConfig,
    matrices: HashMap<u32, PremiumMatrix>,
    detected: Vec<ArbitrageOpportunity>,
    /// Maps pair_id -> symbol for dynamic markets
    symbol_registry: HashMap<u32, String>,
}

impl OpportunityDetector {
    /// Create a new detector with the given configuration.
    pub fn new(config: DetectorConfig) -> Self {
        Self {
            config,
            matrices: HashMap::new(),
            detected: Vec::new(),
            symbol_registry: HashMap::new(),
        }
    }

    /// Register a symbol with its pair_id.
    /// This enables opportunity detection for dynamic markets.
    pub fn register_symbol(&mut self, symbol: &str) -> u32 {
        let pair_id = arbitrage_core::symbol_to_pair_id(symbol);
        self.symbol_registry.insert(pair_id, symbol.to_string());
        pair_id
    }

    /// Get the pair_id for a symbol, registering it if needed.
    pub fn get_or_register_pair_id(&mut self, symbol: &str) -> u32 {
        let pair_id = arbitrage_core::symbol_to_pair_id(symbol);
        if !self.symbol_registry.contains_key(&pair_id) {
            self.symbol_registry.insert(pair_id, symbol.to_string());
        }
        pair_id
    }

    /// Get all registered pair_ids (from both symbol registry and matrices).
    pub fn registered_pair_ids(&self) -> Vec<u32> {
        let mut pair_ids: Vec<u32> = self.symbol_registry.keys().copied().collect();
        // Also include matrices pair_ids that might not be in symbol_registry
        for &pair_id in self.matrices.keys() {
            if !pair_ids.contains(&pair_id) {
                pair_ids.push(pair_id);
            }
        }
        pair_ids
    }

    /// Get detected opportunities.
    pub fn opportunities(&self) -> &[ArbitrageOpportunity] {
        &self.detected
    }

    /// Update price for an exchange/pair with default quote (USD).
    pub fn update_price(&mut self, exchange: Exchange, pair_id: u32, price: FixedPoint) {
        self.update_price_with_quote(exchange, pair_id, price, QuoteCurrency::USD);
    }

    /// Update price for an exchange/pair with specified quote currency.
    pub fn update_price_with_quote(&mut self, exchange: Exchange, pair_id: u32, price: FixedPoint, quote: QuoteCurrency) {
        let matrix = self.matrices
            .entry(pair_id)
            .or_insert_with(|| PremiumMatrix::new(pair_id));
        matrix.update_price_with_quote(exchange, price, quote);
    }

    /// Detect opportunities for a specific pair.
    pub fn detect(&mut self, pair_id: u32) -> Vec<ArbitrageOpportunity> {
        self.detect_with_rates(pair_id, None, None)
    }

    /// Detect opportunities for a specific pair with exchange rates for kimchi/tether premium.
    /// - `usd_krw_rate`: USD/KRW exchange rate (e.g., 1450.0)
    /// - `usdt_krw_rate`: USDT/KRW rate from Korean exchange (e.g., 1455.0)
    pub fn detect_with_rates(
        &mut self,
        pair_id: u32,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
    ) -> Vec<ArbitrageOpportunity> {
        let Some(matrix) = self.matrices.get(&pair_id) else {
            return Vec::new();
        };

        let mut opportunities = Vec::new();
        let asset = asset_for_pair_id(pair_id, &self.symbol_registry);

        // Get all profitable pairs with quote currencies
        let premiums = matrix.all_premiums_with_quotes();

        for (buy_ex, sell_ex, buy_quote, sell_quote, premium) in premiums {
            if premium >= self.config.min_premium_bps {
                let buy_price = matrix.get_price_with_quote(buy_ex, buy_quote).unwrap_or(FixedPoint(0));
                let sell_price = matrix.get_price_with_quote(sell_ex, sell_quote).unwrap_or(FixedPoint(0));

                let opp = ArbitrageOpportunity::with_quotes_and_rates(
                    OPPORTUNITY_ID.fetch_add(1, Ordering::SeqCst),
                    buy_ex,
                    sell_ex,
                    buy_quote,
                    sell_quote,
                    asset.clone(),
                    buy_price,
                    sell_price,
                    usd_krw_rate,
                    usdt_krw_rate,
                );

                opportunities.push(opp);
            }
        }

        // Sort by premium descending
        opportunities.sort_by(|a, b| b.premium_bps.cmp(&a.premium_bps));

        // Store the best opportunity
        if let Some(best) = opportunities.first().cloned() {
            self.detected.push(best);
        }

        opportunities
    }

    /// Detect opportunities for all tracked pairs.
    pub fn detect_all(&mut self) -> Vec<ArbitrageOpportunity> {
        self.detect_all_with_rates(None, None)
    }

    /// Detect opportunities for all tracked pairs with exchange rates.
    pub fn detect_all_with_rates(
        &mut self,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
    ) -> Vec<ArbitrageOpportunity> {
        let pair_ids: Vec<u32> = self.matrices.keys().copied().collect();
        let mut all_opportunities = Vec::new();

        for pair_id in pair_ids {
            all_opportunities.extend(self.detect_with_rates(pair_id, usd_krw_rate, usdt_krw_rate));
        }

        all_opportunities
    }

    /// Clear detected opportunities.
    pub fn clear(&mut self) {
        self.detected.clear();
    }

    /// Get the premium matrix for a pair.
    pub fn matrix(&self, pair_id: u32) -> Option<&PremiumMatrix> {
        self.matrices.get(&pair_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_config_default() {
        let config = DetectorConfig::default();
        assert!(config.min_premium_bps > 0);
        assert!(config.max_staleness_ms > 0);
        assert!(!config.enabled_exchanges.is_empty());
    }

    #[test]
    fn test_detector_new() {
        let config = DetectorConfig::default();
        let detector = OpportunityDetector::new(config);
        assert!(detector.opportunities().is_empty());
    }

    #[test]
    fn test_detector_process_price() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let mut detector = OpportunityDetector::new(config);

        // Add prices with 1% spread (100 bps)
        detector.update_price(Exchange::Binance, 1, FixedPoint::from_f64(50000.0));
        detector.update_price(Exchange::Coinbase, 1, FixedPoint::from_f64(50500.0));

        let opps = detector.detect(1);
        assert!(!opps.is_empty());
    }

    #[test]
    fn test_detector_no_opportunity_below_threshold() {
        let config = DetectorConfig {
            min_premium_bps: 200, // 2% threshold
            ..Default::default()
        };
        let mut detector = OpportunityDetector::new(config);

        // Add prices with only 1% spread (100 bps)
        detector.update_price(Exchange::Binance, 1, FixedPoint::from_f64(50000.0));
        detector.update_price(Exchange::Coinbase, 1, FixedPoint::from_f64(50500.0));

        let opps = detector.detect(1);
        assert!(opps.is_empty()); // Below threshold
    }

    #[test]
    fn test_detector_opportunity_details() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let mut detector = OpportunityDetector::new(config);

        detector.update_price(Exchange::Binance, 1, FixedPoint::from_f64(50000.0));
        detector.update_price(Exchange::Coinbase, 1, FixedPoint::from_f64(50500.0));

        let opps = detector.detect(1);
        let opp = &opps[0];

        assert_eq!(opp.source_exchange, Exchange::Binance);
        assert_eq!(opp.target_exchange, Exchange::Coinbase);
        assert_eq!(opp.premium_bps, 100);
    }

    #[test]
    fn test_detector_multiple_pairs() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let mut detector = OpportunityDetector::new(config);

        // Pair 1: BTC
        detector.update_price(Exchange::Binance, 1, FixedPoint::from_f64(50000.0));
        detector.update_price(Exchange::Coinbase, 1, FixedPoint::from_f64(50500.0));

        // Pair 2: ETH
        detector.update_price(Exchange::Binance, 2, FixedPoint::from_f64(3000.0));
        detector.update_price(Exchange::Coinbase, 2, FixedPoint::from_f64(3050.0));

        let btc_opps = detector.detect(1);
        let eth_opps = detector.detect(2);

        assert!(!btc_opps.is_empty());
        assert!(!eth_opps.is_empty());
    }

    #[test]
    fn test_detector_dynamic_symbol_registration() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let mut detector = OpportunityDetector::new(config);

        // Register a dynamic symbol
        let pair_id = detector.register_symbol("DOGE");
        assert!(pair_id > 0);

        // Update prices for the dynamic symbol
        detector.update_price(Exchange::Binance, pair_id, FixedPoint::from_f64(0.10));
        detector.update_price(Exchange::Coinbase, pair_id, FixedPoint::from_f64(0.102));

        let opps = detector.detect(pair_id);
        assert!(!opps.is_empty());

        // Verify the asset symbol is correct
        assert_eq!(opps[0].asset.symbol.as_str(), "DOGE");
    }

    #[test]
    fn test_detector_get_or_register_pair_id() {
        let config = DetectorConfig::default();
        let mut detector = OpportunityDetector::new(config);

        // First call should register
        let pair_id1 = detector.get_or_register_pair_id("XRP");

        // Second call should return same pair_id
        let pair_id2 = detector.get_or_register_pair_id("XRP");

        assert_eq!(pair_id1, pair_id2);

        // Different symbol should get different pair_id
        let pair_id3 = detector.get_or_register_pair_id("ADA");
        assert_ne!(pair_id1, pair_id3);
    }
}

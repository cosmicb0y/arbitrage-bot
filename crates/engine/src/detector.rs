//! Arbitrage opportunity detector.
//!
//! Monitors price feeds and detects profitable arbitrage opportunities.

use arbitrage_core::{ArbitrageOpportunity, Asset, Chain, Exchange, FixedPoint};
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
    /// Default asset for opportunities.
    pub default_asset: Asset,
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
            default_asset: Asset::native("BTC", Chain::Ethereum, 8),
        }
    }
}

/// Opportunity detector that monitors prices and detects arbitrage.
#[derive(Debug)]
pub struct OpportunityDetector {
    config: DetectorConfig,
    matrices: HashMap<u32, PremiumMatrix>,
    detected: Vec<ArbitrageOpportunity>,
}

impl OpportunityDetector {
    /// Create a new detector with the given configuration.
    pub fn new(config: DetectorConfig) -> Self {
        Self {
            config,
            matrices: HashMap::new(),
            detected: Vec::new(),
        }
    }

    /// Get detected opportunities.
    pub fn opportunities(&self) -> &[ArbitrageOpportunity] {
        &self.detected
    }

    /// Update price for an exchange/pair.
    pub fn update_price(&mut self, exchange: Exchange, pair_id: u32, price: FixedPoint) {
        let matrix = self.matrices
            .entry(pair_id)
            .or_insert_with(|| PremiumMatrix::new(pair_id));
        matrix.update_price(exchange, price);
    }

    /// Detect opportunities for a specific pair.
    pub fn detect(&mut self, pair_id: u32) -> Vec<ArbitrageOpportunity> {
        let Some(matrix) = self.matrices.get(&pair_id) else {
            return Vec::new();
        };

        let mut opportunities = Vec::new();

        // Get all profitable pairs
        let premiums = matrix.all_premiums();
        for (buy_ex, sell_ex, premium) in premiums {
            if premium >= self.config.min_premium_bps {
                let buy_price = matrix.get_price(buy_ex).unwrap_or(FixedPoint(0));
                let sell_price = matrix.get_price(sell_ex).unwrap_or(FixedPoint(0));

                let opp = ArbitrageOpportunity::new(
                    OPPORTUNITY_ID.fetch_add(1, Ordering::SeqCst),
                    buy_ex,
                    sell_ex,
                    self.config.default_asset.clone(),
                    buy_price,
                    sell_price,
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
        let pair_ids: Vec<u32> = self.matrices.keys().copied().collect();
        let mut all_opportunities = Vec::new();

        for pair_id in pair_ids {
            all_opportunities.extend(self.detect(pair_id));
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
}

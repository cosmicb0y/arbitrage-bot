//! Arbitrage opportunity detector.
//!
//! Monitors price feeds and detects profitable arbitrage opportunities.
//! Uses lock-free data structures (DashMap) for real-time performance.

use arbitrage_core::{ArbitrageOpportunity, Asset, Chain, Exchange, FixedPoint, QuoteCurrency, UsdlikePremium, UsdlikeQuote};
use crate::{ConversionRates, PremiumMatrix};
use dashmap::DashMap;
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
            max_staleness_ms: 0, // Disabled - prices are managed by WebSocket reconnection logic
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

/// Get asset for a pair_id using the symbol registry (DashMap version).
fn asset_for_pair_id_dashmap(pair_id: u32, symbol_registry: &DashMap<u32, String>) -> Asset {
    if let Some(symbol) = symbol_registry.get(&pair_id) {
        return Asset::from_symbol(&symbol);
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
/// Uses lock-free DashMap for concurrent price updates without blocking.
pub struct OpportunityDetector {
    config: DetectorConfig,
    /// Per-pair price matrices (lock-free concurrent access)
    matrices: DashMap<u32, PremiumMatrix>,
    /// Maps pair_id -> symbol for dynamic markets (lock-free)
    symbol_registry: DashMap<u32, String>,
}

impl std::fmt::Debug for OpportunityDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpportunityDetector")
            .field("config", &self.config)
            .field("matrices_count", &self.matrices.len())
            .field("symbol_registry_count", &self.symbol_registry.len())
            .finish()
    }
}

impl OpportunityDetector {
    /// Create a new detector with the given configuration.
    pub fn new(config: DetectorConfig) -> Self {
        Self {
            config,
            matrices: DashMap::new(),
            symbol_registry: DashMap::new(),
        }
    }

    /// Register a symbol with its pair_id (lock-free).
    /// This enables opportunity detection for dynamic markets.
    pub fn register_symbol(&self, symbol: &str) -> u32 {
        let pair_id = arbitrage_core::symbol_to_pair_id(symbol);
        self.symbol_registry.insert(pair_id, symbol.to_string());
        pair_id
    }

    /// Get the pair_id for a symbol, registering it if needed (lock-free).
    pub fn get_or_register_pair_id(&self, symbol: &str) -> u32 {
        let pair_id = arbitrage_core::symbol_to_pair_id(symbol);
        self.symbol_registry.entry(pair_id).or_insert_with(|| symbol.to_string());
        pair_id
    }

    /// Get all registered pair_ids (from both symbol registry and matrices).
    /// Uses DashMap iter which holds read locks briefly per shard.
    pub fn registered_pair_ids(&self) -> Vec<u32> {
        // Collect symbol registry keys first (quick iteration)
        let registry_ids: Vec<u32> = self.symbol_registry.iter().map(|r| *r.key()).collect();

        // Collect matrix keys separately (quick iteration)
        let matrix_ids: Vec<u32> = self.matrices.iter().map(|r| *r.key()).collect();

        // Merge without holding any locks
        let mut pair_ids = registry_ids;
        for pair_id in matrix_ids {
            if !pair_ids.contains(&pair_id) {
                pair_ids.push(pair_id);
            }
        }
        pair_ids
    }

    /// Get symbol for a pair_id from the registry.
    pub fn pair_id_to_symbol(&self, pair_id: u32) -> Option<String> {
        self.symbol_registry.get(&pair_id).map(|r| r.value().clone())
    }

    /// Update price for an exchange/pair with default quote (USD).
    /// Lock-free: uses DashMap entry API for concurrent access.
    pub fn update_price(&self, exchange: Exchange, pair_id: u32, price: FixedPoint) {
        self.update_price_with_quote(exchange, pair_id, price, QuoteCurrency::USD);
    }

    /// Update price for an exchange/pair with specified quote currency (lock-free).
    pub fn update_price_with_quote(&self, exchange: Exchange, pair_id: u32, price: FixedPoint, quote: QuoteCurrency) {
        self.update_price_with_bid_ask(exchange, pair_id, price, price, price, FixedPoint::from_f64(0.0), FixedPoint::from_f64(0.0), quote);
    }

    /// Update price for an exchange/pair with bid/ask from orderbook (lock-free).
    /// This enables accurate premium calculation using best bid/ask prices.
    pub fn update_price_with_bid_ask(
        &self,
        exchange: Exchange,
        pair_id: u32,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        quote: QuoteCurrency,
    ) {
        // DashMap: use get_mut for existing entries, insert only if needed
        // This minimizes lock contention vs entry() API
        if let Some(mut matrix) = self.matrices.get_mut(&pair_id) {
            matrix.update_price_with_bid_ask(exchange, price, bid, ask, bid_size, ask_size, quote);
        } else {
            // Insert new matrix with configured staleness threshold
            let mut matrix = PremiumMatrix::with_staleness(pair_id, self.config.max_staleness_ms);
            matrix.update_price_with_bid_ask(exchange, price, bid, ask, bid_size, ask_size, quote);
            self.matrices.insert(pair_id, matrix);
        }
        tracing::trace!(
            pair_id = pair_id,
            exchange = ?exchange,
            price = price.to_f64(),
            bid = bid.to_f64(),
            ask = ask.to_f64(),
            "detector: price updated"
        );
    }

    /// Update price for an exchange/pair with bid/ask and separate raw prices (lock-free).
    /// Use this for KRW exchanges where raw prices differ from USD-normalized prices.
    pub fn update_price_with_bid_ask_and_raw(
        &self,
        exchange: Exchange,
        pair_id: u32,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        raw_bid: FixedPoint,
        raw_ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        quote: QuoteCurrency,
    ) {
        // DashMap: use get_mut for existing entries, insert only if needed
        // This minimizes lock contention vs entry() API
        if let Some(mut matrix) = self.matrices.get_mut(&pair_id) {
            matrix.update_price_with_bid_ask_and_raw(exchange, price, bid, ask, raw_bid, raw_ask, bid_size, ask_size, quote);
        } else {
            // Insert new matrix with configured staleness threshold
            let mut matrix = PremiumMatrix::with_staleness(pair_id, self.config.max_staleness_ms);
            matrix.update_price_with_bid_ask_and_raw(exchange, price, bid, ask, raw_bid, raw_ask, bid_size, ask_size, quote);
            self.matrices.insert(pair_id, matrix);
        }
        tracing::trace!(
            pair_id = pair_id,
            exchange = ?exchange,
            price = price.to_f64(),
            raw_bid = raw_bid.to_f64(),
            raw_ask = raw_ask.to_f64(),
            "detector: price with raw updated"
        );
    }

    /// Detect opportunities for a specific pair.
    pub fn detect(&self, pair_id: u32) -> Vec<ArbitrageOpportunity> {
        self.detect_with_rates(pair_id, None, None)
    }

    /// Detect opportunities for a specific pair with exchange rates for kimchi/tether premium.
    pub fn detect_with_rates(
        &self,
        pair_id: u32,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
    ) -> Vec<ArbitrageOpportunity> {
        self.detect_with_all_rates(pair_id, usd_krw_rate, usdt_krw_rate, usdt_krw_rate)
    }

    /// Detect opportunities for a specific pair with all exchange rates.
    pub fn detect_with_all_rates(
        &self,
        pair_id: u32,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
        usdc_krw_rate: Option<f64>,
    ) -> Vec<ArbitrageOpportunity> {
        let rates = ConversionRates {
            usdt_usd: 1.0,
            usdc_usd: 1.0,
            usd_krw: usd_krw_rate.unwrap_or(0.0),
            upbit_usdt_krw: usdt_krw_rate.unwrap_or(0.0),
            upbit_usdc_krw: usdc_krw_rate.unwrap_or(0.0),
            bithumb_usdt_krw: usdt_krw_rate.unwrap_or(0.0),
            bithumb_usdc_krw: usdc_krw_rate.unwrap_or(0.0),
        };

        self.detect_with_conversion_rates(pair_id, &rates, usd_krw_rate, usdt_krw_rate, usdc_krw_rate)
    }

    /// Detect opportunities using full ConversionRates (supports per-exchange rates).
    pub fn detect_with_conversion_rates(
        &self,
        pair_id: u32,
        rates: &ConversionRates,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
        usdc_krw_rate: Option<f64>,
    ) -> Vec<ArbitrageOpportunity> {
        let Some(matrix) = self.matrices.get(&pair_id) else {
            tracing::debug!(pair_id = pair_id, "detect: no matrix found for pair_id");
            return Vec::new();
        };

        tracing::debug!(
            pair_id = pair_id,
            matrices_count = self.matrices.len(),
            "detect: found matrix"
        );

        let mut opportunities = Vec::new();
        let asset = asset_for_pair_id_dashmap(pair_id, &self.symbol_registry);

        let premiums = matrix.all_premiums_multi_denomination(rates);

        for (buy_ex, sell_ex, buy_quote, sell_quote, buy_ask, sell_bid, buy_ask_raw, sell_bid_raw, buy_ask_size, sell_bid_size, usdlike_premium_bps, _unused, kimchi_premium, buy_timestamp_ms, sell_timestamp_ms) in premiums {
            if buy_ask_size.0 == 0 && sell_bid_size.0 == 0 {
                continue;
            }

            // Broadcast all opportunities - let client decide what to display
            {
                let usdlike_quote = if buy_quote == QuoteCurrency::KRW {
                    UsdlikeQuote::from_quote_currency(sell_quote)
                } else {
                    UsdlikeQuote::from_quote_currency(buy_quote)
                };

                let usdlike_premium = usdlike_quote.map(|quote| UsdlikePremium {
                    bps: usdlike_premium_bps,
                    quote,
                });

                let mut opp = ArbitrageOpportunity::with_all_rates(
                    OPPORTUNITY_ID.fetch_add(1, Ordering::SeqCst),
                    buy_ex,
                    sell_ex,
                    buy_quote,
                    sell_quote,
                    asset.clone(),
                    buy_ask,
                    sell_bid,
                    usd_krw_rate,
                    usdt_krw_rate,
                    usdc_krw_rate,
                )
                .with_depth(buy_ask_size, sell_bid_size)
                .with_price_timestamps(buy_timestamp_ms, sell_timestamp_ms)
                .with_raw_prices(buy_ask_raw, sell_bid_raw);

                opp.usdlike_premium = usdlike_premium;
                opp.kimchi_premium_bps = kimchi_premium;
                opp.premium_bps = usdlike_premium_bps;

                opportunities.push(opp);
            }
        }

        opportunities.sort_by(|a, b| {
            let a_bps = a.usdlike_premium.map(|p| p.bps).unwrap_or(a.premium_bps);
            let b_bps = b.usdlike_premium.map(|p| p.bps).unwrap_or(b.premium_bps);
            b_bps.cmp(&a_bps)
        });

        opportunities
    }

    /// Detect opportunities for all tracked pairs.
    pub fn detect_all(&self) -> Vec<ArbitrageOpportunity> {
        self.detect_all_with_rates(None, None)
    }

    /// Detect opportunities for all tracked pairs with exchange rates.
    pub fn detect_all_with_rates(
        &self,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
    ) -> Vec<ArbitrageOpportunity> {
        self.detect_all_with_all_rates(usd_krw_rate, usdt_krw_rate, usdt_krw_rate)
    }

    /// Detect opportunities for all tracked pairs with all exchange rates.
    pub fn detect_all_with_all_rates(
        &self,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
        usdc_krw_rate: Option<f64>,
    ) -> Vec<ArbitrageOpportunity> {
        let pair_ids: Vec<u32> = self.matrices.iter().map(|r| *r.key()).collect();
        let mut all_opportunities = Vec::new();

        for pair_id in pair_ids {
            all_opportunities.extend(self.detect_with_all_rates(pair_id, usd_krw_rate, usdt_krw_rate, usdc_krw_rate));
        }

        all_opportunities
    }

    /// Clear all prices for a specific exchange.
    /// Call this on reconnection to avoid using stale cached prices.
    pub fn clear_exchange_prices(&self, exchange: Exchange) {
        for mut entry in self.matrices.iter_mut() {
            entry.value_mut().clear_exchange(exchange);
        }
    }

    /// Expire stale prices from all matrices.
    /// Returns total number of entries removed.
    pub fn expire_stale_prices(&self) -> usize {
        let mut total = 0;
        for mut entry in self.matrices.iter_mut() {
            total += entry.value_mut().expire_stale_prices();
        }
        total
    }

    /// Check if a matrix exists for a pair.
    pub fn has_matrix(&self, pair_id: u32) -> bool {
        self.matrices.contains_key(&pair_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrage_core::QuoteCurrency;

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
        // Detector starts with no matrices
        assert!(!detector.has_matrix(1));
    }

    #[test]
    fn test_detector_process_price() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let mut detector = OpportunityDetector::new(config);

        // Add prices with 1% spread (100 bps) and orderbook depth
        detector.update_price_with_bid_ask(
            Exchange::Binance, 1,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0), FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(1.0), FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase, 1,
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50500.0), FixedPoint::from_f64(50501.0),
            FixedPoint::from_f64(1.0), FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );

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

        // Use bid/ask with depth for proper opportunity detection
        detector.update_price_with_bid_ask(
            Exchange::Binance, 1,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0), FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(1.0), FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase, 1,
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50500.0), FixedPoint::from_f64(50501.0),
            FixedPoint::from_f64(1.0), FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );

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

        // Pair 1: BTC with depth
        detector.update_price_with_bid_ask(
            Exchange::Binance, 1,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0), FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(1.0), FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase, 1,
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50500.0), FixedPoint::from_f64(50501.0),
            FixedPoint::from_f64(1.0), FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );

        // Pair 2: ETH with depth
        detector.update_price_with_bid_ask(
            Exchange::Binance, 2,
            FixedPoint::from_f64(3000.0),
            FixedPoint::from_f64(2999.0), FixedPoint::from_f64(3000.0),
            FixedPoint::from_f64(10.0), FixedPoint::from_f64(10.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase, 2,
            FixedPoint::from_f64(3050.0),
            FixedPoint::from_f64(3050.0), FixedPoint::from_f64(3051.0),
            FixedPoint::from_f64(10.0), FixedPoint::from_f64(10.0),
            QuoteCurrency::USD,
        );

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

        // Update prices for the dynamic symbol with depth
        detector.update_price_with_bid_ask(
            Exchange::Binance, pair_id,
            FixedPoint::from_f64(0.10),
            FixedPoint::from_f64(0.0999), FixedPoint::from_f64(0.10),
            FixedPoint::from_f64(10000.0), FixedPoint::from_f64(10000.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase, pair_id,
            FixedPoint::from_f64(0.102),
            FixedPoint::from_f64(0.102), FixedPoint::from_f64(0.1021),
            FixedPoint::from_f64(10000.0), FixedPoint::from_f64(10000.0),
            QuoteCurrency::USD,
        );

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

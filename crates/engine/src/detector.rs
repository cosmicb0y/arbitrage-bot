//! Arbitrage opportunity detector.
//!
//! Monitors price feeds and detects profitable arbitrage opportunities.
//! Uses lock-free data structures (DashMap) for real-time performance.

use crate::{ConversionRates, PremiumMatrix};
use arbitrage_core::{
    ArbitrageOpportunity, Asset, Chain, Exchange, FixedPoint, QuoteCurrency, UsdlikePremium,
    UsdlikeQuote,
};
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
        self.symbol_registry
            .entry(pair_id)
            .or_insert_with(|| symbol.to_string());
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
        self.symbol_registry
            .get(&pair_id)
            .map(|r| r.value().clone())
    }

    /// Get the premium matrix for a pair_id.
    /// Returns a reference guard that can be used to access the matrix.
    pub fn get_matrix(
        &self,
        pair_id: u32,
    ) -> Option<dashmap::mapref::one::Ref<'_, u32, PremiumMatrix>> {
        self.matrices.get(&pair_id)
    }

    /// Update price for an exchange/pair with default quote (USD).
    /// Lock-free: uses DashMap entry API for concurrent access.
    pub fn update_price(&self, exchange: Exchange, pair_id: u32, price: FixedPoint) {
        self.update_price_with_quote(exchange, pair_id, price, QuoteCurrency::USD);
    }

    /// Update price for an exchange/pair with specified quote currency (lock-free).
    pub fn update_price_with_quote(
        &self,
        exchange: Exchange,
        pair_id: u32,
        price: FixedPoint,
        quote: QuoteCurrency,
    ) {
        self.update_price_with_bid_ask(
            exchange,
            pair_id,
            price,
            price,
            price,
            FixedPoint::from_f64(0.0),
            FixedPoint::from_f64(0.0),
            quote,
        );
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
            matrix.update_price_with_bid_ask_and_raw(
                exchange, price, bid, ask, raw_bid, raw_ask, bid_size, ask_size, quote,
            );
        } else {
            // Insert new matrix with configured staleness threshold
            let mut matrix = PremiumMatrix::with_staleness(pair_id, self.config.max_staleness_ms);
            matrix.update_price_with_bid_ask_and_raw(
                exchange, price, bid, ask, raw_bid, raw_ask, bid_size, ask_size, quote,
            );
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

        self.detect_with_conversion_rates(
            pair_id,
            &rates,
            usd_krw_rate,
            usdt_krw_rate,
            usdc_krw_rate,
        )
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

        for (
            buy_ex,
            sell_ex,
            buy_quote,
            sell_quote,
            buy_ask,
            sell_bid,
            buy_ask_raw,
            sell_bid_raw,
            buy_ask_size,
            sell_bid_size,
            usdlike_premium_bps,
            _unused,
            kimchi_premium,
            buy_timestamp_ms,
            sell_timestamp_ms,
        ) in premiums
        {
            if buy_ask_size.0 == 0 && sell_bid_size.0 == 0 {
                continue;
            }

            if usdlike_premium_bps < self.config.min_premium_bps {
                continue;
            }

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
                .with_raw_prices(buy_ask_raw, sell_bid_raw)
                .with_pair_id(pair_id);

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
            all_opportunities.extend(self.detect_with_all_rates(
                pair_id,
                usd_krw_rate,
                usdt_krw_rate,
                usdc_krw_rate,
            ));
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
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    struct TestWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_detector_config_default() {
        let config = DetectorConfig::default();
        assert!(config.min_premium_bps > 0);
        // max_staleness_ms is 0 (disabled) - staleness managed by WebSocket reconnection
        assert_eq!(config.max_staleness_ms, 0);
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
            Exchange::Binance,
            1,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            1,
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50501.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
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
            Exchange::Binance,
            1,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            1,
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50501.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
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
            Exchange::Binance,
            1,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            1,
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50501.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );

        // Pair 2: ETH with depth
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            2,
            FixedPoint::from_f64(3000.0),
            FixedPoint::from_f64(2999.0),
            FixedPoint::from_f64(3000.0),
            FixedPoint::from_f64(10.0),
            FixedPoint::from_f64(10.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            2,
            FixedPoint::from_f64(3050.0),
            FixedPoint::from_f64(3050.0),
            FixedPoint::from_f64(3051.0),
            FixedPoint::from_f64(10.0),
            FixedPoint::from_f64(10.0),
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
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(0.10),
            FixedPoint::from_f64(0.0999),
            FixedPoint::from_f64(0.10),
            FixedPoint::from_f64(10000.0),
            FixedPoint::from_f64(10000.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(0.102),
            FixedPoint::from_f64(0.102),
            FixedPoint::from_f64(0.1021),
            FixedPoint::from_f64(10000.0),
            FixedPoint::from_f64(10000.0),
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

    // Story 5.1: Dynamic market PremiumMatrix tests (AC: #4)
    #[test]
    fn test_dynamic_market_premium_matrix_auto_creation() {
        let config = DetectorConfig::default();
        let mut detector = OpportunityDetector::new(config);

        // Register a new dynamic symbol
        let pair_id = detector.register_symbol("SHIB");

        // Before any price update, no matrix should exist
        assert!(!detector.has_matrix(pair_id));

        // Update price - this should auto-create the PremiumMatrix
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(0.00001),
            FixedPoint::from_f64(0.000009),
            FixedPoint::from_f64(0.000011),
            FixedPoint::from_f64(1000000.0),
            FixedPoint::from_f64(1000000.0),
            QuoteCurrency::USD,
        );

        // Now matrix should exist
        assert!(detector.has_matrix(pair_id));

        // get_matrix should return the matrix
        let matrix_ref = detector.get_matrix(pair_id);
        assert!(matrix_ref.is_some());
    }

    #[test]
    fn test_dynamic_market_get_matrix_retrieval() {
        let config = DetectorConfig::default();
        let mut detector = OpportunityDetector::new(config);

        let pair_id = detector.register_symbol("PEPE");

        // Update prices from multiple exchanges
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(0.000001),
            FixedPoint::from_f64(0.0000009),
            FixedPoint::from_f64(0.0000011),
            FixedPoint::from_f64(100000000.0),
            FixedPoint::from_f64(100000000.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Upbit,
            pair_id,
            FixedPoint::from_f64(0.0000012),
            FixedPoint::from_f64(0.0000011),
            FixedPoint::from_f64(0.0000013),
            FixedPoint::from_f64(50000000.0),
            FixedPoint::from_f64(50000000.0),
            QuoteCurrency::USD,
        );

        // Retrieve matrix and verify it has data for both exchanges
        let matrix_ref = detector.get_matrix(pair_id);
        assert!(matrix_ref.is_some());

        let matrix = matrix_ref.unwrap();
        // Matrix should have prices from both exchanges
        let binance_price = matrix.get_price(Exchange::Binance);
        let upbit_price = matrix.get_price(Exchange::Upbit);

        assert!(binance_price.is_some());
        assert!(upbit_price.is_some());
    }

    #[test]
    fn test_dynamic_market_registered_pair_ids_includes_new_markets() {
        let config = DetectorConfig::default();
        let mut detector = OpportunityDetector::new(config);

        // Register multiple dynamic symbols
        let doge_id = detector.register_symbol("DOGE");
        let shib_id = detector.register_symbol("SHIB");
        let pepe_id = detector.register_symbol("PEPE");

        // All should be in registered_pair_ids
        let pair_ids = detector.registered_pair_ids();

        assert!(pair_ids.contains(&doge_id));
        assert!(pair_ids.contains(&shib_id));
        assert!(pair_ids.contains(&pepe_id));
    }

    #[test]
    fn test_dynamic_market_pair_id_to_symbol() {
        let config = DetectorConfig::default();
        let detector = OpportunityDetector::new(config);

        let pair_id = detector.register_symbol("AVAX");

        // Should be able to retrieve symbol by pair_id
        let symbol = detector.pair_id_to_symbol(pair_id);
        assert_eq!(symbol, Some("AVAX".to_string()));

        // Unknown pair_id should return None
        let unknown = detector.pair_id_to_symbol(999999);
        assert!(unknown.is_none());
    }

    // ============================================================================
    // Story 5.2: OpportunityDetector 새 마켓 통합 테스트 (AC: #1, #3)
    // ============================================================================

    /// Subtask 1.1: 동적으로 등록된 마켓(DOGE, XRP 등)에서 detect(pair_id)가 기회를 반환하는지 테스트
    #[test]
    fn test_dynamic_market_detect_returns_opportunities() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 동적 심볼 등록 (DOGE)
        let pair_id = detector.register_symbol("DOGE");

        // 두 거래소에서 가격 차이가 나는 상황 생성 (약 2% 프리미엄)
        // Binance: ask = 0.10 (매수 가격)
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(0.10),
            FixedPoint::from_f64(0.0999),
            FixedPoint::from_f64(0.10),
            FixedPoint::from_f64(10000.0),
            FixedPoint::from_f64(10000.0),
            QuoteCurrency::USD,
        );

        // Coinbase: bid = 0.102 (매도 가격) - 2% 프리미엄
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(0.102),
            FixedPoint::from_f64(0.102),
            FixedPoint::from_f64(0.1021),
            FixedPoint::from_f64(10000.0),
            FixedPoint::from_f64(10000.0),
            QuoteCurrency::USD,
        );

        // detect(pair_id) 호출 - 동적 마켓에서도 기회 탐지되어야 함
        let opportunities = detector.detect(pair_id);

        // 기회가 탐지되어야 함
        assert!(!opportunities.is_empty(), "동적 마켓에서 기회가 탐지되어야 함");

        // 첫 번째 기회 검증
        let opp = &opportunities[0];
        assert_eq!(opp.asset.symbol.as_str(), "DOGE", "심볼이 DOGE여야 함");
        assert!(opp.premium_bps >= 50, "프리미엄이 임계값 이상이어야 함");
    }

    /// Subtask 1.2: detect_all()이 동적 마켓을 포함하여 모든 마켓 기회를 반환하는지 테스트
    #[test]
    fn test_detect_all_includes_dynamic_markets() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 레거시 마켓 (pair_id 1 = BTC) 설정
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            1, // BTC
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            1,
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50500.0),
            FixedPoint::from_f64(50501.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
            QuoteCurrency::USD,
        );

        // 동적 마켓 (XRP) 등록 및 가격 설정
        let xrp_pair_id = detector.register_symbol("XRP");
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            xrp_pair_id,
            FixedPoint::from_f64(0.50),
            FixedPoint::from_f64(0.499),
            FixedPoint::from_f64(0.50),
            FixedPoint::from_f64(1000.0),
            FixedPoint::from_f64(1000.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            xrp_pair_id,
            FixedPoint::from_f64(0.51),
            FixedPoint::from_f64(0.51),
            FixedPoint::from_f64(0.511),
            FixedPoint::from_f64(1000.0),
            FixedPoint::from_f64(1000.0),
            QuoteCurrency::USD,
        );

        // detect_all() 호출
        let all_opportunities = detector.detect_all();

        // 최소 2개 이상 기회가 있어야 함 (BTC + XRP)
        assert!(
            all_opportunities.len() >= 2,
            "detect_all()은 레거시 + 동적 마켓 기회를 모두 반환해야 함"
        );

        // XRP 기회가 포함되어 있는지 확인
        let xrp_opportunities: Vec<_> = all_opportunities
            .iter()
            .filter(|o| o.asset.symbol.as_str() == "XRP")
            .collect();
        assert!(
            !xrp_opportunities.is_empty(),
            "detect_all()에 동적 마켓(XRP) 기회가 포함되어야 함"
        );
    }

    /// Subtask 1.3: min_premium_bps 임계값 초과 시에만 기회가 탐지되는지 검증
    #[test]
    fn test_dynamic_market_respects_min_premium_threshold() {
        let config = DetectorConfig {
            min_premium_bps: 200, // 2% 임계값
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 동적 마켓 등록
        let pair_id = detector.register_symbol("ADA");

        // 1% 프리미엄만 있는 가격 설정 (임계값 미달)
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(0.50),
            FixedPoint::from_f64(0.499),
            FixedPoint::from_f64(0.50),
            FixedPoint::from_f64(1000.0),
            FixedPoint::from_f64(1000.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(0.505),
            FixedPoint::from_f64(0.505),
            FixedPoint::from_f64(0.506),
            FixedPoint::from_f64(1000.0),
            FixedPoint::from_f64(1000.0),
            QuoteCurrency::USD,
        );

        // detect() 호출 - 임계값 미달이면 기회가 없어야 함
        let opportunities = detector.detect(pair_id);

        assert!(
            opportunities.is_empty(),
            "임계값 미달 시 기회가 반환되면 안됨"
        );
    }

    // ============================================================================
    // Story 5.2: WebSocket 브로드캐스트 통합 테스트 (AC: #2)
    // ============================================================================

    /// Subtask 2.1-2.3: ArbitrageOpportunity 직렬화에 동적 마켓 정보가 포함되는지 검증
    #[test]
    fn test_dynamic_market_opportunity_contains_symbol_info() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 동적 마켓 등록 (LINK)
        let pair_id = detector.register_symbol("LINK");

        // 가격 설정
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(15.0),
            FixedPoint::from_f64(14.99),
            FixedPoint::from_f64(15.0),
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(100.0),
            QuoteCurrency::USD,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(15.30),
            FixedPoint::from_f64(15.30),
            FixedPoint::from_f64(15.31),
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(100.0),
            QuoteCurrency::USD,
        );

        // 기회 탐지
        let opportunities = detector.detect(pair_id);
        assert!(!opportunities.is_empty(), "기회가 탐지되어야 함");

        let opp = &opportunities[0];

        // AC 2.2: ArbitrageOpportunity에 동적 마켓 정보 포함 확인
        assert_eq!(
            opp.asset.symbol.as_str(),
            "LINK",
            "symbol 정보가 올바르게 포함되어야 함"
        );

        // AC 2.2: pair_id 포함 확인
        assert_eq!(opp.pair_id, pair_id, "pair_id 정보가 포함되어야 함");

        // AC 2.3: 직렬화 가능 확인 (브로드캐스트 메시지 포맷)
        let serialized = serde_json::to_string(&opp);
        assert!(serialized.is_ok(), "ArbitrageOpportunity가 JSON 직렬화 가능해야 함");

        let json_str = serialized.unwrap();
        assert!(
            json_str.contains("LINK"),
            "직렬화된 JSON에 symbol 정보가 포함되어야 함"
        );
        assert!(
            json_str.contains("\"pair_id\""),
            "직렬화된 JSON에 pair_id가 포함되어야 함"
        );
        assert!(
            json_str.contains("source_exchange"),
            "직렬화된 JSON에 거래소 정보가 포함되어야 함"
        );
        assert!(
            json_str.contains("premium_bps"),
            "직렬화된 JSON에 프리미엄 정보가 포함되어야 함"
        );
    }

    /// AC 2.2: 동적 마켓 기회에서 다양한 심볼이 올바르게 Asset으로 변환되는지 검증
    #[test]
    fn test_dynamic_market_asset_conversion() {
        let config = DetectorConfig::default();
        let detector = OpportunityDetector::new(config);

        // 여러 동적 마켓 등록
        let symbols = ["DOGE", "SHIB", "PEPE", "FLOKI", "BONK"];

        for symbol in symbols {
            let pair_id = detector.register_symbol(symbol);
            detector.update_price_with_bid_ask(
                Exchange::Binance,
                pair_id,
                FixedPoint::from_f64(0.001),
                FixedPoint::from_f64(0.0009),
                FixedPoint::from_f64(0.0011),
                FixedPoint::from_f64(1000000.0),
                FixedPoint::from_f64(1000000.0),
                QuoteCurrency::USD,
            );
            detector.update_price_with_bid_ask(
                Exchange::Coinbase,
                pair_id,
                FixedPoint::from_f64(0.00102),
                FixedPoint::from_f64(0.00102),
                FixedPoint::from_f64(0.00103),
                FixedPoint::from_f64(1000000.0),
                FixedPoint::from_f64(1000000.0),
                QuoteCurrency::USD,
            );

            let opportunities = detector.detect(pair_id);
            if !opportunities.is_empty() {
                assert_eq!(
                    opportunities[0].asset.symbol.as_str(),
                    symbol,
                    "Asset 심볼이 등록된 심볼과 일치해야 함"
                );
            }
        }
    }

    // ============================================================================
    // Story 5.2: 다중 거래소 프리미엄 계산 검증 (AC: #4)
    // ============================================================================

    /// Subtask 3.1: 새 마켓에서 2개 이상 거래소 가격이 있을 때 프리미엄 계산 테스트
    #[test]
    fn test_dynamic_market_multi_exchange_premium_calculation() {
        let config = DetectorConfig {
            min_premium_bps: 10,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 동적 마켓 등록 (SUI)
        let pair_id = detector.register_symbol("SUI");

        // 3개 거래소에서 가격 설정 (Binance, Coinbase, Upbit)
        // Binance: 1.50 USDT
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(1.50),
            FixedPoint::from_f64(1.499),
            FixedPoint::from_f64(1.50),
            FixedPoint::from_f64(1000.0),
            FixedPoint::from_f64(1000.0),
            QuoteCurrency::USDT,
        );

        // Coinbase: 1.52 USD (약 1.3% 프리미엄)
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(1.52),
            FixedPoint::from_f64(1.52),
            FixedPoint::from_f64(1.521),
            FixedPoint::from_f64(1000.0),
            FixedPoint::from_f64(1000.0),
            QuoteCurrency::USD,
        );

        // Upbit: 1.54 (USD 기준, 약 2.6% 프리미엄) - KRW 환율 적용된 가격으로 가정
        detector.update_price_with_bid_ask(
            Exchange::Upbit,
            pair_id,
            FixedPoint::from_f64(1.54),
            FixedPoint::from_f64(1.54),
            FixedPoint::from_f64(1.541),
            FixedPoint::from_f64(1000.0),
            FixedPoint::from_f64(1000.0),
            QuoteCurrency::USD,
        );

        // detect_all()로 모든 거래소 쌍 프리미엄 확인
        let opportunities = detector.detect_all();

        // 최소 2개 기회 (Binance-Coinbase, Binance-Upbit, Coinbase-Upbit 중)
        assert!(
            !opportunities.is_empty(),
            "다중 거래소에서 기회가 탐지되어야 함"
        );

        // 모든 기회의 asset이 SUI인지 확인
        for opp in &opportunities {
            assert_eq!(opp.asset.symbol.as_str(), "SUI");
        }

        // 적어도 하나의 기회에서 프리미엄이 0보다 큰지 확인
        let has_positive_premium = opportunities.iter().any(|o| o.premium_bps > 0);
        assert!(has_positive_premium, "양수 프리미엄이 존재해야 함");
    }

    /// Subtask 3.2: KRW/USD 환율 적용 시 김치 프리미엄이 올바르게 계산되는지 검증
    #[test]
    fn test_dynamic_market_krw_premium_with_exchange_rates() {
        let config = DetectorConfig {
            min_premium_bps: 10,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 동적 마켓 등록 (MATIC)
        let pair_id = detector.register_symbol("MATIC");

        // Binance: 0.80 USDT
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(0.80),
            FixedPoint::from_f64(0.799),
            FixedPoint::from_f64(0.80),
            FixedPoint::from_f64(10000.0),
            FixedPoint::from_f64(10000.0),
            QuoteCurrency::USDT,
        );

        // Upbit: 1160 KRW (환율 1450원 기준 약 0.80 USD)
        detector.update_price_with_bid_ask(
            Exchange::Upbit,
            pair_id,
            FixedPoint::from_f64(1200.0), // 1200 KRW - 약 3% 김치 프리미엄
            FixedPoint::from_f64(1200.0),
            FixedPoint::from_f64(1201.0),
            FixedPoint::from_f64(10000.0),
            FixedPoint::from_f64(10000.0),
            QuoteCurrency::KRW,
        );

        // 환율 적용하여 기회 탐지
        let usd_krw = Some(1450.0);
        let usdt_krw = Some(1430.0);
        let opportunities = detector.detect_with_rates(pair_id, usd_krw, usdt_krw);

        // 기회가 탐지되어야 함
        assert!(!opportunities.is_empty(), "KRW/USD 환율 적용 시 기회가 탐지되어야 함");

        // 기회에 kimchi_premium_bps/환율 기반 프리미엄이 설정되어 있는지 확인
        let opp = &opportunities[0];
        assert_eq!(opp.asset.symbol.as_str(), "MATIC");
        assert!(
            opp.kimchi_premium_bps > 0,
            "KRW/USD 환율 기반 김치 프리미엄이 계산되어야 함"
        );
        assert!(
            opp.usdlike_premium.is_some(),
            "USD-like 프리미엄이 설정되어야 함"
        );
    }

    /// Subtask 3.3: 서로 다른 QuoteCurrency(USD, USDT, KRW) 간 변환이 정확한지 테스트
    #[test]
    fn test_dynamic_market_multi_quote_currency_handling() {
        let config = DetectorConfig {
            min_premium_bps: 10,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 동적 마켓 등록 (NEAR)
        let pair_id = detector.register_symbol("NEAR");

        // USD 마켓 (Coinbase)
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(5.00),
            FixedPoint::from_f64(4.99),
            FixedPoint::from_f64(5.00),
            FixedPoint::from_f64(500.0),
            FixedPoint::from_f64(500.0),
            QuoteCurrency::USD,
        );

        // USDT 마켓 (Binance)
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(5.05),
            FixedPoint::from_f64(5.05),
            FixedPoint::from_f64(5.06),
            FixedPoint::from_f64(500.0),
            FixedPoint::from_f64(500.0),
            QuoteCurrency::USDT,
        );

        // USDC 마켓 (Bybit)
        detector.update_price_with_bid_ask(
            Exchange::Bybit,
            pair_id,
            FixedPoint::from_f64(5.10),
            FixedPoint::from_f64(5.10),
            FixedPoint::from_f64(5.11),
            FixedPoint::from_f64(500.0),
            FixedPoint::from_f64(500.0),
            QuoteCurrency::USDC,
        );

        // 기회 탐지
        let opportunities = detector.detect(pair_id);

        // 다양한 Quote Currency 조합에서 기회가 탐지되어야 함
        assert!(
            !opportunities.is_empty(),
            "다양한 Quote Currency 마켓 간 기회가 탐지되어야 함"
        );

        // USD(코인베이스) -> USDT(바이낸스) 프리미엄이 정확히 계산되는지 확인
        let expected_bps = FixedPoint::premium_bps(
            FixedPoint::from_f64(5.00),
            FixedPoint::from_f64(5.05),
        );
        let usd_to_usdt = opportunities.iter().find(|opp| {
            opp.source_exchange == Exchange::Coinbase && opp.target_exchange == Exchange::Binance
        });
        assert!(usd_to_usdt.is_some(), "USD->USDT 기회가 포함되어야 함");
        assert_eq!(
            usd_to_usdt.unwrap().premium_bps,
            expected_bps,
            "USD/USDT 변환 기반 프리미엄 계산이 일치해야 함"
        );
    }

    // ============================================================================
    // Story 5.2: 기회 로깅 검증 (AC: #5)
    // ============================================================================

    /// Subtask 4.1-4.3: 동적 마켓 기회 탐지 시 로깅이 기존 패턴과 일관되는지 검증
    /// 참고: 실제 로깅 출력을 캡처하는 대신, 동적 마켓에서 로깅이 호출되는 코드 경로가
    /// 레거시 마켓과 동일한지 검증합니다.
    #[test]
    fn test_dynamic_market_logging_consistency() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = {
            let buffer = Arc::clone(&buffer);
            move || TestWriter {
                buffer: Arc::clone(&buffer),
            }
        };
        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .finish();

        let config = DetectorConfig::default();
        let detector = OpportunityDetector::new(config);

        tracing::subscriber::with_default(subscriber, || {
            // 동적 마켓 등록
            let pair_id = detector.register_symbol("ARB");

            // 가격 업데이트 - tracing::trace! 로그 발생
            detector.update_price_with_bid_ask(
                Exchange::Binance,
                pair_id,
                FixedPoint::from_f64(1.0),
                FixedPoint::from_f64(0.99),
                FixedPoint::from_f64(1.0),
                FixedPoint::from_f64(1000.0),
                FixedPoint::from_f64(1000.0),
                QuoteCurrency::USDT,
            );

            // Matrix가 없을 때 detect 호출 - tracing::debug! 로그 경로 확인
            let no_match_pair_id = 999999;
            let empty_opps = detector.detect(no_match_pair_id);
            assert!(empty_opps.is_empty());

            // 두 번째 거래소 가격 추가
            detector.update_price_with_bid_ask(
                Exchange::Coinbase,
                pair_id,
                FixedPoint::from_f64(1.02),
                FixedPoint::from_f64(1.02),
                FixedPoint::from_f64(1.021),
                FixedPoint::from_f64(1000.0),
                FixedPoint::from_f64(1000.0),
                QuoteCurrency::USD,
            );

            // 기회 탐지 - tracing::debug! "detect: found matrix" 로그 경로 확인
            let opportunities = detector.detect(pair_id);
            assert!(!opportunities.is_empty());

            // 기회 정보가 로깅에 사용될 수 있는 필드들이 모두 설정되어 있는지 확인
            let opp = &opportunities[0];
            assert!(!opp.asset.symbol.as_str().is_empty(), "symbol이 설정되어 있어야 함");
        });

        let logs = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("detector: price updated"),
            "가격 업데이트 로그가 출력되어야 함"
        );
        assert!(
            logs.contains("detect: found matrix"),
            "detect() 로그가 출력되어야 함"
        );
    }

    /// 동적 마켓 기회에서 로깅에 필요한 모든 정보가 포함되어 있는지 검증
    #[test]
    fn test_dynamic_market_opportunity_has_loggable_info() {
        let config = DetectorConfig::default();
        let detector = OpportunityDetector::new(config);

        let pair_id = detector.register_symbol("OP");

        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(2.0),
            FixedPoint::from_f64(1.99),
            FixedPoint::from_f64(2.0),
            FixedPoint::from_f64(500.0),
            FixedPoint::from_f64(500.0),
            QuoteCurrency::USDT,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(2.05),
            FixedPoint::from_f64(2.05),
            FixedPoint::from_f64(2.06),
            FixedPoint::from_f64(500.0),
            FixedPoint::from_f64(500.0),
            QuoteCurrency::USD,
        );

        let opportunities = detector.detect(pair_id);
        assert!(!opportunities.is_empty());

        let opp = &opportunities[0];

        // 로깅에 필요한 핵심 정보 검증
        assert_eq!(opp.asset.symbol.as_str(), "OP", "symbol 정보");
        assert!(opp.source_price > 0, "source_price 설정");
        assert!(opp.target_price > 0, "target_price 설정");
        // premium_bps가 계산되어 있어야 함
        assert_ne!(opp.premium_bps, 0, "premium_bps 계산");

        // 로깅 포맷에 사용되는 Debug trait 구현 확인
        let _debug_str = format!("{:?}", opp);
    }

    // ============================================================================
    // Story 5.2: 통합 테스트 (AC: #1~5)
    // ============================================================================

    /// Subtask 5.1: 동적 구독 → 가격 업데이트 → 기회 탐지 → 브로드캐스트 전체 흐름 테스트
    #[test]
    fn test_full_flow_dynamic_subscription_to_opportunity_detection() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 1. 동적 구독 시뮬레이션 - 새 심볼 등록
        let symbol = "RENDER";
        let pair_id = detector.register_symbol(symbol);
        assert!(pair_id > 0, "pair_id가 생성되어야 함");

        // 등록 확인
        let registered_symbol = detector.pair_id_to_symbol(pair_id);
        assert_eq!(registered_symbol, Some(symbol.to_string()));

        // 2. 첫 번째 거래소 가격 업데이트 (아직 기회 없음)
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(7.50),
            FixedPoint::from_f64(7.49),
            FixedPoint::from_f64(7.50),
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(100.0),
            QuoteCurrency::USDT,
        );

        // PremiumMatrix 자동 생성 확인
        assert!(detector.has_matrix(pair_id), "Matrix가 자동 생성되어야 함");

        // 단일 거래소에서는 기회 없음
        let single_ex_opps = detector.detect(pair_id);
        assert!(
            single_ex_opps.is_empty(),
            "단일 거래소에서는 기회가 없어야 함"
        );

        // 3. 두 번째 거래소 가격 업데이트 (프리미엄 발생)
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(7.70),
            FixedPoint::from_f64(7.70),
            FixedPoint::from_f64(7.71),
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(100.0),
            QuoteCurrency::USD,
        );

        // 4. 기회 탐지
        let opportunities = detector.detect(pair_id);
        assert!(!opportunities.is_empty(), "두 거래소 가격 차이로 기회가 탐지되어야 함");

        let opp = &opportunities[0];

        // 5. 기회 검증 (브로드캐스트 가능 형태)
        assert_eq!(opp.asset.symbol.as_str(), symbol, "심볼 일치");
        assert!(opp.premium_bps > 0, "양수 프리미엄");
        assert!(opp.source_price > 0, "source_price 설정");
        assert!(opp.target_price > 0, "target_price 설정");

        // 6. 직렬화 가능 확인 (브로드캐스트용)
        let json = serde_json::to_string(&opp);
        assert!(json.is_ok(), "JSON 직렬화 가능");
        assert!(json.unwrap().contains(symbol), "JSON에 심볼 포함");

        // 7. detect_all에서도 포함되는지 확인
        let all_opps = detector.detect_all();
        let found = all_opps.iter().any(|o| o.asset.symbol.as_str() == symbol);
        assert!(found, "detect_all에 동적 마켓 기회 포함");
    }

    /// Subtask 5.2: 에지 케이스 테스트 - 단일 거래소
    #[test]
    fn test_edge_case_single_exchange() {
        let config = DetectorConfig::default();
        let detector = OpportunityDetector::new(config);

        let pair_id = detector.register_symbol("EDGE1");

        // 단일 거래소 가격만 있는 경우
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(0.99),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1000.0),
            FixedPoint::from_f64(1000.0),
            QuoteCurrency::USDT,
        );

        let opportunities = detector.detect(pair_id);
        // 단일 거래소에서는 차익거래 불가능
        assert!(
            opportunities.is_empty(),
            "단일 거래소에서는 기회가 없어야 함"
        );
    }

    /// Subtask 5.2: 에지 케이스 테스트 - 가격 없음
    #[test]
    fn test_edge_case_no_prices() {
        let config = DetectorConfig::default();
        let detector = OpportunityDetector::new(config);

        // 등록만 하고 가격 업데이트 없음
        let pair_id = detector.register_symbol("EDGE2");

        // Matrix가 없으므로 기회도 없음
        assert!(!detector.has_matrix(pair_id));

        let opportunities = detector.detect(pair_id);
        assert!(opportunities.is_empty(), "가격 없이는 기회가 없어야 함");
    }

    /// Subtask 5.2: 에지 케이스 테스트 - 제로 깊이
    #[test]
    fn test_edge_case_zero_depth() {
        let config = DetectorConfig::default();
        let detector = OpportunityDetector::new(config);

        let pair_id = detector.register_symbol("EDGE3");

        // 깊이가 0인 가격 업데이트
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(0.99),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(0.0), // 제로 깊이
            FixedPoint::from_f64(0.0), // 제로 깊이
            QuoteCurrency::USDT,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(1.05),
            FixedPoint::from_f64(1.05),
            FixedPoint::from_f64(1.06),
            FixedPoint::from_f64(0.0), // 제로 깊이
            FixedPoint::from_f64(0.0), // 제로 깊이
            QuoteCurrency::USD,
        );

        // 깊이가 0인 경우 기회가 필터링될 수 있음
        let opportunities = detector.detect(pair_id);
        // 로직에 따라 빈 결과일 수 있음 (depth 0 스킵 로직)
        // 현재 로직은 depth 0이면 skip하므로 빈 결과 예상
        assert!(opportunities.is_empty(), "제로 깊이에서는 기회가 스킵되어야 함");
    }

    /// Subtask 5.2: 에지 케이스 테스트 - 동일 가격 (프리미엄 0)
    #[test]
    fn test_edge_case_same_price() {
        let config = DetectorConfig {
            min_premium_bps: 10,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        let pair_id = detector.register_symbol("EDGE4");

        // 동일한 가격
        detector.update_price_with_bid_ask(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(0.999),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(100.0),
            QuoteCurrency::USDT,
        );
        detector.update_price_with_bid_ask(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.0),
            FixedPoint::from_f64(1.001),
            FixedPoint::from_f64(100.0),
            FixedPoint::from_f64(100.0),
            QuoteCurrency::USD,
        );

        let opportunities = detector.detect(pair_id);
        assert!(opportunities.is_empty(), "임계값 미달 시 기회가 없어야 함");
    }

    /// 다수의 동적 마켓에 대한 병렬 처리 검증
    #[test]
    fn test_multiple_dynamic_markets_concurrent_detection() {
        let config = DetectorConfig {
            min_premium_bps: 50,
            ..Default::default()
        };
        let detector = OpportunityDetector::new(config);

        // 10개 동적 마켓 등록
        let symbols = [
            "MKT1", "MKT2", "MKT3", "MKT4", "MKT5", "MKT6", "MKT7", "MKT8", "MKT9", "MKT10",
        ];

        for (i, symbol) in symbols.iter().enumerate() {
            let pair_id = detector.register_symbol(symbol);

            // 각 마켓에 프리미엄이 있는 가격 설정
            let base_price = (i + 1) as f64;
            detector.update_price_with_bid_ask(
                Exchange::Binance,
                pair_id,
                FixedPoint::from_f64(base_price),
                FixedPoint::from_f64(base_price - 0.01),
                FixedPoint::from_f64(base_price),
                FixedPoint::from_f64(100.0),
                FixedPoint::from_f64(100.0),
                QuoteCurrency::USDT,
            );
            detector.update_price_with_bid_ask(
                Exchange::Coinbase,
                pair_id,
                FixedPoint::from_f64(base_price * 1.02), // 2% 프리미엄
                FixedPoint::from_f64(base_price * 1.02),
                FixedPoint::from_f64(base_price * 1.021),
                FixedPoint::from_f64(100.0),
                FixedPoint::from_f64(100.0),
                QuoteCurrency::USD,
            );
        }

        // detect_all()로 모든 마켓 기회 탐지
        let all_opportunities = detector.detect_all();

        // 모든 마켓에서 기회가 탐지되어야 함
        assert!(
            all_opportunities.len() >= symbols.len(),
            "모든 동적 마켓에서 기회가 탐지되어야 함"
        );

        // 각 심볼에 대한 기회가 있는지 확인
        for symbol in &symbols {
            let found = all_opportunities
                .iter()
                .any(|o| o.asset.symbol.as_str() == *symbol);
            assert!(found, "심볼 {} 기회가 detect_all에 포함되어야 함", symbol);
        }
    }
}

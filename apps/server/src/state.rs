//! Application state management.

use crate::config::AppConfig;
use arbitrage_core::{symbol_to_pair_id, ArbitrageOpportunity, Exchange, FixedPoint, QuoteCurrency};
use arbitrage_engine::{DetectorConfig, OpportunityDetector, PremiumMatrix};
use arbitrage_feeds::{CommonMarkets, PriceAggregator};
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Statistics for the bot.
#[derive(Debug, Default)]
pub struct BotStats {
    /// Number of price updates received.
    pub price_updates: AtomicU64,
    /// Number of opportunities detected.
    pub opportunities_detected: AtomicU64,
    /// Number of trades executed.
    pub trades_executed: AtomicU64,
    /// Total profit in basis points.
    pub total_profit_bps: AtomicU64,
    /// Start time in milliseconds.
    pub started_at_ms: AtomicU64,
}

impl BotStats {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            started_at_ms: AtomicU64::new(now),
            ..Default::default()
        }
    }

    pub fn record_price_update(&self) {
        self.price_updates.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_opportunity(&self) {
        self.opportunities_detected.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_trade(&self, profit_bps: i32) {
        self.trades_executed.fetch_add(1, Ordering::Relaxed);
        if profit_bps > 0 {
            self.total_profit_bps
                .fetch_add(profit_bps as u64, Ordering::Relaxed);
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        (now - self.started_at_ms.load(Ordering::Relaxed)) / 1000
    }

    pub fn summary(&self) -> StatsSummary {
        StatsSummary {
            price_updates: self.price_updates.load(Ordering::Relaxed),
            opportunities_detected: self.opportunities_detected.load(Ordering::Relaxed),
            trades_executed: self.trades_executed.load(Ordering::Relaxed),
            total_profit_bps: self.total_profit_bps.load(Ordering::Relaxed),
            uptime_secs: self.uptime_secs(),
        }
    }
}

/// Summary of statistics.
#[derive(Debug, Clone)]
pub struct StatsSummary {
    pub price_updates: u64,
    pub opportunities_detected: u64,
    pub trades_executed: u64,
    pub total_profit_bps: u64,
    pub uptime_secs: u64,
}

/// Depth cache key: (exchange, symbol)
type DepthCacheKey = (String, String);

/// Depth cache value: (bid_size, ask_size)
type DepthCacheValue = (FixedPoint, FixedPoint);

/// Application state shared across components.
pub struct AppState {
    /// Configuration.
    pub config: RwLock<AppConfig>,
    /// Price aggregator.
    pub prices: PriceAggregator,
    /// Opportunity detector.
    pub detector: RwLock<OpportunityDetector>,
    /// Premium matrices per pair.
    pub matrices: DashMap<u32, PremiumMatrix>,
    /// Recent opportunities.
    pub opportunities: RwLock<Vec<ArbitrageOpportunity>>,
    /// Bot statistics.
    pub stats: BotStats,
    /// Running flag.
    pub running: AtomicBool,
    /// USDT/KRW price from Upbit (for KRW to USD conversion).
    /// Stored as FixedPoint (e.g., 1438.5 KRW per USDT).
    upbit_usdt_krw: AtomicU64,
    /// USDT/KRW price from Bithumb.
    /// Stored as FixedPoint.
    bithumb_usdt_krw: AtomicU64,
    /// USDT/USD price (e.g., 1.0001 USD per USDT).
    /// Stored as FixedPoint.
    usdt_usd_price: AtomicU64,
    /// USDC/USDT price for USDC/USD calculation.
    /// Stored as FixedPoint.
    usdc_usdt_price: AtomicU64,
    /// Common markets across exchanges.
    pub common_markets: RwLock<Option<CommonMarkets>>,
    /// Orderbook depth cache: (exchange, symbol) -> (bid_size, ask_size)
    /// Updated on every price update, used to enrich opportunity data.
    depth_cache: DashMap<DepthCacheKey, DepthCacheValue>,
}

impl AppState {
    /// Create new application state.
    pub fn new(config: AppConfig) -> Self {
        let detector_config: DetectorConfig = (&config.detector).into();

        Self {
            config: RwLock::new(config),
            prices: PriceAggregator::new(),
            detector: RwLock::new(OpportunityDetector::new(detector_config)),
            matrices: DashMap::new(),
            opportunities: RwLock::new(Vec::new()),
            stats: BotStats::new(),
            running: AtomicBool::new(false),
            upbit_usdt_krw: AtomicU64::new(0), // 0 means not yet received
            bithumb_usdt_krw: AtomicU64::new(0), // 0 means not yet received
            usdt_usd_price: AtomicU64::new(FixedPoint::from_f64(1.0).0), // Default 1:1
            usdc_usdt_price: AtomicU64::new(FixedPoint::from_f64(1.0).0), // Default 1:1
            common_markets: RwLock::new(None),
            depth_cache: DashMap::new(),
        }
    }

    /// Update USDT/KRW price from Upbit.
    pub fn update_upbit_usdt_krw(&self, price: FixedPoint) {
        self.upbit_usdt_krw.store(price.0, Ordering::Relaxed);
    }

    /// Get Upbit USDT/KRW price. Returns None if not yet received.
    pub fn get_upbit_usdt_krw(&self) -> Option<FixedPoint> {
        let price = self.upbit_usdt_krw.load(Ordering::Relaxed);
        if price == 0 {
            None
        } else {
            Some(FixedPoint(price))
        }
    }

    /// Update USDT/KRW price from Bithumb.
    pub fn update_bithumb_usdt_krw(&self, price: FixedPoint) {
        self.bithumb_usdt_krw.store(price.0, Ordering::Relaxed);
    }

    /// Get Bithumb USDT/KRW price. Returns None if not yet received.
    pub fn get_bithumb_usdt_krw(&self) -> Option<FixedPoint> {
        let price = self.bithumb_usdt_krw.load(Ordering::Relaxed);
        if price == 0 {
            None
        } else {
            Some(FixedPoint(price))
        }
    }

    /// Get USDT/KRW price for a specific exchange.
    pub fn get_usdt_krw_for_exchange(&self, exchange: Exchange) -> Option<FixedPoint> {
        match exchange {
            Exchange::Upbit => self.get_upbit_usdt_krw(),
            Exchange::Bithumb => self.get_bithumb_usdt_krw(),
            _ => None, // Non-KRW exchanges don't need USDT/KRW
        }
    }

    /// Update USDT/USD price from exchange feed.
    pub fn update_usdt_usd_price(&self, price: FixedPoint) {
        self.usdt_usd_price.store(price.0, Ordering::Relaxed);
    }

    /// Get USDT/USD price.
    pub fn get_usdt_usd_price(&self) -> FixedPoint {
        FixedPoint(self.usdt_usd_price.load(Ordering::Relaxed))
    }

    /// Update USDC/USDT price from exchange feed.
    pub fn update_usdc_usdt_price(&self, price: FixedPoint) {
        self.usdc_usdt_price.store(price.0, Ordering::Relaxed);
    }

    /// Get USDC/USDT price.
    pub fn get_usdc_usdt_price(&self) -> FixedPoint {
        FixedPoint(self.usdc_usdt_price.load(Ordering::Relaxed))
    }

    /// Get USDC/USD price (calculated from USDC/USDT * USDT/USD).
    pub fn get_usdc_usd_price(&self) -> FixedPoint {
        let usdc_usdt = self.get_usdc_usdt_price().to_f64();
        let usdt_usd = self.get_usdt_usd_price().to_f64();
        FixedPoint::from_f64(usdc_usdt * usdt_usd)
    }

    /// Update common markets.
    pub async fn update_common_markets(&self, markets: CommonMarkets) {
        let mut stored = self.common_markets.write().await;
        *stored = Some(markets);
    }

    /// Get common markets.
    pub async fn get_common_markets(&self) -> Option<CommonMarkets> {
        self.common_markets.read().await.clone()
    }

    /// Start the bot.
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    /// Stop the bot.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Update price from a feed with default quote (USD).
    pub async fn update_price(&self, exchange: Exchange, pair_id: u32, price: FixedPoint) {
        self.update_price_with_quote(exchange, pair_id, price, QuoteCurrency::USD).await;
    }

    /// Update price from a feed with specified quote currency.
    /// Uses mid price as bid/ask when only price is available.
    pub async fn update_price_with_quote(&self, exchange: Exchange, pair_id: u32, price: FixedPoint, quote: QuoteCurrency) {
        self.update_price_with_bid_ask(exchange, pair_id, price, price, price, FixedPoint::from_f64(0.0), FixedPoint::from_f64(0.0), quote).await;
    }

    /// Update price from a feed with bid/ask from orderbook.
    /// This enables accurate premium calculation using best bid/ask prices.
    pub async fn update_price_with_bid_ask(
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
        // Update price aggregator
        let tick = arbitrage_core::PriceTick::with_depth(exchange, pair_id, price, bid, ask, bid_size, ask_size, quote);
        self.prices.update(tick);

        // Update detector with bid/ask for accurate premium calculation
        {
            let mut detector = self.detector.write().await;
            // Get or compute symbol for depth cache
            // First try registry, then compute from pair_id if possible
            let symbol = detector.pair_id_to_symbol(pair_id);
            if let Some(sym) = &symbol {
                // Update depth cache (only if we have non-zero depth)
                if bid_size.0 > 0 || ask_size.0 > 0 {
                    let key = (format!("{:?}", exchange), sym.clone());
                    self.depth_cache.insert(key, (bid_size, ask_size));
                }
            }
            detector.update_price_with_bid_ask(exchange, pair_id, price, bid, ask, bid_size, ask_size, quote);
        }

        self.stats.record_price_update();
    }

    /// Update price from a feed with bid/ask from orderbook and symbol.
    /// Use this when symbol is known to ensure depth cache is updated.
    pub async fn update_price_with_bid_ask_and_symbol(
        &self,
        exchange: Exchange,
        pair_id: u32,
        symbol: &str,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        quote: QuoteCurrency,
    ) {
        // Update price aggregator
        let tick = arbitrage_core::PriceTick::with_depth(exchange, pair_id, price, bid, ask, bid_size, ask_size, quote);
        self.prices.update(tick);

        // Update depth cache directly (we know the symbol)
        if bid_size.0 > 0 || ask_size.0 > 0 {
            let key = (format!("{:?}", exchange), symbol.to_string());
            self.depth_cache.insert(key, (bid_size, ask_size));
        }

        // Update detector with bid/ask for accurate premium calculation
        {
            let mut detector = self.detector.write().await;
            // Register symbol if not already registered
            detector.get_or_register_pair_id(symbol);
            detector.update_price_with_bid_ask(exchange, pair_id, price, bid, ask, bid_size, ask_size, quote);
        }

        self.stats.record_price_update();
    }

    /// Get cached orderbook depth for an exchange and symbol.
    /// Returns (bid_size, ask_size) or None if not cached.
    pub fn get_depth(&self, exchange: &str, symbol: &str) -> Option<(FixedPoint, FixedPoint)> {
        let key = (exchange.to_string(), symbol.to_string());
        self.depth_cache.get(&key).map(|v| *v)
    }

    /// Update price for a symbol (dynamic markets).
    /// Uses mid price as bid/ask when only price is available.
    pub async fn update_price_for_symbol(
        &self,
        exchange: Exchange,
        symbol: &str,
        price: FixedPoint,
    ) {
        self.update_price_for_symbol_with_bid_ask(exchange, symbol, price, price, price, FixedPoint::from_f64(0.0), FixedPoint::from_f64(0.0)).await;
    }

    /// Update price for a symbol with bid/ask from orderbook.
    pub async fn update_price_for_symbol_with_bid_ask(
        &self,
        exchange: Exchange,
        symbol: &str,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
    ) {
        let pair_id = symbol_to_pair_id(symbol);

        // Update price aggregator
        let tick = arbitrage_core::PriceTick::new(exchange, pair_id, price, bid, ask).with_sizes(bid_size, ask_size);
        self.prices.update(tick);

        // Update detector with bid/ask (registers symbol if needed)
        {
            let mut detector = self.detector.write().await;
            detector.get_or_register_pair_id(symbol);
            detector.update_price_with_bid_ask(exchange, pair_id, price, bid, ask, bid_size, ask_size, arbitrage_core::QuoteCurrency::USD);
        }

        self.stats.record_price_update();
    }

    /// Register symbols from common markets for opportunity detection.
    pub async fn register_common_markets(&self, markets: &CommonMarkets) {
        let mut detector = self.detector.write().await;
        for symbol in markets.common_bases() {
            detector.register_symbol(&symbol);
        }
    }

    /// Get all registered pair_ids for opportunity detection.
    pub async fn get_registered_pair_ids(&self) -> Vec<u32> {
        let detector = self.detector.read().await;
        detector.registered_pair_ids()
    }

    /// Detect opportunities for a pair.
    /// Returns all detected opportunities (both new and updated).
    pub async fn detect_opportunities(&self, pair_id: u32) -> Vec<ArbitrageOpportunity> {
        let mut detector = self.detector.write().await;

        // Get exchange rates for kimchi/tether premium calculation
        // Use Upbit's USDT/KRW as both usd_krw and usdt_krw base
        let usdt_krw = self.get_upbit_usdt_krw().map(|p| p.to_f64());
        // For kimchi premium, we use the API rate if available, otherwise fall back to USDT/KRW
        let usd_krw = crate::exchange_rate::get_api_rate().or(usdt_krw);

        let opps = detector.detect_with_rates(pair_id, usd_krw, usdt_krw);

        if !opps.is_empty() {
            // Store recent opportunities (deduplicate by exchange pair)
            let mut stored = self.opportunities.write().await;

            for opp in &opps {
                // Check if we already have this exchange pair for this asset
                let existing_idx = stored.iter().position(|existing| {
                    existing.asset.symbol == opp.asset.symbol
                        && existing.source_exchange == opp.source_exchange
                        && existing.target_exchange == opp.target_exchange
                });

                if let Some(idx) = existing_idx {
                    // Update existing opportunity
                    stored[idx] = opp.clone();
                } else {
                    // New opportunity
                    stored.push(opp.clone());
                    self.stats.record_opportunity();
                }
            }

            // Keep only last 100
            if stored.len() > 100 {
                let drain_count = stored.len() - 100;
                stored.drain(0..drain_count);
            }
        }

        // Return all detected opportunities for broadcasting
        opps
    }

    /// Get recent opportunities.
    pub async fn recent_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        self.opportunities.read().await.clone()
    }

    /// Get statistics summary.
    pub fn stats_summary(&self) -> StatsSummary {
        self.stats.summary()
    }
}

/// Shared state handle.
pub type SharedState = Arc<AppState>;

/// Create shared state.
pub fn create_state(config: AppConfig) -> SharedState {
    Arc::new(AppState::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_stats_new() {
        let stats = BotStats::new();
        assert_eq!(stats.price_updates.load(Ordering::Relaxed), 0);
        assert!(stats.started_at_ms.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_bot_stats_record() {
        let stats = BotStats::new();
        stats.record_price_update();
        stats.record_price_update();
        assert_eq!(stats.price_updates.load(Ordering::Relaxed), 2);

        stats.record_opportunity();
        assert_eq!(stats.opportunities_detected.load(Ordering::Relaxed), 1);

        stats.record_trade(50);
        assert_eq!(stats.trades_executed.load(Ordering::Relaxed), 1);
        assert_eq!(stats.total_profit_bps.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn test_stats_summary() {
        let stats = BotStats::new();
        stats.record_price_update();
        stats.record_opportunity();

        let summary = stats.summary();
        assert_eq!(summary.price_updates, 1);
        assert_eq!(summary.opportunities_detected, 1);
    }

    #[tokio::test]
    async fn test_app_state_new() {
        let config = AppConfig::default();
        let state = AppState::new(config);
        assert!(!state.is_running());
    }

    #[tokio::test]
    async fn test_app_state_start_stop() {
        let config = AppConfig::default();
        let state = AppState::new(config);

        state.start();
        assert!(state.is_running());

        state.stop();
        assert!(!state.is_running());
    }

    #[tokio::test]
    async fn test_app_state_update_price() {
        let config = AppConfig::default();
        let state = AppState::new(config);

        state
            .update_price(Exchange::Binance, 1, FixedPoint::from_f64(50000.0))
            .await;

        let summary = state.stats_summary();
        assert_eq!(summary.price_updates, 1);
    }

    #[tokio::test]
    async fn test_shared_state() {
        let config = AppConfig::default();
        let state = create_state(config);

        state.start();
        assert!(state.is_running());
    }
}

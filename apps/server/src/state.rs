//! Application state management.

use crate::config::AppConfig;
use arbitrage_core::{ArbitrageOpportunity, Exchange, FixedPoint};
use arbitrage_engine::{DetectorConfig, OpportunityDetector, PremiumMatrix};
use arbitrage_feeds::PriceAggregator;
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
    usdt_krw_price: AtomicU64,
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
            usdt_krw_price: AtomicU64::new(0), // 0 means not yet received
        }
    }

    /// Update USDT/KRW price from Upbit.
    pub fn update_usdt_krw_price(&self, price: FixedPoint) {
        self.usdt_krw_price.store(price.0, Ordering::Relaxed);
    }

    /// Get USDT/KRW price. Returns None if not yet received.
    pub fn get_usdt_krw_price(&self) -> Option<FixedPoint> {
        let price = self.usdt_krw_price.load(Ordering::Relaxed);
        if price == 0 {
            None
        } else {
            Some(FixedPoint(price))
        }
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

    /// Update price from a feed.
    pub async fn update_price(&self, exchange: Exchange, pair_id: u32, price: FixedPoint) {
        // Update price aggregator
        let tick = arbitrage_core::PriceTick::new(exchange, pair_id, price, price, price);
        self.prices.update(tick);

        // Update detector
        {
            let mut detector = self.detector.write().await;
            detector.update_price(exchange, pair_id, price);
        }

        self.stats.record_price_update();
    }

    /// Detect opportunities for a pair.
    pub async fn detect_opportunities(&self, pair_id: u32) -> Vec<ArbitrageOpportunity> {
        let mut detector = self.detector.write().await;
        let opps = detector.detect(pair_id);

        if !opps.is_empty() {
            self.stats.record_opportunity();

            // Store recent opportunities
            let mut stored = self.opportunities.write().await;
            for opp in &opps {
                stored.push(opp.clone());
            }
            // Keep only last 100
            if stored.len() > 100 {
                let drain_count = stored.len() - 100;
                stored.drain(0..drain_count);
            }
        }

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

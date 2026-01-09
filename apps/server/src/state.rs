//! Application state management.

use crate::config::AppConfig;
use arbitrage_core::{symbol_to_pair_id, ArbitrageOpportunity, Exchange, FixedPoint, OptimalSizeReason, QuoteCurrency};
use arbitrage_engine::{DetectorConfig, FeeManager, OpportunityDetector, OrderbookCache, PremiumMatrix};
use arbitrage_feeds::{CommonMarkets, PriceAggregator};
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock; // Still needed for other fields

/// Event sent when a price is updated.
#[derive(Debug, Clone)]
pub struct PriceUpdateEvent {
    pub exchange: Exchange,
    pub pair_id: u32,
    pub symbol: String,
}

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

    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub total_profit_bps: u64,
    pub uptime_secs: u64,
}

/// Depth cache key: (exchange, symbol)
type DepthCacheKey = (String, String);

/// Depth cache value: (bid_size, ask_size)
type DepthCacheValue = (FixedPoint, FixedPoint);

/// Stablecoin prices for an exchange.
/// Stores USDT/USD and USDC/USD (or derived) prices.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExchangeStablecoinPrices {
    /// USDT/USD price (direct or derived)
    pub usdt_usd: Option<f64>,
    /// USDC/USD price (direct or derived)
    pub usdc_usd: Option<f64>,
    /// USDC/USDT price (for derivation)
    pub usdc_usdt: Option<f64>,
    /// USDT/USDC price (for derivation)
    pub usdt_usdc: Option<f64>,
    /// Reference crypto price in USD (e.g., BTC/USD) for cross-derivation
    pub ref_crypto_usd: Option<f64>,
    /// Reference crypto price in USDT (e.g., BTC/USDT) for cross-derivation
    pub ref_crypto_usdt: Option<f64>,
    /// Reference crypto price in USDC (e.g., BTC/USDC) for cross-derivation
    pub ref_crypto_usdc: Option<f64>,
}

impl ExchangeStablecoinPrices {
    /// Get effective USDT/USD price (direct or derived from cross pairs).
    #[allow(dead_code)]
    pub fn get_usdt_usd(&self, fallback_usdc_usd: f64) -> f64 {
        // 1. Direct USDT/USD
        if let Some(price) = self.usdt_usd {
            return price;
        }
        // 2. Derive from crypto prices: USDT/USD = crypto_USD / crypto_USDT
        if let (Some(usd), Some(usdt)) = (self.ref_crypto_usd, self.ref_crypto_usdt) {
            if usdt > 0.0 {
                return usd / usdt;
            }
        }
        // 3. Derive from USDT/USDC * USDC/USD
        if let Some(usdt_usdc) = self.usdt_usdc {
            return usdt_usdc * fallback_usdc_usd;
        }
        // 4. Derive from (1/USDC_USDT) * USDC/USD
        if let Some(usdc_usdt) = self.usdc_usdt {
            return (1.0 / usdc_usdt) * fallback_usdc_usd;
        }
        1.0 // Default fallback
    }

    /// Get effective USDC/USD price (direct or derived from cross pairs).
    #[allow(dead_code)]
    pub fn get_usdc_usd(&self, fallback_usdt_usd: f64) -> f64 {
        // 1. Direct USDC/USD
        if let Some(price) = self.usdc_usd {
            return price;
        }
        // 2. Derive from crypto prices: USDC/USD = crypto_USD / crypto_USDC
        if let (Some(usd), Some(usdc)) = (self.ref_crypto_usd, self.ref_crypto_usdc) {
            if usdc > 0.0 {
                return usd / usdc;
            }
        }
        // 3. Derive from USDC/USDT * USDT/USD
        if let Some(usdc_usdt) = self.usdc_usdt {
            return usdc_usdt * fallback_usdt_usd;
        }
        // 4. Derive from (1/USDT_USDC) * USDT/USD
        if let Some(usdt_usdc) = self.usdt_usdc {
            return (1.0 / usdt_usdc) * fallback_usdt_usd;
        }
        1.0 // Default fallback
    }
}

/// Application state shared across components.
pub struct AppState {
    /// Configuration.
    #[allow(dead_code)]
    pub config: RwLock<AppConfig>,
    /// Price aggregator.
    pub prices: PriceAggregator,
    /// Opportunity detector (internally lock-free via DashMap).
    pub detector: OpportunityDetector,
    /// Premium matrices per pair.
    #[allow(dead_code)]
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
    /// USDC/KRW price from Upbit.
    /// Stored as FixedPoint (e.g., 1435.0 KRW per USDC).
    upbit_usdc_krw: AtomicU64,
    /// USDT/KRW price from Bithumb.
    /// Stored as FixedPoint.
    bithumb_usdt_krw: AtomicU64,
    /// USDC/KRW price from Bithumb.
    /// Stored as FixedPoint.
    bithumb_usdc_krw: AtomicU64,
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
    /// Exchange-specific stablecoin prices (USDT/USD, USDC/USD, cross pairs).
    stablecoin_prices: DashMap<Exchange, ExchangeStablecoinPrices>,
    /// Full orderbook cache: (exchange, pair_id) -> OrderbookCache
    /// Used for optimal size calculation via depth walking algorithm.
    orderbook_cache: DashMap<(Exchange, u32), OrderbookCache>,
    /// Fee manager for all exchanges.
    fee_manager: RwLock<FeeManager>,
    /// Channel to notify detector of price updates.
    price_update_tx: mpsc::Sender<PriceUpdateEvent>,
}

impl AppState {
    /// Create new application state.
    /// Returns the state and a receiver for price update events.
    pub fn new(config: AppConfig) -> (Self, mpsc::Receiver<PriceUpdateEvent>) {
        let detector_config: DetectorConfig = (&config.detector).into();
        // Channel for price update notifications (bounded to prevent backpressure)
        let (price_update_tx, price_update_rx) = mpsc::channel(1024);

        let state = Self {
            config: RwLock::new(config),
            prices: PriceAggregator::new(),
            detector: OpportunityDetector::new(detector_config),
            matrices: DashMap::new(),
            opportunities: RwLock::new(Vec::new()),
            stats: BotStats::new(),
            running: AtomicBool::new(false),
            upbit_usdt_krw: AtomicU64::new(0), // 0 means not yet received
            upbit_usdc_krw: AtomicU64::new(0), // 0 means not yet received
            bithumb_usdt_krw: AtomicU64::new(0), // 0 means not yet received
            bithumb_usdc_krw: AtomicU64::new(0), // 0 means not yet received
            usdt_usd_price: AtomicU64::new(FixedPoint::from_f64(1.0).0), // Default 1:1
            usdc_usdt_price: AtomicU64::new(FixedPoint::from_f64(1.0).0), // Default 1:1
            common_markets: RwLock::new(None),
            depth_cache: DashMap::new(),
            stablecoin_prices: DashMap::new(),
            orderbook_cache: DashMap::new(),
            fee_manager: RwLock::new(FeeManager::new()),
            price_update_tx,
        };
        (state, price_update_rx)
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

    /// Update USDC/KRW price from Upbit.
    pub fn update_upbit_usdc_krw(&self, price: FixedPoint) {
        self.upbit_usdc_krw.store(price.0, Ordering::Relaxed);
    }

    /// Get Upbit USDC/KRW price. Returns None if not yet received.
    pub fn get_upbit_usdc_krw(&self) -> Option<FixedPoint> {
        let price = self.upbit_usdc_krw.load(Ordering::Relaxed);
        if price == 0 {
            None
        } else {
            Some(FixedPoint(price))
        }
    }

    /// Update USDC/KRW price from Bithumb.
    pub fn update_bithumb_usdc_krw(&self, price: FixedPoint) {
        self.bithumb_usdc_krw.store(price.0, Ordering::Relaxed);
    }

    /// Get Bithumb USDC/KRW price. Returns None if not yet received.
    pub fn get_bithumb_usdc_krw(&self) -> Option<FixedPoint> {
        let price = self.bithumb_usdc_krw.load(Ordering::Relaxed);
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

    /// Get USDC/KRW price for a specific exchange.
    pub fn get_usdc_krw_for_exchange(&self, exchange: Exchange) -> Option<FixedPoint> {
        match exchange {
            Exchange::Upbit => self.get_upbit_usdc_krw(),
            Exchange::Bithumb => self.get_bithumb_usdc_krw(),
            _ => None, // Non-KRW exchanges don't need USDC/KRW
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
    #[allow(dead_code)]
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

    /// Update exchange-specific stablecoin price.
    /// Call this for each stablecoin pair received from an exchange.
    pub fn update_exchange_stablecoin_price(
        &self,
        exchange: Exchange,
        base: &str,
        quote: &str,
        price: f64,
    ) {
        let mut entry = self.stablecoin_prices.entry(exchange).or_default();
        let prices = entry.value_mut();

        match (base, quote) {
            ("USDT", "USD") => prices.usdt_usd = Some(price),
            ("USDC", "USD") => prices.usdc_usd = Some(price),
            ("USDC", "USDT") => prices.usdc_usdt = Some(price),
            ("USDT", "USDC") => prices.usdt_usdc = Some(price),
            _ => {}
        }
    }

    /// Update reference crypto prices for deriving stablecoin rates.
    /// Uses BTC as reference: BTC/USD, BTC/USDT, BTC/USDC prices.
    /// Call this for exchanges like Bybit that have USD markets but no direct stablecoin/USD pairs.
    pub fn update_exchange_ref_crypto_price(
        &self,
        exchange: Exchange,
        quote: &str,
        price: f64,
    ) {
        let mut entry = self.stablecoin_prices.entry(exchange).or_default();
        let prices = entry.value_mut();

        match quote {
            "USD" => prices.ref_crypto_usd = Some(price),
            "USDT" => prices.ref_crypto_usdt = Some(price),
            "USDC" => prices.ref_crypto_usdc = Some(price),
            _ => {}
        }
    }

    /// Get stablecoin prices for an exchange.
    #[allow(dead_code)]
    pub fn get_exchange_stablecoin_prices(&self, exchange: Exchange) -> ExchangeStablecoinPrices {
        self.stablecoin_prices
            .get(&exchange)
            .map(|r| *r.value())
            .unwrap_or_default()
    }

    /// Get USDT/USD price for a specific exchange.
    /// Falls back to global average if not available.
    #[allow(dead_code)]
    pub fn get_usdt_usd_for_exchange(&self, exchange: Exchange) -> f64 {
        let prices = self.get_exchange_stablecoin_prices(exchange);
        let global_usdc_usd = self.get_usdc_usd_price().to_f64();
        prices.get_usdt_usd(global_usdc_usd)
    }

    /// Get USDC/USD price for a specific exchange.
    /// Falls back to global average if not available.
    #[allow(dead_code)]
    pub fn get_usdc_usd_for_exchange(&self, exchange: Exchange) -> f64 {
        let prices = self.get_exchange_stablecoin_prices(exchange);
        let global_usdt_usd = self.get_usdt_usd_price().to_f64();
        prices.get_usdc_usd(global_usdt_usd)
    }

    /// Get stablecoin/USD price for a specific exchange and quote currency.
    #[allow(dead_code)]
    pub fn get_stablecoin_usd_for_exchange(&self, exchange: Exchange, quote: QuoteCurrency) -> f64 {
        match quote {
            QuoteCurrency::USDT => self.get_usdt_usd_for_exchange(exchange),
            QuoteCurrency::USDC => self.get_usdc_usd_for_exchange(exchange),
            QuoteCurrency::USD => 1.0,
            QuoteCurrency::BUSD => 1.0, // BUSD is pegged to USD
            QuoteCurrency::KRW => 1.0, // KRW conversion handled separately
        }
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

        // Update detector with bid/ask for accurate premium calculation (lock-free)
        // Get or compute symbol for depth cache
        let symbol = self.detector.pair_id_to_symbol(pair_id);
        if let Some(sym) = &symbol {
            // OPTIMIZATION: Use entry() API for atomic read-modify-write
            // Skip storing if both new values are zero (wait for WebSocket to provide depth)
            if bid_size.0 > 0 || ask_size.0 > 0 {
                let key = (format!("{:?}", exchange), sym.clone());
                self.depth_cache
                    .entry(key)
                    .and_modify(|existing| {
                        if bid_size.0 > 0 {
                            existing.0 = bid_size;
                        }
                        if ask_size.0 > 0 {
                            existing.1 = ask_size;
                        }
                    })
                    .or_insert((bid_size, ask_size));
            }
        }
        self.detector.update_price_with_bid_ask(exchange, pair_id, price, bid, ask, bid_size, ask_size, quote);

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

        // OPTIMIZATION: Use entry() API for atomic read-modify-write
        // Skip storing if both new values are zero (wait for WebSocket to provide depth)
        if bid_size.0 > 0 || ask_size.0 > 0 {
            let key = (format!("{:?}", exchange), symbol.to_string());
            self.depth_cache
                .entry(key)
                .and_modify(|existing| {
                    if bid_size.0 > 0 {
                        existing.0 = bid_size;
                    }
                    if ask_size.0 > 0 {
                        existing.1 = ask_size;
                    }
                })
                .or_insert((bid_size, ask_size));
        }

        // Update detector with bid/ask for accurate premium calculation (lock-free)
        // Register symbol if not already registered
        self.detector.get_or_register_pair_id(symbol);
        self.detector.update_price_with_bid_ask(exchange, pair_id, price, bid, ask, bid_size, ask_size, quote);

        self.stats.record_price_update();
    }

    /// Update price from a feed with bid/ask from orderbook, symbol, and separate raw prices.
    /// Use this for KRW exchanges where raw prices (original KRW) differ from USD-normalized prices.
    ///
    /// # Arguments
    /// * `exchange` - The exchange
    /// * `pair_id` - The pair ID
    /// * `symbol` - The symbol (e.g., "BTC")
    /// * `price` - USD-normalized mid price
    /// * `bid` - USD-normalized bid price
    /// * `ask` - USD-normalized ask price
    /// * `raw_bid` - Original exchange bid price (e.g., KRW)
    /// * `raw_ask` - Original exchange ask price (e.g., KRW)
    /// * `bid_size` - Bid size
    /// * `ask_size` - Ask size
    /// * `quote` - Quote currency
    pub async fn update_price_with_bid_ask_and_raw(
        &self,
        exchange: Exchange,
        pair_id: u32,
        symbol: &str,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        raw_bid: FixedPoint,
        raw_ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        quote: QuoteCurrency,
    ) {
        // Update price aggregator with USD-normalized prices
        let tick = arbitrage_core::PriceTick::with_depth(exchange, pair_id, price, bid, ask, bid_size, ask_size, quote);
        self.prices.update(tick);

        // Update depth cache
        if bid_size.0 > 0 || ask_size.0 > 0 {
            let key = (format!("{:?}", exchange), symbol.to_string());
            self.depth_cache
                .entry(key)
                .and_modify(|existing| {
                    if bid_size.0 > 0 {
                        existing.0 = bid_size;
                    }
                    if ask_size.0 > 0 {
                        existing.1 = ask_size;
                    }
                })
                .or_insert((bid_size, ask_size));
        }

        // Update detector with bid/ask and raw prices (lock-free)
        self.detector.get_or_register_pair_id(symbol);
        self.detector.update_price_with_bid_ask_and_raw(exchange, pair_id, price, bid, ask, raw_bid, raw_ask, bid_size, ask_size, quote);

        self.stats.record_price_update();

        // Notify detector of price update (non-blocking)
        let _ = self.price_update_tx.try_send(PriceUpdateEvent {
            exchange,
            pair_id,
            symbol: symbol.to_string(),
        });
    }

    /// Get cached orderbook depth for an exchange and symbol.
    /// Returns (bid_size, ask_size) or None if not cached.
    pub fn get_depth(&self, exchange: &str, symbol: &str) -> Option<(FixedPoint, FixedPoint)> {
        let key = (exchange.to_string(), symbol.to_string());
        self.depth_cache.get(&key).map(|v| *v)
    }

    /// Update full orderbook snapshot for depth walking calculation.
    ///
    /// # Arguments
    /// * `exchange` - The exchange
    /// * `pair_id` - The pair ID
    /// * `bids` - Bids as (price, qty) in f64, sorted descending by price
    /// * `asks` - Asks as (price, qty) in f64, sorted ascending by price
    pub fn update_orderbook_snapshot(
        &self,
        exchange: Exchange,
        pair_id: u32,
        bids: &[(f64, f64)],
        asks: &[(f64, f64)],
    ) {
        let key = (exchange, pair_id);
        let mut entry = self.orderbook_cache.entry(key).or_insert_with(OrderbookCache::default);
        entry.update_snapshot_f64(bids, asks);
    }

    /// Apply delta updates to an existing orderbook.
    /// Delta updates only contain changed levels, not the full orderbook.
    pub fn apply_orderbook_delta(
        &self,
        exchange: Exchange,
        pair_id: u32,
        bids: &[(f64, f64)],
        asks: &[(f64, f64)],
    ) {
        use arbitrage_engine::Side;

        let key = (exchange, pair_id);
        if let Some(mut entry) = self.orderbook_cache.get_mut(&key) {
            for &(price, qty) in bids {
                entry.apply_delta_f64(Side::Bid, price, qty);
            }
            for &(price, qty) in asks {
                entry.apply_delta_f64(Side::Ask, price, qty);
            }
        }
        // If no existing orderbook, delta is ignored (need snapshot first)
    }

    /// Get best bid and ask prices from orderbook cache.
    /// Returns (best_bid_price, best_ask_price, best_bid_size, best_ask_size) if available.
    pub fn get_best_bid_ask(&self, exchange: Exchange, pair_id: u32) -> Option<(f64, f64, f64, f64)> {
        self.orderbook_cache.get(&(exchange, pair_id)).and_then(|cache| {
            let (bid_price, bid_size) = cache.best_bid()?;
            let (ask_price, ask_size) = cache.best_ask()?;
            Some((bid_price.to_f64(), ask_price.to_f64(), bid_size.to_f64(), ask_size.to_f64()))
        })
    }

    /// Get orderbook for an exchange and pair.
    #[allow(dead_code)]
    pub fn get_orderbook(&self, exchange: Exchange, pair_id: u32) -> Option<OrderbookCache> {
        self.orderbook_cache.get(&(exchange, pair_id)).map(|v| v.clone())
    }

    /// Clear all orderbook and depth cache for a specific exchange.
    pub fn clear_orderbooks_for_exchange(&self, exchange: Exchange) {
        let exchange_str = format!("{:?}", exchange);
        self.orderbook_cache.retain(|key, _| key.0 != exchange);
        self.depth_cache.retain(|key, _| key.0 != exchange_str);
        tracing::info!("{:?}: Orderbook cache cleared (No OB until snapshot)", exchange);
    }

    /// Clear all cached data for a specific exchange (orderbooks, depth, and detector prices).
    /// Call this on WebSocket reconnection to avoid using stale data.
    pub fn clear_exchange_caches(&self, exchange: Exchange) {
        // Clear orderbook and depth cache (sync)
        self.clear_orderbooks_for_exchange(exchange);

        // Clear detector prices (lock-free)
        self.detector.clear_exchange_prices(exchange);
        tracing::info!("{:?}: Detector prices cleared", exchange);
    }

    /// Expire stale prices from all detector matrices.
    /// Call this periodically to clean up old data.
    pub fn expire_stale_prices(&self) -> usize {
        self.detector.expire_stale_prices()
    }

    /// Get orderbooks for both sides of an arbitrage opportunity.
    /// Returns (buy_orderbook, sell_orderbook) if both are available.
    #[allow(dead_code)]
    pub fn get_arbitrage_orderbooks(
        &self,
        buy_exchange: Exchange,
        sell_exchange: Exchange,
        pair_id: u32,
    ) -> Option<(OrderbookCache, OrderbookCache)> {
        let buy_ob = self.orderbook_cache.get(&(buy_exchange, pair_id))?;
        let sell_ob = self.orderbook_cache.get(&(sell_exchange, pair_id))?;
        Some((buy_ob.clone(), sell_ob.clone()))
    }

    /// Get fee manager for reading fees.
    #[allow(dead_code)]
    pub async fn get_fee_manager(&self) -> tokio::sync::RwLockReadGuard<'_, FeeManager> {
        self.fee_manager.read().await
    }

    /// Update withdrawal fee for an asset (from exchange API).
    #[allow(dead_code)]
    pub async fn update_withdrawal_fee(
        &self,
        exchange: Exchange,
        asset: &str,
        fee: u64,
        min_withdrawal: u64,
    ) {
        let mut manager = self.fee_manager.write().await;
        manager.update_withdrawal_fee(exchange, asset, fee, min_withdrawal, None);
    }

    /// Update price for a symbol (dynamic markets).
    /// Uses mid price as bid/ask when only price is available.
    #[allow(dead_code)]
    pub async fn update_price_for_symbol(
        &self,
        exchange: Exchange,
        symbol: &str,
        price: FixedPoint,
    ) {
        self.update_price_for_symbol_with_bid_ask(exchange, symbol, price, price, price, FixedPoint::from_f64(0.0), FixedPoint::from_f64(0.0)).await;
    }

    /// Update price for a symbol with bid/ask from orderbook.
    #[allow(dead_code)]
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

        // Update detector with bid/ask (registers symbol if needed) - lock-free
        self.detector.get_or_register_pair_id(symbol);
        self.detector.update_price_with_bid_ask(exchange, pair_id, price, bid, ask, bid_size, ask_size, arbitrage_core::QuoteCurrency::USD);

        self.stats.record_price_update();
    }

    /// Register symbols from common markets for opportunity detection.
    pub fn register_common_markets(&self, markets: &CommonMarkets) {
        for symbol in markets.common_bases() {
            self.detector.register_symbol(&symbol);
        }
    }

    /// Get all registered pair_ids for opportunity detection.
    pub fn get_registered_pair_ids(&self) -> Vec<u32> {
        self.detector.registered_pair_ids()
    }

    /// Detect opportunities for a pair.
    /// Returns all detected opportunities (both new and updated).
    pub async fn detect_opportunities(&self, pair_id: u32) -> Vec<ArbitrageOpportunity> {
        use arbitrage_engine::{calculate_optimal_size, DepthFeeConfig};

        // Get exchange rates for multi-denomination premium calculation (lock-free atomic reads)
        let usdt_krw = self.get_upbit_usdt_krw().map(|p| p.to_f64());
        let usdc_krw = self.get_upbit_usdc_krw().map(|p| p.to_f64());
        let usd_krw = crate::exchange_rate::get_api_rate().or(usdt_krw);

        // Detection is now lock-free (DashMap internally)
        let mut opps = self.detector.detect_with_all_rates(pair_id, usd_krw, usdt_krw, usdc_krw);

        // OPTIMIZATION: Collect all fee data upfront, then release lock immediately
        let fee_data: Vec<_> = {
            let fee_manager = self.fee_manager.read().await;
            opps.iter()
                .map(|opp| {
                    fee_manager.get_arbitrage_fees(
                        opp.source_exchange,
                        opp.target_exchange,
                        &opp.asset.symbol,
                    )
                })
                .collect()
        }; // fee_manager lock released here

        // Calculate optimal_size for each opportunity using orderbook depth (no locks held)
        for (i, opp) in opps.iter_mut().enumerate() {
            // Get orderbooks for both sides (DashMap - lock-free)
            let buy_ob = self.orderbook_cache.get(&(opp.source_exchange, pair_id));
            let sell_ob = self.orderbook_cache.get(&(opp.target_exchange, pair_id));

            // Check if orderbook available for both sides
            if buy_ob.is_none() || sell_ob.is_none() {
                opp.optimal_size_reason = OptimalSizeReason::NoOrderbook;
                continue;
            }

            if let (Some(buy_ob), Some(sell_ob)) = (buy_ob, sell_ob) {
                let mut buy_asks = buy_ob.asks_vec();
                let mut sell_bids = sell_ob.bids_vec();

                // Normalize prices to the overseas exchange's quote currency
                let source_is_krw = opp.source_quote == QuoteCurrency::KRW;
                let target_is_krw = opp.target_quote == QuoteCurrency::KRW;

                // Determine the overseas quote currency (non-KRW side)
                let overseas_quote = if source_is_krw {
                    opp.target_quote
                } else if target_is_krw {
                    opp.source_quote
                } else {
                    QuoteCurrency::USD
                };

                // Get the appropriate KRW rate based on overseas quote
                let get_krw_rate_for_quote = |exchange: Exchange, quote: QuoteCurrency| -> Option<u64> {
                    match quote {
                        QuoteCurrency::USDT => self.get_usdt_krw_for_exchange(exchange).map(|p| p.0),
                        QuoteCurrency::USDC => self.get_usdc_krw_for_exchange(exchange).map(|p| p.0),
                        _ => self.get_usdt_krw_for_exchange(exchange).map(|p| p.0),
                    }
                };

                // OPTIMIZATION: In-place KRW conversion to avoid extra heap allocation
                if source_is_krw {
                    match get_krw_rate_for_quote(opp.source_exchange, overseas_quote) {
                        Some(rate) if rate > 0 => {
                            for (price, _) in buy_asks.iter_mut() {
                                *price = (*price as u128 * FixedPoint::SCALE as u128 / rate as u128) as u64;
                            }
                        }
                        _ => {
                            opp.optimal_size_reason = OptimalSizeReason::NoConversionRate;
                            continue;
                        }
                    }
                }

                if target_is_krw {
                    match get_krw_rate_for_quote(opp.target_exchange, overseas_quote) {
                        Some(rate) if rate > 0 => {
                            for (price, _) in sell_bids.iter_mut() {
                                *price = (*price as u128 * FixedPoint::SCALE as u128 / rate as u128) as u64;
                            }
                        }
                        _ => {
                            opp.optimal_size_reason = OptimalSizeReason::NoConversionRate;
                            continue;
                        }
                    }
                }

                // Use pre-fetched fee data
                let (buy_fee, sell_fee, withdrawal_fee) = fee_data[i];

                let fees = DepthFeeConfig {
                    buy_fee_bps: buy_fee,
                    sell_fee_bps: sell_fee,
                    withdrawal_fee,
                };

                // Calculate optimal size using depth walking algorithm
                let result = calculate_optimal_size(
                    &buy_asks,
                    &sell_bids,
                    fees,
                );

                opp.optimal_size = result.amount;
                opp.optimal_profit = result.profit;

                if result.amount > 0 {
                    opp.optimal_size_reason = OptimalSizeReason::Ok;
                } else {
                    opp.optimal_size_reason = OptimalSizeReason::NotProfitable;
                }
            }
        }

        // Record stats for new opportunities (lock-free)
        for opp in &opps {
            // Use a simple heuristic: count unique opportunities
            // The actual deduplication happens at broadcast time
            self.stats.record_opportunity();
            let _ = opp; // suppress unused warning
        }

        opps
    }

    /// Get recent opportunities.
    #[allow(dead_code)]
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

/// Receiver for price update events.
pub type PriceUpdateReceiver = mpsc::Receiver<PriceUpdateEvent>;

/// Create shared state and price update receiver.
pub fn create_state(config: AppConfig) -> (SharedState, PriceUpdateReceiver) {
    let (state, rx) = AppState::new(config);
    (Arc::new(state), rx)
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
        let (state, _rx) = AppState::new(config);
        assert!(!state.is_running());
    }

    #[tokio::test]
    async fn test_app_state_start_stop() {
        let config = AppConfig::default();
        let (state, _rx) = AppState::new(config);

        state.start();
        assert!(state.is_running());

        state.stop();
        assert!(!state.is_running());
    }

    #[tokio::test]
    async fn test_app_state_update_price() {
        let config = AppConfig::default();
        let (state, _rx) = AppState::new(config);

        state
            .update_price(Exchange::Binance, 1, FixedPoint::from_f64(50000.0))
            .await;

        let summary = state.stats_summary();
        assert_eq!(summary.price_updates, 1);
    }

    #[tokio::test]
    async fn test_shared_state() {
        let config = AppConfig::default();
        let (state, _rx) = create_state(config);

        state.start();
        assert!(state.is_running());
    }
}

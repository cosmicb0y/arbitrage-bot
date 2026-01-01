//! Arbitrage Bot - Headless Server
//!
//! A high-performance cryptocurrency arbitrage detection and execution bot.

mod config;
mod exchange_rate;
mod state;
mod wallet_status;
mod ws_server;

use clap::Parser;
use config::{AppConfig, ExecutionMode};
use state::{create_state, SharedState};
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use arbitrage_alerts::{Database, Notifier, NotifierConfig, TelegramBot};

use arbitrage_core::{Exchange, FixedPoint, PriceTick, QuoteCurrency};
use arbitrage_feeds::{
    load_mappings, BinanceAdapter, BithumbAdapter, BybitAdapter, CoinbaseAdapter, FeedConfig,
    GateIOAdapter, MarketDiscovery, SymbolMappings, UpbitAdapter, WsClient, WsMessage,
    BinanceRestFetcher, BybitRestFetcher, GateIORestFetcher, UpbitRestFetcher,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use ws_server::BroadcastSender;

/// Arbitrage Bot CLI
#[derive(Parser, Debug)]
#[command(name = "arbitrage-bot")]
#[command(about = "High-performance crypto arbitrage bot", long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.json")]
    config: String,

    /// Minimum premium in basis points
    #[arg(short = 'p', long, default_value_t = 30)]
    min_premium: i32,

    /// Execution mode: auto, manual, alert
    #[arg(short, long, default_value = "alert")]
    mode: String,

    /// Log level: trace, debug, info, warn, error
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Dry run (no actual trades)
    #[arg(long, default_value_t = true)]
    dry_run: bool,

    /// Use live WebSocket feeds instead of simulator
    #[arg(long, default_value_t = false)]
    live: bool,

    /// WebSocket server port for clients (Tauri app)
    #[arg(long, default_value_t = 9001)]
    ws_port: u16,

    /// Enable Telegram alerts (requires TELEGRAM_BOT_TOKEN env var)
    #[arg(long, default_value_t = false)]
    telegram: bool,

    /// SQLite database path for alert configuration
    #[arg(long, default_value = "data/alerts.db")]
    db_path: String,
}

fn init_logging(level: &str) {
    // Build filter that applies the requested level to our crates
    // but filters out noisy dependencies (reqwest, hyper, rustls, etc.)
    let filter = EnvFilter::try_new(format!(
        "{level},\
         hyper=warn,\
         hyper_util=warn,\
         reqwest=warn,\
         rustls=warn,\
         tokio_tungstenite=warn,\
         tungstenite=warn,\
         h2=warn,\
         tower=warn",
        level = level
    ))
    .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .compact(),
        )
        .init();
}

fn parse_mode(mode: &str) -> ExecutionMode {
    match mode.to_lowercase().as_str() {
        "auto" => ExecutionMode::Auto,
        "manual" => ExecutionMode::ManualApproval,
        _ => ExecutionMode::AlertOnly,
    }
}

async fn run_detector_loop(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    notifier: Option<Arc<Notifier>>,
) {
    info!("Starting detector loop");

    // Legacy hardcoded pair IDs for backwards compatibility
    let legacy_pair_ids = vec![1u32, 2, 3]; // BTC, ETH, SOL

    while state.is_running() {
        // Get all registered pair_ids (includes dynamic markets from discovery)
        let mut pair_ids = state.get_registered_pair_ids().await;

        // Add legacy pair_ids if not already present
        for &legacy_id in &legacy_pair_ids {
            if !pair_ids.contains(&legacy_id) {
                pair_ids.push(legacy_id);
            }
        }

        for &pair_id in &pair_ids {
            let opps = state.detect_opportunities(pair_id).await;

            for opp in opps {
                if opp.premium_bps >= 30 {
                    tracing::debug!(
                        "🎯 Opportunity: {} {:?} -> {:?} | Premium: {} bps | Buy: {} | Sell: {}",
                        opp.asset.symbol,
                        opp.source_exchange,
                        opp.target_exchange,
                        opp.premium_bps,
                        opp.source_price,
                        opp.target_price
                    );
                    // Broadcast opportunity to clients
                    ws_server::broadcast_opportunity(&broadcast_tx, &state, &opp);

                    // Send Telegram alert if notifier is configured
                    if let Some(ref notifier) = notifier {
                        if let Err(e) = notifier.process_opportunity(&opp).await {
                            tracing::warn!("Failed to send Telegram alert: {}", e);
                        }
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    info!("Detector loop stopped");
}

async fn run_price_simulator(state: SharedState, broadcast_tx: BroadcastSender) {
    info!("Starting price simulator (demo mode)");

    // Include Upbit for kimchi premium simulation
    let exchanges = vec![
        Exchange::Binance,
        Exchange::Coinbase,
        Exchange::Kraken,
        Exchange::Okx,
        Exchange::Upbit,
    ];

    let symbols = ["BTC", "ETH", "SOL"];
    let mut base_prices = vec![50000.0, 3000.0, 100.0]; // BTC, ETH, SOL
    let mut counter = 0u64;

    while state.is_running() {
        for (idx, base_price) in base_prices.iter_mut().enumerate() {
            let pair_id = (idx + 1) as u32;
            let symbol = symbols[idx];

            for (i, &exchange) in exchanges.iter().enumerate() {
                // Add some variance per exchange
                // Upbit gets extra premium (kimchi premium simulation: ~2-5%)
                let base_variance = if exchange == Exchange::Upbit {
                    1.02 + (counter as f64 * 0.0002).sin() * 0.02 // 2-4% premium
                } else {
                    1.0 + (i as f64 * 0.001) + (counter as f64 * 0.0001).sin() * 0.005
                };

                let price = *base_price * base_variance;
                let fp_price = FixedPoint::from_f64(price);

                state.update_price(exchange, pair_id, fp_price).await;

                // Broadcast price update to clients
                let tick = PriceTick::new(exchange, pair_id, fp_price, fp_price, fp_price);
                ws_server::broadcast_price(&broadcast_tx, exchange, pair_id, symbol, &tick);
            }

            // Drift base price slightly
            *base_price *= 1.0 + (counter as f64 * 0.001).sin() * 0.0001;
        }

        counter += 1;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    info!("Price simulator stopped");
}

async fn run_stats_reporter(state: SharedState, broadcast_tx: BroadcastSender) {
    info!("Starting stats reporter");

    loop {
        // Check every 100ms if we should stop, but only print every 10s
        for _ in 0..100 {
            if !state.is_running() {
                info!("Stats reporter stopped");
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let summary = state.stats_summary();
        info!(
            "📊 Stats | Uptime: {}s | Prices: {} | Opportunities: {} | Trades: {}",
            summary.uptime_secs,
            summary.price_updates,
            summary.opportunities_detected,
            summary.trades_executed
        );
        // Broadcast stats to clients
        ws_server::broadcast_stats(&broadcast_tx, &state);
    }
}

/// Convert KRW price to USD using exchange-specific USDT/KRW rate.
/// Uses USDT/KRW from the same exchange, then converts via USDT/USD.
/// Returns None if exchange rate is not available yet.
fn convert_krw_to_usd_for_exchange(
    krw_price: FixedPoint,
    exchange: Exchange,
    state: &SharedState,
) -> Option<FixedPoint> {
    // Get exchange-specific USDT/KRW rate
    let usdt_krw = state.get_usdt_krw_for_exchange(exchange)?;
    let usdt_usd = state.get_usdt_usd_price();

    // KRW -> USDT -> USD
    // price_usdt = krw_price / usdt_krw
    // price_usd = price_usdt * usdt_usd
    let price_usdt = krw_price.to_f64() / usdt_krw.to_f64();
    let price_usd = price_usdt * usdt_usd.to_f64();

    Some(FixedPoint::from_f64(price_usd))
}

/// Cache for orderbook bid/ask and sizes (code -> (bid, ask, bid_size, ask_size) in KRW)
type OrderbookCache = std::sync::Arc<dashmap::DashMap<String, (FixedPoint, FixedPoint, FixedPoint, FixedPoint)>>;

/// Process Upbit ticker data by market code.
fn process_upbit_ticker(
    code: &str,
    price: FixedPoint,
    state: &SharedState,
    broadcast_tx: &BroadcastSender,
    symbol_mappings: &SymbolMappings,
    orderbook_cache: &OrderbookCache,
) {
    // Handle USDT/KRW for exchange rate
    if UpbitAdapter::is_usdt_market(code) {
        state.update_upbit_usdt_krw(price);
        let rate = price.to_f64();
        ws_server::broadcast_exchange_rate(broadcast_tx, state, rate);
        tracing::debug!("Updated Upbit USDT/KRW rate: {:.2}", rate);
        return;
    }

    // Handle trading pairs - extract symbol from code (e.g., "KRW-BTC" -> "BTC")
    if let Some(symbol) = UpbitAdapter::extract_base_symbol(code) {
        // Use canonical name if mapping exists
        let display_symbol = symbol_mappings.canonical_name("Upbit", &symbol);
        let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

        // Get bid/ask and sizes from orderbook cache (in KRW), default to price if not available
        let (bid_krw, ask_krw, bid_size, ask_size) = orderbook_cache
            .get(code)
            .map(|r| *r.value())
            .unwrap_or((price, price, FixedPoint::from_f64(0.0), FixedPoint::from_f64(0.0)));

        // Convert KRW to USD using Upbit's USDT/KRW rate
        // Skip if exchange rate is not available yet
        if let Some(price_usd) = convert_krw_to_usd_for_exchange(price, Exchange::Upbit, state) {
            let bid_usd = convert_krw_to_usd_for_exchange(bid_krw, Exchange::Upbit, state).unwrap_or(price_usd);
            let ask_usd = convert_krw_to_usd_for_exchange(ask_krw, Exchange::Upbit, state).unwrap_or(price_usd);
            let tick_usd = PriceTick::new(Exchange::Upbit, pair_id, price_usd, bid_usd, ask_usd)
                .with_sizes(bid_size, ask_size);

            // Update state asynchronously with KRW quote and bid/ask
            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.update_price_with_bid_ask(Exchange::Upbit, pair_id, price_usd, bid_usd, ask_usd, bid_size, ask_size, QuoteCurrency::KRW).await;
            });

            ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::Upbit, pair_id, &display_symbol, Some("KRW"), &tick_usd);
        }
    }
}

/// Process Upbit orderbook data by market code.
fn process_upbit_orderbook(
    code: &str,
    bid: FixedPoint,
    ask: FixedPoint,
    bid_size: FixedPoint,
    ask_size: FixedPoint,
    orderbook_cache: &OrderbookCache,
) {
    // Skip USDT market
    if UpbitAdapter::is_usdt_market(code) {
        return;
    }
    // Store bid/ask and sizes in cache for use by ticker handler
    orderbook_cache.insert(code.to_string(), (bid, ask, bid_size, ask_size));
}

/// Run live WebSocket feed for Upbit
async fn run_upbit_feed(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    mut rx: mpsc::Receiver<WsMessage>,
    symbol_mappings: Arc<SymbolMappings>,
) {
    info!("Starting Upbit live feed processor");
    let orderbook_cache: OrderbookCache = std::sync::Arc::new(dashmap::DashMap::new());

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Try orderbook first (has bid/ask)
                if UpbitAdapter::is_orderbook_message(&text) {
                    if let Ok((code, bid, ask, bid_size, ask_size)) = UpbitAdapter::parse_orderbook_with_code(&text) {
                        process_upbit_orderbook(&code, bid, ask, bid_size, ask_size, &orderbook_cache);
                    }
                } else if let Ok((code, price)) = UpbitAdapter::parse_ticker_with_code(&text) {
                    // Parse ticker with market code
                    process_upbit_ticker(&code, price, &state, &broadcast_tx, &symbol_mappings, &orderbook_cache);
                }
            }
            WsMessage::Binary(data) => {
                // Upbit sends binary MessagePack data
                // Try orderbook first
                if let Ok((code, bid, ask, bid_size, ask_size)) = UpbitAdapter::parse_orderbook_binary_with_code(&data) {
                    process_upbit_orderbook(&code, bid, ask, bid_size, ask_size, &orderbook_cache);
                } else if let Ok((code, price)) = UpbitAdapter::parse_ticker_binary_with_code(&data) {
                    process_upbit_ticker(&code, price, &state, &broadcast_tx, &symbol_mappings, &orderbook_cache);
                } else {
                    // Fallback: try parsing as UTF-8 JSON
                    if let Ok(text) = String::from_utf8(data.clone()) {
                        if UpbitAdapter::is_orderbook_message(&text) {
                            if let Ok((code, bid, ask, bid_size, ask_size)) = UpbitAdapter::parse_orderbook_with_code(&text) {
                                process_upbit_orderbook(&code, bid, ask, bid_size, ask_size, &orderbook_cache);
                            }
                        } else if let Ok((code, price)) = UpbitAdapter::parse_ticker_with_code(&text) {
                            process_upbit_ticker(&code, price, &state, &broadcast_tx, &symbol_mappings, &orderbook_cache);
                        }
                    }
                }
            }
            WsMessage::Connected => {
                info!("Upbit: Connected to WebSocket");
            }
            WsMessage::Disconnected => {
                warn!("Upbit: Disconnected from WebSocket");
            }
            WsMessage::Error(e) => {
                warn!("Upbit: Error - {}", e);
            }
        }
    }

    info!("Upbit feed processor stopped");
}

/// Run live WebSocket feed for Binance
async fn run_binance_feed(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    mut rx: mpsc::Receiver<WsMessage>,
    symbol_mappings: Arc<SymbolMappings>,
) {
    info!("Starting Binance live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Process bookTicker only (real-time bid/ask with depth)
                if BinanceAdapter::is_book_ticker_message(&text) {
                    if let Ok((tick, symbol, quote)) = BinanceAdapter::parse_book_ticker_with_base_quote(&text) {
                        // Use canonical name if mapping exists
                        let display_symbol = symbol_mappings.canonical_name("Binance", &symbol);
                        let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
                        let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

                        // Update state with orderbook bid/ask (use symbol variant to ensure depth cache is updated)
                        state.update_price_with_bid_ask_and_symbol(Exchange::Binance, pair_id, &display_symbol, tick.price(), tick.bid(), tick.ask(), tick.bid_size(), tick.ask_size(), quote_currency).await;
                        ws_server::broadcast_price_with_quote(&broadcast_tx, Exchange::Binance, pair_id, &display_symbol, Some(&quote), &tick);
                    }
                }
            }
            WsMessage::Connected => {
                info!("Binance: Connected to WebSocket");
            }
            WsMessage::Disconnected => {
                warn!("Binance: Disconnected from WebSocket");
            }
            WsMessage::Error(e) => {
                warn!("Binance: Error - {}", e);
            }
            _ => {}
        }
    }

    info!("Binance feed processor stopped");
}

/// Run live WebSocket feed for Coinbase
async fn run_coinbase_feed(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    mut rx: mpsc::Receiver<WsMessage>,
    symbol_mappings: Arc<SymbolMappings>,
) {
    info!("Starting Coinbase live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Process level2 snapshot only (orderbook depth with bid/ask sizes)
                if CoinbaseAdapter::is_level2_message(&text) {
                    if let Ok((product_id, bid, ask, bid_size, ask_size)) = CoinbaseAdapter::parse_level2_snapshot(&text) {
                        // Extract base symbol and quote from product_id (e.g., BTC-USD -> BTC, USD)
                        if let Some((symbol, quote)) = CoinbaseAdapter::extract_base_quote(&product_id) {
                            // Use canonical name if mapping exists
                            let display_symbol = symbol_mappings.canonical_name("Coinbase", &symbol);
                            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
                            // Treat Coinbase USD as USDC (native USD markets behave like USDC)
                            let normalized_quote = if quote == "USD" { "USDC".to_string() } else { quote };
                            let quote_currency = QuoteCurrency::from_str(&normalized_quote).unwrap_or(QuoteCurrency::USDC);

                            // Calculate mid price from bid/ask
                            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

                            let tick = PriceTick::new(Exchange::Coinbase, pair_id, mid_price, bid, ask)
                                .with_sizes(bid_size, ask_size);

                            // Update state with orderbook data
                            state.update_price_with_bid_ask_and_symbol(Exchange::Coinbase, pair_id, &display_symbol, mid_price, bid, ask, bid_size, ask_size, quote_currency).await;
                            ws_server::broadcast_price_with_quote(&broadcast_tx, Exchange::Coinbase, pair_id, &display_symbol, Some(&normalized_quote), &tick);
                        }
                    }
                }
            }
            WsMessage::Connected => {
                info!("Coinbase: Connected to WebSocket");
            }
            WsMessage::Disconnected => {
                warn!("Coinbase: Disconnected from WebSocket");
            }
            WsMessage::Error(e) => {
                warn!("Coinbase: Error - {}", e);
            }
            _ => {}
        }
    }

    info!("Coinbase feed processor stopped");
}

/// Process Bithumb ticker data by market code.
fn process_bithumb_ticker(
    code: &str,
    price: FixedPoint,
    state: &SharedState,
    broadcast_tx: &BroadcastSender,
    symbol_mappings: &SymbolMappings,
    orderbook_cache: &OrderbookCache,
) {
    // Handle USDT/KRW for exchange rate
    if BithumbAdapter::is_usdt_market(code) {
        state.update_bithumb_usdt_krw(price);
        let rate = price.to_f64();
        ws_server::broadcast_exchange_rate(broadcast_tx, state, rate);
        tracing::debug!("Updated Bithumb USDT/KRW rate: {:.2}", rate);
        return;
    }

    // Handle trading pairs - extract symbol from code (e.g., "KRW-BTC" -> "BTC")
    if let Some(symbol) = BithumbAdapter::extract_base_symbol(code) {
        // Use canonical name if mapping exists
        let display_symbol = symbol_mappings.canonical_name("Bithumb", &symbol);
        let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

        // Get bid/ask and sizes from orderbook cache (in KRW), default to price if not available
        let (bid_krw, ask_krw, bid_size, ask_size) = orderbook_cache
            .get(code)
            .map(|r| *r.value())
            .unwrap_or((price, price, FixedPoint::from_f64(0.0), FixedPoint::from_f64(0.0)));

        // Convert KRW to USD using Bithumb's USDT/KRW rate
        // Skip if exchange rate is not available yet
        if let Some(price_usd) = convert_krw_to_usd_for_exchange(price, Exchange::Bithumb, state) {
            let bid_usd = convert_krw_to_usd_for_exchange(bid_krw, Exchange::Bithumb, state).unwrap_or(price_usd);
            let ask_usd = convert_krw_to_usd_for_exchange(ask_krw, Exchange::Bithumb, state).unwrap_or(price_usd);
            let tick_usd = PriceTick::new(Exchange::Bithumb, pair_id, price_usd, bid_usd, ask_usd)
                .with_sizes(bid_size, ask_size);

            // Update state asynchronously with KRW quote and bid/ask
            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.update_price_with_bid_ask(Exchange::Bithumb, pair_id, price_usd, bid_usd, ask_usd, bid_size, ask_size, QuoteCurrency::KRW).await;
            });

            ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::Bithumb, pair_id, &display_symbol, Some("KRW"), &tick_usd);
        }
    }
}

/// Process Bithumb orderbook data by market code.
fn process_bithumb_orderbook(
    code: &str,
    bid: FixedPoint,
    ask: FixedPoint,
    bid_size: FixedPoint,
    ask_size: FixedPoint,
    orderbook_cache: &OrderbookCache,
) {
    // Skip USDT market
    if BithumbAdapter::is_usdt_market(code) {
        return;
    }
    // Store bid/ask and sizes in cache for use by ticker handler
    orderbook_cache.insert(code.to_string(), (bid, ask, bid_size, ask_size));
}

/// Run live WebSocket feed for Bithumb
async fn run_bithumb_feed(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    mut rx: mpsc::Receiver<WsMessage>,
    symbol_mappings: Arc<SymbolMappings>,
) {
    info!("Starting Bithumb live feed processor");
    let orderbook_cache: OrderbookCache = std::sync::Arc::new(dashmap::DashMap::new());

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Try orderbook first (has bid/ask)
                if BithumbAdapter::is_orderbook_message(&text) {
                    if let Ok((code, bid, ask, bid_size, ask_size)) = BithumbAdapter::parse_orderbook_with_code(&text) {
                        process_bithumb_orderbook(&code, bid, ask, bid_size, ask_size, &orderbook_cache);
                    }
                } else if let Ok((code, price)) = BithumbAdapter::parse_ticker_with_code(&text) {
                    // Parse ticker with market code
                    process_bithumb_ticker(&code, price, &state, &broadcast_tx, &symbol_mappings, &orderbook_cache);
                }
            }
            WsMessage::Binary(data) => {
                // Bithumb sends binary MessagePack data
                // Try orderbook first
                if let Ok((code, bid, ask, bid_size, ask_size)) = BithumbAdapter::parse_orderbook_binary_with_code(&data) {
                    process_bithumb_orderbook(&code, bid, ask, bid_size, ask_size, &orderbook_cache);
                } else if let Ok((code, price)) = BithumbAdapter::parse_ticker_binary_with_code(&data) {
                    process_bithumb_ticker(&code, price, &state, &broadcast_tx, &symbol_mappings, &orderbook_cache);
                } else {
                    // Fallback: try parsing as UTF-8 JSON
                    if let Ok(text) = String::from_utf8(data.clone()) {
                        if BithumbAdapter::is_orderbook_message(&text) {
                            if let Ok((code, bid, ask, bid_size, ask_size)) = BithumbAdapter::parse_orderbook_with_code(&text) {
                                process_bithumb_orderbook(&code, bid, ask, bid_size, ask_size, &orderbook_cache);
                            }
                        } else if let Ok((code, price)) = BithumbAdapter::parse_ticker_with_code(&text) {
                            process_bithumb_ticker(&code, price, &state, &broadcast_tx, &symbol_mappings, &orderbook_cache);
                        }
                    }
                }
            }
            WsMessage::Connected => {
                info!("Bithumb: Connected to WebSocket");
            }
            WsMessage::Disconnected => {
                warn!("Bithumb: Disconnected from WebSocket");
            }
            WsMessage::Error(e) => {
                warn!("Bithumb: Error - {}", e);
            }
        }
    }

    info!("Bithumb feed processor stopped");
}

/// Run live WebSocket feed for Bybit
async fn run_bybit_feed(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    mut rx: mpsc::Receiver<WsMessage>,
    symbol_mappings: Arc<SymbolMappings>,
) {
    info!("Starting Bybit live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Process orderbook only (accurate bid/ask with depth)
                if BybitAdapter::is_orderbook_message(&text) {
                    if let Ok((tick, symbol, quote)) = BybitAdapter::parse_orderbook_with_base_quote(&text) {
                        // Use canonical name if mapping exists
                        let display_symbol = symbol_mappings.canonical_name("Bybit", &symbol);
                        let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
                        let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

                        // Update state with orderbook bid/ask (use symbol variant to ensure depth cache is updated)
                        state.update_price_with_bid_ask_and_symbol(Exchange::Bybit, pair_id, &display_symbol, tick.price(), tick.bid(), tick.ask(), tick.bid_size(), tick.ask_size(), quote_currency).await;
                        ws_server::broadcast_price_with_quote(&broadcast_tx, Exchange::Bybit, pair_id, &display_symbol, Some(&quote), &tick);
                    }
                }
            }
            WsMessage::Binary(_) => {
                // Bybit uses JSON text, not binary
            }
            WsMessage::Connected => {
                info!("Bybit: Connected to WebSocket");
            }
            WsMessage::Disconnected => {
                warn!("Bybit: Disconnected from WebSocket");
            }
            WsMessage::Error(e) => {
                warn!("Bybit: Error - {}", e);
            }
        }
    }

    info!("Bybit feed processor stopped");
}

async fn run_gateio_feed(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    mut rx: mpsc::Receiver<WsMessage>,
    symbol_mappings: Arc<SymbolMappings>,
) {
    info!("Starting Gate.io live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Process orderbook message only (depth with bid/ask sizes)
                if GateIOAdapter::is_orderbook_message(&text) {
                    if let Ok((currency_pair, bid, ask, bid_size, ask_size)) = GateIOAdapter::parse_orderbook_with_symbol(&text) {
                        // Extract base symbol and quote from currency_pair (e.g., BTC_USDT -> BTC, USDT)
                        if let Some((symbol, quote)) = GateIOAdapter::extract_base_quote(&currency_pair) {
                            // Use canonical name if mapping exists
                            let display_symbol = symbol_mappings.canonical_name("GateIO", &symbol);
                            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
                            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

                            // Calculate mid price from bid/ask
                            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

                            let tick = PriceTick::new(Exchange::GateIO, pair_id, mid_price, bid, ask)
                                .with_sizes(bid_size, ask_size);

                            // Update state with orderbook data
                            state.update_price_with_bid_ask_and_symbol(Exchange::GateIO, pair_id, &display_symbol, mid_price, bid, ask, bid_size, ask_size, quote_currency).await;
                            ws_server::broadcast_price_with_quote(&broadcast_tx, Exchange::GateIO, pair_id, &display_symbol, Some(&quote), &tick);
                        }
                    }
                }
            }
            WsMessage::Binary(_) => {
                // Gate.io uses JSON text, not binary
            }
            WsMessage::Connected => {
                info!("Gate.io: Connected to WebSocket");
            }
            WsMessage::Disconnected => {
                warn!("Gate.io: Disconnected from WebSocket");
            }
            WsMessage::Error(e) => {
                warn!("Gate.io: Error - {}", e);
            }
        }
    }

    info!("Gate.io feed processor stopped");
}

/// Fetch initial orderbooks via REST API and populate state.
/// Only fetches from exchanges with batch/bulk ticker APIs for efficiency.
/// Coinbase and Bithumb are skipped - they rely on WebSocket for initial data.
async fn fetch_initial_orderbooks(
    state: &SharedState,
    binance_symbols: &[String],
    bybit_symbols: &[String],
    gateio_symbols: &[String],
    upbit_symbols: &[String],
    symbol_mappings: &SymbolMappings,
) {
    // Only fetch from exchanges that support batch/bulk ticker APIs
    // Binance, Bybit, GateIO: single API call fetches all tickers
    // Upbit: batch API with comma-separated markets
    // Coinbase, Bithumb: No batch API - skip and rely on WebSocket
    info!("📚 Fetching initial orderbooks via REST API (batch-capable exchanges only)...");
    info!("  Binance: {}, Bybit: {}, GateIO: {}, Upbit: {} (Coinbase/Bithumb: WebSocket only)",
        binance_symbols.len(), bybit_symbols.len(), gateio_symbols.len(), upbit_symbols.len());

    // Fetch from batch-capable exchanges in parallel
    let (binance_result, bybit_result, gateio_result, upbit_result) = tokio::join!(
        BinanceRestFetcher::fetch_orderbooks(binance_symbols),
        BybitRestFetcher::fetch_orderbooks(bybit_symbols),
        GateIORestFetcher::fetch_orderbooks(gateio_symbols),
        UpbitRestFetcher::fetch_orderbooks(upbit_symbols),
    );

    let mut total_updated = 0;

    // Process Binance orderbooks
    for (symbol, (bid, ask, bid_size, ask_size)) in &binance_result {
        // Extract base and quote from symbol (e.g., btcusdt -> BTC, USDT)
        if let Some((base, quote)) = extract_binance_base_quote(symbol) {
            let display_symbol = symbol_mappings.canonical_name("Binance", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);
            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

            state.update_price_with_bid_ask_and_symbol(
                Exchange::Binance, pair_id, &display_symbol, mid_price, *bid, *ask, *bid_size, *ask_size, quote_currency
            ).await;
            total_updated += 1;
        }
    }
    info!("  Binance: {} orderbooks loaded", binance_result.len());

    // Process Bybit orderbooks
    for (symbol, (bid, ask, bid_size, ask_size)) in &bybit_result {
        // Extract base and quote from Bybit symbol (e.g., BTCUSDT -> BTC, USDT)
        if let Some((base, quote)) = extract_bybit_base_quote(symbol) {
            let display_symbol = symbol_mappings.canonical_name("Bybit", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);
            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

            state.update_price_with_bid_ask_and_symbol(
                Exchange::Bybit, pair_id, &display_symbol, mid_price, *bid, *ask, *bid_size, *ask_size, quote_currency
            ).await;
            total_updated += 1;
        }
    }
    info!("  Bybit: {} orderbooks loaded", bybit_result.len());

    // Process GateIO orderbooks
    for (currency_pair, (bid, ask, bid_size, ask_size)) in &gateio_result {
        if let Some((base, quote)) = GateIOAdapter::extract_base_quote(currency_pair) {
            let display_symbol = symbol_mappings.canonical_name("GateIO", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);
            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

            state.update_price_with_bid_ask_and_symbol(
                Exchange::GateIO, pair_id, &display_symbol, mid_price, *bid, *ask, *bid_size, *ask_size, quote_currency
            ).await;
            total_updated += 1;
        }
    }
    info!("  GateIO: {} orderbooks loaded", gateio_result.len());

    // Process Upbit orderbooks (prices are in KRW, need conversion to USD)
    // First, extract USDT/KRW rate from the result (if KRW-USDT was fetched)
    let usdt_krw_rate = upbit_result.get("KRW-USDT")
        .map(|(bid, ask, _, _)| (bid.to_f64() + ask.to_f64()) / 2.0);

    if let Some(rate) = usdt_krw_rate {
        // Store the USDT/KRW rate in state for other uses
        state.update_upbit_usdt_krw(FixedPoint::from_f64(rate));
        info!("  Upbit: USDT/KRW rate from REST: {:.2}", rate);
    }

    // Get the USDT/KRW rate for conversion (from REST or existing state)
    let usdt_krw = usdt_krw_rate.or_else(|| state.get_upbit_usdt_krw().map(|p| p.to_f64()));
    let usdt_usd = state.get_usdt_usd_price().to_f64();

    for (market, (bid, ask, bid_size, ask_size)) in &upbit_result {
        // Skip USDT market (already processed above)
        if market == "KRW-USDT" {
            continue;
        }

        // Extract base from market (e.g., "KRW-BTC" -> "BTC")
        if let Some(base) = UpbitAdapter::extract_base_symbol(market) {
            let display_symbol = symbol_mappings.canonical_name("Upbit", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

            // Convert KRW prices to USD if we have the exchange rate
            if let Some(usdt_krw_rate) = usdt_krw {
                // KRW -> USDT -> USD
                let bid_usd = bid.to_f64() / usdt_krw_rate * usdt_usd;
                let ask_usd = ask.to_f64() / usdt_krw_rate * usdt_usd;
                let mid_price_usd = (bid_usd + ask_usd) / 2.0;

                state.update_price_with_bid_ask_and_symbol(
                    Exchange::Upbit, pair_id, &display_symbol,
                    FixedPoint::from_f64(mid_price_usd),
                    FixedPoint::from_f64(bid_usd),
                    FixedPoint::from_f64(ask_usd),
                    *bid_size, *ask_size, QuoteCurrency::KRW
                ).await;
            } else {
                // No exchange rate available yet, store raw KRW prices
                // They will be updated when WebSocket provides the rate
                let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);
                state.update_price_with_bid_ask_and_symbol(
                    Exchange::Upbit, pair_id, &display_symbol, mid_price, *bid, *ask, *bid_size, *ask_size, QuoteCurrency::KRW
                ).await;
                debug!("  Upbit: {} stored as KRW (no exchange rate yet)", display_symbol);
            }
            total_updated += 1;
        }
    }
    info!("  Upbit: {} orderbooks loaded", upbit_result.len());

    info!("📚 Initial orderbook fetch complete: {} total orderbooks loaded", total_updated);
}

/// Extract base and quote from Binance symbol (e.g., btcusdt -> BTC, USDT)
fn extract_binance_base_quote(symbol: &str) -> Option<(String, String)> {
    let s = symbol.to_uppercase();
    // Try known quote currencies in order of length (longer first)
    for quote in &["USDT", "USDC", "BUSD", "USD", "BTC", "ETH", "BNB"] {
        if s.ends_with(quote) {
            let base = &s[..s.len() - quote.len()];
            if !base.is_empty() {
                return Some((base.to_string(), quote.to_string()));
            }
        }
    }
    None
}

/// Extract base and quote from Bybit symbol (e.g., BTCUSDT -> BTC, USDT)
fn extract_bybit_base_quote(symbol: &str) -> Option<(String, String)> {
    // Bybit uses same format as Binance
    extract_binance_base_quote(symbol)
}

/// Spawn live WebSocket feeds
async fn spawn_live_feeds(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    symbol_mappings: &SymbolMappings,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    // First, do an initial market discovery to get common symbols
    info!("🔍 Starting initial market discovery...");
    let discovery = MarketDiscovery::new();
    let all_markets = discovery.fetch_all().await;
    info!("🔍 Fetched markets from {} exchanges", all_markets.len());
    for (exchange, markets) in &all_markets {
        debug!("  {}: {} markets", exchange, markets.markets.len());
    }

    // Find common markets across exchanges we support for live feeds
    // Apply symbol mappings to exclude mismatched symbols
    let exchanges = ["Binance", "Coinbase", "Upbit", "Bithumb", "Bybit", "GateIO"];
    let common = MarketDiscovery::find_markets_on_n_exchanges_with_mappings(
        &all_markets,
        &exchanges,
        2,
        Some(symbol_mappings),
    );
    info!("🔍 Found {} common markets, {} by_quote entries", common.common.len(), common.by_quote.len());

    // Get symbols for each exchange from by_quote markets (includes both USDT and USDC)
    // This ensures we subscribe to all quote variants (BTC-USDT, BTC-USDC, etc.)
    use std::collections::HashSet;
    let mut binance_set: HashSet<String> = HashSet::new();
    let mut coinbase_set: HashSet<String> = HashSet::new();
    let mut upbit_set: HashSet<String> = HashSet::new();
    let mut bithumb_set: HashSet<String> = HashSet::new();
    let mut bybit_set: HashSet<String> = HashSet::new();
    let mut gateio_set: HashSet<String> = HashSet::new();

    // Always include USDT for exchange rate
    upbit_set.insert("KRW-USDT".to_string());

    // Use by_quote to get all quote variants (USDT, USDC, KRW)
    // by_quote already filters to markets on 2+ exchanges
    for (key, exchange_markets) in &common.by_quote {
        for (exchange, market_info) in exchange_markets {
            match exchange.as_str() {
                "Binance" => { binance_set.insert(market_info.symbol.to_lowercase()); }
                "Coinbase" => { coinbase_set.insert(market_info.symbol.clone()); }
                "Upbit" => { upbit_set.insert(market_info.symbol.clone()); }
                "Bithumb" => { bithumb_set.insert(market_info.symbol.clone()); }
                "Bybit" => { bybit_set.insert(market_info.symbol.clone()); }
                "GateIO" => { gateio_set.insert(market_info.symbol.clone()); }
                _ => {}
            }
        }
        // Log each market group for debugging
        if key.starts_with("BTC/") || key.starts_with("ETH/") {
            debug!(
                "📊 {} -> {} exchanges: {:?}",
                key,
                exchange_markets.len(),
                exchange_markets.iter().map(|(ex, m)| format!("{}:{}", ex, m.symbol)).collect::<Vec<_>>()
            );
        }
    }

    // Log by_quote stats
    let usdt_markets = common.by_quote.keys().filter(|k| k.ends_with("/USDT")).count();
    let usdc_markets = common.by_quote.keys().filter(|k| k.ends_with("/USDC")).count();
    let krw_markets = common.by_quote.keys().filter(|k| k.ends_with("/KRW")).count();
    info!(
        "📊 By quote markets: {} USDT, {} USDC, {} KRW (on 2+ exchanges)",
        usdt_markets, usdc_markets, krw_markets
    );

    let binance_symbols: Vec<String> = binance_set.into_iter().collect();
    let coinbase_symbols: Vec<String> = coinbase_set.into_iter().collect();
    let upbit_symbols: Vec<String> = upbit_set.into_iter().collect();
    let bithumb_symbols: Vec<String> = bithumb_set.into_iter().collect();
    let bybit_symbols: Vec<String> = bybit_set.into_iter().collect();
    let gateio_symbols: Vec<String> = gateio_set.into_iter().collect();

    // Subscribe to all common markets
    // - Binance: supports up to 1024 streams per connection
    // - Coinbase: supports many subscriptions per connection
    // - Upbit/Bithumb: supports many codes per connection
    // - Bybit: supports many tickers per connection
    // - Gate.io: supports many tickers per connection

    info!(
        "📡 Subscribing to live feeds: Binance={}, Coinbase={}, Upbit={}, Bithumb={}, Bybit={}, GateIO={}",
        binance_symbols.len(),
        coinbase_symbols.len(),
        upbit_symbols.len(),
        bithumb_symbols.len(),
        bybit_symbols.len(),
        gateio_symbols.len()
    );

    // Register all symbols for opportunity detection
    state.register_common_markets(&common).await;

    // Fetch initial orderbooks via REST API before WebSocket feeds start
    // Only batch-capable exchanges: Binance, Bybit, GateIO, Upbit
    // Coinbase and Bithumb rely on WebSocket for initial data
    fetch_initial_orderbooks(
        &state,
        &binance_symbols,
        &bybit_symbols,
        &gateio_symbols,
        &upbit_symbols,
        symbol_mappings,
    ).await;

    // Convert symbol_mappings to Arc for sharing across tasks
    let symbol_mappings_arc = Arc::new(symbol_mappings.clone());

    // Binance
    if !binance_symbols.is_empty() {
        let binance_config = FeedConfig::for_exchange(Exchange::Binance);
        let (binance_tx, binance_rx) = mpsc::channel(1000);

        // Add stablecoin rate symbols to subscription
        let mut all_binance_symbols = binance_symbols.clone();
        all_binance_symbols.push("USDTUSD".to_string());
        all_binance_symbols.push("USDCUSDT".to_string());

        let binance_subscribe_msgs = BinanceAdapter::subscribe_messages(&all_binance_symbols);

        let binance_client = WsClient::new(binance_config.clone(), binance_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = binance_client.run_with_messages(Some(binance_subscribe_msgs)).await {
                warn!("Binance WebSocket error: {}", e);
            }
        }));

        let binance_state = state.clone();
        let binance_broadcast = broadcast_tx.clone();
        let binance_mappings = symbol_mappings_arc.clone();
        handles.push(tokio::spawn(async move {
            run_binance_feed(binance_state, binance_broadcast, binance_rx, binance_mappings).await;
        }));
    }

    // Coinbase
    if !coinbase_symbols.is_empty() {
        let coinbase_config = FeedConfig::for_exchange(Exchange::Coinbase);
        let (coinbase_tx, coinbase_rx) = mpsc::channel(1000);
        // Subscribe to both ticker and level2_batch for orderbook depth
        let coinbase_subscribe_msgs = CoinbaseAdapter::subscribe_messages(&coinbase_symbols);

        let coinbase_client = WsClient::new(coinbase_config.clone(), coinbase_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = coinbase_client.run_with_messages(Some(coinbase_subscribe_msgs)).await {
                warn!("Coinbase WebSocket error: {}", e);
            }
        }));

        let coinbase_state = state.clone();
        let coinbase_broadcast = broadcast_tx.clone();
        let coinbase_mappings = symbol_mappings_arc.clone();
        handles.push(tokio::spawn(async move {
            run_coinbase_feed(coinbase_state, coinbase_broadcast, coinbase_rx, coinbase_mappings).await;
        }));
    }

    // Upbit
    if !upbit_symbols.is_empty() {
        let upbit_config = FeedConfig::for_exchange(Exchange::Upbit);
        let (upbit_tx, upbit_rx) = mpsc::channel(1000);
        let upbit_subscribe = UpbitAdapter::subscribe_message(&upbit_symbols);

        let upbit_client = WsClient::new(upbit_config.clone(), upbit_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = upbit_client.run(Some(upbit_subscribe)).await {
                warn!("Upbit WebSocket error: {}", e);
            }
        }));

        let upbit_state = state.clone();
        let upbit_broadcast = broadcast_tx.clone();
        let upbit_mappings = symbol_mappings_arc.clone();
        handles.push(tokio::spawn(async move {
            run_upbit_feed(upbit_state, upbit_broadcast, upbit_rx, upbit_mappings).await;
        }));
    }

    // Bithumb
    if !bithumb_symbols.is_empty() {
        let bithumb_config = FeedConfig::for_exchange(Exchange::Bithumb);
        let (bithumb_tx, bithumb_rx) = mpsc::channel(1000);
        let bithumb_subscribe = BithumbAdapter::subscribe_message(&bithumb_symbols);

        let bithumb_client = WsClient::new(bithumb_config.clone(), bithumb_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = bithumb_client.run(Some(bithumb_subscribe)).await {
                warn!("Bithumb WebSocket error: {}", e);
            }
        }));

        let bithumb_state = state.clone();
        let bithumb_broadcast = broadcast_tx.clone();
        let bithumb_mappings = symbol_mappings_arc.clone();
        handles.push(tokio::spawn(async move {
            run_bithumb_feed(bithumb_state, bithumb_broadcast, bithumb_rx, bithumb_mappings).await;
        }));
    }

    // Bybit
    if !bybit_symbols.is_empty() {
        let bybit_config = FeedConfig::for_exchange(Exchange::Bybit);
        let (bybit_tx, bybit_rx) = mpsc::channel(1000);
        // Bybit has a limit of 10 args per subscription, so we batch them
        let bybit_subscribe_msgs = BybitAdapter::subscribe_messages(&bybit_symbols);

        let bybit_client = WsClient::new(bybit_config.clone(), bybit_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = bybit_client.run_with_messages(Some(bybit_subscribe_msgs)).await {
                warn!("Bybit WebSocket error: {}", e);
            }
        }));

        let bybit_state = state.clone();
        let bybit_broadcast = broadcast_tx.clone();
        let bybit_mappings = symbol_mappings_arc.clone();
        handles.push(tokio::spawn(async move {
            run_bybit_feed(bybit_state, bybit_broadcast, bybit_rx, bybit_mappings).await;
        }));
    }

    // Gate.io
    if !gateio_symbols.is_empty() {
        let gateio_config = FeedConfig::for_exchange(Exchange::GateIO);
        let (gateio_tx, gateio_rx) = mpsc::channel(1000);
        // Subscribe to both ticker and order_book for orderbook depth
        let gateio_subscribe_msgs = GateIOAdapter::subscribe_messages(&gateio_symbols);

        let gateio_client = WsClient::new(gateio_config.clone(), gateio_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = gateio_client.run_with_messages(Some(gateio_subscribe_msgs)).await {
                warn!("Gate.io WebSocket error: {}", e);
            }
        }));

        let gateio_state = state.clone();
        let gateio_broadcast = broadcast_tx.clone();
        let gateio_mappings = symbol_mappings_arc.clone();
        handles.push(tokio::spawn(async move {
            run_gateio_feed(gateio_state, gateio_broadcast, gateio_rx, gateio_mappings).await;
        }));
    }

    handles
}

/// Run market discovery loop - periodically fetches markets from exchanges
/// and broadcasts common markets to clients.
async fn run_market_discovery(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    symbol_mappings: Arc<SymbolMappings>,
) {
    info!("Starting market discovery loop");

    let discovery = MarketDiscovery::new();
    let exchanges = ["Binance", "Coinbase", "Upbit", "Bithumb", "Bybit", "GateIO"];

    loop {
        // Reload symbol mappings on each iteration (in case they were updated)
        let current_mappings = load_mappings();
        let excluded_count = current_mappings.excluded_pairs().len();
        if excluded_count > 0 {
            info!(
                "🔍 Fetching markets from exchanges... ({} symbols excluded by mappings)",
                excluded_count
            );
        } else {
            info!("🔍 Fetching markets from exchanges...");
        }

        let all_markets = discovery.fetch_all().await;

        if all_markets.len() >= 2 {
            // Find markets available on 2+ exchanges (not just all exchanges)
            // Apply symbol mappings to exclude mismatched symbols
            let common = MarketDiscovery::find_markets_on_n_exchanges_with_mappings(
                &all_markets,
                &exchanges,
                2,
                Some(&current_mappings),
            );

            // Count markets by availability
            let on_all = common.common.values().filter(|v| v.len() == exchanges.len()).count();
            let on_partial = common.common.len() - on_all;

            info!(
                "📊 Found {} tradable markets: {} on all {} exchanges, {} on 2+ exchanges",
                common.common.len(),
                on_all,
                exchanges.len(),
                on_partial
            );

            // Log top 10 markets (sorted by exchange count descending, then alphabetically)
            let mut bases: Vec<_> = common.common.iter()
                .map(|(base, markets)| (base.clone(), markets.len()))
                .collect();
            bases.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

            for (base, count) in bases.iter().take(10) {
                info!("  - {} ({}/{} exchanges)", base, count, exchanges.len());
            }

            if bases.len() > 10 {
                info!("  ... and {} more", bases.len() - 10);
            }

            // Register all common markets for opportunity detection
            state.register_common_markets(&common).await;
            info!(
                "📈 Registered {} symbols for opportunity detection",
                common.common.len()
            );

            // Store in state for initial sync
            state.update_common_markets(common.clone()).await;

            // Broadcast to connected clients
            ws_server::broadcast_common_markets(&broadcast_tx, &common);
        } else {
            warn!(
                "Only {} exchanges responded, need at least 2 for comparison",
                all_markets.len()
            );
        }

        // Refresh every 5 minutes
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

#[tokio::main]
async fn main() {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    let args = Args::parse();

    init_logging(&args.log_level);

    info!("🚀 Arbitrage Bot starting...");
    info!("  Mode: {}", args.mode);
    info!("  Min Premium: {} bps", args.min_premium);
    info!("  Dry Run: {}", args.dry_run);
    info!("  Live Feeds: {}", args.live);
    info!("  WebSocket Port: {}", args.ws_port);
    info!("  Telegram Alerts: {}", args.telegram);

    // Load network name mapping for cross-exchange transfer path detection
    wallet_status::init_network_mapping();

    // Load symbol mappings for filtering mismatched coins
    let symbol_mappings = Arc::new(load_mappings());
    let excluded_count = symbol_mappings.excluded_pairs().len();
    if excluded_count > 0 {
        info!(
            "  Symbol Mappings: {} excluded pairs loaded",
            excluded_count
        );
        for (exchange, symbol) in symbol_mappings.excluded_pairs() {
            info!("    - {}/{} excluded", exchange, symbol);
        }
    } else {
        info!("  Symbol Mappings: none configured");
    }

    // Create config
    let mut config = AppConfig::default();
    config.detector.min_premium_bps = args.min_premium;
    config.execution.mode = parse_mode(&args.mode);
    config.log_level = args.log_level.clone();

    // Create shared state
    let state = create_state(config);
    state.start();

    // Start WebSocket server for clients (Tauri app) - must start first to get broadcast_tx
    let broadcast_tx = match ws_server::start_ws_server(state.clone(), args.ws_port).await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start WebSocket server: {}", e);
            return;
        }
    };

    // Fetch initial exchange rate after WebSocket server is ready
    match exchange_rate::fetch_exchange_rate().await {
        Ok(rate) => {
            info!("Initial USD/KRW exchange rate: {:.2}", rate);
            // Broadcast to any already-connected clients
            ws_server::broadcast_exchange_rate(&broadcast_tx, &state, rate);
        }
        Err(e) => {
            warn!("Failed to fetch initial exchange rate, using default: {}", e);
        }
    }

    // Fetch initial wallet status so it's cached for new clients
    {
        let statuses = wallet_status::fetch_all_wallet_status().await;
        if !statuses.is_empty() {
            info!("Initial wallet status fetched for {} exchanges", statuses.len());
            // Update cache first so new clients can get it
            wallet_status::update_cache(statuses.clone());
            // Then broadcast to any already-connected clients
            ws_server::broadcast_wallet_status(&broadcast_tx, statuses);
        }
    }

    // Initialize Telegram notifier if enabled
    let notifier: Option<Arc<Notifier>> = if args.telegram {
        match std::env::var("TELEGRAM_BOT_TOKEN") {
            Ok(token) => {
                // Ensure data directory exists
                let db_dir = std::path::Path::new(&args.db_path).parent();
                if let Some(dir) = db_dir {
                    if !dir.exists() {
                        if let Err(e) = std::fs::create_dir_all(dir) {
                            warn!("Failed to create data directory: {}", e);
                        }
                    }
                }

                // Connect to database
                let db_url = format!("sqlite:{}", args.db_path);
                match Database::connect(&db_url).await {
                    Ok(db) => {
                        let bot = Arc::new(TelegramBot::new(&token, db.clone()));
                        let notifier_config = NotifierConfig::default();

                        // Create notifier with transfer path checker
                        let notifier = Notifier::new(db, bot.clone(), notifier_config)
                            .with_transfer_path_checker(Box::new(|asset, source, target| {
                                wallet_status::has_transfer_path(asset, source, target)
                            }));
                        let notifier = Arc::new(notifier);

                        info!("📱 Telegram alerts enabled (with transfer path filtering)");

                        // Start bot command handler in background
                        let bot_clone = bot.clone();
                        tokio::spawn(async move {
                            bot_clone.run().await;
                        });

                        Some(notifier)
                    }
                    Err(e) => {
                        warn!("Failed to connect to alert database: {}", e);
                        None
                    }
                }
            }
            Err(_) => {
                warn!("TELEGRAM_BOT_TOKEN not set, Telegram alerts disabled");
                None
            }
        }
    } else {
        None
    };

    // Spawn background tasks with broadcast sender
    let detector_state = state.clone();
    let detector_broadcast = broadcast_tx.clone();
    let detector_notifier = notifier.clone();
    let detector_handle = tokio::spawn(async move {
        run_detector_loop(detector_state, detector_broadcast, detector_notifier).await;
    });

    let stats_state = state.clone();
    let stats_broadcast = broadcast_tx.clone();
    let stats_handle = tokio::spawn(async move {
        run_stats_reporter(stats_state, stats_broadcast).await;
    });

    // Start exchange rate updater for Upbit KRW to USD conversion
    let rate_broadcast = broadcast_tx.clone();
    let rate_state = state.clone();
    tokio::spawn(async move {
        exchange_rate::run_exchange_rate_updater(rate_broadcast, rate_state).await;
    });

    // Start wallet status updater for deposit/withdraw availability
    let wallet_broadcast = broadcast_tx.clone();
    tokio::spawn(async move {
        wallet_status::run_wallet_status_updater(wallet_broadcast).await;
    });

    // Start market discovery loop
    let discovery_state = state.clone();
    let discovery_broadcast = broadcast_tx.clone();
    let discovery_mappings = symbol_mappings.clone();
    tokio::spawn(async move {
        run_market_discovery(discovery_state, discovery_broadcast, discovery_mappings).await;
    });

    // Spawn price source (live or simulated)
    let feed_handles: Vec<tokio::task::JoinHandle<()>> = if args.live {
        info!("📡 Using LIVE WebSocket feeds");
        spawn_live_feeds(state.clone(), broadcast_tx.clone(), &symbol_mappings).await
    } else {
        info!("🎮 Using SIMULATED price feeds");
        let price_state = state.clone();
        let price_broadcast = broadcast_tx.clone();
        vec![tokio::spawn(async move {
            run_price_simulator(price_state, price_broadcast).await;
        })]
    };

    // Handle shutdown
    info!("Press Ctrl+C to stop...");

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");

    warn!("Shutdown signal received");
    state.stop();

    // Wait for tasks with timeout, then abort
    let _ = tokio::time::timeout(Duration::from_secs(2), detector_handle).await;
    let _ = tokio::time::timeout(Duration::from_secs(1), stats_handle).await;

    // Abort WebSocket tasks (they may be blocked on I/O)
    for handle in feed_handles {
        handle.abort();
    }

    // Final stats
    let summary = state.stats_summary();
    info!("📈 Final Stats:");
    info!("  Total uptime: {} seconds", summary.uptime_secs);
    info!("  Price updates: {}", summary.price_updates);
    info!("  Opportunities: {}", summary.opportunities_detected);
    info!("  Trades executed: {}", summary.trades_executed);

    info!("👋 Arbitrage Bot stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mode() {
        assert_eq!(parse_mode("auto"), ExecutionMode::Auto);
        assert_eq!(parse_mode("manual"), ExecutionMode::ManualApproval);
        assert_eq!(parse_mode("alert"), ExecutionMode::AlertOnly);
        assert_eq!(parse_mode("unknown"), ExecutionMode::AlertOnly);
    }

    #[tokio::test]
    async fn test_state_integration() {
        let config = AppConfig::default();
        let state = create_state(config);

        state.start();
        assert!(state.is_running());

        // Update some prices
        state
            .update_price(Exchange::Binance, 1, FixedPoint::from_f64(50000.0))
            .await;
        state
            .update_price(Exchange::Coinbase, 1, FixedPoint::from_f64(50100.0))
            .await;

        // Detect
        let opps = state.detect_opportunities(1).await;
        // May or may not have opportunities depending on threshold

        state.stop();
        assert!(!state.is_running());
    }
}

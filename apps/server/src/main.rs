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
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use arbitrage_core::{Exchange, FixedPoint, PriceTick};
use arbitrage_feeds::{
    BinanceAdapter, BithumbAdapter, BybitAdapter, CoinbaseAdapter, FeedConfig, MarketDiscovery,
    UpbitAdapter, WsClient, WsMessage,
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
}

fn init_logging(level: &str) {
    let level = match level {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
}

fn parse_mode(mode: &str) -> ExecutionMode {
    match mode.to_lowercase().as_str() {
        "auto" => ExecutionMode::Auto,
        "manual" => ExecutionMode::ManualApproval,
        _ => ExecutionMode::AlertOnly,
    }
}

async fn run_detector_loop(state: SharedState, broadcast_tx: BroadcastSender) {
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
                    ws_server::broadcast_opportunity(&broadcast_tx, &opp);
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

/// Convert Upbit KRW price to USD using exchange rate API.
/// Returns None if exchange rate is not available yet.
fn convert_krw_to_usd(krw_price: FixedPoint) -> Option<FixedPoint> {
    let rate = exchange_rate::get_api_rate()?;
    if rate > 0.0 {
        Some(FixedPoint::from_f64(krw_price.to_f64() / rate))
    } else {
        None
    }
}

/// Convert Upbit KRW price tick to USD.
/// Returns None if exchange rate is not available yet.
fn convert_upbit_tick_to_usd(tick: &PriceTick) -> Option<PriceTick> {
    let price_usd = convert_krw_to_usd(tick.price())?;
    let bid_usd = convert_krw_to_usd(tick.bid())?;
    let ask_usd = convert_krw_to_usd(tick.ask())?;

    Some(PriceTick::new(tick.exchange(), tick.pair_id(), price_usd, bid_usd, ask_usd))
}

/// Process Upbit ticker data by market code.
fn process_upbit_ticker(
    code: &str,
    price: FixedPoint,
    state: &SharedState,
    broadcast_tx: &BroadcastSender,
) {
    // Handle USDT/KRW for exchange rate
    if UpbitAdapter::is_usdt_market(code) {
        state.update_usdt_krw_price(price);
        let rate = price.to_f64();
        ws_server::broadcast_exchange_rate(broadcast_tx, rate);
        tracing::debug!("Updated USDT/KRW rate: {:.2}", rate);
        return;
    }

    // Handle trading pairs - extract symbol from code (e.g., "KRW-BTC" -> "BTC")
    if let Some(symbol) = UpbitAdapter::extract_base_symbol(code) {
        let pair_id = arbitrage_core::symbol_to_pair_id(&symbol);

        // Convert KRW to USD and broadcast
        // Skip if exchange rate is not available yet
        if let Some(price_usd) = convert_krw_to_usd(price) {
            let tick_usd = PriceTick::new(Exchange::Upbit, pair_id, price_usd, price_usd, price_usd);

            // Update state asynchronously (fire and forget for now)
            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.update_price(Exchange::Upbit, pair_id, price_usd).await;
            });

            ws_server::broadcast_price(broadcast_tx, Exchange::Upbit, pair_id, &symbol, &tick_usd);
        }
    }
}

/// Run live WebSocket feed for Upbit
async fn run_upbit_feed(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    mut rx: mpsc::Receiver<WsMessage>,
) {
    info!("Starting Upbit live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Parse ticker with market code
                if let Ok((code, price)) = UpbitAdapter::parse_ticker_with_code(&text) {
                    process_upbit_ticker(&code, price, &state, &broadcast_tx);
                }
            }
            WsMessage::Binary(data) => {
                // Upbit sends binary MessagePack data
                if let Ok((code, price)) = UpbitAdapter::parse_ticker_binary_with_code(&data) {
                    process_upbit_ticker(&code, price, &state, &broadcast_tx);
                } else {
                    // Fallback: try parsing as UTF-8 JSON
                    if let Ok(text) = String::from_utf8(data) {
                        if let Ok((code, price)) = UpbitAdapter::parse_ticker_with_code(&text) {
                            process_upbit_ticker(&code, price, &state, &broadcast_tx);
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
) {
    info!("Starting Binance live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Auto-detect symbol and pair_id from message
                if let Ok((tick, symbol)) = BinanceAdapter::parse_ticker_with_symbol(&text) {
                    let pair_id = tick.pair_id();
                    state.update_price(Exchange::Binance, pair_id, tick.price()).await;
                    ws_server::broadcast_price(&broadcast_tx, Exchange::Binance, pair_id, &symbol, &tick);
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
) {
    info!("Starting Coinbase live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Auto-detect product and pair_id from message
                if let Ok((tick, symbol)) = CoinbaseAdapter::parse_ticker_with_symbol(&text) {
                    let pair_id = tick.pair_id();
                    state.update_price(Exchange::Coinbase, pair_id, tick.price()).await;
                    ws_server::broadcast_price(&broadcast_tx, Exchange::Coinbase, pair_id, &symbol, &tick);
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
) {
    // Handle USDT/KRW for exchange rate (if Bithumb has it)
    if BithumbAdapter::is_usdt_market(code) {
        state.update_usdt_krw_price(price);
        let rate = price.to_f64();
        ws_server::broadcast_exchange_rate(broadcast_tx, rate);
        tracing::debug!("Updated USDT/KRW rate from Bithumb: {:.2}", rate);
        return;
    }

    // Handle trading pairs - extract symbol from code (e.g., "KRW-BTC" -> "BTC")
    if let Some(symbol) = BithumbAdapter::extract_base_symbol(code) {
        let pair_id = arbitrage_core::symbol_to_pair_id(&symbol);

        // Convert KRW to USD and broadcast
        // Skip if exchange rate is not available yet
        if let Some(price_usd) = convert_krw_to_usd(price) {
            let tick_usd = PriceTick::new(Exchange::Bithumb, pair_id, price_usd, price_usd, price_usd);

            // Update state asynchronously (fire and forget for now)
            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.update_price(Exchange::Bithumb, pair_id, price_usd).await;
            });

            ws_server::broadcast_price(broadcast_tx, Exchange::Bithumb, pair_id, &symbol, &tick_usd);
        }
    }
}

/// Run live WebSocket feed for Bithumb
async fn run_bithumb_feed(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    mut rx: mpsc::Receiver<WsMessage>,
) {
    info!("Starting Bithumb live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Parse ticker with market code
                if let Ok((code, price)) = BithumbAdapter::parse_ticker_with_code(&text) {
                    process_bithumb_ticker(&code, price, &state, &broadcast_tx);
                }
            }
            WsMessage::Binary(data) => {
                // Bithumb sends binary MessagePack data
                if let Ok((code, price)) = BithumbAdapter::parse_ticker_binary_with_code(&data) {
                    process_bithumb_ticker(&code, price, &state, &broadcast_tx);
                } else {
                    // Fallback: try parsing as UTF-8 JSON
                    if let Ok(text) = String::from_utf8(data) {
                        if let Ok((code, price)) = BithumbAdapter::parse_ticker_with_code(&text) {
                            process_bithumb_ticker(&code, price, &state, &broadcast_tx);
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
) {
    info!("Starting Bybit live feed processor");

    while let Some(msg) = rx.recv().await {
        if !state.is_running() {
            break;
        }

        match msg {
            WsMessage::Text(text) => {
                // Parse ticker with symbol (auto-detects pair_id from symbol)
                if let Ok((tick, symbol)) = BybitAdapter::parse_ticker_with_symbol(&text) {
                    let pair_id = tick.pair_id();
                    state.update_price(Exchange::Bybit, pair_id, tick.price()).await;
                    ws_server::broadcast_price(&broadcast_tx, Exchange::Bybit, pair_id, &symbol, &tick);
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

/// Spawn live WebSocket feeds
async fn spawn_live_feeds(
    state: SharedState,
    broadcast_tx: BroadcastSender,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    // First, do an initial market discovery to get common symbols
    let discovery = MarketDiscovery::new();
    let all_markets = discovery.fetch_all().await;

    // Find common markets across exchanges we support for live feeds
    let exchanges = ["Binance", "Coinbase", "Upbit", "Bithumb", "Bybit"];
    let common = MarketDiscovery::find_markets_on_n_exchanges(&all_markets, &exchanges, 2);

    // Get symbols for each exchange from common markets
    let mut binance_symbols: Vec<String> = Vec::new();
    let mut coinbase_symbols: Vec<String> = Vec::new();
    let mut upbit_symbols: Vec<String> = vec!["KRW-USDT".to_string()]; // Always include for exchange rate
    let mut bithumb_symbols: Vec<String> = Vec::new();
    let mut bybit_symbols: Vec<String> = Vec::new();

    for (base, exchange_markets) in &common.common {
        for (exchange, market_info) in exchange_markets {
            match exchange.as_str() {
                "Binance" => binance_symbols.push(market_info.symbol.to_lowercase()),
                "Coinbase" => coinbase_symbols.push(market_info.symbol.clone()),
                "Upbit" => upbit_symbols.push(market_info.symbol.clone()),
                "Bithumb" => bithumb_symbols.push(market_info.symbol.clone()),
                "Bybit" => bybit_symbols.push(market_info.symbol.clone()),
                _ => {}
            }
        }
    }

    // Subscribe to all common markets
    // - Binance: supports up to 1024 streams per connection
    // - Coinbase: supports many subscriptions per connection
    // - Upbit/Bithumb: supports many codes per connection
    // - Bybit: supports many tickers per connection

    info!(
        "📡 Subscribing to live feeds: Binance={}, Coinbase={}, Upbit={}, Bithumb={}, Bybit={}",
        binance_symbols.len(),
        coinbase_symbols.len(),
        upbit_symbols.len(),
        bithumb_symbols.len(),
        bybit_symbols.len()
    );

    // Register all symbols for opportunity detection
    state.register_common_markets(&common).await;

    // Binance
    if !binance_symbols.is_empty() {
        let binance_config = FeedConfig::for_exchange(Exchange::Binance);
        let (binance_tx, binance_rx) = mpsc::channel(1000);
        let binance_subscribe = BinanceAdapter::subscribe_message(&binance_symbols);

        let binance_client = WsClient::new(binance_config.clone(), binance_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = binance_client.run(Some(binance_subscribe)).await {
                warn!("Binance WebSocket error: {}", e);
            }
        }));

        let binance_state = state.clone();
        let binance_broadcast = broadcast_tx.clone();
        handles.push(tokio::spawn(async move {
            run_binance_feed(binance_state, binance_broadcast, binance_rx).await;
        }));
    }

    // Coinbase
    if !coinbase_symbols.is_empty() {
        let coinbase_config = FeedConfig::for_exchange(Exchange::Coinbase);
        let (coinbase_tx, coinbase_rx) = mpsc::channel(1000);
        let coinbase_subscribe = CoinbaseAdapter::subscribe_message(&coinbase_symbols);

        let coinbase_client = WsClient::new(coinbase_config.clone(), coinbase_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = coinbase_client.run(Some(coinbase_subscribe)).await {
                warn!("Coinbase WebSocket error: {}", e);
            }
        }));

        let coinbase_state = state.clone();
        let coinbase_broadcast = broadcast_tx.clone();
        handles.push(tokio::spawn(async move {
            run_coinbase_feed(coinbase_state, coinbase_broadcast, coinbase_rx).await;
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
        handles.push(tokio::spawn(async move {
            run_upbit_feed(upbit_state, upbit_broadcast, upbit_rx).await;
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
        handles.push(tokio::spawn(async move {
            run_bithumb_feed(bithumb_state, bithumb_broadcast, bithumb_rx).await;
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
        handles.push(tokio::spawn(async move {
            run_bybit_feed(bybit_state, bybit_broadcast, bybit_rx).await;
        }));
    }

    handles
}

/// Run market discovery loop - periodically fetches markets from exchanges
/// and broadcasts common markets to clients.
async fn run_market_discovery(state: SharedState, broadcast_tx: BroadcastSender) {
    info!("Starting market discovery loop");

    let discovery = MarketDiscovery::new();
    let exchanges = ["Binance", "Coinbase", "Upbit", "Bithumb", "Bybit"];

    loop {
        info!("🔍 Fetching markets from exchanges...");

        let all_markets = discovery.fetch_all().await;

        if all_markets.len() >= 2 {
            // Find markets available on 2+ exchanges (not just all exchanges)
            let common = MarketDiscovery::find_markets_on_n_exchanges(&all_markets, &exchanges, 2);

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
            ws_server::broadcast_exchange_rate(&broadcast_tx, rate);
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

    // Spawn background tasks with broadcast sender
    let detector_state = state.clone();
    let detector_broadcast = broadcast_tx.clone();
    let detector_handle = tokio::spawn(async move {
        run_detector_loop(detector_state, detector_broadcast).await;
    });

    let stats_state = state.clone();
    let stats_broadcast = broadcast_tx.clone();
    let stats_handle = tokio::spawn(async move {
        run_stats_reporter(stats_state, stats_broadcast).await;
    });

    // Start exchange rate updater for Upbit KRW to USD conversion
    let rate_broadcast = broadcast_tx.clone();
    tokio::spawn(async move {
        exchange_rate::run_exchange_rate_updater(rate_broadcast).await;
    });

    // Start wallet status updater for deposit/withdraw availability
    let wallet_broadcast = broadcast_tx.clone();
    tokio::spawn(async move {
        wallet_status::run_wallet_status_updater(wallet_broadcast).await;
    });

    // Start market discovery loop
    let discovery_state = state.clone();
    let discovery_broadcast = broadcast_tx.clone();
    tokio::spawn(async move {
        run_market_discovery(discovery_state, discovery_broadcast).await;
    });

    // Spawn price source (live or simulated)
    let feed_handles: Vec<tokio::task::JoinHandle<()>> = if args.live {
        info!("📡 Using LIVE WebSocket feeds");
        spawn_live_feeds(state.clone(), broadcast_tx.clone()).await
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

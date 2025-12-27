//! Arbitrage Bot - Headless Server
//!
//! A high-performance cryptocurrency arbitrage detection and execution bot.

mod config;
mod exchange_rate;
mod state;
mod ws_server;

use clap::Parser;
use config::{AppConfig, ExecutionMode};
use state::{create_state, SharedState};
use std::time::Duration;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use arbitrage_core::{Exchange, FixedPoint, PriceTick};
use arbitrage_feeds::{
    BinanceAdapter, CoinbaseAdapter, FeedConfig, UpbitAdapter, WsClient, WsMessage,
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

    // Simulated pair IDs
    let pair_ids = vec![1u32, 2, 3]; // BTC, ETH, SOL

    while state.is_running() {
        for &pair_id in &pair_ids {
            let opps = state.detect_opportunities(pair_id).await;

            for opp in opps {
                if opp.premium_bps >= 30 {
                    info!(
                        "🎯 Opportunity: {:?} -> {:?} | Premium: {} bps | Buy: {} | Sell: {}",
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

    let mut base_prices = vec![50000.0, 3000.0, 100.0]; // BTC, ETH, SOL
    let mut counter = 0u64;

    while state.is_running() {
        for (pair_id, base_price) in base_prices.iter_mut().enumerate() {
            let pair_id = (pair_id + 1) as u32;

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
                ws_server::broadcast_price(&broadcast_tx, exchange, pair_id, &tick);
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

/// Convert Upbit KRW price to USD using USDT/KRW rate from Upbit.
/// Returns None if USDT/KRW rate is not available yet.
fn convert_krw_to_usd(krw_price: FixedPoint, state: &SharedState) -> Option<FixedPoint> {
    let usdt_krw = state.get_usdt_krw_price()?;
    let rate = usdt_krw.to_f64();
    if rate > 0.0 {
        Some(FixedPoint::from_f64(krw_price.to_f64() / rate))
    } else {
        None
    }
}

/// Convert Upbit KRW price tick to USD.
/// Returns None if USDT/KRW rate is not available yet.
fn convert_upbit_tick_to_usd(tick: &PriceTick, state: &SharedState) -> Option<PriceTick> {
    let price_usd = convert_krw_to_usd(tick.price(), state)?;
    let bid_usd = convert_krw_to_usd(tick.bid(), state)?;
    let ask_usd = convert_krw_to_usd(tick.ask(), state)?;

    Some(PriceTick::new(tick.exchange(), tick.pair_id(), price_usd, bid_usd, ask_usd))
}

/// Process Upbit ticker data by market code.
fn process_upbit_ticker(
    code: &str,
    price: FixedPoint,
    state: &SharedState,
    broadcast_tx: &BroadcastSender,
) {
    match code {
        "KRW-USDT" => {
            // Update USDT/KRW rate for currency conversion
            state.update_usdt_krw_price(price);
            // Also broadcast the exchange rate to clients
            let rate = price.to_f64();
            ws_server::broadcast_exchange_rate(broadcast_tx, rate);
            tracing::debug!("Updated USDT/KRW rate: {:.2}", rate);
        }
        "KRW-BTC" => {
            // Convert BTC/KRW to BTC/USD and broadcast
            // Skip if USDT/KRW rate is not available yet
            if let Some(price_usd) = convert_krw_to_usd(price, state) {
                let tick_usd = PriceTick::new(Exchange::Upbit, 1, price_usd, price_usd, price_usd);

                // Update state asynchronously (fire and forget for now)
                let state_clone = state.clone();
                tokio::spawn(async move {
                    state_clone.update_price(Exchange::Upbit, 1, price_usd).await;
                });

                ws_server::broadcast_price(broadcast_tx, Exchange::Upbit, 1, &tick_usd);
            }
        }
        _ => {
            // Ignore other markets for now
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
                // Try to parse as book ticker (has bid/ask)
                if let Ok(tick) = BinanceAdapter::parse_book_ticker(&text, 1) {
                    state.update_price(Exchange::Binance, 1, tick.price()).await;
                    ws_server::broadcast_price(&broadcast_tx, Exchange::Binance, 1, &tick);
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
                if let Ok(tick) = CoinbaseAdapter::parse_ticker(&text, 1) {
                    state.update_price(Exchange::Coinbase, 1, tick.price()).await;
                    ws_server::broadcast_price(&broadcast_tx, Exchange::Coinbase, 1, &tick);
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

/// Spawn live WebSocket feeds
async fn spawn_live_feeds(
    state: SharedState,
    broadcast_tx: BroadcastSender,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    // Binance BTC/USDT
    let binance_config = FeedConfig::for_exchange(Exchange::Binance);
    let (binance_tx, binance_rx) = mpsc::channel(1000);
    let binance_subscribe = BinanceAdapter::subscribe_message(&["btcusdt".to_string()]);

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

    // Coinbase BTC-USD
    let coinbase_config = FeedConfig::for_exchange(Exchange::Coinbase);
    let (coinbase_tx, coinbase_rx) = mpsc::channel(1000);
    let coinbase_subscribe = CoinbaseAdapter::subscribe_message(&["BTC-USD".to_string()]);

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

    // Upbit KRW-BTC and KRW-USDT (for exchange rate)
    let upbit_config = FeedConfig::for_exchange(Exchange::Upbit);
    let (upbit_tx, upbit_rx) = mpsc::channel(1000);
    let upbit_subscribe = UpbitAdapter::subscribe_message(&[
        "KRW-USDT".to_string(), // For KRW to USD conversion
        "KRW-BTC".to_string(),
    ]);

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

    handles
}

#[tokio::main]
async fn main() {
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

//! Arbitrage Bot - Headless Server
//!
//! A high-performance cryptocurrency arbitrage detection and execution bot.

mod config;
mod exchange_rate;
mod feeds;
mod state;
mod status_notifier;
mod wallet_status;
mod ws_server;

use status_notifier::{StatusEvent, StatusNotifierHandle};

use clap::Parser;
use config::{AppConfig, ExecutionMode};
use state::{create_state, SharedState};
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use arbitrage_alerts::{Database, Notifier, NotifierConfig, TelegramBot};

use arbitrage_core::{Exchange, FixedPoint, PriceTick, QuoteCurrency};
use arbitrage_feeds::{
    load_mappings, BinanceAdapter, BithumbAdapter, BybitAdapter, CoinbaseAdapter, CoinbaseCredentials,
    ExchangeAdapter, FeedConfig, GateIOAdapter, MarketDiscovery, SymbolMappings,
    UpbitAdapter, WsClient, BinanceRestFetcher, BybitRestFetcher, GateIORestFetcher,
    UpbitRestFetcher, BithumbRestFetcher, CoinbaseRestFetcher,
};
use feeds::common::{convert_stablecoin_to_usd_for_exchange, extract_binance_base_quote, extract_bybit_base_quote};
use feeds::FeedContext;
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
    debug!("Starting detector loop");

    // Legacy hardcoded pair IDs for backwards compatibility
    let legacy_pair_ids = vec![1u32, 2, 3]; // BTC, ETH, SOL

    while state.is_running() {
        // Get all registered pair_ids (includes dynamic markets from discovery)
        let mut pair_ids = state.get_registered_pair_ids();

        tracing::debug!(
            pair_ids_count = pair_ids.len(),
            "detector loop: registered pair_ids"
        );

        // Add legacy pair_ids if not already present
        for &legacy_id in &legacy_pair_ids {
            if !pair_ids.contains(&legacy_id) {
                pair_ids.push(legacy_id);
            }
        }

        // Detect and broadcast opportunities immediately per pair
        // Collect all opportunities to clear inactive ones after
        let mut all_opps = Vec::new();

        for &pair_id in &pair_ids {
            let opps = state.detect_opportunities(pair_id).await;

            // Broadcast each opportunity immediately as it's detected
            for opp in &opps {
                tracing::debug!(
                    "🎯 Opportunity: {} {:?} -> {:?} | Premium: {} bps | Buy: {} | Sell: {}",
                    opp.asset.symbol,
                    opp.source_exchange,
                    opp.target_exchange,
                    opp.premium_bps,
                    opp.source_price,
                    opp.target_price
                );
                ws_server::broadcast_opportunity(&broadcast_tx, &state, opp);
            }

            // Send Telegram alerts (notifier handles its own threshold filtering)
            if let Some(ref notifier) = notifier {
                for opp in &opps {
                    if let Err(e) = notifier.process_opportunity(opp).await {
                        tracing::warn!("Failed to send Telegram alert: {}", e);
                    }
                }
            }

            all_opps.extend(opps);
        }

        // Clear opportunities that fell below threshold
        if let Some(ref notifier) = notifier {
            if let Err(e) = notifier.clear_missing_opportunities(&all_opps).await {
                tracing::warn!("Failed to clear missing opportunities: {}", e);
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    debug!("Detector loop stopped");
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

    debug!("Price simulator stopped");
}

async fn run_stats_reporter(state: SharedState, broadcast_tx: BroadcastSender) {
    debug!("Starting stats reporter");

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


/// Fetch initial orderbooks via REST API and populate state.
/// Fetches from all exchanges with REST APIs and broadcasts to connected clients.
async fn fetch_initial_orderbooks(
    state: &SharedState,
    broadcast_tx: &BroadcastSender,
    binance_symbols: &[String],
    bybit_symbols: &[String],
    gateio_symbols: &[String],
    upbit_symbols: &[String],
    bithumb_symbols: &[String],
    symbol_mappings: &SymbolMappings,
) {
    // Fetch from all exchanges with REST APIs
    // Binance, Bybit, GateIO: batch ticker APIs
    // Upbit: batch API with comma-separated markets
    // Coinbase, Bithumb: individual API calls for stablecoins
    debug!("📚 Fetching initial orderbooks via REST API...");

    // Fetch from all exchanges in parallel
    // Coinbase stablecoin prices via individual API calls
    // Note: Coinbase has USDT-USD and USDT-USDC, but no USDC-USD (USDC is base currency)
    let coinbase_stablecoins = vec!["USDT-USD".to_string(), "USDT-USDC".to_string()];
    let (binance_result, bybit_result, gateio_result, upbit_result, bithumb_result, coinbase_stablecoin_result) = tokio::join!(
        BinanceRestFetcher::fetch_orderbooks(binance_symbols),
        BybitRestFetcher::fetch_orderbooks(bybit_symbols),
        GateIORestFetcher::fetch_orderbooks(gateio_symbols),
        UpbitRestFetcher::fetch_orderbooks(upbit_symbols),
        BithumbRestFetcher::fetch_orderbooks(bithumb_symbols),
        CoinbaseRestFetcher::fetch_orderbooks(&coinbase_stablecoins),
    );

    let mut total_updated = 0;

    // Process Binance orderbooks
    for (symbol, (bid, ask, bid_size, ask_size)) in &binance_result {
        // Extract base and quote from symbol (e.g., btcusdt -> BTC, USDT)
        if let Some((base, quote)) = extract_binance_base_quote(symbol) {
            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

            // Update stablecoin prices for this exchange
            if base == "USDT" || base == "USDC" {
                state.update_exchange_stablecoin_price(Exchange::Binance, &base, &quote, mid_price.to_f64());
                if base == "USDT" && quote == "USD" {
                    state.update_usdt_usd_price(mid_price);
                }
            }

            let display_symbol = symbol_mappings.canonical_name("Binance", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

            // Convert stablecoin prices to USD
            let mid_usd = convert_stablecoin_to_usd_for_exchange(mid_price, quote_currency, Exchange::Binance, state);
            let bid_usd = convert_stablecoin_to_usd_for_exchange(*bid, quote_currency, Exchange::Binance, state);
            let ask_usd = convert_stablecoin_to_usd_for_exchange(*ask, quote_currency, Exchange::Binance, state);

            state.update_price_with_bid_ask_and_raw(
                Exchange::Binance, pair_id, &display_symbol,
                mid_usd, bid_usd, ask_usd,    // USD-normalized
                *bid, *ask,                    // Original USDT/USDC
                *bid_size, *ask_size, quote_currency
            ).await;

            // Broadcast to connected clients
            let tick = PriceTick::with_depth(Exchange::Binance, pair_id, mid_price, *bid, *ask, *bid_size, *ask_size, quote_currency);
            ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::Binance, pair_id, &display_symbol, Some(&quote), &tick);
            total_updated += 1;
        }
    }
    debug!("  Binance: {} orderbooks loaded", binance_result.len());

    // Process Bybit orderbooks
    for (symbol, (bid, ask, bid_size, ask_size)) in &bybit_result {
        // Extract base and quote from Bybit symbol (e.g., BTCUSDT -> BTC, USDT)
        if let Some((base, quote)) = extract_bybit_base_quote(symbol) {
            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

            // Update stablecoin prices for this exchange
            if base == "USDT" || base == "USDC" {
                state.update_exchange_stablecoin_price(Exchange::Bybit, &base, &quote, mid_price.to_f64());
            }

            // Use BTC as reference crypto for deriving stablecoin rates
            if base == "BTC" && (quote == "USD" || quote == "USDT" || quote == "USDC") {
                state.update_exchange_ref_crypto_price(Exchange::Bybit, &quote, mid_price.to_f64());
            }

            let display_symbol = symbol_mappings.canonical_name("Bybit", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

            // Convert stablecoin prices to USD
            let mid_usd = convert_stablecoin_to_usd_for_exchange(mid_price, quote_currency, Exchange::Bybit, state);
            let bid_usd = convert_stablecoin_to_usd_for_exchange(*bid, quote_currency, Exchange::Bybit, state);
            let ask_usd = convert_stablecoin_to_usd_for_exchange(*ask, quote_currency, Exchange::Bybit, state);

            state.update_price_with_bid_ask_and_raw(
                Exchange::Bybit, pair_id, &display_symbol,
                mid_usd, bid_usd, ask_usd,    // USD-normalized
                *bid, *ask,                    // Original USDT/USDC
                *bid_size, *ask_size, quote_currency
            ).await;

            // Broadcast to connected clients
            let tick = PriceTick::with_depth(Exchange::Bybit, pair_id, mid_price, *bid, *ask, *bid_size, *ask_size, quote_currency);
            ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::Bybit, pair_id, &display_symbol, Some(&quote), &tick);
            total_updated += 1;
        }
    }
    debug!("  Bybit: {} orderbooks loaded", bybit_result.len());

    // Process GateIO orderbooks
    for (currency_pair, (bid, ask, bid_size, ask_size)) in &gateio_result {
        if let Some((base, quote)) = GateIOAdapter::extract_base_quote(currency_pair) {
            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

            // Update stablecoin prices for this exchange
            if base == "USDT" || base == "USDC" {
                state.update_exchange_stablecoin_price(Exchange::GateIO, &base, &quote, mid_price.to_f64());
            }

            let display_symbol = symbol_mappings.canonical_name("GateIO", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

            // Convert stablecoin prices to USD
            let mid_usd = convert_stablecoin_to_usd_for_exchange(mid_price, quote_currency, Exchange::GateIO, state);
            let bid_usd = convert_stablecoin_to_usd_for_exchange(*bid, quote_currency, Exchange::GateIO, state);
            let ask_usd = convert_stablecoin_to_usd_for_exchange(*ask, quote_currency, Exchange::GateIO, state);

            state.update_price_with_bid_ask_and_raw(
                Exchange::GateIO, pair_id, &display_symbol,
                mid_usd, bid_usd, ask_usd,    // USD-normalized
                *bid, *ask,                    // Original USDT/USDC
                *bid_size, *ask_size, quote_currency
            ).await;

            // Broadcast to connected clients
            let tick = PriceTick::with_depth(Exchange::GateIO, pair_id, mid_price, *bid, *ask, *bid_size, *ask_size, quote_currency);
            ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::GateIO, pair_id, &display_symbol, Some(&quote), &tick);
            total_updated += 1;
        }
    }
    debug!("  GateIO: {} orderbooks loaded", gateio_result.len());

    // Process Upbit orderbooks (prices are in KRW, need conversion to USD)
    // First, extract USDT/KRW and USDC/KRW rates from the result
    let usdt_krw_rate = upbit_result.get("KRW-USDT")
        .map(|(bid, ask, _, _)| (bid.to_f64() + ask.to_f64()) / 2.0);
    let usdc_krw_rate = upbit_result.get("KRW-USDC")
        .map(|(bid, ask, _, _)| (bid.to_f64() + ask.to_f64()) / 2.0);

    if let Some(rate) = usdt_krw_rate {
        state.update_upbit_usdt_krw(FixedPoint::from_f64(rate));
        debug!("  Upbit: USDT/KRW rate from REST: {:.2}", rate);
    }
    if let Some(rate) = usdc_krw_rate {
        state.update_upbit_usdc_krw(FixedPoint::from_f64(rate));
        debug!("  Upbit: USDC/KRW rate from REST: {:.2}", rate);
    }

    // Get the USDT/KRW rate for conversion (from REST or existing state)
    let usdt_krw = usdt_krw_rate.or_else(|| state.get_upbit_usdt_krw().map(|p| p.to_f64()));
    let usdt_usd = state.get_usdt_usd_price().to_f64();

    for (market, (bid, ask, bid_size, ask_size)) in &upbit_result {
        // Skip stablecoin markets (already processed above)
        if market == "KRW-USDT" || market == "KRW-USDC" {
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

                let mid_price_fp = FixedPoint::from_f64(mid_price_usd);
                let bid_fp = FixedPoint::from_f64(bid_usd);
                let ask_fp = FixedPoint::from_f64(ask_usd);

                // Pass both USD-normalized and original KRW prices
                state.update_price_with_bid_ask_and_raw(
                    Exchange::Upbit, pair_id, &display_symbol,
                    mid_price_fp, bid_fp, ask_fp,  // USD-normalized
                    *bid, *ask,                     // Original KRW
                    *bid_size, *ask_size, QuoteCurrency::KRW
                ).await;

                // Broadcast to connected clients
                let tick = PriceTick::with_depth(Exchange::Upbit, pair_id, mid_price_fp, bid_fp, ask_fp, *bid_size, *ask_size, QuoteCurrency::KRW);
                ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::Upbit, pair_id, &display_symbol, Some("KRW"), &tick);
            } else {
                // No exchange rate available yet, store raw KRW prices
                // They will be updated when WebSocket provides the rate
                let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);
                // When no exchange rate, raw = normalized (both KRW)
                state.update_price_with_bid_ask_and_raw(
                    Exchange::Upbit, pair_id, &display_symbol,
                    mid_price, *bid, *ask,  // KRW (no conversion)
                    *bid, *ask,              // Original KRW
                    *bid_size, *ask_size, QuoteCurrency::KRW
                ).await;

                // Broadcast to connected clients
                let tick = PriceTick::with_depth(Exchange::Upbit, pair_id, mid_price, *bid, *ask, *bid_size, *ask_size, QuoteCurrency::KRW);
                ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::Upbit, pair_id, &display_symbol, Some("KRW"), &tick);
                debug!("  Upbit: {} stored as KRW (no exchange rate yet)", display_symbol);
            }
            total_updated += 1;
        }
    }
    debug!("  Upbit: {} orderbooks loaded", upbit_result.len());

    // Process Bithumb orderbooks (similar to Upbit, prices are in KRW)
    // Extract USDT/KRW and USDC/KRW rates if available
    let bithumb_usdt_krw = bithumb_result.get("USDT")
        .map(|(bid, ask, _, _)| (bid.to_f64() + ask.to_f64()) / 2.0);
    let bithumb_usdc_krw = bithumb_result.get("USDC")
        .map(|(bid, ask, _, _)| (bid.to_f64() + ask.to_f64()) / 2.0);

    if let Some(rate) = bithumb_usdt_krw {
        state.update_bithumb_usdt_krw(FixedPoint::from_f64(rate));
        debug!("  Bithumb: USDT/KRW rate from REST: {:.2}", rate);
    }
    if let Some(rate) = bithumb_usdc_krw {
        state.update_bithumb_usdc_krw(FixedPoint::from_f64(rate));
        debug!("  Bithumb: USDC/KRW rate from REST: {:.2}", rate);
    }

    for (symbol, (bid, ask, bid_size, ask_size)) in &bithumb_result {
        // Skip stablecoin markets (already processed above)
        if symbol == "USDT" || symbol == "USDC" {
            continue;
        }

        // Bithumb REST fetcher uses base symbol (e.g., "BTC", "ETH")
        let display_symbol = symbol_mappings.canonical_name("Bithumb", symbol);
        let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);

        // All Bithumb prices are in KRW
        if let Some(usdt_krw_rate) = bithumb_usdt_krw {
            let bid_usd = bid.to_f64() / usdt_krw_rate * usdt_usd;
            let ask_usd = ask.to_f64() / usdt_krw_rate * usdt_usd;
            let mid_price_usd = (bid_usd + ask_usd) / 2.0;

            let mid_price_fp = FixedPoint::from_f64(mid_price_usd);
            let bid_fp = FixedPoint::from_f64(bid_usd);
            let ask_fp = FixedPoint::from_f64(ask_usd);

            // Pass both USD-normalized and original KRW prices
            state.update_price_with_bid_ask_and_raw(
                Exchange::Bithumb, pair_id, &display_symbol,
                mid_price_fp, bid_fp, ask_fp,  // USD-normalized
                *bid, *ask,                     // Original KRW
                *bid_size, *ask_size, QuoteCurrency::KRW
            ).await;

            // Broadcast to connected clients
            let tick = PriceTick::with_depth(Exchange::Bithumb, pair_id, mid_price_fp, bid_fp, ask_fp, *bid_size, *ask_size, QuoteCurrency::KRW);
            ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::Bithumb, pair_id, &display_symbol, Some("KRW"), &tick);
        }
        // Note: If USDT/KRW rate is not available, we skip this symbol.
        // KRW prices without conversion would be invalid in USD terms.
        total_updated += 1;
    }
    debug!("  Bithumb: {} orderbooks loaded", bithumb_result.len());

    // Process Coinbase stablecoin orderbooks
    for (product_id, (bid, ask, bid_size, ask_size)) in &coinbase_stablecoin_result {
        // Extract base and quote from product_id (e.g., USDT-USD -> USDT, USD)
        if let Some((base, quote)) = product_id.split_once('-') {
            let mid_price = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

            // Update stablecoin prices for this exchange
            state.update_exchange_stablecoin_price(Exchange::Coinbase, base, quote, mid_price.to_f64());

            let pair_id = arbitrage_core::symbol_to_pair_id(base);
            let quote_currency = QuoteCurrency::from_str(quote).unwrap_or(QuoteCurrency::USD);

            // Convert stablecoin prices to USD
            let mid_usd = convert_stablecoin_to_usd_for_exchange(mid_price, quote_currency, Exchange::Coinbase, state);
            let bid_usd = convert_stablecoin_to_usd_for_exchange(*bid, quote_currency, Exchange::Coinbase, state);
            let ask_usd = convert_stablecoin_to_usd_for_exchange(*ask, quote_currency, Exchange::Coinbase, state);

            state.update_price_with_bid_ask_and_raw(
                Exchange::Coinbase, pair_id, base,
                mid_usd, bid_usd, ask_usd,    // USD-normalized
                *bid, *ask,                    // Original USD/USDC
                *bid_size, *ask_size, quote_currency
            ).await;

            // Broadcast to connected clients
            let tick = PriceTick::with_depth(Exchange::Coinbase, pair_id, mid_price, *bid, *ask, *bid_size, *ask_size, quote_currency);
            ws_server::broadcast_price_with_quote(broadcast_tx, Exchange::Coinbase, pair_id, base, Some(quote), &tick);
            total_updated += 1;
            debug!("  Coinbase: {} @ {:.4} (REST)", product_id, mid_price.to_f64());
        }
    }

    info!("📚 Initial orderbook fetch complete: {} total orderbooks loaded", total_updated);
}

/// Spawn live WebSocket feeds
async fn spawn_live_feeds(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    symbol_mappings: &SymbolMappings,
    status_notifier: Option<StatusNotifierHandle>,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    // First, do an initial market discovery to get common symbols
    let discovery = MarketDiscovery::new();
    let all_markets = discovery.fetch_all().await;

    // Find common markets across exchanges we support for live feeds
    // Apply symbol mappings to exclude mismatched symbols
    let exchanges = ["Binance", "Coinbase", "Upbit", "Bithumb", "Bybit", "GateIO"];
    let common = MarketDiscovery::find_markets_on_n_exchanges_with_mappings(
        &all_markets,
        &exchanges,
        2,
        Some(symbol_mappings),
    );
    info!("🔍 Found {} common markets from {} exchanges", common.common.len(), all_markets.len());

    // Get symbols for each exchange from by_quote markets (includes both USDT and USDC)
    // This ensures we subscribe to all quote variants (BTC-USDT, BTC-USDC, etc.)
    use std::collections::HashSet;
    let mut binance_set: HashSet<String> = HashSet::new();
    let mut coinbase_set: HashSet<String> = HashSet::new();
    let mut upbit_set: HashSet<String> = HashSet::new();
    let mut bithumb_set: HashSet<String> = HashSet::new();
    let mut bybit_set: HashSet<String> = HashSet::new();
    let mut gateio_set: HashSet<String> = HashSet::new();

    // Always include USDT and USDC for exchange rate calculation
    upbit_set.insert("KRW-USDT".to_string());
    upbit_set.insert("KRW-USDC".to_string());
    bithumb_set.insert("KRW-USDT".to_string());
    bithumb_set.insert("KRW-USDC".to_string());

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
    state.register_common_markets(&common);

    // Fetch initial orderbooks via REST API before WebSocket feeds start
    // Only batch-capable exchanges: Binance, Bybit, GateIO, Upbit
    // Coinbase and Bithumb rely on WebSocket for initial data
    // Include stablecoin symbols for initial price fetch
    let mut binance_symbols_with_stablecoins = binance_symbols.clone();
    // Binance stablecoin pairs: USDT/USD, USDC/USDT, USDC/USD
    binance_symbols_with_stablecoins.push("USDTUSD".to_string());
    binance_symbols_with_stablecoins.push("USDCUSDT".to_string());
    binance_symbols_with_stablecoins.push("USDCUSD".to_string());

    let mut bybit_symbols_with_stablecoins = bybit_symbols.clone();
    // Bybit stablecoin pairs: USDC/USDT
    // Also add BTC/USD and BTC/USDC for deriving stablecoin rates (BTCUSD / BTCUSDT = USDT/USD)
    bybit_symbols_with_stablecoins.push("USDCUSDT".to_string());
    bybit_symbols_with_stablecoins.push("BTCUSD".to_string());
    bybit_symbols_with_stablecoins.push("BTCUSDC".to_string());

    // GateIO stablecoin pairs: USDT/USD, USDC/USDT
    // Put stablecoins FIRST to ensure they're fetched before rate limiting kicks in
    let mut gateio_symbols_with_stablecoins = vec!["USDC_USDT".to_string(), "USDT_USD".to_string()];
    gateio_symbols_with_stablecoins.extend(gateio_symbols.clone());

    // Bithumb uses base symbol format (e.g., "USDT", "BTC")
    // Extract base symbols from KRW-XXX format
    let bithumb_base_symbols: Vec<String> = bithumb_symbols.iter()
        .filter_map(|s| s.strip_prefix("KRW-").map(|base| base.to_string()))
        .collect();

    fetch_initial_orderbooks(
        &state,
        &broadcast_tx,
        &binance_symbols_with_stablecoins,
        &bybit_symbols_with_stablecoins,
        &gateio_symbols_with_stablecoins,
        &upbit_symbols,
        &bithumb_base_symbols,
        symbol_mappings,
    ).await;

    // Convert symbol_mappings to Arc for sharing across tasks
    let symbol_mappings_arc = Arc::new(symbol_mappings.clone());

    // Binance
    if !binance_symbols.is_empty() {
        let (binance_tx, binance_rx) = mpsc::channel(5000); // ~10x symbols for high volume

        // Add stablecoin rate symbols to subscription
        let mut all_binance_symbols = binance_symbols.clone();
        all_binance_symbols.push("USDTUSD".to_string());
        all_binance_symbols.push("USDCUSDT".to_string());
        all_binance_symbols.push("USDCUSD".to_string());

        // Use combined stream URL (includes stream name in response, needed for depth streams)
        // Depth stream responses don't include symbol, so we need the wrapper format
        let combined_url = BinanceAdapter::ws_url_combined(&all_binance_symbols);
        let mut binance_config = FeedConfig::for_exchange(Exchange::Binance);
        binance_config.ws_url = combined_url;

        let binance_client = WsClient::new(binance_config.clone(), binance_tx);
        handles.push(tokio::spawn(async move {
            // Combined stream URL auto-subscribes, no need to send SUBSCRIBE messages
            if let Err(e) = binance_client.run(None).await {
                warn!("Binance WebSocket error: {}", e);
            }
        }));

        let binance_ctx = FeedContext::new(
            state.clone(),
            broadcast_tx.clone(),
            symbol_mappings_arc.clone(),
            status_notifier.clone(),
        );
        handles.push(tokio::spawn(async move {
            feeds::run_binance_feed(binance_ctx, binance_rx).await;
        }));
    }

    // Coinbase (requires authentication for level2 channel)
    // Send multiple subscription messages in batches to avoid "too many L2 streams" error
    if !coinbase_symbols.is_empty() {
        if let Some(credentials) = CoinbaseCredentials::from_env() {
            let coinbase_config = FeedConfig::for_exchange(Exchange::Coinbase);
            let (coinbase_tx, coinbase_rx) = mpsc::channel(5000);

            const BATCH_SIZE: usize = 20;

            // Priority symbols - major coins first
            let priority_bases = ["BTC", "ETH", "SOL", "XRP", "DOGE", "ADA", "LINK", "AVAX", "DOT", "MATIC"];
            let mut prioritized_symbols: Vec<String> = Vec::new();

            // Add priority symbols first (both USD and USDT pairs)
            for base in &priority_bases {
                for suffix in &["-USD", "-USDT"] {
                    let symbol = format!("{}{}", base, suffix);
                    if coinbase_symbols.contains(&symbol) && !prioritized_symbols.contains(&symbol) {
                        prioritized_symbols.push(symbol);
                    }
                }
            }

            // Add remaining symbols
            for symbol in &coinbase_symbols {
                if !prioritized_symbols.contains(symbol) {
                    prioritized_symbols.push(symbol.clone());
                }
            }

            // Add stablecoin rate symbols
            for stablecoin in &["USDT-USD", "USDT-USDC"] {
                if !prioritized_symbols.contains(&stablecoin.to_string()) {
                    prioritized_symbols.push(stablecoin.to_string());
                }
            }

            // Generate subscription messages in batches
            let mut all_subscribe_msgs: Vec<String> = Vec::new();
            for chunk in prioritized_symbols.chunks(BATCH_SIZE) {
                match CoinbaseAdapter::subscribe_messages_with_auth(&chunk.to_vec(), &credentials) {
                    Ok(msgs) => all_subscribe_msgs.extend(msgs),
                    Err(e) => warn!("Coinbase: Failed to generate subscription for batch: {}", e),
                }
            }

            if !all_subscribe_msgs.is_empty() {
                debug!(
                    "Coinbase: Subscribing to {} symbols in {} batches",
                    prioritized_symbols.len(),
                    all_subscribe_msgs.len()
                );

                let coinbase_client = WsClient::new(coinbase_config.clone(), coinbase_tx);
                handles.push(tokio::spawn(async move {
                    if let Err(e) = coinbase_client.run_with_messages(Some(all_subscribe_msgs)).await {
                        warn!("Coinbase WebSocket error: {}", e);
                    }
                }));

                let coinbase_ctx = FeedContext::new(
                    state.clone(),
                    broadcast_tx.clone(),
                    symbol_mappings_arc.clone(),
                    status_notifier.clone(),
                );
                handles.push(tokio::spawn(async move {
                    feeds::run_coinbase_feed(coinbase_ctx, coinbase_rx).await;
                }));
            }
        } else {
            warn!("Coinbase: No API credentials found (COINBASE_API_KEY_ID, COINBASE_SECRET_KEY). Skipping Coinbase feed.");
        }
    }

    // Upbit
    if !upbit_symbols.is_empty() {
        let upbit_config = FeedConfig::for_exchange(Exchange::Upbit);
        let (upbit_tx, upbit_rx) = mpsc::channel(5000); // ~22x symbols for high volume
        let upbit_subscribe = UpbitAdapter::subscribe_message(&upbit_symbols);

        let upbit_client = WsClient::new(upbit_config.clone(), upbit_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = upbit_client.run(Some(upbit_subscribe)).await {
                warn!("Upbit WebSocket error: {}", e);
            }
        }));

        let upbit_ctx = FeedContext::new(
            state.clone(),
            broadcast_tx.clone(),
            symbol_mappings_arc.clone(),
            status_notifier.clone(),
        );
        handles.push(tokio::spawn(async move {
            feeds::run_upbit_feed(upbit_ctx, upbit_rx).await;
        }));
    }

    // Bithumb
    if !bithumb_symbols.is_empty() {
        let bithumb_config = FeedConfig::for_exchange(Exchange::Bithumb);
        let (bithumb_tx, bithumb_rx) = mpsc::channel(5000); // ~22x symbols for high volume
        let bithumb_subscribe = BithumbAdapter::subscribe_message(&bithumb_symbols);

        let bithumb_client = WsClient::new(bithumb_config.clone(), bithumb_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = bithumb_client.run(Some(bithumb_subscribe)).await {
                warn!("Bithumb WebSocket error: {}", e);
            }
        }));

        let bithumb_ctx = FeedContext::new(
            state.clone(),
            broadcast_tx.clone(),
            symbol_mappings_arc.clone(),
            status_notifier.clone(),
        );
        handles.push(tokio::spawn(async move {
            feeds::run_bithumb_feed(bithumb_ctx, bithumb_rx).await;
        }));
    }

    // Bybit
    if !bybit_symbols.is_empty() {
        let bybit_config = FeedConfig::for_exchange(Exchange::Bybit);
        let (bybit_tx, bybit_rx) = mpsc::channel(10000); // Larger buffer for orderbook.50 (50 levels per update)

        // Add stablecoin rate symbols to subscription
        // Bybit doesn't have USDT/USD pair, but has USDC/USDT
        // Also add BTC/USD and BTC/USDC for deriving stablecoin rates
        let mut all_bybit_symbols = bybit_symbols.clone();
        all_bybit_symbols.push("USDCUSDT".to_string());
        all_bybit_symbols.push("BTCUSD".to_string());
        all_bybit_symbols.push("BTCUSDC".to_string());

        // Bybit has a limit of 10 args per subscription, so we batch them
        let bybit_subscribe_msgs = BybitAdapter::subscribe_messages(&all_bybit_symbols);

        let bybit_client = WsClient::new(bybit_config.clone(), bybit_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = bybit_client.run_with_messages(Some(bybit_subscribe_msgs)).await {
                warn!("Bybit WebSocket error: {}", e);
            }
        }));

        let bybit_ctx = FeedContext::new(
            state.clone(),
            broadcast_tx.clone(),
            symbol_mappings_arc.clone(),
            status_notifier.clone(),
        );
        handles.push(tokio::spawn(async move {
            feeds::run_bybit_feed(bybit_ctx, bybit_rx).await;
        }));
    }

    // Gate.io
    if !gateio_symbols.is_empty() {
        let gateio_config = FeedConfig::for_exchange(Exchange::GateIO);
        let (gateio_tx, gateio_rx) = mpsc::channel(5000); // Larger buffer for Gate.io (754 symbols, high message volume)

        // Add stablecoin rate symbols to subscription
        let mut all_gateio_symbols = gateio_symbols.clone();
        all_gateio_symbols.push("USDT_USD".to_string());
        all_gateio_symbols.push("USDC_USDT".to_string());

        // Subscribe to both ticker and order_book for orderbook depth
        let gateio_subscribe_msgs = GateIOAdapter::subscribe_messages(&all_gateio_symbols);

        let gateio_client = WsClient::new(gateio_config.clone(), gateio_tx);
        handles.push(tokio::spawn(async move {
            if let Err(e) = gateio_client.run_with_messages(Some(gateio_subscribe_msgs)).await {
                warn!("Gate.io WebSocket error: {}", e);
            }
        }));

        let gateio_ctx = FeedContext::new(
            state.clone(),
            broadcast_tx.clone(),
            symbol_mappings_arc.clone(),
            status_notifier.clone(),
        );
        handles.push(tokio::spawn(async move {
            feeds::run_gateio_feed(gateio_ctx, gateio_rx).await;
        }));
    }

    handles
}

/// Run market discovery loop - periodically fetches markets from exchanges
/// and broadcasts common markets to clients.
async fn run_market_discovery(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    _symbol_mappings: Arc<SymbolMappings>,
) {
    debug!("Starting market discovery loop");

    let discovery = MarketDiscovery::new();
    let exchanges = ["Binance", "Coinbase", "Upbit", "Bithumb", "Bybit", "GateIO"];

    loop {
        // Reload symbol mappings on each iteration (in case they were updated)
        let current_mappings = load_mappings();

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

            // Register all common markets for opportunity detection
            state.register_common_markets(&common);

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

    // Initialize status notifier for WebSocket connection monitoring
    let status_notifier = status_notifier::try_start_status_notifier();
    if status_notifier.is_some() {
        info!("📱 Status notifier enabled for WebSocket connection monitoring");
    }

    // Send server started notification
    if let Some(ref notifier) = status_notifier {
        notifier.try_send(StatusEvent::ServerStarted);
    }

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

    // Start stale price cleanup task (runs every 10 seconds)
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let expired = cleanup_state.expire_stale_prices();
            if expired > 0 {
                tracing::debug!("Expired {} stale price entries", expired);
            }
        }
    });

    // Spawn price source (live or simulated)
    let feed_handles: Vec<tokio::task::JoinHandle<()>> = if args.live {
        info!("📡 Using LIVE WebSocket feeds");
        spawn_live_feeds(state.clone(), broadcast_tx.clone(), &symbol_mappings, status_notifier.clone()).await
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

    // Send server stopping notification
    if let Some(ref notifier) = status_notifier {
        notifier.try_send(StatusEvent::ServerStopping);
    }

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

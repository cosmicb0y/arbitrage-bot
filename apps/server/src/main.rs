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
use state::{create_state, PriceUpdateEvent, SharedState};
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use arbitrage_alerts::{Database, Notifier, NotifierConfig, TelegramBot};

use arbitrage_core::{Exchange, FixedPoint, PriceTick, QuoteCurrency};
use arbitrage_engine::ConversionRates;
use arbitrage_feeds::{
    load_mappings, runner as feed_runner, BinanceAdapter, BinanceConnectionPool, BinanceRestFetcher,
    BithumbAdapter, BithumbRestFetcher, BithumbSubscriptionBuilder, BybitAdapter, BybitRestFetcher,
    BybitSubscriptionBuilder, CoinbaseAdapter, CoinbaseConnectionPool, CoinbaseCredentials,
    CoinbaseRestFetcher, ExchangeAdapter, FeedConfig, FeedMessage, GateIOAdapter, GateIORestFetcher,
    GateIOSubscriptionBuilder, MarketDiscovery, SubscriptionManager, SymbolMappings, UpbitAdapter,
    UpbitRestFetcher, UpbitSubscriptionBuilder, WsClient,
};
use feeds::common::{
    convert_stablecoin_to_usd_for_exchange, extract_binance_base_quote, extract_bybit_base_quote,
};
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

/// Broadcast premium matrix for a pair to all WebSocket clients.
fn broadcast_premium_matrix_for_pair(
    state: &SharedState,
    broadcast_tx: &BroadcastSender,
    pair_id: u32,
    symbol: &str,
) {
    // Get conversion rates from state
    let usdt_krw = state.get_upbit_usdt_krw().map(|p| p.to_f64());
    let usdc_krw = state.get_upbit_usdc_krw().map(|p| p.to_f64());
    let usd_krw = exchange_rate::get_api_rate().or(usdt_krw);
    let bithumb_usdt_krw = state.get_bithumb_usdt_krw().map(|p| p.to_f64());
    let bithumb_usdc_krw = state.get_bithumb_usdc_krw().map(|p| p.to_f64());

    let rates = ConversionRates {
        usdt_usd: state.get_usdt_usd_price().to_f64(),
        usdc_usd: state.get_usdc_usd_price().to_f64(),
        usd_krw: usd_krw.unwrap_or(0.0),
        upbit_usdt_krw: usdt_krw.unwrap_or(0.0),
        upbit_usdc_krw: usdc_krw.unwrap_or(0.0),
        bithumb_usdt_krw: bithumb_usdt_krw.unwrap_or(0.0),
        bithumb_usdc_krw: bithumb_usdc_krw.unwrap_or(0.0),
    };

    // Get the premium matrix for this pair
    if let Some(matrix) = state.detector.get_matrix(pair_id) {
        let premiums = matrix.all_premiums_multi_denomination(&rates);

        // Log conversion rates and entry count for KRW debugging
        let krw_entries: Vec<_> = premiums
            .iter()
            .filter(|p| p.2 == QuoteCurrency::KRW || p.3 == QuoteCurrency::KRW)
            .collect();
        if !krw_entries.is_empty() {
            tracing::debug!(
                symbol = symbol,
                total_entries = premiums.len(),
                krw_entries = krw_entries.len(),
                upbit_usdt_krw = rates.upbit_usdt_krw,
                bithumb_usdt_krw = rates.bithumb_usdt_krw,
                usd_krw = rates.usd_krw,
                "Premium matrix for symbol with KRW markets"
            );
            // Log first few KRW entries for debugging
            for (i, e) in krw_entries.iter().take(2).enumerate() {
                tracing::debug!(
                    i = i,
                    buy_ex = ?e.0,
                    sell_ex = ?e.1,
                    buy_quote = ?e.2,
                    sell_quote = ?e.3,
                    tether_bps = e.10,
                    kimchi_bps = e.12,
                    "KRW entry details"
                );
            }
        }

        // Convert to WsPremiumEntry
        // Use Display format for quote currency (e.g., "KRW", "USDT")
        // Use Debug format for exchange (e.g., "Binance", "Upbit")
        let entries: Vec<ws_server::WsPremiumEntry> = premiums
            .iter()
            .map(|p| ws_server::WsPremiumEntry {
                buy_exchange: format!("{:?}", p.0),
                sell_exchange: format!("{:?}", p.1),
                buy_quote: p.2.as_str().to_string(),
                sell_quote: p.3.as_str().to_string(),
                tether_premium_bps: p.10, // usdlike_premium_bps
                kimchi_premium_bps: p.12, // kimchi_premium
            })
            .collect();

        ws_server::broadcast_premium_matrix(broadcast_tx, symbol, pair_id, entries);
    }
}

/// Event-driven opportunity detector.
///
/// Reacts immediately to price update events from feed handlers.
/// Only detects opportunities for the pair that had a price update.
async fn run_event_driven_detector(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    notifier: Option<Arc<Notifier>>,
    mut price_rx: mpsc::Receiver<PriceUpdateEvent>,
) {
    debug!("Starting event-driven detector");

    while state.is_running() {
        match price_rx.recv().await {
            Some(event) => {
                // Immediately detect opportunities for this pair
                let opps = state.detect_opportunities(event.pair_id).await;

                // Broadcast each opportunity
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

                // Broadcast premium matrix for this symbol (all exchange pairs)
                broadcast_premium_matrix_for_pair(
                    &state,
                    &broadcast_tx,
                    event.pair_id,
                    &event.symbol,
                );

                // Send Telegram alerts
                if let Some(ref notifier) = notifier {
                    for opp in &opps {
                        if let Err(e) = notifier.process_opportunity(opp).await {
                            tracing::warn!("Failed to send Telegram alert: {}", e);
                        }
                    }

                    // Clear opportunities that fell below threshold
                    if let Err(e) = notifier.clear_missing_opportunities(&opps).await {
                        tracing::warn!("Failed to clear missing opportunities: {}", e);
                    }
                }
            }
            None => {
                // Channel closed, exit
                break;
            }
        }
    }

    debug!("Event-driven detector stopped");
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
    let (
        binance_result,
        bybit_result,
        gateio_result,
        upbit_result,
        bithumb_result,
        coinbase_stablecoin_result,
    ) = tokio::join!(
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
                state.update_exchange_stablecoin_price(
                    Exchange::Binance,
                    &base,
                    &quote,
                    mid_price.to_f64(),
                );
                if base == "USDT" && quote == "USD" {
                    state.update_usdt_usd_price(mid_price);
                }
            }

            let display_symbol = symbol_mappings.canonical_name("Binance", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

            // Convert stablecoin prices to USD
            let mid_usd = convert_stablecoin_to_usd_for_exchange(
                mid_price,
                quote_currency,
                Exchange::Binance,
                state,
            );
            let bid_usd = convert_stablecoin_to_usd_for_exchange(
                *bid,
                quote_currency,
                Exchange::Binance,
                state,
            );
            let ask_usd = convert_stablecoin_to_usd_for_exchange(
                *ask,
                quote_currency,
                Exchange::Binance,
                state,
            );

            state
                .update_price_with_bid_ask_and_raw(
                    Exchange::Binance,
                    pair_id,
                    &display_symbol,
                    mid_usd,
                    bid_usd,
                    ask_usd, // USD-normalized
                    *bid,
                    *ask, // Original USDT/USDC
                    *bid_size,
                    *ask_size,
                    quote_currency,
                )
                .await;

            // Broadcast to connected clients with USD prices for comparison
            let tick = PriceTick::with_depth(
                Exchange::Binance,
                pair_id,
                mid_price,
                *bid,
                *ask,
                *bid_size,
                *ask_size,
                quote_currency,
            );
            ws_server::broadcast_price_with_quote_and_usd(
                broadcast_tx,
                Exchange::Binance,
                pair_id,
                &display_symbol,
                Some(&quote),
                &tick,
                Some(mid_usd.to_f64()),
                Some(bid_usd.to_f64()),
                Some(ask_usd.to_f64()),
            );
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
                state.update_exchange_stablecoin_price(
                    Exchange::Bybit,
                    &base,
                    &quote,
                    mid_price.to_f64(),
                );
            }

            // Use BTC as reference crypto for deriving stablecoin rates
            if base == "BTC" && (quote == "USD" || quote == "USDT" || quote == "USDC") {
                state.update_exchange_ref_crypto_price(Exchange::Bybit, &quote, mid_price.to_f64());
            }

            let display_symbol = symbol_mappings.canonical_name("Bybit", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

            // Convert stablecoin prices to USD
            let mid_usd = convert_stablecoin_to_usd_for_exchange(
                mid_price,
                quote_currency,
                Exchange::Bybit,
                state,
            );
            let bid_usd = convert_stablecoin_to_usd_for_exchange(
                *bid,
                quote_currency,
                Exchange::Bybit,
                state,
            );
            let ask_usd = convert_stablecoin_to_usd_for_exchange(
                *ask,
                quote_currency,
                Exchange::Bybit,
                state,
            );

            state
                .update_price_with_bid_ask_and_raw(
                    Exchange::Bybit,
                    pair_id,
                    &display_symbol,
                    mid_usd,
                    bid_usd,
                    ask_usd, // USD-normalized
                    *bid,
                    *ask, // Original USDT/USDC
                    *bid_size,
                    *ask_size,
                    quote_currency,
                )
                .await;

            // Broadcast to connected clients with USD prices for comparison
            let tick = PriceTick::with_depth(
                Exchange::Bybit,
                pair_id,
                mid_price,
                *bid,
                *ask,
                *bid_size,
                *ask_size,
                quote_currency,
            );
            ws_server::broadcast_price_with_quote_and_usd(
                broadcast_tx,
                Exchange::Bybit,
                pair_id,
                &display_symbol,
                Some(&quote),
                &tick,
                Some(mid_usd.to_f64()),
                Some(bid_usd.to_f64()),
                Some(ask_usd.to_f64()),
            );
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
                state.update_exchange_stablecoin_price(
                    Exchange::GateIO,
                    &base,
                    &quote,
                    mid_price.to_f64(),
                );
            }

            let display_symbol = symbol_mappings.canonical_name("GateIO", &base);
            let pair_id = arbitrage_core::symbol_to_pair_id(&display_symbol);
            let quote_currency = QuoteCurrency::from_str(&quote).unwrap_or(QuoteCurrency::USD);

            // Convert stablecoin prices to USD
            let mid_usd = convert_stablecoin_to_usd_for_exchange(
                mid_price,
                quote_currency,
                Exchange::GateIO,
                state,
            );
            let bid_usd = convert_stablecoin_to_usd_for_exchange(
                *bid,
                quote_currency,
                Exchange::GateIO,
                state,
            );
            let ask_usd = convert_stablecoin_to_usd_for_exchange(
                *ask,
                quote_currency,
                Exchange::GateIO,
                state,
            );

            state
                .update_price_with_bid_ask_and_raw(
                    Exchange::GateIO,
                    pair_id,
                    &display_symbol,
                    mid_usd,
                    bid_usd,
                    ask_usd, // USD-normalized
                    *bid,
                    *ask, // Original USDT/USDC
                    *bid_size,
                    *ask_size,
                    quote_currency,
                )
                .await;

            // Broadcast to connected clients with USD prices for comparison
            let tick = PriceTick::with_depth(
                Exchange::GateIO,
                pair_id,
                mid_price,
                *bid,
                *ask,
                *bid_size,
                *ask_size,
                quote_currency,
            );
            ws_server::broadcast_price_with_quote_and_usd(
                broadcast_tx,
                Exchange::GateIO,
                pair_id,
                &display_symbol,
                Some(&quote),
                &tick,
                Some(mid_usd.to_f64()),
                Some(bid_usd.to_f64()),
                Some(ask_usd.to_f64()),
            );
            total_updated += 1;
        }
    }
    debug!("  GateIO: {} orderbooks loaded", gateio_result.len());

    // Process Upbit orderbooks (prices are in KRW, need conversion to USD)
    // First, extract USDT/KRW and USDC/KRW rates from the result
    let usdt_krw_rate = upbit_result
        .get("KRW-USDT")
        .map(|(bid, ask, _, _)| (bid.to_f64() + ask.to_f64()) / 2.0);
    let usdc_krw_rate = upbit_result
        .get("KRW-USDC")
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

            // Original KRW prices for tick
            let mid_price_krw = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

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
                state
                    .update_price_with_bid_ask_and_raw(
                        Exchange::Upbit,
                        pair_id,
                        &display_symbol,
                        mid_price_fp,
                        bid_fp,
                        ask_fp, // USD-normalized
                        *bid,
                        *ask, // Original KRW
                        *bid_size,
                        *ask_size,
                        QuoteCurrency::KRW,
                    )
                    .await;

                // Broadcast with original KRW prices in tick, USD prices in separate fields
                let tick = PriceTick::with_depth(
                    Exchange::Upbit,
                    pair_id,
                    mid_price_krw,
                    *bid,
                    *ask,
                    *bid_size,
                    *ask_size,
                    QuoteCurrency::KRW,
                );
                ws_server::broadcast_price_with_quote_and_usd(
                    broadcast_tx,
                    Exchange::Upbit,
                    pair_id,
                    &display_symbol,
                    Some("KRW"),
                    &tick,
                    Some(mid_price_usd),
                    Some(bid_usd),
                    Some(ask_usd),
                );
            } else {
                // No exchange rate available yet, store raw KRW prices
                // When no exchange rate, raw = normalized (both KRW)
                state
                    .update_price_with_bid_ask_and_raw(
                        Exchange::Upbit,
                        pair_id,
                        &display_symbol,
                        mid_price_krw,
                        *bid,
                        *ask, // KRW (no conversion)
                        *bid,
                        *ask, // Original KRW
                        *bid_size,
                        *ask_size,
                        QuoteCurrency::KRW,
                    )
                    .await;

                // Broadcast with original KRW prices, no USD conversion available
                let tick = PriceTick::with_depth(
                    Exchange::Upbit,
                    pair_id,
                    mid_price_krw,
                    *bid,
                    *ask,
                    *bid_size,
                    *ask_size,
                    QuoteCurrency::KRW,
                );
                ws_server::broadcast_price_with_quote_and_usd(
                    broadcast_tx,
                    Exchange::Upbit,
                    pair_id,
                    &display_symbol,
                    Some("KRW"),
                    &tick,
                    None,
                    None,
                    None,
                );
                debug!(
                    "  Upbit: {} stored as KRW (no exchange rate yet)",
                    display_symbol
                );
            }
            total_updated += 1;
        }
    }
    debug!("  Upbit: {} orderbooks loaded", upbit_result.len());

    // Process Bithumb orderbooks (similar to Upbit, prices are in KRW)
    // Extract USDT/KRW and USDC/KRW rates if available
    let bithumb_usdt_krw = bithumb_result
        .get("USDT")
        .map(|(bid, ask, _, _)| (bid.to_f64() + ask.to_f64()) / 2.0);
    let bithumb_usdc_krw = bithumb_result
        .get("USDC")
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

        // Original KRW prices for tick
        let mid_price_krw = FixedPoint::from_f64((bid.to_f64() + ask.to_f64()) / 2.0);

        // All Bithumb prices are in KRW
        if let Some(usdt_krw_rate) = bithumb_usdt_krw {
            let bid_usd = bid.to_f64() / usdt_krw_rate * usdt_usd;
            let ask_usd = ask.to_f64() / usdt_krw_rate * usdt_usd;
            let mid_price_usd = (bid_usd + ask_usd) / 2.0;

            let mid_price_fp = FixedPoint::from_f64(mid_price_usd);
            let bid_fp = FixedPoint::from_f64(bid_usd);
            let ask_fp = FixedPoint::from_f64(ask_usd);

            // Pass both USD-normalized and original KRW prices
            state
                .update_price_with_bid_ask_and_raw(
                    Exchange::Bithumb,
                    pair_id,
                    &display_symbol,
                    mid_price_fp,
                    bid_fp,
                    ask_fp, // USD-normalized
                    *bid,
                    *ask, // Original KRW
                    *bid_size,
                    *ask_size,
                    QuoteCurrency::KRW,
                )
                .await;

            // Broadcast with original KRW prices in tick, USD prices in separate fields
            let tick = PriceTick::with_depth(
                Exchange::Bithumb,
                pair_id,
                mid_price_krw,
                *bid,
                *ask,
                *bid_size,
                *ask_size,
                QuoteCurrency::KRW,
            );
            ws_server::broadcast_price_with_quote_and_usd(
                broadcast_tx,
                Exchange::Bithumb,
                pair_id,
                &display_symbol,
                Some("KRW"),
                &tick,
                Some(mid_price_usd),
                Some(bid_usd),
                Some(ask_usd),
            );
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
            state.update_exchange_stablecoin_price(
                Exchange::Coinbase,
                base,
                quote,
                mid_price.to_f64(),
            );

            let pair_id = arbitrage_core::symbol_to_pair_id(base);
            let quote_currency = QuoteCurrency::from_str(quote).unwrap_or(QuoteCurrency::USD);

            // Convert stablecoin prices to USD
            let mid_usd = convert_stablecoin_to_usd_for_exchange(
                mid_price,
                quote_currency,
                Exchange::Coinbase,
                state,
            );
            let bid_usd = convert_stablecoin_to_usd_for_exchange(
                *bid,
                quote_currency,
                Exchange::Coinbase,
                state,
            );
            let ask_usd = convert_stablecoin_to_usd_for_exchange(
                *ask,
                quote_currency,
                Exchange::Coinbase,
                state,
            );

            state
                .update_price_with_bid_ask_and_raw(
                    Exchange::Coinbase,
                    pair_id,
                    base,
                    mid_usd,
                    bid_usd,
                    ask_usd, // USD-normalized
                    *bid,
                    *ask, // Original USD/USDC
                    *bid_size,
                    *ask_size,
                    quote_currency,
                )
                .await;

            // Broadcast with original prices in tick, USD prices in separate fields
            let tick = PriceTick::with_depth(
                Exchange::Coinbase,
                pair_id,
                mid_price,
                *bid,
                *ask,
                *bid_size,
                *ask_size,
                quote_currency,
            );
            ws_server::broadcast_price_with_quote_and_usd(
                broadcast_tx,
                Exchange::Coinbase,
                pair_id,
                base,
                Some(quote),
                &tick,
                Some(mid_usd.to_f64()),
                Some(bid_usd.to_f64()),
                Some(ask_usd.to_f64()),
            );
            total_updated += 1;
            debug!(
                "  Coinbase: {} @ {:.4} (REST)",
                product_id,
                mid_price.to_f64()
            );
        }
    }

    info!(
        "📚 Initial orderbook fetch complete: {} total orderbooks loaded",
        total_updated
    );
}

/// Spawn live WebSocket feeds
/// Returns task handles and the SubscriptionManager for runtime subscription updates.
async fn spawn_live_feeds(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    symbol_mappings: &SymbolMappings,
    status_notifier: Option<StatusNotifierHandle>,
) -> (Vec<tokio::task::JoinHandle<()>>, Arc<SubscriptionManager>) {
    let mut handles = Vec::new();

    // Create SubscriptionManager for runtime dynamic subscriptions
    let mut subscription_manager = SubscriptionManager::new();

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
    info!(
        "🔍 Found {} common markets from {} exchanges",
        common.common.len(),
        all_markets.len()
    );

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
                "Binance" => {
                    binance_set.insert(market_info.symbol.to_lowercase());
                }
                "Coinbase" => {
                    coinbase_set.insert(market_info.symbol.clone());
                }
                "Upbit" => {
                    upbit_set.insert(market_info.symbol.clone());
                }
                "Bithumb" => {
                    bithumb_set.insert(market_info.symbol.clone());
                }
                "Bybit" => {
                    bybit_set.insert(market_info.symbol.clone());
                }
                "GateIO" => {
                    gateio_set.insert(market_info.symbol.clone());
                }
                _ => {}
            }
        }
        // Log each market group for debugging
        if key.starts_with("BTC/") || key.starts_with("ETH/") {
            debug!(
                "📊 {} -> {} exchanges: {:?}",
                key,
                exchange_markets.len(),
                exchange_markets
                    .iter()
                    .map(|(ex, m)| format!("{}:{}", ex, m.symbol))
                    .collect::<Vec<_>>()
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
    let bithumb_base_symbols: Vec<String> = bithumb_symbols
        .iter()
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
    )
    .await;

    // Convert symbol_mappings to Arc for sharing across tasks
    let symbol_mappings_arc = Arc::new(symbol_mappings.clone());

    // Create shared channel for all feed messages
    // All runners send FeedMessage to this channel, one handler processes them
    let (feed_tx, feed_rx) = mpsc::channel::<FeedMessage>(30000);

    // Start the common feed handler
    let handler_ctx = FeedContext::new(
        state.clone(),
        broadcast_tx.clone(),
        symbol_mappings_arc.clone(),
        status_notifier.clone(),
    );
    handles.push(tokio::spawn(async move {
        feeds::run_feed_handler(feed_rx, handler_ctx).await;
    }));

    // Binance - use connection pool to distribute symbols across multiple connections
    // (Binance limits each WebSocket connection to 1024 streams)
    if !binance_symbols.is_empty() {
        // Add stablecoin rate symbols to subscription
        let mut all_binance_symbols = binance_symbols.clone();
        all_binance_symbols.push("USDTUSD".to_string());
        all_binance_symbols.push("USDCUSDT".to_string());
        all_binance_symbols.push("USDCUSD".to_string());

        let num_connections = BinanceAdapter::connections_needed(all_binance_symbols.len());
        info!(
            "Binance: {} symbols require {} WebSocket connection(s)",
            all_binance_symbols.len(),
            num_connections
        );

        // Create connection pool and connect all symbols
        let mut binance_pool = BinanceConnectionPool::new();
        let mut pool_senders = Vec::new();
        let handles_and_receivers = binance_pool
            .connect_all(&all_binance_symbols, feed_tx.clone(), &mut pool_senders)
            .await;

        // Track initial subscriptions to prevent duplicate subscription on market discovery
        subscription_manager
            .track_initial_subscriptions(Exchange::Binance, all_binance_symbols.clone());

        // Register pool with subscription manager for dynamic subscriptions
        subscription_manager.register_exchange_pool(Exchange::Binance, pool_senders);

        // Add connection handles and start feed runners
        for (conn_idx, (handle, ws_rx)) in handles_and_receivers.into_iter().enumerate() {
            handles.push(handle);

            // Runner: WsMessage -> FeedMessage for each connection
            let feed_tx_clone = feed_tx.clone();
            handles.push(tokio::spawn(async move {
                tracing::debug!("Starting Binance feed runner for connection {}", conn_idx);
                feed_runner::run_binance(ws_rx, feed_tx_clone).await;
            }));
        }

        info!(
            "Binance: Started {} connection(s) with {} total symbols",
            binance_pool.connection_count(),
            binance_pool.total_symbol_count()
        );
    }

    // Coinbase (requires authentication for level2 channel)
    // Uses connection pool to distribute symbols across multiple connections (30 L2 streams each)
    if !coinbase_symbols.is_empty() {
        if let Some(credentials) = CoinbaseCredentials::from_env() {
            // Priority symbols - major coins first
            let priority_bases = [
                "BTC", "ETH", "SOL", "XRP", "DOGE", "ADA", "LINK", "AVAX", "DOT", "MATIC",
            ];
            let mut prioritized_symbols: Vec<String> = Vec::new();

            // Add priority symbols first (both USD and USDT pairs)
            for base in &priority_bases {
                for suffix in &["-USD", "-USDT"] {
                    let symbol = format!("{}{}", base, suffix);
                    if coinbase_symbols.contains(&symbol) && !prioritized_symbols.contains(&symbol)
                    {
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

            let symbols_to_subscribe = prioritized_symbols;

            info!(
                "Coinbase: {} symbols require {} WebSocket connection(s) (30 L2 streams per connection)",
                symbols_to_subscribe.len(),
                CoinbaseAdapter::connections_needed(symbols_to_subscribe.len())
            );

            // Track initial subscriptions to prevent duplicate subscription on market discovery
            subscription_manager
                .track_initial_subscriptions(Exchange::Coinbase, symbols_to_subscribe.clone());

            // Create connection pool and connect all symbols
            let mut coinbase_pool = CoinbaseConnectionPool::new(credentials);
            let mut pool_senders = Vec::new();
            let handles_and_receivers = coinbase_pool
                .connect_all(&symbols_to_subscribe, feed_tx.clone(), &mut pool_senders)
                .await;

            // Register pool with subscription manager for dynamic subscriptions
            // NOTE: Symbols already tracked via track_initial_subscriptions will be skipped (diff only)
            subscription_manager.register_exchange_pool(Exchange::Coinbase, pool_senders);

            // Add connection handles and start feed runners
            for (conn_idx, (handle, ws_rx)) in handles_and_receivers.into_iter().enumerate() {
                handles.push(handle);

                // Runner: WsMessage -> FeedMessage for each connection
                let feed_tx_clone = feed_tx.clone();
                handles.push(tokio::spawn(async move {
                    tracing::debug!("Starting Coinbase feed runner for connection {}", conn_idx);
                    feed_runner::run_coinbase(ws_rx, feed_tx_clone).await;
                }));
            }

            info!(
                "Coinbase: Started {} connection(s) with {} total symbols",
                coinbase_pool.connection_count(),
                coinbase_pool.total_symbol_count()
            );
        } else {
            warn!("Coinbase: No API credentials found (COINBASE_API_KEY_ID, COINBASE_SECRET_KEY). Skipping Coinbase feed.");
        }
    }

    // Upbit
    if !upbit_symbols.is_empty() {
        let upbit_config = FeedConfig::for_exchange(Exchange::Upbit);
        let (ws_tx, ws_rx) = mpsc::channel(5000);

        // Create subscription channel for runtime dynamic subscriptions
        let (sub_tx, sub_rx) = SubscriptionManager::create_channel();
        subscription_manager.register_exchange(Exchange::Upbit, sub_tx);

        let upbit_subscribe = UpbitAdapter::subscribe_message(&upbit_symbols);

        // Track initial subscriptions to prevent duplicate subscription on market discovery
        subscription_manager.track_initial_subscriptions(Exchange::Upbit, upbit_symbols.clone());

        let upbit_client = WsClient::new(upbit_config.clone(), ws_tx)
            .with_subscription_channel(sub_rx, Box::new(UpbitSubscriptionBuilder::new()));
        handles.push(tokio::spawn(async move {
            if let Err(e) = upbit_client.run(Some(upbit_subscribe)).await {
                warn!("Upbit WebSocket error: {}", e);
            }
        }));

        // Runner: WsMessage -> FeedMessage
        let feed_tx_clone = feed_tx.clone();
        handles.push(tokio::spawn(async move {
            feed_runner::run_upbit(ws_rx, feed_tx_clone).await;
        }));
    }

    // Bithumb
    if !bithumb_symbols.is_empty() {
        let bithumb_config = FeedConfig::for_exchange(Exchange::Bithumb);
        let (ws_tx, ws_rx) = mpsc::channel(5000);

        // Create subscription channel for runtime dynamic subscriptions
        let (sub_tx, sub_rx) = SubscriptionManager::create_channel();
        subscription_manager.register_exchange(Exchange::Bithumb, sub_tx);

        let bithumb_subscribe = BithumbAdapter::subscribe_message(&bithumb_symbols);

        // Track initial subscriptions to prevent duplicate subscription on market discovery
        subscription_manager.track_initial_subscriptions(Exchange::Bithumb, bithumb_symbols.clone());

        let bithumb_client = WsClient::new(bithumb_config.clone(), ws_tx)
            .with_subscription_channel(sub_rx, Box::new(BithumbSubscriptionBuilder::new()));
        handles.push(tokio::spawn(async move {
            if let Err(e) = bithumb_client.run(Some(bithumb_subscribe)).await {
                warn!("Bithumb WebSocket error: {}", e);
            }
        }));

        // Runner: WsMessage -> FeedMessage
        let feed_tx_clone = feed_tx.clone();
        handles.push(tokio::spawn(async move {
            feed_runner::run_bithumb(ws_rx, feed_tx_clone).await;
        }));
    }

    // Bybit
    if !bybit_symbols.is_empty() {
        let bybit_config = FeedConfig::for_exchange(Exchange::Bybit);
        let (ws_tx, ws_rx) = mpsc::channel(10000);

        // Create subscription channel for runtime dynamic subscriptions
        let (sub_tx, sub_rx) = SubscriptionManager::create_channel();
        subscription_manager.register_exchange(Exchange::Bybit, sub_tx);

        // Add stablecoin rate symbols to subscription
        let mut all_bybit_symbols = bybit_symbols.clone();
        all_bybit_symbols.push("USDCUSDT".to_string());
        all_bybit_symbols.push("BTCUSD".to_string());
        all_bybit_symbols.push("BTCUSDC".to_string());

        // Bybit has a limit of 10 args per subscription, so we batch them
        let bybit_subscribe_msgs = BybitAdapter::subscribe_messages(&all_bybit_symbols);

        // Track initial subscriptions to prevent duplicate subscription on market discovery
        subscription_manager.track_initial_subscriptions(Exchange::Bybit, all_bybit_symbols.clone());

        let bybit_client = WsClient::new(bybit_config.clone(), ws_tx)
            .with_subscription_channel(sub_rx, Box::new(BybitSubscriptionBuilder::new()));
        handles.push(tokio::spawn(async move {
            if let Err(e) = bybit_client
                .run_with_messages(Some(bybit_subscribe_msgs))
                .await
            {
                warn!("Bybit WebSocket error: {}", e);
            }
        }));

        // Runner: WsMessage -> FeedMessage
        let feed_tx_clone = feed_tx.clone();
        handles.push(tokio::spawn(async move {
            feed_runner::run_bybit(ws_rx, feed_tx_clone).await;
        }));
    }

    // Gate.io
    if !gateio_symbols.is_empty() {
        let gateio_config = FeedConfig::for_exchange(Exchange::GateIO);
        let (ws_tx, ws_rx) = mpsc::channel(5000);

        // Create subscription channel for runtime dynamic subscriptions
        let (sub_tx, sub_rx) = SubscriptionManager::create_channel();
        subscription_manager.register_exchange(Exchange::GateIO, sub_tx);

        // Add stablecoin rate symbols to subscription
        let mut all_gateio_symbols = gateio_symbols.clone();
        all_gateio_symbols.push("USDT_USD".to_string());
        all_gateio_symbols.push("USDC_USDT".to_string());

        // Subscribe to orderbook channel
        let gateio_subscribe_msgs = GateIOAdapter::subscribe_messages(&all_gateio_symbols);

        // Track initial subscriptions to prevent duplicate subscription on market discovery
        subscription_manager.track_initial_subscriptions(Exchange::GateIO, all_gateio_symbols.clone());

        let gateio_client = WsClient::new(gateio_config.clone(), ws_tx)
            .with_subscription_channel(sub_rx, Box::new(GateIOSubscriptionBuilder::new()));
        handles.push(tokio::spawn(async move {
            if let Err(e) = gateio_client
                .run_with_messages(Some(gateio_subscribe_msgs))
                .await
            {
                warn!("Gate.io WebSocket error: {}", e);
            }
        }));

        // Runner: WsMessage -> FeedMessage
        let feed_tx_clone = feed_tx.clone();
        handles.push(tokio::spawn(async move {
            feed_runner::run_gateio(ws_rx, feed_tx_clone).await;
        }));
    }

    // Wrap SubscriptionManager in Arc for sharing with run_market_discovery
    let subscription_manager = Arc::new(subscription_manager);

    (handles, subscription_manager)
}

/// Run market discovery loop - periodically fetches markets from exchanges
/// and broadcasts common markets to clients.
/// Also triggers runtime subscription updates via SubscriptionManager.
async fn run_market_discovery(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    _symbol_mappings: Arc<SymbolMappings>,
    subscription_manager: Arc<SubscriptionManager>,
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

            // Trigger runtime subscription updates for new markets
            // Extract symbols per exchange from by_quote and update subscriptions
            // This will calculate diff and only subscribe to new markets
            use std::collections::HashSet;
            let mut binance_markets: HashSet<String> = HashSet::new();
            let mut coinbase_markets: HashSet<String> = HashSet::new();
            let mut upbit_markets: HashSet<String> = HashSet::new();
            let mut bithumb_markets: HashSet<String> = HashSet::new();
            let mut bybit_markets: HashSet<String> = HashSet::new();
            let mut gateio_markets: HashSet<String> = HashSet::new();

            for (_key, exchange_markets) in &common.by_quote {
                for (exchange, market_info) in exchange_markets {
                    match exchange.as_str() {
                        "Binance" => {
                            binance_markets.insert(market_info.symbol.to_lowercase());
                        }
                        "Coinbase" => {
                            coinbase_markets.insert(market_info.symbol.clone());
                        }
                        "Upbit" => {
                            upbit_markets.insert(market_info.symbol.clone());
                        }
                        "Bithumb" => {
                            bithumb_markets.insert(market_info.symbol.clone());
                        }
                        "Bybit" => {
                            bybit_markets.insert(market_info.symbol.clone());
                        }
                        "GateIO" => {
                            gateio_markets.insert(market_info.symbol.clone());
                        }
                        _ => {}
                    }
                }
            }

            // Update subscriptions for each exchange (diff is calculated internally)
            // Note: Actual subscription messages won't be sent until Epic 2
            // when SubscriptionBuilder implementations are added
            let binance_vec: Vec<String> = binance_markets.into_iter().collect();
            let coinbase_vec: Vec<String> = coinbase_markets.into_iter().collect();
            let upbit_vec: Vec<String> = upbit_markets.into_iter().collect();
            let bithumb_vec: Vec<String> = bithumb_markets.into_iter().collect();
            let bybit_vec: Vec<String> = bybit_markets.into_iter().collect();
            let gateio_vec: Vec<String> = gateio_markets.into_iter().collect();

            // Only log if there are new subscriptions (update_subscriptions returns count)
            if let Ok(count) = subscription_manager
                .update_subscriptions(Exchange::Binance, &binance_vec)
                .await
            {
                if count > 0 {
                    info!("📡 Binance: {} new markets queued for subscription", count);
                }
            }
            if let Ok(count) = subscription_manager
                .update_subscriptions(Exchange::Coinbase, &coinbase_vec)
                .await
            {
                if count > 0 {
                    info!("📡 Coinbase: {} new markets queued for subscription", count);
                }
            }
            if let Ok(count) = subscription_manager
                .update_subscriptions(Exchange::Upbit, &upbit_vec)
                .await
            {
                if count > 0 {
                    info!("📡 Upbit: {} new markets queued for subscription", count);
                }
            }
            if let Ok(count) = subscription_manager
                .update_subscriptions(Exchange::Bithumb, &bithumb_vec)
                .await
            {
                if count > 0 {
                    info!("📡 Bithumb: {} new markets queued for subscription", count);
                }
            }
            if let Ok(count) = subscription_manager
                .update_subscriptions(Exchange::Bybit, &bybit_vec)
                .await
            {
                if count > 0 {
                    info!("📡 Bybit: {} new markets queued for subscription", count);
                }
            }
            if let Ok(count) = subscription_manager
                .update_subscriptions(Exchange::GateIO, &gateio_vec)
                .await
            {
                if count > 0 {
                    info!("📡 GateIO: {} new markets queued for subscription", count);
                }
            }
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

    // Create shared state and price update receiver
    let (state, price_update_rx) = create_state(config);
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
            warn!(
                "Failed to fetch initial exchange rate, using default: {}",
                e
            );
        }
    }

    // Fetch initial wallet status so it's cached for new clients
    {
        let statuses = wallet_status::fetch_all_wallet_status().await;
        if !statuses.is_empty() {
            info!(
                "Initial wallet status fetched for {} exchanges",
                statuses.len()
            );
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

    // Spawn event-driven detector with price update receiver
    let detector_state = state.clone();
    let detector_broadcast = broadcast_tx.clone();
    let detector_notifier = notifier.clone();
    let detector_handle = tokio::spawn(async move {
        run_event_driven_detector(
            detector_state,
            detector_broadcast,
            detector_notifier,
            price_update_rx,
        )
        .await;
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
    // For live feeds, we also get the SubscriptionManager for runtime subscription updates
    let (feed_handles, subscription_manager): (
        Vec<tokio::task::JoinHandle<()>>,
        Option<Arc<SubscriptionManager>>,
    ) = if args.live {
        info!("📡 Using LIVE WebSocket feeds");
        let (handles, sub_mgr) = spawn_live_feeds(
            state.clone(),
            broadcast_tx.clone(),
            &symbol_mappings,
            status_notifier.clone(),
        )
        .await;
        (handles, Some(sub_mgr))
    } else {
        info!("🎮 Using SIMULATED price feeds");
        let price_state = state.clone();
        let price_broadcast = broadcast_tx.clone();
        let handles = vec![tokio::spawn(async move {
            run_price_simulator(price_state, price_broadcast).await;
        })];
        (handles, None)
    };

    // Start market discovery loop
    // Pass SubscriptionManager for runtime subscription updates (live mode only)
    let discovery_state = state.clone();
    let discovery_broadcast = broadcast_tx.clone();
    let discovery_mappings = symbol_mappings.clone();
    if let Some(sub_mgr) = subscription_manager {
        tokio::spawn(async move {
            run_market_discovery(
                discovery_state,
                discovery_broadcast,
                discovery_mappings,
                sub_mgr,
            )
            .await;
        });
    } else {
        // Simulated mode - no SubscriptionManager, use dummy version
        tokio::spawn(async move {
            run_market_discovery(
                discovery_state,
                discovery_broadcast,
                discovery_mappings,
                Arc::new(SubscriptionManager::new()),
            )
            .await;
        });
    }

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
        let (state, _price_rx) = create_state(config);

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
        let _opps = state.detect_opportunities(1).await;
        // May or may not have opportunities depending on threshold

        state.stop();
        assert!(!state.is_running());
    }
}

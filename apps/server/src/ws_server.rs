//! WebSocket server for real-time data broadcasting to clients.
//!
//! Event-driven: broadcasts data when new prices/stats/opportunities arrive.

use crate::exchange_rate;
use crate::state::SharedState;
use crate::wallet_status;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, info, warn};
use arbitrage_core::{Exchange, FixedPoint, PriceTick};

/// Price data for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsPriceData {
    pub exchange: String,
    pub symbol: String,
    pub pair_id: u32,
    pub price: f64,
    pub bid: f64,
    pub ask: f64,
    pub volume_24h: f64,
    pub timestamp: u64,
    /// Quote currency (e.g., "USDT", "USDC", "USD", "KRW")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
}

/// Stats data for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsStatsData {
    pub uptime_secs: u64,
    pub price_updates: u64,
    pub opportunities_detected: u64,
    pub trades_executed: u64,
    pub is_running: bool,
}

/// USD-like premium for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsUsdlikePremium {
    /// Premium in basis points
    pub bps: i32,
    /// Which stablecoin was used for comparison ("USDT", "USDC", "BUSD")
    pub quote: String,
}

/// Opportunity data for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsOpportunityData {
    pub id: u64,
    pub symbol: String,
    pub source_exchange: String,
    pub target_exchange: String,
    /// Quote currency at source exchange (e.g., "USDT", "USDC", "KRW")
    pub source_quote: String,
    /// Quote currency at target exchange (e.g., "USDT", "USDC", "KRW")
    pub target_quote: String,
    /// Raw premium in basis points (direct price comparison)
    pub premium_bps: i32,
    /// USD-like premium: same stablecoin comparison (USDT vs USDT or USDC vs USDC)
    /// For KRW markets, converts to overseas market's quote currency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usdlike_premium: Option<WsUsdlikePremium>,
    /// Kimchi premium: USD price comparison (KRW/USD_KRW forex vs overseas USD)
    pub kimchi_premium_bps: i32,
    pub source_price: f64,
    pub target_price: f64,
    pub net_profit_bps: i32,
    pub confidence_score: u8,
    pub timestamp: u64,
    /// Common networks available for transfer between source and target exchanges.
    /// Empty if no common network is available (opportunity not executable).
    #[serde(default)]
    pub common_networks: Vec<String>,
    /// Whether this opportunity has a viable transfer path.
    /// - true: Common network exists, transfer is possible
    /// - false: No common network, transfer not possible
    #[serde(default)]
    pub has_transfer_path: bool,
    /// Whether wallet status data is available for this opportunity.
    /// - true: Wallet status loaded, has_transfer_path is reliable
    /// - false: Wallet status not yet loaded, has_transfer_path may be inaccurate
    #[serde(default)]
    pub wallet_status_known: bool,
    /// Orderbook depth at source (ask size - quantity available to buy)
    #[serde(default)]
    pub source_depth: f64,
    /// Orderbook depth at target (bid size - quantity available to sell)
    #[serde(default)]
    pub target_depth: f64,
    /// Optimal trade size from depth walking algorithm (in base asset)
    #[serde(default)]
    pub optimal_size: f64,
    /// Expected profit at optimal_size (in quote currency, e.g., USDT)
    #[serde(default)]
    pub optimal_profit: f64,
    /// Reason for optimal_size value: "ok" | "no_orderbook" | "not_profitable"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimal_size_reason: Option<String>,
}

/// Exchange rate data for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsExchangeRateData {
    /// USDT/KRW price from Upbit (for backward compatibility & kimchi premium)
    pub usd_krw: f64,
    /// USDT/KRW from Upbit
    pub upbit_usdt_krw: f64,
    /// USDC/KRW from Upbit
    pub upbit_usdc_krw: f64,
    /// USDT/KRW from Bithumb
    pub bithumb_usdt_krw: f64,
    /// USDC/KRW from Bithumb
    pub bithumb_usdc_krw: f64,
    /// USD/KRW rate from 하나은행 API (for kimchi premium)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_rate: Option<f64>,
    /// USDT/USD price from exchange feed
    pub usdt_usd: f64,
    /// USDC/USD price (calculated from USDC/USDT * USDT/USD)
    pub usdc_usd: f64,
    pub timestamp: u64,
}

/// Market info for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsMarketInfo {
    pub base: String,
    pub symbol: String,
    pub exchange: String,
}

/// Common markets data for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsCommonMarketsData {
    /// List of common base assets
    pub common_bases: Vec<String>,
    /// Markets by base asset
    pub markets: std::collections::HashMap<String, Vec<WsMarketInfo>>,
    /// Exchanges that were compared
    pub exchanges: Vec<String>,
    pub timestamp: u64,
}

/// Network status for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsNetworkStatus {
    pub network: String,
    pub name: String,
    pub deposit_enabled: bool,
    pub withdraw_enabled: bool,
    pub min_withdraw: f64,
    pub withdraw_fee: f64,
    pub confirms_required: u32,
}

/// Asset wallet status for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsAssetWalletStatus {
    pub asset: String,
    pub name: String,
    pub networks: Vec<WsNetworkStatus>,
    pub can_deposit: bool,
    pub can_withdraw: bool,
}

/// Exchange wallet status for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsExchangeWalletStatus {
    pub exchange: String,
    pub wallet_status: Vec<WsAssetWalletStatus>,
    pub last_updated: u64,
}

/// Wallet status data for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsWalletStatusData {
    pub exchanges: Vec<WsExchangeWalletStatus>,
    pub timestamp: u64,
}

/// WebSocket message types.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum WsServerMessage {
    /// Single price update (event-driven)
    #[serde(rename = "price")]
    Price(WsPriceData),
    /// Batch of prices (for initial sync)
    #[serde(rename = "prices")]
    Prices(Vec<WsPriceData>),
    #[serde(rename = "stats")]
    Stats(WsStatsData),
    #[serde(rename = "opportunity")]
    Opportunity(WsOpportunityData),
    /// Batch of opportunities (for initial sync)
    #[serde(rename = "opportunities")]
    Opportunities(Vec<WsOpportunityData>),
    #[serde(rename = "exchange_rate")]
    ExchangeRate(WsExchangeRateData),
    /// Common markets across exchanges
    #[serde(rename = "common_markets")]
    CommonMarkets(WsCommonMarketsData),
    /// Wallet status for deposit/withdraw
    #[serde(rename = "wallet_status")]
    WalletStatus(WsWalletStatusData),
}

/// Broadcast channel sender.
pub type BroadcastSender = broadcast::Sender<WsServerMessage>;

/// WebSocket server state.
pub struct WsServerState {
    pub app_state: SharedState,
    pub broadcast_tx: BroadcastSender,
}

/// Create WebSocket server router.
pub fn create_ws_router(state: Arc<WsServerState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health_handler))
        .layer(cors)
        .with_state(state)
}

/// Health check handler.
async fn health_handler() -> &'static str {
    "OK"
}

/// WebSocket upgrade handler.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection.
async fn handle_socket(socket: WebSocket, state: Arc<WsServerState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    debug!("WebSocket client connected");

    // Send initial data
    let initial_prices = collect_prices(&state.app_state).await;
    let initial_stats = collect_stats(&state.app_state);
    let initial_rate = collect_exchange_rate(&state.app_state);
    let initial_opportunities = collect_opportunities(&state.app_state).await;
    let initial_common_markets = collect_common_markets(&state.app_state).await;
    let initial_wallet_status = collect_wallet_status();

    if let Ok(json) = serde_json::to_string(&WsServerMessage::Prices(initial_prices)) {
        let _ = sender.send(Message::Text(json)).await;
    }
    if let Ok(json) = serde_json::to_string(&WsServerMessage::Stats(initial_stats)) {
        let _ = sender.send(Message::Text(json)).await;
    }
    if let Some(rate) = initial_rate {
        if let Ok(json) = serde_json::to_string(&WsServerMessage::ExchangeRate(rate)) {
            let _ = sender.send(Message::Text(json)).await;
        }
    }
    if !initial_opportunities.is_empty() {
        if let Ok(json) = serde_json::to_string(&WsServerMessage::Opportunities(initial_opportunities)) {
            let _ = sender.send(Message::Text(json)).await;
        }
    }
    if let Some(common_markets) = initial_common_markets {
        if let Ok(json) = serde_json::to_string(&WsServerMessage::CommonMarkets(common_markets)) {
            let _ = sender.send(Message::Text(json)).await;
        }
    }
    if let Some(wallet_status) = initial_wallet_status {
        if let Ok(json) = serde_json::to_string(&WsServerMessage::WalletStatus(wallet_status)) {
            let _ = sender.send(Message::Text(json)).await;
        }
    }

    // Spawn task to send broadcast messages to this client
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = broadcast_rx.recv() => {
                    match msg {
                        Ok(ws_msg) => {
                            if let Ok(json) = serde_json::to_string(&ws_msg) {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // Skip lagged messages
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Handle incoming messages (ping/pong, close)
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Ping(data)) => {
                // Pong is handled automatically by axum
                let _ = data;
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                warn!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Abort send task when connection closes
    send_task.abort();
    debug!("WebSocket client disconnected");
}

/// Collect current prices from state.
async fn collect_prices(state: &SharedState) -> Vec<WsPriceData> {
    let mut prices = Vec::new();

    // Get all prices from aggregator
    let all_ticks = state.prices.get_all_prices();

    // Get symbol registry from detector for pair_id -> symbol mapping
    let detector = state.detector.read().await;

    for tick in all_ticks {
        // Get symbol from registry, or compute from pair_id
        let symbol = detector.pair_id_to_symbol(tick.pair_id())
            .unwrap_or_else(|| format!("PAIR_{}", tick.pair_id()));

        // Get quote currency from tick
        let quote = Some(format!("{:?}", tick.quote_currency()));

        prices.push(WsPriceData {
            exchange: format!("{:?}", tick.exchange()),
            symbol,
            pair_id: tick.pair_id(),
            price: tick.price().to_f64(),
            bid: tick.bid().to_f64(),
            ask: tick.ask().to_f64(),
            volume_24h: tick.volume_24h().to_f64(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            quote,
        });
    }

    prices
}

/// Collect current stats from state.
fn collect_stats(state: &SharedState) -> WsStatsData {
    let summary = state.stats_summary();
    WsStatsData {
        uptime_secs: summary.uptime_secs,
        price_updates: summary.price_updates,
        opportunities_detected: summary.opportunities_detected,
        trades_executed: summary.trades_executed,
        is_running: state.is_running(),
    }
}

/// Collect current exchange rate if loaded.
fn collect_exchange_rate(state: &SharedState) -> Option<WsExchangeRateData> {
    let upbit_usdt_krw = state.get_upbit_usdt_krw().map(|p| p.to_f64()).unwrap_or(0.0);
    let upbit_usdc_krw = state.get_upbit_usdc_krw().map(|p| p.to_f64()).unwrap_or(0.0);
    let bithumb_usdt_krw = state.get_bithumb_usdt_krw().map(|p| p.to_f64()).unwrap_or(0.0);
    let bithumb_usdc_krw = state.get_bithumb_usdc_krw().map(|p| p.to_f64()).unwrap_or(0.0);

    exchange_rate::get_usd_krw_rate().map(|usd_krw| WsExchangeRateData {
        usd_krw,
        upbit_usdt_krw,
        upbit_usdc_krw,
        bithumb_usdt_krw,
        bithumb_usdc_krw,
        api_rate: exchange_rate::get_api_rate(),
        usdt_usd: state.get_usdt_usd_price().to_f64(),
        usdc_usd: state.get_usdc_usd_price().to_f64(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    })
}

/// Collect cached wallet status for initial sync.
fn collect_wallet_status() -> Option<WsWalletStatusData> {
    let cached = wallet_status::get_cached_wallet_status();
    if cached.is_empty() {
        return None;
    }

    let exchanges: Vec<WsExchangeWalletStatus> = cached
        .into_iter()
        .map(|s| WsExchangeWalletStatus {
            exchange: s.exchange,
            wallet_status: s.wallet_status
                .into_iter()
                .map(|ws| WsAssetWalletStatus {
                    asset: ws.asset,
                    name: ws.name,
                    networks: ws.networks
                        .into_iter()
                        .map(|n| WsNetworkStatus {
                            network: n.network,
                            name: n.name,
                            deposit_enabled: n.deposit_enabled,
                            withdraw_enabled: n.withdraw_enabled,
                            min_withdraw: n.min_withdraw,
                            withdraw_fee: n.withdraw_fee,
                            confirms_required: n.confirms_required,
                        })
                        .collect(),
                    can_deposit: ws.can_deposit,
                    can_withdraw: ws.can_withdraw,
                })
                .collect(),
            last_updated: s.last_updated,
        })
        .collect();

    Some(WsWalletStatusData {
        exchanges,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    })
}

/// Collect current opportunities from state.
async fn collect_opportunities(state: &SharedState) -> Vec<WsOpportunityData> {
    let opps = state.opportunities.read().await;

    let result: Vec<WsOpportunityData> = opps.iter().map(|opp| {
        let source_ex = format!("{:?}", opp.source_exchange);
        let target_ex = format!("{:?}", opp.target_exchange);
        let symbol = opp.asset.symbol.to_string();

        // Check if wallet status is known for these exchanges
        let wallet_status_known = wallet_status::is_wallet_status_known(&source_ex, &target_ex);

        // Find common networks for this opportunity
        let (common_networks, _, _) = wallet_status::find_common_networks_for_asset(
            &symbol,
            &source_ex,
            &target_ex,
        );
        let has_transfer_path = !common_networks.is_empty();

        // Get latest depth from cache (source = ask_size for buying, target = bid_size for selling)
        let source_depth = state.get_depth(&source_ex, &symbol)
            .map(|(_, ask_size)| ask_size.to_f64())
            .unwrap_or_else(|| FixedPoint(opp.source_depth).to_f64());
        let target_depth = state.get_depth(&target_ex, &symbol)
            .map(|(bid_size, _)| bid_size.to_f64())
            .unwrap_or_else(|| FixedPoint(opp.target_depth).to_f64());

        // Convert UsdlikePremium to WsUsdlikePremium
        let usdlike_premium = opp.usdlike_premium.map(|p| WsUsdlikePremium {
            bps: p.bps,
            quote: p.quote.as_str().to_string(),
        });

        WsOpportunityData {
            id: opp.id,
            symbol,
            source_exchange: source_ex,
            target_exchange: target_ex,
            source_quote: opp.source_quote.as_str().to_string(),
            target_quote: opp.target_quote.as_str().to_string(),
            premium_bps: opp.premium_bps,
            usdlike_premium,
            kimchi_premium_bps: opp.kimchi_premium_bps,
            source_price: FixedPoint(opp.source_price).to_f64(),
            target_price: FixedPoint(opp.target_price).to_f64(),
            net_profit_bps: (opp.net_profit_estimate / 100) as i32,
            confidence_score: opp.confidence_score,
            timestamp: opp.discovered_at_ms,
            common_networks,
            has_transfer_path,
            wallet_status_known,
            source_depth,
            target_depth,
            optimal_size: FixedPoint(opp.optimal_size).to_f64(),
            optimal_profit: FixedPoint(opp.optimal_profit as u64).to_f64(),
            optimal_size_reason: Some(match opp.optimal_size_reason {
                arbitrage_core::OptimalSizeReason::Ok => "ok".to_string(),
                arbitrage_core::OptimalSizeReason::NoOrderbook => "no_orderbook".to_string(),
                arbitrage_core::OptimalSizeReason::NotProfitable => "not_profitable".to_string(),
                arbitrage_core::OptimalSizeReason::NoConversionRate => "no_conversion_rate".to_string(),
            }),
        }
    }).collect();

    result
}

/// Collect current common markets from state.
async fn collect_common_markets(state: &SharedState) -> Option<WsCommonMarketsData> {
    let common = state.get_common_markets().await?;

    let mut markets_map = std::collections::HashMap::new();

    for (base, exchange_markets) in &common.common {
        let ws_markets: Vec<WsMarketInfo> = exchange_markets
            .iter()
            .map(|(ex, info)| WsMarketInfo {
                base: info.base.clone(),
                symbol: info.symbol.clone(),
                exchange: ex.clone(),
            })
            .collect();
        markets_map.insert(base.clone(), ws_markets);
    }

    Some(WsCommonMarketsData {
        common_bases: common.common_bases(),
        markets: markets_map,
        exchanges: common.exchanges.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    })
}

/// Broadcast a single price update (event-driven).
pub fn broadcast_price(tx: &BroadcastSender, exchange: Exchange, pair_id: u32, symbol: &str, tick: &PriceTick) {
    broadcast_price_with_quote(tx, exchange, pair_id, symbol, None, tick);
}

/// Broadcast a single price update with quote currency (event-driven).
pub fn broadcast_price_with_quote(tx: &BroadcastSender, exchange: Exchange, pair_id: u32, symbol: &str, quote: Option<&str>, tick: &PriceTick) {
    let price_data = WsPriceData {
        exchange: format!("{:?}", exchange),
        symbol: symbol.to_string(),
        pair_id,
        price: tick.price().to_f64(),
        bid: tick.bid().to_f64(),
        ask: tick.ask().to_f64(),
        volume_24h: tick.volume_24h().to_f64(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        quote: quote.map(|q| q.to_string()),
    };

    let _ = tx.send(WsServerMessage::Price(price_data));
}

/// Broadcast stats update.
pub fn broadcast_stats(tx: &BroadcastSender, state: &SharedState) {
    let stats = collect_stats(state);
    let _ = tx.send(WsServerMessage::Stats(stats));
}

/// Broadcast a new opportunity to all clients.
pub fn broadcast_opportunity(tx: &BroadcastSender, state: &SharedState, opp: &arbitrage_core::ArbitrageOpportunity) {
    let source_ex = format!("{:?}", opp.source_exchange);
    let target_ex = format!("{:?}", opp.target_exchange);
    let symbol = opp.asset.symbol.to_string();

    // Check if wallet status is known for these exchanges
    let wallet_status_known = wallet_status::is_wallet_status_known(&source_ex, &target_ex);

    // Find common networks for this opportunity
    let (common_networks, _, _) = wallet_status::find_common_networks_for_asset(
        &symbol,
        &source_ex,
        &target_ex,
    );
    let has_transfer_path = !common_networks.is_empty();

    // Get latest depth from cache (source = ask_size for buying, target = bid_size for selling)
    let source_depth_cache = state.get_depth(&source_ex, &symbol);
    let target_depth_cache = state.get_depth(&target_ex, &symbol);

    let source_depth = source_depth_cache
        .map(|(_, ask_size)| ask_size.to_f64())
        .unwrap_or_else(|| FixedPoint(opp.source_depth).to_f64());
    let target_depth = target_depth_cache
        .map(|(bid_size, _)| bid_size.to_f64())
        .unwrap_or_else(|| FixedPoint(opp.target_depth).to_f64());

    // Convert UsdlikePremium to WsUsdlikePremium
    let usdlike_premium = opp.usdlike_premium.map(|p| WsUsdlikePremium {
        bps: p.bps,
        quote: p.quote.as_str().to_string(),
    });

    let optimal_size_f64 = FixedPoint(opp.optimal_size).to_f64();
    let optimal_profit_f64 = FixedPoint(opp.optimal_profit as u64).to_f64();

    let ws_opp = WsOpportunityData {
        id: opp.id,
        symbol,
        source_exchange: source_ex,
        target_exchange: target_ex,
        source_quote: opp.source_quote.as_str().to_string(),
        target_quote: opp.target_quote.as_str().to_string(),
        premium_bps: opp.premium_bps,
        usdlike_premium,
        kimchi_premium_bps: opp.kimchi_premium_bps,
        source_price: FixedPoint(opp.source_price).to_f64(),
        target_price: FixedPoint(opp.target_price).to_f64(),
        net_profit_bps: (opp.net_profit_estimate / 100) as i32, // Convert to bps approximation
        confidence_score: opp.confidence_score,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        common_networks,
        has_transfer_path,
        wallet_status_known,
        source_depth,
        target_depth,
        optimal_size: optimal_size_f64,
        optimal_profit: optimal_profit_f64,
        optimal_size_reason: Some(match opp.optimal_size_reason {
            arbitrage_core::OptimalSizeReason::Ok => "ok".to_string(),
            arbitrage_core::OptimalSizeReason::NoOrderbook => "no_orderbook".to_string(),
            arbitrage_core::OptimalSizeReason::NotProfitable => "not_profitable".to_string(),
            arbitrage_core::OptimalSizeReason::NoConversionRate => "no_conversion_rate".to_string(),
        }),
    };

    let _ = tx.send(WsServerMessage::Opportunity(ws_opp));
}

/// Broadcast exchange rate update to all clients.
pub fn broadcast_exchange_rate(tx: &BroadcastSender, state: &SharedState, usd_krw: f64) {
    let upbit_usdt_krw = state.get_upbit_usdt_krw().map(|p| p.to_f64()).unwrap_or(0.0);
    let upbit_usdc_krw = state.get_upbit_usdc_krw().map(|p| p.to_f64()).unwrap_or(0.0);
    let bithumb_usdt_krw = state.get_bithumb_usdt_krw().map(|p| p.to_f64()).unwrap_or(0.0);
    let bithumb_usdc_krw = state.get_bithumb_usdc_krw().map(|p| p.to_f64()).unwrap_or(0.0);

    let rate_data = WsExchangeRateData {
        usd_krw,
        upbit_usdt_krw,
        upbit_usdc_krw,
        bithumb_usdt_krw,
        bithumb_usdc_krw,
        api_rate: exchange_rate::get_api_rate(),
        usdt_usd: state.get_usdt_usd_price().to_f64(),
        usdc_usd: state.get_usdc_usd_price().to_f64(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };

    let _ = tx.send(WsServerMessage::ExchangeRate(rate_data));
}

/// Broadcast common markets to all clients.
pub fn broadcast_common_markets(
    tx: &BroadcastSender,
    common: &arbitrage_feeds::CommonMarkets,
) {
    let mut markets_map = std::collections::HashMap::new();

    for (base, exchange_markets) in &common.common {
        let ws_markets: Vec<WsMarketInfo> = exchange_markets
            .iter()
            .map(|(ex, info)| WsMarketInfo {
                base: info.base.clone(),
                symbol: info.symbol.clone(),
                exchange: ex.clone(),
            })
            .collect();
        markets_map.insert(base.clone(), ws_markets);
    }

    let data = WsCommonMarketsData {
        common_bases: common.common_bases(),
        markets: markets_map,
        exchanges: common.exchanges.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };

    let _ = tx.send(WsServerMessage::CommonMarkets(data));
}

/// Broadcast wallet status to all clients.
pub fn broadcast_wallet_status(
    tx: &BroadcastSender,
    statuses: Vec<wallet_status::ExchangeWalletStatus>,
) {
    let exchanges: Vec<WsExchangeWalletStatus> = statuses
        .into_iter()
        .map(|s| WsExchangeWalletStatus {
            exchange: s.exchange,
            wallet_status: s.wallet_status
                .into_iter()
                .map(|ws| WsAssetWalletStatus {
                    asset: ws.asset,
                    name: ws.name,
                    networks: ws.networks
                        .into_iter()
                        .map(|n| WsNetworkStatus {
                            network: n.network,
                            name: n.name,
                            deposit_enabled: n.deposit_enabled,
                            withdraw_enabled: n.withdraw_enabled,
                            min_withdraw: n.min_withdraw,
                            withdraw_fee: n.withdraw_fee,
                            confirms_required: n.confirms_required,
                        })
                        .collect(),
                    can_deposit: ws.can_deposit,
                    can_withdraw: ws.can_withdraw,
                })
                .collect(),
            last_updated: s.last_updated,
        })
        .collect();

    let data = WsWalletStatusData {
        exchanges,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };

    let _ = tx.send(WsServerMessage::WalletStatus(data));
}

/// Create WebSocket server and return the broadcast sender for event-driven updates.
/// The caller should use the returned sender to broadcast price/stats/opportunity updates.
pub fn create_ws_server(state: SharedState) -> (Router, BroadcastSender) {
    let (broadcast_tx, _) = broadcast::channel::<WsServerMessage>(1000);

    let ws_state = Arc::new(WsServerState {
        app_state: state,
        broadcast_tx: broadcast_tx.clone(),
    });

    let app = create_ws_router(ws_state);
    (app, broadcast_tx)
}

/// Start the WebSocket server and return the broadcast sender.
pub async fn start_ws_server(
    state: SharedState,
    port: u16,
) -> Result<BroadcastSender, Box<dyn std::error::Error + Send + Sync>> {
    let (app, broadcast_tx) = create_ws_server(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!("WebSocket server listening on ws://0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Spawn server in background and return the broadcast sender
    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("WebSocket server error: {}", e);
        }
    });

    Ok(tx_clone)
}

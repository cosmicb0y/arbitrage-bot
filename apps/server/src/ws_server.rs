//! WebSocket server for real-time data broadcasting to clients.
//!
//! Event-driven: broadcasts data when new prices/stats/opportunities arrive.

use crate::exchange_rate;
use crate::state::SharedState;
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
use tracing::{info, warn};
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
    pub timestamp: u64,
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

/// Opportunity data for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsOpportunityData {
    pub id: u64,
    pub symbol: String,
    pub source_exchange: String,
    pub target_exchange: String,
    pub premium_bps: i32,
    pub source_price: f64,
    pub target_price: f64,
    pub net_profit_bps: i32,
    pub confidence_score: u8,
    pub timestamp: u64,
}

/// Exchange rate data for WebSocket broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct WsExchangeRateData {
    /// USDT/KRW price from Upbit
    pub usd_krw: f64,
    /// USD/KRW rate from exchange rate API (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_rate: Option<f64>,
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

    info!("WebSocket client connected");

    // Send initial data
    let initial_prices = collect_prices(&state.app_state).await;
    let initial_stats = collect_stats(&state.app_state);
    let initial_rate = collect_exchange_rate();
    let initial_opportunities = collect_opportunities(&state.app_state).await;
    let initial_common_markets = collect_common_markets(&state.app_state).await;

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
    info!("WebSocket client disconnected");
}

/// Collect current prices from state.
async fn collect_prices(state: &SharedState) -> Vec<WsPriceData> {
    let symbols = ["BTC", "ETH", "SOL"];
    let mut prices = Vec::new();

    // Get all prices from aggregator
    for pair_id in 1..=3u32 {
        let symbol = symbols.get(pair_id as usize - 1).unwrap_or(&"UNKNOWN");

        for exchange in arbitrage_core::Exchange::all_cex() {
            if let Some(tick) = state.prices.get_price(*exchange, pair_id) {
                prices.push(WsPriceData {
                    exchange: format!("{:?}", exchange),
                    symbol: symbol.to_string(),
                    pair_id,
                    price: tick.price().to_f64(),
                    bid: tick.bid().to_f64(),
                    ask: tick.ask().to_f64(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                });
            }
        }
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
fn collect_exchange_rate() -> Option<WsExchangeRateData> {
    exchange_rate::get_usd_krw_rate().map(|usd_krw| WsExchangeRateData {
        usd_krw,
        api_rate: exchange_rate::get_api_rate(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    })
}

/// Collect current opportunities from state.
async fn collect_opportunities(state: &SharedState) -> Vec<WsOpportunityData> {
    let opps = state.opportunities.read().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    opps.iter().map(|opp| WsOpportunityData {
        id: opp.id,
        symbol: opp.asset.symbol.to_string(),
        source_exchange: format!("{:?}", opp.source_exchange),
        target_exchange: format!("{:?}", opp.target_exchange),
        premium_bps: opp.premium_bps,
        source_price: FixedPoint(opp.source_price).to_f64(),
        target_price: FixedPoint(opp.target_price).to_f64(),
        net_profit_bps: (opp.net_profit_estimate / 100) as i32,
        confidence_score: opp.confidence_score,
        timestamp: now,
    }).collect()
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
    let price_data = WsPriceData {
        exchange: format!("{:?}", exchange),
        symbol: symbol.to_string(),
        pair_id,
        price: tick.price().to_f64(),
        bid: tick.bid().to_f64(),
        ask: tick.ask().to_f64(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };

    let _ = tx.send(WsServerMessage::Price(price_data));
}

/// Broadcast stats update.
pub fn broadcast_stats(tx: &BroadcastSender, state: &SharedState) {
    let stats = collect_stats(state);
    let _ = tx.send(WsServerMessage::Stats(stats));
}

/// Broadcast a new opportunity to all clients.
pub fn broadcast_opportunity(tx: &BroadcastSender, opp: &arbitrage_core::ArbitrageOpportunity) {
    let ws_opp = WsOpportunityData {
        id: opp.id,
        symbol: opp.asset.symbol.to_string(),
        source_exchange: format!("{:?}", opp.source_exchange),
        target_exchange: format!("{:?}", opp.target_exchange),
        premium_bps: opp.premium_bps,
        source_price: FixedPoint(opp.source_price).to_f64(),
        target_price: FixedPoint(opp.target_price).to_f64(),
        net_profit_bps: (opp.net_profit_estimate / 100) as i32, // Convert to bps approximation
        confidence_score: opp.confidence_score,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };

    let _ = tx.send(WsServerMessage::Opportunity(ws_opp));
}

/// Broadcast exchange rate update to all clients.
pub fn broadcast_exchange_rate(tx: &BroadcastSender, usd_krw: f64) {
    let rate_data = WsExchangeRateData {
        usd_krw,
        api_rate: exchange_rate::get_api_rate(),
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

//! Application state management for the desktop app.
//!
//! Connects to CLI server via WebSocket to receive real-time data.

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

/// Price data from CLI server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub exchange: String,
    pub symbol: String,
    pub pair_id: u32,
    pub price: f64,
    pub bid: f64,
    pub ask: f64,
    #[serde(default)]
    pub volume_24h: f64,
    pub timestamp: u64,
    /// Quote currency (e.g., "USDT", "USDC", "USD", "KRW")
    #[serde(default)]
    pub quote: Option<String>,
}

/// Bot statistics from CLI server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotStats {
    pub uptime_secs: u64,
    pub price_updates: u64,
    pub opportunities_detected: u64,
    pub trades_executed: u64,
    pub is_running: bool,
}

impl Default for BotStats {
    fn default() -> Self {
        Self {
            uptime_secs: 0,
            price_updates: 0,
            opportunities_detected: 0,
            trades_executed: 0,
            is_running: false,
        }
    }
}

/// USD-like stablecoin premium data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdlikePremium {
    /// Premium in basis points
    pub bps: i32,
    /// Which stablecoin was used for comparison ("USDT", "USDC", "BUSD")
    pub quote: String,
}

/// Opportunity data from CLI server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityData {
    pub id: u64,
    pub symbol: String,
    pub source_exchange: String,
    pub target_exchange: String,
    /// Quote currency at source exchange (e.g., "USDT", "USDC", "KRW")
    #[serde(default)]
    pub source_quote: String,
    /// Quote currency at target exchange (e.g., "USDT", "USDC", "KRW")
    #[serde(default)]
    pub target_quote: String,
    /// Raw premium in basis points (direct price comparison)
    pub premium_bps: i32,
    /// USD-like premium: same stablecoin comparison (USDT vs USDT or USDC vs USDC)
    #[serde(default)]
    pub usdlike_premium: Option<UsdlikePremium>,
    /// Kimchi premium: KRW converted via USD/KRW rate
    #[serde(default)]
    pub kimchi_premium_bps: i32,
    pub source_price: f64,
    pub target_price: f64,
    pub net_profit_bps: i32,
    pub confidence_score: u8,
    pub timestamp: u64,
    /// Common networks available for transfer (canonical names)
    #[serde(default)]
    pub common_networks: Vec<String>,
    /// Whether this opportunity has a viable transfer path
    #[serde(default)]
    pub has_transfer_path: bool,
    /// Whether wallet status data is available for this opportunity
    #[serde(default)]
    pub wallet_status_known: bool,
    /// Orderbook depth at source (ask size - quantity available to buy)
    #[serde(default)]
    pub source_depth: Option<f64>,
    /// Orderbook depth at target (bid size - quantity available to sell)
    #[serde(default)]
    pub target_depth: Option<f64>,
    /// Optimal trade size from depth walking algorithm (in base asset)
    #[serde(default)]
    pub optimal_size: f64,
    /// Expected profit at optimal_size (in quote currency, e.g., USDT)
    #[serde(default)]
    pub optimal_profit: f64,
    /// Reason for optimal_size value: "ok" | "no_orderbook" | "not_profitable"
    #[serde(default)]
    pub optimal_size_reason: Option<String>,
    /// Raw price from source exchange in original quote currency (e.g., KRW for Korean exchanges)
    #[serde(default)]
    pub source_raw_price: f64,
    /// Raw price from target exchange in original quote currency
    #[serde(default)]
    pub target_raw_price: f64,
}

/// Execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub mode: String,
    pub min_premium_bps: i32,
    pub max_slippage_bps: u16,
    pub dry_run: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            mode: "alert".to_string(),
            min_premium_bps: 30,
            max_slippage_bps: 50,
            dry_run: true,
        }
    }
}

/// Exchange rate data from CLI server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateData {
    pub usd_krw: f64,
    /// USDT/KRW from Upbit
    #[serde(default)]
    pub upbit_usdt_krw: f64,
    /// USDT/KRW from Bithumb
    #[serde(default)]
    pub bithumb_usdt_krw: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_rate: Option<f64>,
    /// USDT/USD price
    #[serde(default = "default_usdt_usd")]
    pub usdt_usd: f64,
    /// USDC/USD price
    #[serde(default = "default_usdc_usd")]
    pub usdc_usd: f64,
    pub timestamp: u64,
}

fn default_usdt_usd() -> f64 { 1.0 }
fn default_usdc_usd() -> f64 { 1.0 }

/// Market info for a single exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInfo {
    pub base: String,
    pub symbol: String,
    pub exchange: String,
}

/// Common markets data from CLI server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonMarketsData {
    pub common_bases: Vec<String>,
    pub markets: std::collections::HashMap<String, Vec<MarketInfo>>,
    pub exchanges: Vec<String>,
    pub timestamp: u64,
}

/// Network status for deposit/withdraw.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub network: String,
    pub name: String,
    pub deposit_enabled: bool,
    pub withdraw_enabled: bool,
    pub min_withdraw: f64,
    pub withdraw_fee: f64,
    pub confirms_required: u32,
}

/// Asset wallet status from CLI server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetWalletStatus {
    pub asset: String,
    pub name: String,
    pub networks: Vec<NetworkStatus>,
    pub can_deposit: bool,
    pub can_withdraw: bool,
}

/// Exchange wallet status from CLI server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeWalletStatus {
    pub exchange: String,
    pub wallet_status: Vec<AssetWalletStatus>,
    pub last_updated: u64,
}

/// Wallet status data from CLI server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletStatusData {
    pub exchanges: Vec<ExchangeWalletStatus>,
    pub timestamp: u64,
}

/// Premium entry for a single exchange pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumEntry {
    pub buy_exchange: String,
    pub sell_exchange: String,
    pub buy_quote: String,
    pub sell_quote: String,
    pub tether_premium_bps: i32,
    pub kimchi_premium_bps: i32,
}

/// Premium matrix for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumMatrixData {
    pub symbol: String,
    pub pair_id: u32,
    pub entries: Vec<PremiumEntry>,
    pub timestamp: u64,
}

/// WebSocket message types from CLI server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsServerMessage {
    /// Single price update (event-driven)
    #[serde(rename = "price")]
    Price(PriceData),
    /// Batch of prices (for initial sync)
    #[serde(rename = "prices")]
    Prices(Vec<PriceData>),
    #[serde(rename = "stats")]
    Stats(BotStats),
    #[serde(rename = "opportunity")]
    Opportunity(OpportunityData),
    /// Batch of opportunities (for initial sync)
    #[serde(rename = "opportunities")]
    Opportunities(Vec<OpportunityData>),
    #[serde(rename = "exchange_rate")]
    ExchangeRate(ExchangeRateData),
    /// Common markets across exchanges
    #[serde(rename = "common_markets")]
    CommonMarkets(CommonMarketsData),
    /// Wallet status for deposit/withdraw
    #[serde(rename = "wallet_status")]
    WalletStatus(WalletStatusData),
    /// Premium matrix for a symbol (all exchange pairs)
    #[serde(rename = "premium_matrix")]
    PremiumMatrix(PremiumMatrixData),
}

/// Application state shared across commands.
pub struct AppState {
    /// Connected to CLI server
    connected: AtomicBool,
    /// CLI server WebSocket URL
    server_url: std::sync::RwLock<String>,
    /// Cached prices from CLI server (key: "exchange:symbol")
    prices: DashMap<String, PriceData>,
    /// Cached stats from CLI server
    stats: std::sync::RwLock<BotStats>,
    /// Recent opportunities
    opportunities: std::sync::RwLock<Vec<OpportunityData>>,
    /// Execution config
    config: std::sync::RwLock<ExecutionConfig>,
    /// Exchange rate
    exchange_rate: std::sync::RwLock<Option<ExchangeRateData>>,
    /// Common markets
    common_markets: std::sync::RwLock<Option<CommonMarketsData>>,
    /// Wallet status for deposit/withdraw
    wallet_status: std::sync::RwLock<Option<WalletStatusData>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connected: AtomicBool::new(false),
            server_url: std::sync::RwLock::new("ws://127.0.0.1:9001/ws".to_string()),
            prices: DashMap::new(),
            stats: std::sync::RwLock::new(BotStats::default()),
            opportunities: std::sync::RwLock::new(Vec::new()),
            config: std::sync::RwLock::new(ExecutionConfig::default()),
            exchange_rate: std::sync::RwLock::new(None),
            common_markets: std::sync::RwLock::new(None),
            wallet_status: std::sync::RwLock::new(None),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn set_connected(&self, connected: bool) {
        self.connected.store(connected, Ordering::SeqCst);
    }

    pub fn set_server_url(&self, url: String) {
        *self.server_url.write().unwrap() = url;
    }

    pub fn get_server_url(&self) -> String {
        self.server_url.read().unwrap().clone()
    }

    pub fn update_price(&self, price: PriceData) {
        // Key includes quote to differentiate USDT vs USDC markets for same exchange/symbol
        let quote = price.quote.as_deref().unwrap_or("USD");
        let key = format!("{}:{}:{}", price.exchange, price.symbol, quote);
        self.prices.insert(key, price);
    }

    pub fn update_prices(&self, prices: Vec<PriceData>) {
        for price in prices {
            self.update_price(price);
        }
    }

    pub fn get_prices(&self) -> Vec<PriceData> {
        self.prices.iter().map(|r| r.value().clone()).collect()
    }

    pub fn update_stats(&self, stats: BotStats) {
        *self.stats.write().unwrap() = stats;
    }

    pub fn get_stats(&self) -> BotStats {
        self.stats.read().unwrap().clone()
    }

    pub fn add_opportunity(&self, opp: OpportunityData) {
        let mut opps = self.opportunities.write().unwrap();
        // Deduplicate by exchange pair
        let exists = opps.iter().position(|o| {
            o.symbol == opp.symbol
                && o.source_exchange == opp.source_exchange
                && o.target_exchange == opp.target_exchange
        });
        if let Some(idx) = exists {
            opps[idx] = opp;
        } else {
            opps.insert(0, opp);
            // Keep only last 50
            if opps.len() > 50 {
                opps.truncate(50);
            }
        }
    }

    pub fn set_opportunities(&self, opportunities: Vec<OpportunityData>) {
        *self.opportunities.write().unwrap() = opportunities;
    }

    pub fn get_opportunities(&self) -> Vec<OpportunityData> {
        self.opportunities.read().unwrap().clone()
    }

    pub fn get_config(&self) -> ExecutionConfig {
        self.config.read().unwrap().clone()
    }

    pub fn update_config(&self, config: ExecutionConfig) {
        *self.config.write().unwrap() = config;
    }

    pub fn update_exchange_rate(&self, rate: ExchangeRateData) {
        *self.exchange_rate.write().unwrap() = Some(rate);
    }

    pub fn get_exchange_rate(&self) -> Option<ExchangeRateData> {
        self.exchange_rate.read().unwrap().clone()
    }

    pub fn update_common_markets(&self, markets: CommonMarketsData) {
        *self.common_markets.write().unwrap() = Some(markets);
    }

    pub fn get_common_markets(&self) -> Option<CommonMarketsData> {
        self.common_markets.read().unwrap().clone()
    }

    pub fn update_wallet_status(&self, status: WalletStatusData) {
        *self.wallet_status.write().unwrap() = Some(status);
    }

    pub fn get_wallet_status(&self) -> Option<WalletStatusData> {
        self.wallet_status.read().unwrap().clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Connect to CLI server WebSocket and receive real-time data.
pub async fn run_server_connection(state: Arc<AppState>, app: AppHandle) {
    info!("Starting CLI server connection loop");

    loop {
        let url = state.get_server_url();
        info!("Connecting to CLI server: {}", url);

        match connect_to_server(&state, &app, &url).await {
            Ok(_) => {
                info!("Disconnected from CLI server, reconnecting...");
            }
            Err(e) => {
                warn!("CLI server connection error: {}", e);
            }
        }

        state.set_connected(false);

        // Wait before reconnecting
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn connect_to_server(
    state: &Arc<AppState>,
    app: &AppHandle,
    url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    state.set_connected(true);
    info!("Connected to CLI server");

    // Emit connection status
    let _ = app.emit("server_connected", true);

    while let Some(msg_result) = read.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                if let Ok(ws_msg) = serde_json::from_str::<WsServerMessage>(&text) {
                    match ws_msg {
                        WsServerMessage::Price(price) => {
                            let _ = app.emit("price", &price);
                            state.update_price(price);
                        }
                        WsServerMessage::Prices(prices) => {
                            state.update_prices(prices);
                            let _ = app.emit("price_update", state.get_prices());
                        }
                        WsServerMessage::Stats(stats) => {
                            let _ = app.emit("stats", &stats);
                            state.update_stats(stats);
                        }
                        WsServerMessage::Opportunity(opp) => {
                            let _ = app.emit("new_opportunity", &opp);
                            state.add_opportunity(opp);
                        }
                        WsServerMessage::Opportunities(opps) => {
                            let _ = app.emit("opportunities", &opps);
                            state.set_opportunities(opps);
                        }
                        WsServerMessage::ExchangeRate(rate) => {
                            let _ = app.emit("exchange_rate", &rate);
                            state.update_exchange_rate(rate);
                        }
                        WsServerMessage::CommonMarkets(markets) => {
                            let _ = app.emit("common_markets", &markets);
                            state.update_common_markets(markets);
                        }
                        WsServerMessage::WalletStatus(wallet_status) => {
                            let _ = app.emit("wallet_status", &wallet_status);
                            state.update_wallet_status(wallet_status);
                        }
                        WsServerMessage::PremiumMatrix(matrix) => {
                            let _ = app.emit("premium_matrix", &matrix);
                        }
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = write.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(_)) => {
                info!("CLI server closed connection");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    let _ = app.emit("server_connected", false);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert!(!state.is_connected());
    }

    #[test]
    fn test_connection_status() {
        let state = AppState::new();
        state.set_connected(true);
        assert!(state.is_connected());
        state.set_connected(false);
        assert!(!state.is_connected());
    }

    #[test]
    fn test_prices_update() {
        let state = AppState::new();
        let prices = vec![
            PriceData {
                exchange: "Binance".to_string(),
                symbol: "BTC".to_string(),
                pair_id: 1,
                price: 50000.0,
                bid: 49999.0,
                ask: 50001.0,
                volume_24h: 1000000.0,
                timestamp: 0,
                quote: Some("USDT".to_string()),
            },
        ];
        state.update_prices(prices);
        assert_eq!(state.get_prices().len(), 1);
    }

    #[test]
    fn test_config_update() {
        let state = AppState::new();
        let mut config = state.get_config();
        config.min_premium_bps = 50;
        state.update_config(config.clone());

        let updated = state.get_config();
        assert_eq!(updated.min_premium_bps, 50);
    }
}

//! Wallet status fetching for deposit/withdraw availability.
//!
//! Fetches wallet status from exchanges periodically and broadcasts via WebSocket.

use crate::ws_server::{self, BroadcastSender};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Cached wallet status for initial sync when clients connect.
static CACHED_WALLET_STATUS: RwLock<Vec<ExchangeWalletStatus>> = RwLock::new(Vec::new());

/// Network name mapping: canonical network name -> { exchange -> network_id }
/// Loaded from network_name_mapping.json
static NETWORK_NAME_MAPPING: RwLock<Option<NetworkNameMapping>> = RwLock::new(None);

/// Network name mapping structure.
/// Maps canonical network names to exchange-specific network IDs.
#[derive(Debug, Clone, Default)]
pub struct NetworkNameMapping {
    /// canonical_name -> { exchange_name -> network_id }
    pub mappings: HashMap<String, HashMap<String, Option<String>>>,
    /// Reverse lookup: (exchange, network_id) -> canonical_name
    pub reverse: HashMap<(String, String), String>,
}

impl NetworkNameMapping {
    /// Load from JSON file.
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path, e))?;

        let raw: HashMap<String, HashMap<String, Option<String>>> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path, e))?;

        // Build reverse lookup
        let mut reverse = HashMap::new();
        for (canonical, exchanges) in &raw {
            for (exchange, network_id) in exchanges {
                if let Some(id) = network_id {
                    reverse.insert((exchange.clone(), id.clone()), canonical.clone());
                    // Also add lowercase variant for case-insensitive lookup
                    reverse.insert((exchange.clone(), id.to_lowercase()), canonical.clone());
                }
            }
        }

        Ok(Self {
            mappings: raw,
            reverse,
        })
    }

    /// Get canonical network name for an exchange's network ID.
    pub fn get_canonical(&self, exchange: &str, network_id: &str) -> Option<String> {
        // Try exact match first
        if let Some(canonical) = self.reverse.get(&(exchange.to_string(), network_id.to_string())) {
            return Some(canonical.clone());
        }
        // Try lowercase
        self.reverse.get(&(exchange.to_string(), network_id.to_lowercase())).cloned()
    }

    /// Check if two exchanges share a common network for transfers.
    /// Returns the list of common canonical network names.
    pub fn find_common_networks(
        &self,
        exchange1: &str,
        exchange2: &str,
        networks1: &[String],
        networks2: &[String],
    ) -> Vec<String> {
        let mut common = Vec::new();

        // Convert network IDs to canonical names
        let canonical1: HashSet<String> = networks1
            .iter()
            .filter_map(|n| self.get_canonical(exchange1, n))
            .collect();

        let canonical2: HashSet<String> = networks2
            .iter()
            .filter_map(|n| self.get_canonical(exchange2, n))
            .collect();

        // Find intersection
        for name in &canonical1 {
            if canonical2.contains(name) {
                common.push(name.clone());
            }
        }

        common
    }
}

/// Load network name mapping from file.
pub fn load_network_mapping() -> Option<NetworkNameMapping> {
    // Try loading from project root first, then from current directory
    let paths = [
        "network_name_mapping.json",
        "./network_name_mapping.json",
        "../network_name_mapping.json",
        "../../network_name_mapping.json",
    ];

    for path in &paths {
        if let Ok(mapping) = NetworkNameMapping::load_from_file(path) {
            debug!("Loaded network name mapping from {} ({} networks)", path, mapping.mappings.len());
            return Some(mapping);
        }
    }

    warn!("Could not load network_name_mapping.json - common network filtering disabled");
    None
}

/// Initialize network mapping (call once at startup).
pub fn init_network_mapping() {
    let mapping = load_network_mapping();
    let mut stored = NETWORK_NAME_MAPPING.write().unwrap();
    *stored = mapping;
}

/// Get cached network mapping.
pub fn get_network_mapping() -> Option<NetworkNameMapping> {
    NETWORK_NAME_MAPPING.read().unwrap().clone()
}

/// Find common networks between two exchanges for a specific asset.
/// Returns (common_networks, source_networks, target_networks).
///
/// Note: If wallet status cache is empty, returns empty vectors.
/// The caller should check `has_transfer_path` being false and `common_networks` being empty
/// to determine if the transfer is not possible or if data is not yet available.
pub fn find_common_networks_for_asset(
    asset: &str,
    source_exchange: &str,
    target_exchange: &str,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let cached = get_cached_wallet_status();
    let mapping = get_network_mapping();

    // If cache is empty, we don't have wallet status data yet
    if cached.is_empty() {
        tracing::trace!("Wallet status cache is empty, cannot determine transfer path for {}", asset);
        return (Vec::new(), Vec::new(), Vec::new());
    }

    // Find source exchange networks for this asset (withdraw-enabled only)
    let source_networks: Vec<String> = cached
        .iter()
        .find(|e| e.exchange == source_exchange)
        .and_then(|e| e.wallet_status.iter().find(|a| a.asset == asset))
        .map(|a| a.networks.iter()
            .filter(|n| n.withdraw_enabled)  // Only consider withdrawable networks
            .map(|n| n.network.clone())
            .collect())
        .unwrap_or_default();

    // Find target exchange networks for this asset (deposit-enabled only)
    let target_networks: Vec<String> = cached
        .iter()
        .find(|e| e.exchange == target_exchange)
        .and_then(|e| e.wallet_status.iter().find(|a| a.asset == asset))
        .map(|a| a.networks.iter()
            .filter(|n| n.deposit_enabled)  // Only consider depositable networks
            .map(|n| n.network.clone())
            .collect())
        .unwrap_or_default();

    // Find common networks using mapping
    let common = if let Some(ref mapping) = mapping {
        mapping.find_common_networks(source_exchange, target_exchange, &source_networks, &target_networks)
    } else {
        // Fallback: direct string matching (case-insensitive)
        let source_set: HashSet<String> = source_networks.iter().map(|s| s.to_uppercase()).collect();
        let target_set: HashSet<String> = target_networks.iter().map(|s| s.to_uppercase()).collect();
        source_set.intersection(&target_set).cloned().collect()
    };

    // Debug logging for troubleshooting
    if tracing::enabled!(tracing::Level::DEBUG) && (source_networks.len() > 0 || target_networks.len() > 0) {
        tracing::debug!(
            "Network check for {} ({} -> {}): source_withdraw={:?}, target_deposit={:?}, common={:?}",
            asset, source_exchange, target_exchange, source_networks, target_networks, common
        );
    }

    (common, source_networks, target_networks)
}

/// Check if an opportunity has a viable transfer path.
/// Returns true if there's at least one common network between source (withdraw) and target (deposit).
pub fn has_transfer_path(asset: &str, source_exchange: &str, target_exchange: &str) -> bool {
    let (common, _, _) = find_common_networks_for_asset(asset, source_exchange, target_exchange);
    !common.is_empty()
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

/// Asset wallet status with network info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetWalletStatus {
    pub asset: String,
    pub name: String,
    pub networks: Vec<NetworkStatus>,
    pub can_deposit: bool,
    pub can_withdraw: bool,
}

/// Complete wallet status for an exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeWalletStatus {
    pub exchange: String,
    pub wallet_status: Vec<AssetWalletStatus>,
    pub last_updated: u64,
}

fn timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ============================================================================
// Upbit Client (Public API)
// ============================================================================

#[derive(Debug, Deserialize)]
struct UpbitNetworkStatus {
    currency: String,
    wallet_state: String,
    net_type: String,
    network_name: String,
}

/// Fetch Upbit wallet status (public API - no auth required)
async fn fetch_upbit_wallet_status() -> Result<ExchangeWalletStatus, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let status_resp = client
        .get("https://ccx.upbit.com/api/v1/status/network/wallet")
        .send()
        .await
        .map_err(|e| format!("Upbit wallet status request failed: {}", e))?;

    if !status_resp.status().is_success() {
        let error_text = status_resp.text().await.unwrap_or_default();
        return Err(format!("Upbit wallet status API error: {}", error_text));
    }

    let statuses: Vec<UpbitNetworkStatus> = status_resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Upbit wallet status: {}", e))?;

    // Group by currency
    let mut currency_map: HashMap<String, Vec<UpbitNetworkStatus>> = HashMap::new();
    for status in statuses {
        currency_map
            .entry(status.currency.clone())
            .or_default()
            .push(status);
    }

    let wallet_status: Vec<AssetWalletStatus> = currency_map
        .into_iter()
        .map(|(currency, networks)| {
            let network_statuses: Vec<NetworkStatus> = networks
                .iter()
                .map(|n| {
                    let (deposit_enabled, withdraw_enabled) = match n.wallet_state.as_str() {
                        "working" => (true, true),
                        "withdraw_only" => (false, true),
                        "deposit_only" => (true, false),
                        "paused" | "unsupported" => (false, false),
                        _ => (false, false),
                    };

                    NetworkStatus {
                        network: n.net_type.clone(),
                        name: n.network_name.clone(),
                        deposit_enabled,
                        withdraw_enabled,
                        min_withdraw: 0.0,
                        withdraw_fee: 0.0,
                        confirms_required: 0,
                    }
                })
                .collect();

            let can_deposit = network_statuses.iter().any(|n| n.deposit_enabled);
            let can_withdraw = network_statuses.iter().any(|n| n.withdraw_enabled);

            AssetWalletStatus {
                asset: currency.clone(),
                name: currency,
                networks: network_statuses,
                can_deposit,
                can_withdraw,
            }
        })
        .collect();

    Ok(ExchangeWalletStatus {
        exchange: "Upbit".to_string(),
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Coinbase Client (Public API)
// ============================================================================

#[derive(Debug, Deserialize)]
struct CoinbaseCurrency {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    supported_networks: Vec<CoinbaseNetwork>,
    #[serde(default)]
    details: Option<CoinbaseCurrencyDetails>,
}

#[derive(Debug, Deserialize)]
struct CoinbaseCurrencyDetails {
    #[serde(rename = "type")]
    currency_type: Option<String>,
    #[serde(default)]
    min_withdrawal_amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CoinbaseNetwork {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    min_withdrawal_amount: Option<f64>,
    #[serde(default)]
    network_confirmations: Option<u32>,
}

/// Fetch Coinbase wallet status (public Exchange API - no auth required)
async fn fetch_coinbase_wallet_status() -> Result<ExchangeWalletStatus, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let resp = client
        .get("https://api.exchange.coinbase.com/currencies")
        .header("User-Agent", "arbitrage-bot")
        .send()
        .await
        .map_err(|e| format!("Coinbase currencies request failed: {}", e))?;

    if !resp.status().is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("Coinbase currencies API error: {}", error_text));
    }

    let currencies: Vec<CoinbaseCurrency> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Coinbase currencies: {}", e))?;

    let wallet_status: Vec<AssetWalletStatus> = currencies
        .into_iter()
        .filter(|c| {
            !c.supported_networks.is_empty()
                || c.details
                    .as_ref()
                    .and_then(|d| d.currency_type.as_ref())
                    .map(|t| t == "crypto")
                    .unwrap_or(false)
        })
        .map(|c| {
            let network_statuses: Vec<NetworkStatus> = if c.supported_networks.is_empty() {
                let is_online = c.status == "online";
                vec![NetworkStatus {
                    network: c.id.clone(),
                    name: c.name.clone(),
                    deposit_enabled: is_online,
                    withdraw_enabled: is_online,
                    min_withdraw: c
                        .details
                        .as_ref()
                        .and_then(|d| d.min_withdrawal_amount)
                        .unwrap_or(0.0),
                    withdraw_fee: 0.0,
                    confirms_required: 0,
                }]
            } else {
                c.supported_networks
                    .iter()
                    .map(|n| {
                        let is_online = n.status == "online";
                        NetworkStatus {
                            network: n.id.clone(),
                            name: n.name.clone(),
                            deposit_enabled: is_online,
                            withdraw_enabled: is_online,
                            min_withdraw: n.min_withdrawal_amount.unwrap_or(0.0),
                            withdraw_fee: 0.0,
                            confirms_required: n.network_confirmations.unwrap_or(0),
                        }
                    })
                    .collect()
            };

            let can_deposit = network_statuses.iter().any(|n| n.deposit_enabled);
            let can_withdraw = network_statuses.iter().any(|n| n.withdraw_enabled);

            AssetWalletStatus {
                asset: c.id,
                name: c.name,
                networks: network_statuses,
                can_deposit,
                can_withdraw,
            }
        })
        .collect();

    Ok(ExchangeWalletStatus {
        exchange: "Coinbase".to_string(),
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Bithumb Client (API 2.0 with JWT authentication)
// ============================================================================

#[derive(Debug, Deserialize)]
struct BithumbWalletAsset {
    currency: String,
    #[serde(default)]
    net_type: Option<String>,
    #[serde(default)]
    wallet_state: Option<String>, // "working", "withdraw_only", "deposit_only", "suspended"
}

/// Generate JWT token for Bithumb API 2.0
fn generate_bithumb_jwt(api_key: &str, secret_key: &str) -> Result<String, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // JWT header
    let header = r#"{"alg":"HS256","typ":"JWT"}"#;

    // JWT payload with required claims
    let payload = format!(
        r#"{{"access_key":"{}","nonce":"{}","timestamp":{}}}"#,
        api_key, now, now
    );

    let header_b64 = URL_SAFE_NO_PAD.encode(header);
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload);
    let message = format!("{}.{}", header_b64, payload_b64);

    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
        .map_err(|e| format!("HMAC error: {}", e))?;
    mac.update(message.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Ok(format!("{}.{}.{}", header_b64, payload_b64, signature))
}

/// Fetch Bithumb wallet status (API 2.0 with JWT authentication)
async fn fetch_bithumb_wallet_status() -> Result<ExchangeWalletStatus, String> {
    let api_key = std::env::var("BITHUMB_API_KEY").unwrap_or_default();
    let secret_key = std::env::var("BITHUMB_SECRET_KEY").unwrap_or_default();

    if api_key.is_empty() || secret_key.is_empty() {
        // No API keys - return empty status
        info!("Bithumb API keys not configured (BITHUMB_API_KEY, BITHUMB_SECRET_KEY)");
        return Ok(ExchangeWalletStatus {
            exchange: "Bithumb".to_string(),
            wallet_status: Vec::new(),
            last_updated: timestamp_ms(),
        });
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    // Generate JWT token for authentication
    let token = generate_bithumb_jwt(&api_key, &secret_key)?;

    // Call wallet status endpoint
    let resp = client
        .get("https://api.bithumb.com/v1/status/wallet")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Bithumb wallet status request failed: {}", e))?;

    if !resp.status().is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("Bithumb wallet status API error: {}", error_text));
    }

    // Response is a direct array, not wrapped in {status, data}
    let assets: Vec<BithumbWalletAsset> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Bithumb wallet status: {}", e))?;

    // Group by currency
    let mut currency_map: HashMap<String, Vec<BithumbWalletAsset>> = HashMap::new();
    for asset in assets {
        currency_map
            .entry(asset.currency.clone())
            .or_default()
            .push(asset);
    }

    let wallet_status: Vec<AssetWalletStatus> = currency_map
        .into_iter()
        .map(|(currency, networks)| {
            let network_statuses: Vec<NetworkStatus> = networks
                .iter()
                .map(|n| {
                    // Parse wallet_state: "working", "withdraw_only", "deposit_only", "suspended"
                    let (deposit_enabled, withdraw_enabled) =
                        match n.wallet_state.as_deref().unwrap_or("suspended") {
                            "working" => (true, true),
                            "withdraw_only" => (false, true),
                            "deposit_only" => (true, false),
                            _ => (false, false), // "suspended" or unknown
                        };

                    NetworkStatus {
                        network: n.net_type.clone().unwrap_or_else(|| currency.clone()),
                        name: n.net_type.clone().unwrap_or_else(|| "Default".to_string()),
                        deposit_enabled,
                        withdraw_enabled,
                        min_withdraw: 0.0,
                        withdraw_fee: 0.0,
                        confirms_required: 0,
                    }
                })
                .collect();

            let can_deposit = network_statuses.iter().any(|n| n.deposit_enabled);
            let can_withdraw = network_statuses.iter().any(|n| n.withdraw_enabled);

            AssetWalletStatus {
                asset: currency.clone(),
                name: currency,
                networks: network_statuses,
                can_deposit,
                can_withdraw,
            }
        })
        .collect();

    Ok(ExchangeWalletStatus {
        exchange: "Bithumb".to_string(),
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Binance Client (Requires API Key)
// ============================================================================

#[derive(Debug, Deserialize)]
struct BinanceCoinInfo {
    coin: String,
    name: String,
    #[serde(rename = "networkList")]
    network_list: Vec<BinanceNetwork>,
    #[serde(rename = "depositAllEnable")]
    deposit_all_enable: bool,
    #[serde(rename = "withdrawAllEnable")]
    withdraw_all_enable: bool,
}

#[derive(Debug, Deserialize)]
struct BinanceNetwork {
    network: String,
    name: String,
    #[serde(rename = "depositEnable")]
    deposit_enable: bool,
    #[serde(rename = "withdrawEnable")]
    withdraw_enable: bool,
    #[serde(rename = "withdrawMin", default)]
    withdraw_min: String,
    #[serde(rename = "withdrawFee", default)]
    withdraw_fee: String,
    #[serde(rename = "minConfirm", default)]
    min_confirm: u32,
}

fn sign_binance(query: &str, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Fetch Binance wallet status (requires API key)
/// Reads API key and secret from environment variables:
/// - BINANCE_API_KEY
/// - BINANCE_SECRET_KEY
async fn fetch_binance_wallet_status() -> Result<ExchangeWalletStatus, String> {
    let api_key = std::env::var("BINANCE_API_KEY").unwrap_or_default();
    let secret_key = std::env::var("BINANCE_SECRET_KEY").unwrap_or_default();

    if api_key.is_empty() || secret_key.is_empty() {
        // Return empty status if no API keys configured
        info!("Binance API keys not configured (BINANCE_API_KEY, BINANCE_SECRET_KEY)");
        return Ok(ExchangeWalletStatus {
            exchange: "Binance".to_string(),
            wallet_status: Vec::new(),
            last_updated: timestamp_ms(),
        });
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let timestamp = timestamp_ms();
    let query = format!("timestamp={}", timestamp);
    let signature = sign_binance(&query, &secret_key);
    let url = format!(
        "https://api.binance.com/sapi/v1/capital/config/getall?{}&signature={}",
        query, signature
    );

    let resp = client
        .get(&url)
        .header("X-MBX-APIKEY", &api_key)
        .send()
        .await
        .map_err(|e| format!("Binance wallet status request failed: {}", e))?;

    if !resp.status().is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("Binance wallet status API error: {}", error_text));
    }

    let coins: Vec<BinanceCoinInfo> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Binance coins: {}", e))?;

    let wallet_status: Vec<AssetWalletStatus> = coins
        .into_iter()
        .map(|c| {
            let networks: Vec<NetworkStatus> = c
                .network_list
                .into_iter()
                .map(|n| NetworkStatus {
                    network: n.network,
                    name: n.name,
                    deposit_enabled: n.deposit_enable,
                    withdraw_enabled: n.withdraw_enable,
                    min_withdraw: n.withdraw_min.parse().unwrap_or(0.0),
                    withdraw_fee: n.withdraw_fee.parse().unwrap_or(0.0),
                    confirms_required: n.min_confirm,
                })
                .collect();

            AssetWalletStatus {
                asset: c.coin,
                name: c.name,
                networks,
                can_deposit: c.deposit_all_enable,
                can_withdraw: c.withdraw_all_enable,
            }
        })
        .collect();

    Ok(ExchangeWalletStatus {
        exchange: "Binance".to_string(),
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Bybit Client (V5 API with HMAC-SHA256 signing)
// ============================================================================

#[derive(Debug, Deserialize)]
struct BybitCoinInfo {
    coin: String,
    #[serde(rename = "coinName", default)]
    coin_name: String,
    chains: Vec<BybitChainInfo>,
}

#[derive(Debug, Deserialize)]
struct BybitChainInfo {
    chain: String,
    #[serde(rename = "chainDeposit", default)]
    chain_deposit: String,
    #[serde(rename = "chainWithdraw", default)]
    chain_withdraw: String,
    #[serde(rename = "minWithdraw", default)]
    min_withdraw: String,
    #[serde(rename = "withdrawFee", default)]
    withdraw_fee: String,
    #[serde(rename = "confirmation", default)]
    confirmation: String,
}

#[derive(Debug, Deserialize)]
struct BybitCoinInfoResponse {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: BybitCoinInfoResult,
}

#[derive(Debug, Deserialize)]
struct BybitCoinInfoResult {
    rows: Vec<BybitCoinInfo>,
}

fn sign_bybit(timestamp: u64, api_key: &str, recv_window: &str, query: &str, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;
    let sign_str = format!("{}{}{}{}", timestamp, api_key, recv_window, query);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(sign_str.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Fetch Bybit wallet status (requires API key)
async fn fetch_bybit_wallet_status() -> Result<ExchangeWalletStatus, String> {
    let api_key = std::env::var("BYBIT_API_KEY").unwrap_or_default();
    let secret_key = std::env::var("BYBIT_SECRET_KEY").unwrap_or_default();

    if api_key.is_empty() || secret_key.is_empty() {
        info!("Bybit API keys not configured (BYBIT_API_KEY, BYBIT_SECRET_KEY)");
        return Ok(ExchangeWalletStatus {
            exchange: "Bybit".to_string(),
            wallet_status: Vec::new(),
            last_updated: timestamp_ms(),
        });
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let timestamp = timestamp_ms();
    let recv_window = "5000";
    let query = "";

    let signature = sign_bybit(timestamp, &api_key, recv_window, query, &secret_key);

    let resp = client
        .get("https://api.bybit.com/v5/asset/coin/query-info")
        .header("X-BAPI-API-KEY", &api_key)
        .header("X-BAPI-SIGN", signature)
        .header("X-BAPI-TIMESTAMP", timestamp.to_string())
        .header("X-BAPI-RECV-WINDOW", recv_window)
        .send()
        .await
        .map_err(|e| format!("Bybit coin info request failed: {}", e))?;

    if !resp.status().is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("Bybit coin info API error: {}", error_text));
    }

    let response: BybitCoinInfoResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Bybit coin info: {}", e))?;

    if response.ret_code != 0 {
        return Err(format!("Bybit API error: {} - {}", response.ret_code, response.ret_msg));
    }

    let wallet_status: Vec<AssetWalletStatus> = response.result.rows
        .into_iter()
        .map(|coin| {
            let network_statuses: Vec<NetworkStatus> = coin.chains
                .into_iter()
                .map(|chain| {
                    let deposit_enabled = chain.chain_deposit == "1";
                    let withdraw_enabled = chain.chain_withdraw == "1";

                    NetworkStatus {
                        network: chain.chain.clone(),
                        name: chain.chain,
                        deposit_enabled,
                        withdraw_enabled,
                        min_withdraw: chain.min_withdraw.parse().unwrap_or(0.0),
                        withdraw_fee: chain.withdraw_fee.parse().unwrap_or(0.0),
                        confirms_required: chain.confirmation.parse().unwrap_or(0),
                    }
                })
                .collect();

            let can_deposit = network_statuses.iter().any(|n| n.deposit_enabled);
            let can_withdraw = network_statuses.iter().any(|n| n.withdraw_enabled);

            AssetWalletStatus {
                asset: coin.coin.clone(),
                name: if coin.coin_name.is_empty() { coin.coin } else { coin.coin_name },
                networks: network_statuses,
                can_deposit,
                can_withdraw,
            }
        })
        .collect();

    Ok(ExchangeWalletStatus {
        exchange: "Bybit".to_string(),
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Gate.io Client (Public API)
// ============================================================================

/// Gate.io chain info
#[derive(Debug, Deserialize)]
struct GateIOChain {
    name: String,
    #[serde(default)]
    deposit_disabled: bool,
    #[serde(default)]
    withdraw_disabled: bool,
}

/// Gate.io currency response
#[derive(Debug, Deserialize)]
struct GateIOCurrency {
    currency: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    deposit_disabled: bool,
    #[serde(default)]
    withdraw_disabled: bool,
    #[serde(default)]
    delisted: bool,
    #[serde(default)]
    trade_disabled: bool,
    #[serde(default)]
    chains: Vec<GateIOChain>,
}

/// Fetch Gate.io wallet status (public API - no auth required)
async fn fetch_gateio_wallet_status() -> Result<ExchangeWalletStatus, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let resp = client
        .get("https://api.gateio.ws/api/v4/spot/currencies")
        .send()
        .await
        .map_err(|e| format!("Gate.io currencies request failed: {}", e))?;

    if !resp.status().is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("Gate.io currencies API error: {}", error_text));
    }

    let currencies: Vec<GateIOCurrency> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Gate.io currencies: {}", e))?;

    let wallet_status: Vec<AssetWalletStatus> = currencies
        .into_iter()
        .filter(|c| !c.delisted && !c.trade_disabled)
        .map(|c| {
            // Build network statuses from chains array
            let network_statuses: Vec<NetworkStatus> = if c.chains.is_empty() {
                vec![NetworkStatus {
                    network: c.currency.clone(),
                    name: c.currency.clone(),
                    deposit_enabled: !c.deposit_disabled,
                    withdraw_enabled: !c.withdraw_disabled,
                    min_withdraw: 0.0,
                    withdraw_fee: 0.0,
                    confirms_required: 0,
                }]
            } else {
                c.chains
                    .iter()
                    .map(|chain| NetworkStatus {
                        network: chain.name.clone(),
                        name: chain.name.clone(),
                        deposit_enabled: !chain.deposit_disabled,
                        withdraw_enabled: !chain.withdraw_disabled,
                        min_withdraw: 0.0,
                        withdraw_fee: 0.0,
                        confirms_required: 0,
                    })
                    .collect()
            };

            let can_deposit = network_statuses.iter().any(|n| n.deposit_enabled);
            let can_withdraw = network_statuses.iter().any(|n| n.withdraw_enabled);

            AssetWalletStatus {
                asset: c.currency.clone(),
                name: if c.name.is_empty() { c.currency } else { c.name },
                networks: network_statuses,
                can_deposit,
                can_withdraw,
            }
        })
        .collect();

    Ok(ExchangeWalletStatus {
        exchange: "GateIO".to_string(),
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Aggregate and Update Functions
// ============================================================================

/// Fetch wallet status from all exchanges.
pub async fn fetch_all_wallet_status() -> Vec<ExchangeWalletStatus> {
    let mut results = Vec::new();

    // Fetch in parallel
    let (upbit, bithumb, coinbase, binance, bybit, gateio) = tokio::join!(
        fetch_upbit_wallet_status(),
        fetch_bithumb_wallet_status(),
        fetch_coinbase_wallet_status(),
        fetch_binance_wallet_status(),
        fetch_bybit_wallet_status(),
        fetch_gateio_wallet_status()
    );

    if let Ok(status) = upbit {
        if !status.wallet_status.is_empty() {
            results.push(status);
        }
    } else if let Err(e) = upbit {
        warn!("Failed to fetch Upbit wallet status: {}", e);
    }

    if let Ok(status) = bithumb {
        if !status.wallet_status.is_empty() {
            results.push(status);
        }
    } else if let Err(e) = bithumb {
        warn!("Failed to fetch Bithumb wallet status: {}", e);
    }

    if let Ok(status) = coinbase {
        if !status.wallet_status.is_empty() {
            results.push(status);
        }
    } else if let Err(e) = coinbase {
        warn!("Failed to fetch Coinbase wallet status: {}", e);
    }

    if let Ok(status) = binance {
        if !status.wallet_status.is_empty() {
            results.push(status);
        }
    } else if let Err(e) = binance {
        warn!("Failed to fetch Binance wallet status: {}", e);
    }

    if let Ok(status) = bybit {
        if !status.wallet_status.is_empty() {
            results.push(status);
        }
    } else if let Err(e) = bybit {
        warn!("Failed to fetch Bybit wallet status: {}", e);
    }

    if let Ok(status) = gateio {
        if !status.wallet_status.is_empty() {
            results.push(status);
        }
    } else if let Err(e) = gateio {
        warn!("Failed to fetch Gate.io wallet status: {}", e);
    }

    results
}

/// Get cached wallet status for initial sync.
pub fn get_cached_wallet_status() -> Vec<ExchangeWalletStatus> {
    CACHED_WALLET_STATUS.read().unwrap().clone()
}

/// Check if wallet status is known for both exchanges.
/// Returns true if we have wallet status data for both source and target exchanges.
pub fn is_wallet_status_known(source_exchange: &str, target_exchange: &str) -> bool {
    let cached = CACHED_WALLET_STATUS.read().unwrap();
    if cached.is_empty() {
        return false;
    }
    let has_source = cached.iter().any(|e| e.exchange == source_exchange);
    let has_target = cached.iter().any(|e| e.exchange == target_exchange);
    has_source && has_target
}

/// Update the cached wallet status.
pub fn update_cache(statuses: Vec<ExchangeWalletStatus>) {
    let mut cache = CACHED_WALLET_STATUS.write().unwrap();
    *cache = statuses;
}

/// Networks that are obviously the same across exchanges (no mapping needed).
const OBVIOUS_NETWORKS: &[&str] = &[
    // Native chains - same name everywhere
    "SOL", "solana", "Solana", "SOLANA",
    "ETH", "ethereum", "Ethereum", "ETHEREUM", "ERC20",
    "BTC", "bitcoin", "Bitcoin", "BITCOIN",
    "TRX", "tron", "Tron", "TRON", "TRC20",
    "MATIC", "polygon", "Polygon", "POLYGON",
    "AVAX", "avalanche", "Avalanche", "AVAXC",
    "BNB", "BSC", "BEP20", "bsc", "binancesmartchain",
    "ARB", "ARBITRUM", "arbitrum", "Arbitrum",
    "OP", "OPTIMISM", "optimism", "Optimism",
    "BASE", "base", "Base",
    "TON", "ton", "Ton",
    "XRP", "ripple", "Ripple", "RIPPLE",
    "DOGE", "dogecoin", "Dogecoin",
    "LTC", "litecoin", "Litecoin",
    "ADA", "cardano", "Cardano",
    "DOT", "polkadot", "Polkadot",
    "ATOM", "cosmos", "Cosmos",
    "NEAR", "near", "Near",
    "APT", "aptos", "Aptos",
    "SUI", "sui", "Sui",
];

/// Save all network names per asset across exchanges to a JSON file.
/// This helps identify which networks are the same across different exchanges.
pub fn save_network_mappings(statuses: &[ExchangeWalletStatus]) {
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::Write;

    // Collect all networks per asset across exchanges
    // Key: asset name, Value: Map<exchange, Vec<network_name>>
    let mut asset_networks: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();

    for exchange_status in statuses {
        for asset_status in &exchange_status.wallet_status {
            // Filter out obvious networks from the list
            let networks: Vec<String> = asset_status
                .networks
                .iter()
                .map(|n| n.network.clone())
                .filter(|net| {
                    !OBVIOUS_NETWORKS.iter().any(|&obvious| {
                        net.eq_ignore_ascii_case(obvious)
                    })
                })
                .collect();

            if !networks.is_empty() {
                asset_networks
                    .entry(asset_status.asset.clone())
                    .or_default()
                    .insert(exchange_status.exchange.clone(), networks);
            }
        }
    }

    // Filter to assets on 2+ exchanges (cross-exchange transfer candidates)
    // Obvious networks are already filtered out above
    let multi_exchange_assets: BTreeMap<String, BTreeMap<String, Vec<String>>> = asset_networks
        .into_iter()
        .filter(|(_, exchanges)| exchanges.len() >= 2)
        .collect();

    // Save to JSON file
    let json = serde_json::to_string_pretty(&multi_exchange_assets).unwrap_or_default();
    let path = "network_mappings.json";

    match File::create(path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(json.as_bytes()) {
                warn!("Failed to write network mappings: {}", e);
            } else {
                debug!("Saved network mappings to {} ({} assets need mapping)", path, multi_exchange_assets.len());
            }
        }
        Err(e) => {
            warn!("Failed to create network mappings file: {}", e);
        }
    }
}

/// Run wallet status updater loop.
/// Updates every 5 minutes.
pub async fn run_wallet_status_updater(broadcast_tx: BroadcastSender) {
    let mut first_run = true;

    loop {
        let statuses = fetch_all_wallet_status().await;

        if !statuses.is_empty() {

            // Save network mappings on first run for cross-exchange transfer mapping
            if first_run {
                save_network_mappings(&statuses);
                first_run = false;
            }

            // Update cache first
            update_cache(statuses.clone());
            // Then broadcast
            ws_server::broadcast_wallet_status(&broadcast_tx, statuses);
        }

        // Update every 5 minutes (same as exchange rate)
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_upbit_wallet_status() {
        let result = fetch_upbit_wallet_status().await;
        // Should not panic, may fail due to network
        if let Ok(status) = result {
            assert_eq!(status.exchange, "Upbit");
        }
    }

    #[tokio::test]
    async fn test_fetch_coinbase_wallet_status() {
        let result = fetch_coinbase_wallet_status().await;
        // Should not panic, may fail due to network
        if let Ok(status) = result {
            assert_eq!(status.exchange, "Coinbase");
        }
    }
}

//! Wallet status fetching for deposit/withdraw availability.
//!
//! Fetches wallet status from exchanges periodically and broadcasts via WebSocket.

use crate::ws_server::{self, BroadcastSender};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;
use tracing::{info, warn};

/// Cached wallet status for initial sync when clients connect.
static CACHED_WALLET_STATUS: RwLock<Vec<ExchangeWalletStatus>> = RwLock::new(Vec::new());

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
// Bithumb Client (Public API)
// ============================================================================

#[derive(Debug, Deserialize)]
struct BithumbStatusResponse {
    status: String,
    data: HashMap<String, BithumbAssetStatus>,
}

#[derive(Debug, Deserialize)]
struct BithumbAssetStatus {
    #[serde(default)]
    withdrawal_status: Option<i32>, // 1 = available, 0 = unavailable
    #[serde(default)]
    deposit_status: Option<i32>, // 1 = available, 0 = unavailable
}

/// Fetch Bithumb wallet status (public API - no auth required)
async fn fetch_bithumb_wallet_status() -> Result<ExchangeWalletStatus, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    // Bithumb has a status endpoint per currency
    // Use ticker/ALL to get list of currencies, then assume all are enabled
    // (Bithumb doesn't have a public wallet status endpoint like Upbit)
    let resp = client
        .get("https://api.bithumb.com/public/ticker/ALL_KRW")
        .send()
        .await
        .map_err(|e| format!("Bithumb ticker request failed: {}", e))?;

    if !resp.status().is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("Bithumb ticker API error: {}", error_text));
    }

    #[derive(Debug, Deserialize)]
    struct BithumbTickerResponse {
        status: String,
        data: serde_json::Value,
    }

    let ticker_resp: BithumbTickerResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Bithumb ticker: {}", e))?;

    if ticker_resp.status != "0000" {
        return Err(format!(
            "Bithumb API error: status {}",
            ticker_resp.status
        ));
    }

    // Extract currency symbols from ticker data
    let currencies: Vec<String> = if let serde_json::Value::Object(map) = ticker_resp.data {
        map.keys()
            .filter(|k| *k != "date")
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    // For Bithumb, we assume all currencies are available since there's no public status API
    // In production, you might want to check the account/balance endpoint with auth
    let wallet_status: Vec<AssetWalletStatus> = currencies
        .into_iter()
        .map(|currency| {
            AssetWalletStatus {
                asset: currency.clone(),
                name: currency.clone(),
                networks: vec![NetworkStatus {
                    network: "KRW".to_string(),
                    name: "Bithumb".to_string(),
                    deposit_enabled: true,
                    withdraw_enabled: true,
                    min_withdraw: 0.0,
                    withdraw_fee: 0.0,
                    confirms_required: 0,
                }],
                can_deposit: true,
                can_withdraw: true,
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
// Aggregate and Update Functions
// ============================================================================

/// Fetch wallet status from all exchanges.
pub async fn fetch_all_wallet_status() -> Vec<ExchangeWalletStatus> {
    let mut results = Vec::new();

    // Fetch in parallel
    let (upbit, bithumb, coinbase, binance) = tokio::join!(
        fetch_upbit_wallet_status(),
        fetch_bithumb_wallet_status(),
        fetch_coinbase_wallet_status(),
        fetch_binance_wallet_status()
    );

    if let Ok(status) = upbit {
        if !status.wallet_status.is_empty() {
            info!("Fetched Upbit wallet status: {} assets", status.wallet_status.len());
            results.push(status);
        }
    } else if let Err(e) = upbit {
        warn!("Failed to fetch Upbit wallet status: {}", e);
    }

    if let Ok(status) = bithumb {
        if !status.wallet_status.is_empty() {
            info!("Fetched Bithumb wallet status: {} assets", status.wallet_status.len());
            results.push(status);
        }
    } else if let Err(e) = bithumb {
        warn!("Failed to fetch Bithumb wallet status: {}", e);
    }

    if let Ok(status) = coinbase {
        if !status.wallet_status.is_empty() {
            info!("Fetched Coinbase wallet status: {} assets", status.wallet_status.len());
            results.push(status);
        }
    } else if let Err(e) = coinbase {
        warn!("Failed to fetch Coinbase wallet status: {}", e);
    }

    if let Ok(status) = binance {
        if !status.wallet_status.is_empty() {
            info!("Fetched Binance wallet status: {} assets", status.wallet_status.len());
            results.push(status);
        }
    }

    results
}

/// Get cached wallet status for initial sync.
pub fn get_cached_wallet_status() -> Vec<ExchangeWalletStatus> {
    CACHED_WALLET_STATUS.read().unwrap().clone()
}

/// Update the cached wallet status.
pub fn update_cache(statuses: Vec<ExchangeWalletStatus>) {
    let mut cache = CACHED_WALLET_STATUS.write().unwrap();
    *cache = statuses;
}

/// Run wallet status updater loop.
/// Updates every 5 minutes.
pub async fn run_wallet_status_updater(broadcast_tx: BroadcastSender) {
    info!("Starting wallet status updater");

    loop {
        let statuses = fetch_all_wallet_status().await;

        if !statuses.is_empty() {
            info!("Broadcasting wallet status for {} exchanges", statuses.len());
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

//! Exchange API clients for wallet and deposit/withdraw status.

use crate::credentials;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

type HmacSha256 = Hmac<Sha256>;

/// Asset balance on an exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalance {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
    pub total: f64,
    pub usd_value: Option<f64>,
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

/// Complete wallet info for an exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeWalletInfo {
    pub exchange: String,
    pub balances: Vec<AssetBalance>,
    pub wallet_status: Vec<AssetWalletStatus>,
    pub last_updated: u64,
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ============================================================================
// Binance Client
// ============================================================================

#[derive(Debug, Deserialize)]
struct BinanceBalance {
    asset: String,
    free: String,
    locked: String,
}

#[derive(Debug, Deserialize)]
struct BinanceAccountInfo {
    balances: Vec<BinanceBalance>,
}

#[derive(Debug, Deserialize)]
struct BinanceNetwork {
    network: String,
    name: String,
    #[serde(rename = "depositEnable")]
    deposit_enable: bool,
    #[serde(rename = "withdrawEnable")]
    withdraw_enable: bool,
    #[serde(rename = "withdrawMin")]
    withdraw_min: String,
    #[serde(rename = "withdrawFee")]
    withdraw_fee: String,
    #[serde(rename = "minConfirm")]
    min_confirm: u32,
}

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

pub async fn fetch_binance_wallet() -> Result<ExchangeWalletInfo, String> {
    let creds = credentials::load_credentials();
    if creds.binance.api_key.is_empty() || creds.binance.secret_key.is_empty() {
        return Err("Binance API credentials not configured".to_string());
    }

    let client = Client::new();
    let timestamp = timestamp_ms();

    // Fetch account balances
    let query = format!("timestamp={}", timestamp);
    let signature = sign_binance(&query, &creds.binance.secret_key);
    let url = format!(
        "https://api.binance.com/api/v3/account?{}&signature={}",
        query, signature
    );

    let account_resp = client
        .get(&url)
        .header("X-MBX-APIKEY", &creds.binance.api_key)
        .send()
        .await
        .map_err(|e| format!("Binance account request failed: {}", e))?;

    if !account_resp.status().is_success() {
        let error_text = account_resp.text().await.unwrap_or_default();
        return Err(format!("Binance account API error: {}", error_text));
    }

    let account: BinanceAccountInfo = account_resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Binance account: {}", e))?;

    // Fetch coin info (deposit/withdraw status)
    let query = format!("timestamp={}", timestamp_ms());
    let signature = sign_binance(&query, &creds.binance.secret_key);
    let url = format!(
        "https://api.binance.com/sapi/v1/capital/config/getall?{}&signature={}",
        query, signature
    );

    let coins_resp = client
        .get(&url)
        .header("X-MBX-APIKEY", &creds.binance.api_key)
        .send()
        .await
        .map_err(|e| format!("Binance coins request failed: {}", e))?;

    let wallet_status = if coins_resp.status().is_success() {
        let coins: Vec<BinanceCoinInfo> = coins_resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Binance coins: {}", e))?;

        coins
            .into_iter()
            .map(|c| AssetWalletStatus {
                asset: c.coin.clone(),
                name: c.name,
                networks: c
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
                    .collect(),
                can_deposit: c.deposit_all_enable,
                can_withdraw: c.withdraw_all_enable,
            })
            .collect()
    } else {
        warn!("Failed to fetch Binance coin info");
        Vec::new()
    };

    let balances: Vec<AssetBalance> = account
        .balances
        .into_iter()
        .filter_map(|b| {
            let free: f64 = b.free.parse().unwrap_or(0.0);
            let locked: f64 = b.locked.parse().unwrap_or(0.0);
            let total = free + locked;
            if total > 0.0 {
                Some(AssetBalance {
                    asset: b.asset,
                    free,
                    locked,
                    total,
                    usd_value: None,
                })
            } else {
                None
            }
        })
        .collect();

    info!("Fetched Binance wallet: {} balances, {} assets with status",
          balances.len(), wallet_status.len());

    Ok(ExchangeWalletInfo {
        exchange: "Binance".to_string(),
        balances,
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

fn sign_binance(query: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// ============================================================================
// Upbit Client
// ============================================================================

#[derive(Debug, Deserialize)]
struct UpbitAccount {
    currency: String,
    balance: String,
    locked: String,
}

/// Upbit public network/wallet status (no auth required)
#[derive(Debug, Deserialize)]
struct UpbitNetworkStatus {
    currency: String,
    wallet_state: String,
    net_type: String,
    network_name: String,
}

/// Fetch Upbit wallet status (public API - no auth required)
pub async fn fetch_upbit_wallet_status() -> Result<Vec<AssetWalletStatus>, String> {
    let client = Client::new();

    // Use public API endpoint (no auth required)
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
    use std::collections::HashMap;
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

            // Asset can_deposit/can_withdraw = true if ANY network supports it
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

    info!("Fetched Upbit wallet status: {} assets", wallet_status.len());
    Ok(wallet_status)
}

/// Fetch Upbit wallet with balances (requires auth) and status (public)
pub async fn fetch_upbit_wallet() -> Result<ExchangeWalletInfo, String> {
    let creds = credentials::load_credentials();
    let client = Client::new();

    // Always fetch wallet status first (public API - no auth required)
    let wallet_status = fetch_upbit_wallet_status().await.unwrap_or_else(|e| {
        warn!("Failed to fetch Upbit wallet status: {}", e);
        Vec::new()
    });

    // Try to fetch balances if credentials are available
    let balances = if !creds.upbit.api_key.is_empty() && !creds.upbit.secret_key.is_empty() {
        let token = generate_upbit_token(&creds.upbit.api_key, &creds.upbit.secret_key, None)?;

        let accounts_resp = client
            .get("https://api.upbit.com/v1/accounts")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Upbit accounts request failed: {}", e))?;

        if accounts_resp.status().is_success() {
            let accounts: Vec<UpbitAccount> = accounts_resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse Upbit accounts: {}", e))?;

            accounts
                .into_iter()
                .filter_map(|a| {
                    let free: f64 = a.balance.parse().unwrap_or(0.0);
                    let locked: f64 = a.locked.parse().unwrap_or(0.0);
                    let total = free + locked;
                    if total > 0.0 {
                        Some(AssetBalance {
                            asset: a.currency,
                            free,
                            locked,
                            total,
                            usd_value: None,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            warn!("Failed to fetch Upbit balances");
            Vec::new()
        }
    } else {
        // No credentials - return empty balances but still have wallet status
        Vec::new()
    };

    info!(
        "Fetched Upbit wallet: {} balances, {} assets with status",
        balances.len(),
        wallet_status.len()
    );

    Ok(ExchangeWalletInfo {
        exchange: "Upbit".to_string(),
        balances,
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

fn generate_upbit_token(
    access_key: &str,
    secret_key: &str,
    _query: Option<&str>,
) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    // Use nanoseconds + random component for unique nonce
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    let nonce = format!("{}{:06}", now.as_millis(), now.subsec_nanos() % 1_000_000);

    // Simple JWT structure for Upbit
    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let payload = format!(
        r#"{{"access_key":"{}","nonce":"{}"}}"#,
        access_key,
        nonce
    );

    let header_b64 = STANDARD.encode(header);
    let payload_b64 = STANDARD.encode(&payload);
    let message = format!("{}.{}", header_b64, payload_b64);

    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
        .map_err(|e| format!("HMAC error: {}", e))?;
    mac.update(message.as_bytes());
    let signature = STANDARD.encode(mac.finalize().into_bytes());

    Ok(format!("{}.{}.{}", header_b64, payload_b64, signature))
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

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
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
pub async fn fetch_bithumb_wallet_status() -> Result<Vec<AssetWalletStatus>, String> {
    let creds = credentials::load_credentials();

    if creds.bithumb.api_key.is_empty() || creds.bithumb.secret_key.is_empty() {
        // No API keys - return empty status
        info!("Bithumb API keys not configured");
        return Ok(Vec::new());
    }

    let client = Client::new();

    // Generate JWT token for authentication
    let token = generate_bithumb_jwt(&creds.bithumb.api_key, &creds.bithumb.secret_key)?;

    // Call wallet status endpoint
    let resp = client
        .get("https://api.bithumb.com/v1/status/wallet")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Bithumb wallet status request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    info!("Bithumb wallet status API response: status={}, body_len={}, body_preview={}",
          status, body.len(), &body[..body.len().min(500)]);

    if !status.is_success() {
        return Err(format!("Bithumb wallet status API error: status={}, body={}", status, body));
    }

    // Response is a direct array, not wrapped in {status, data}
    let assets: Vec<BithumbWalletAsset> = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Bithumb wallet status: {}", e))?;

    info!("Bithumb parsed {} raw assets from API", assets.len());

    // Group by currency
    let mut currency_map: std::collections::HashMap<String, Vec<BithumbWalletAsset>> =
        std::collections::HashMap::new();
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

    info!(
        "Fetched Bithumb wallet status: {} assets",
        wallet_status.len()
    );
    Ok(wallet_status)
}

/// Bithumb account response
#[derive(Debug, Deserialize)]
struct BithumbAccount {
    currency: String,
    balance: String,
    locked: String,
    #[allow(dead_code)]
    avg_buy_price: Option<String>,
    #[allow(dead_code)]
    avg_buy_price_modified: Option<bool>,
    #[allow(dead_code)]
    unit_currency: Option<String>,
}

/// Fetch Bithumb balances (requires JWT auth)
async fn fetch_bithumb_balances() -> Result<Vec<AssetBalance>, String> {
    let creds = credentials::load_credentials();

    if creds.bithumb.api_key.is_empty() || creds.bithumb.secret_key.is_empty() {
        info!("Bithumb API keys not configured for balance query");
        return Ok(Vec::new());
    }

    let client = Client::new();
    let token = generate_bithumb_jwt(&creds.bithumb.api_key, &creds.bithumb.secret_key)?;

    let resp = client
        .get("https://api.bithumb.com/v1/accounts")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Bithumb accounts request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    info!("Bithumb accounts API response: status={}, body_preview={}",
          status, &body[..body.len().min(500)]);

    if !status.is_success() {
        return Err(format!("Bithumb accounts API error: status={}, body={}", status, body));
    }

    let accounts: Vec<BithumbAccount> = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Bithumb accounts: {}", e))?;

    let balances: Vec<AssetBalance> = accounts
        .into_iter()
        .filter_map(|a| {
            let free: f64 = a.balance.parse().unwrap_or(0.0);
            let locked: f64 = a.locked.parse().unwrap_or(0.0);
            let total = free + locked;
            if total > 0.0 {
                Some(AssetBalance {
                    asset: a.currency,
                    free,
                    locked,
                    total,
                    usd_value: None,
                })
            } else {
                None
            }
        })
        .collect();

    info!("Bithumb fetched {} balances with value", balances.len());
    Ok(balances)
}

/// Fetch Bithumb wallet with balances and status
pub async fn fetch_bithumb_wallet() -> Result<ExchangeWalletInfo, String> {
    let creds = credentials::load_credentials();

    // Fetch wallet status first
    let wallet_status = fetch_bithumb_wallet_status().await.unwrap_or_else(|e| {
        warn!("Failed to fetch Bithumb wallet status: {}", e);
        Vec::new()
    });

    // Fetch balances if credentials are available
    let balances = if !creds.bithumb.api_key.is_empty() && !creds.bithumb.secret_key.is_empty() {
        fetch_bithumb_balances().await.unwrap_or_else(|e| {
            warn!("Failed to fetch Bithumb balances: {}", e);
            Vec::new()
        })
    } else {
        Vec::new()
    };

    info!(
        "Fetched Bithumb wallet: {} balances, {} assets with status",
        balances.len(),
        wallet_status.len()
    );

    Ok(ExchangeWalletInfo {
        exchange: "Bithumb".to_string(),
        balances,
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Coinbase Client
// ============================================================================

/// Coinbase Exchange API currency response
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
    #[serde(default)]
    max_withdrawal_amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CoinbaseNetwork {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    min_withdrawal_amount: Option<f64>,
    #[serde(default)]
    max_withdrawal_amount: Option<f64>,
    #[serde(default)]
    network_confirmations: Option<u32>,
}

/// Fetch Coinbase wallet status (public Exchange API - no auth required)
pub async fn fetch_coinbase_wallet_status() -> Result<Vec<AssetWalletStatus>, String> {
    let client = Client::new();

    // Use public Exchange API endpoint (no auth required)
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

    // Filter to crypto only and build wallet status
    let wallet_status: Vec<AssetWalletStatus> = currencies
        .into_iter()
        .filter(|c| {
            // Filter to crypto currencies only (those with supported_networks)
            !c.supported_networks.is_empty()
                || c.details
                    .as_ref()
                    .and_then(|d| d.currency_type.as_ref())
                    .map(|t| t == "crypto")
                    .unwrap_or(false)
        })
        .map(|c| {
            let network_statuses: Vec<NetworkStatus> = if c.supported_networks.is_empty() {
                // Single network (legacy format)
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
                // Multiple networks
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

    info!(
        "Fetched Coinbase wallet status: {} assets",
        wallet_status.len()
    );
    Ok(wallet_status)
}

/// Coinbase Advanced Trade API account response (Legacy Key format)
#[derive(Debug, Deserialize)]
struct CoinbaseAccountsResponse {
    accounts: Vec<CoinbaseAccount>,
}

#[derive(Debug, Deserialize)]
struct CoinbaseAccount {
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    currency: String,
    #[serde(default)]
    available_balance: CoinbaseBalance,
    #[serde(default)]
    hold: CoinbaseBalance,
}

#[derive(Debug, Deserialize, Default)]
struct CoinbaseBalance {
    #[serde(default)]
    value: String,
    #[serde(default)]
    currency: String,
}

/// Generate JWT token for Coinbase App API (ES256/ECDSA signing)
/// Documentation: https://docs.cdp.coinbase.com/coinbase-app/authentication-authorization/api-key-authentication
fn generate_coinbase_cdp_jwt(
    key_name: &str,
    secret_key_pem: &str,
    method: &str,
    request_path: &str,
) -> Result<String, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use p256::ecdsa::{SigningKey, Signature, signature::Signer};
    use p256::pkcs8::DecodePrivateKey;
    use p256::SecretKey;

    // Log key format for debugging (first 50 chars only)
    info!("Coinbase secret key format: starts_with={:?}, len={}",
          &secret_key_pem.chars().take(50).collect::<String>(),
          secret_key_pem.len());

    // Parse EC private key from PEM format (ES256 = P-256/secp256r1)
    // Try SEC1 format first (-----BEGIN EC PRIVATE KEY-----), then PKCS#8 (-----BEGIN PRIVATE KEY-----)
    let signing_key = if secret_key_pem.contains("EC PRIVATE KEY") {
        SecretKey::from_sec1_pem(secret_key_pem)
            .map(|sk| SigningKey::from(&sk))
            .map_err(|e| format!("Failed to parse SEC1 EC private key: {}. Key preview: {:?}", e, &secret_key_pem.chars().take(100).collect::<String>()))?
    } else if secret_key_pem.contains("PRIVATE KEY") {
        SigningKey::from_pkcs8_pem(secret_key_pem)
            .map_err(|e| format!("Failed to parse PKCS#8 private key: {}", e))?
    } else {
        return Err(format!("Invalid key format. Expected PEM format but got: {:?}", &secret_key_pem.chars().take(50).collect::<String>()));
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Generate random nonce (32 bytes hex string)
    let nonce = format!("{:016x}{:016x}", rand::random::<u64>(), rand::random::<u64>());

    // JWT Header: {"alg": "ES256", "typ": "JWT", "kid": key_name, "nonce": nonce}
    let header = serde_json::json!({
        "alg": "ES256",
        "typ": "JWT",
        "kid": key_name,
        "nonce": nonce
    });

    // URI for the request: "{METHOD} {HOST}{PATH}"
    let uri = format!("{} api.coinbase.com{}", method, request_path);

    // JWT Payload
    let payload = serde_json::json!({
        "iss": "cdp",
        "sub": key_name,
        "nbf": now,
        "exp": now + 120, // 120 seconds expiry
        "uri": uri
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
    let message = format!("{}.{}", header_b64, payload_b64);

    // Sign with ES256 (ECDSA P-256)
    let signature: Signature = signing_key.sign(message.as_bytes());
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    Ok(format!("{}.{}.{}", header_b64, payload_b64, signature_b64))
}

/// Fetch Coinbase balances (CDP API with ES256 JWT auth)
async fn fetch_coinbase_balances() -> Result<Vec<AssetBalance>, String> {
    let creds = credentials::load_credentials();

    if !creds.coinbase.is_configured() {
        info!("Coinbase API keys not configured");
        return Ok(Vec::new());
    }

    let client = Client::new();
    let path = "/api/v3/brokerage/accounts";

    // Generate CDP JWT token with ES256 signature
    let jwt_token = generate_coinbase_cdp_jwt(
        &creds.coinbase.key_name(),
        &creds.coinbase.secret_key,
        "GET",
        path,
    )?;

    let resp = client
        .get(format!("https://api.coinbase.com{}", path))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Coinbase accounts request failed: {}", e))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    info!(
        "Coinbase accounts API response: status={}, body_preview={}",
        status,
        &body_text[..body_text.len().min(500)]
    );

    if !status.is_success() {
        return Err(format!(
            "Coinbase accounts API error: status={}, body={}",
            status, body_text
        ));
    }

    let response: CoinbaseAccountsResponse = serde_json::from_str(&body_text)
        .map_err(|e| format!("Failed to parse Coinbase accounts: {}", e))?;

    let balances: Vec<AssetBalance> = response.accounts
        .into_iter()
        .filter_map(|a| {
            let free: f64 = a.available_balance.value.parse().unwrap_or(0.0);
            let locked: f64 = a.hold.value.parse().unwrap_or(0.0);
            let total = free + locked;

            if total > 0.0 {
                Some(AssetBalance {
                    asset: a.currency,
                    free,
                    locked,
                    total,
                    usd_value: None,
                })
            } else {
                None
            }
        })
        .collect();

    info!("Coinbase fetched {} balances with value", balances.len());
    Ok(balances)
}

/// Fetch Coinbase wallet with balances (CDP API with ES256 auth) and status
pub async fn fetch_coinbase_wallet() -> Result<ExchangeWalletInfo, String> {
    let creds = credentials::load_credentials();

    // Fetch wallet status (public API - no auth required)
    let wallet_status = fetch_coinbase_wallet_status().await.unwrap_or_else(|e| {
        warn!("Failed to fetch Coinbase wallet status: {}", e);
        Vec::new()
    });

    // Fetch balances if credentials are available
    let balances = if creds.coinbase.is_configured() {
        fetch_coinbase_balances().await.unwrap_or_else(|e| {
            warn!("Failed to fetch Coinbase balances: {}", e);
            Vec::new()
        })
    } else {
        Vec::new()
    };

    info!(
        "Fetched Coinbase wallet: {} balances, {} assets with status",
        balances.len(),
        wallet_status.len()
    );

    Ok(ExchangeWalletInfo {
        exchange: "Coinbase".to_string(),
        balances,
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Aggregate function
// ============================================================================

pub async fn fetch_all_wallets() -> Vec<ExchangeWalletInfo> {
    let mut results = Vec::new();

    // Fetch in parallel
    let (binance, upbit, bithumb, coinbase) = tokio::join!(
        fetch_binance_wallet(),
        fetch_upbit_wallet(),
        fetch_bithumb_wallet(),
        fetch_coinbase_wallet()
    );

    if let Ok(info) = binance {
        results.push(info);
    }
    if let Ok(info) = upbit {
        results.push(info);
    }
    if let Ok(info) = bithumb {
        results.push(info);
    }
    if let Ok(info) = coinbase {
        results.push(info);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_binance() {
        let query = "timestamp=1234567890";
        let secret = "test_secret";
        let sig = sign_binance(query, secret);
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64); // SHA256 hex = 64 chars
    }
}

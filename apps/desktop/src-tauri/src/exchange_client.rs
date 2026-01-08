//! Exchange API clients for wallet and deposit/withdraw status.

use crate::credentials;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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

    if !status.is_success() {
        return Err(format!("Bithumb wallet status API error: status={}, body={}", status, body));
    }

    // Response is a direct array, not wrapped in {status, data}
    let assets: Vec<BithumbWalletAsset> = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Bithumb wallet status: {}", e))?;

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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    #[serde(default)]
    uuid: String,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
// Bybit Client (V5 API with HMAC-SHA256 signing)
// ============================================================================

/// Bybit coin info for wallet status
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
    chain_deposit: String, // "0" or "1"
    #[serde(rename = "chainWithdraw", default)]
    chain_withdraw: String, // "0" or "1"
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

/// Bybit wallet balance response
#[derive(Debug, Deserialize)]
struct BybitBalanceResponse {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[allow(dead_code)]
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: BybitBalanceResult,
}

#[derive(Debug, Deserialize)]
struct BybitBalanceResult {
    list: Vec<BybitAccountBalance>,
}

#[derive(Debug, Deserialize)]
struct BybitAccountBalance {
    #[allow(dead_code)]
    #[serde(rename = "accountType")]
    account_type: String,
    coin: Vec<BybitCoinBalance>,
}

#[derive(Debug, Deserialize)]
struct BybitCoinBalance {
    coin: String,
    #[serde(rename = "walletBalance", default)]
    wallet_balance: String,
    #[serde(rename = "availableToWithdraw", default)]
    available_to_withdraw: String,
    #[serde(rename = "locked", default)]
    locked: String,
}

/// Sign request for Bybit V5 API (HMAC-SHA256)
fn sign_bybit(timestamp: u64, api_key: &str, recv_window: &str, query: &str, secret: &str) -> String {
    let sign_str = format!("{}{}{}{}", timestamp, api_key, recv_window, query);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(sign_str.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Fetch Bybit wallet status (coin info with deposit/withdraw status)
pub async fn fetch_bybit_wallet_status() -> Result<Vec<AssetWalletStatus>, String> {
    let creds = credentials::load_credentials();

    if creds.bybit.api_key.is_empty() || creds.bybit.secret_key.is_empty() {
        info!("Bybit API keys not configured");
        return Ok(Vec::new());
    }

    let client = Client::new();
    let timestamp = timestamp_ms();
    let recv_window = "5000";
    let query = "";

    let signature = sign_bybit(
        timestamp,
        &creds.bybit.api_key,
        recv_window,
        query,
        &creds.bybit.secret_key,
    );

    let resp = client
        .get("https://api.bybit.com/v5/asset/coin/query-info")
        .header("X-BAPI-API-KEY", &creds.bybit.api_key)
        .header("X-BAPI-SIGN", signature)
        .header("X-BAPI-TIMESTAMP", timestamp.to_string())
        .header("X-BAPI-RECV-WINDOW", recv_window)
        .send()
        .await
        .map_err(|e| format!("Bybit coin info request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Bybit coin info API error: status={}, body={}", status, body));
    }

    let response: BybitCoinInfoResponse = serde_json::from_str(&body)
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

    info!("Fetched Bybit wallet status: {} assets", wallet_status.len());
    Ok(wallet_status)
}

/// Bybit FUND account balance response
#[derive(Debug, Deserialize)]
struct BybitFundBalanceResponse {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[allow(dead_code)]
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: BybitFundBalanceResult,
}

#[derive(Debug, Deserialize)]
struct BybitFundBalanceResult {
    #[serde(default)]
    balance: Vec<BybitFundCoin>,
}

#[derive(Debug, Deserialize)]
struct BybitFundCoin {
    coin: String,
    #[serde(rename = "walletBalance", default)]
    wallet_balance: String,
    #[allow(dead_code)]
    #[serde(rename = "transferBalance", default)]
    transfer_balance: String,
}

/// Bybit EARN position response
#[derive(Debug, Deserialize)]
struct BybitEarnResponse {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[allow(dead_code)]
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: BybitEarnResult,
}

#[derive(Debug, Deserialize)]
struct BybitEarnResult {
    #[serde(default)]
    list: Vec<BybitEarnPosition>,
}

#[derive(Debug, Deserialize)]
struct BybitEarnPosition {
    coin: String,
    #[serde(default)]
    amount: String,
    #[serde(rename = "claimableYield", default)]
    claimable_yield: String,
}

/// Fetch Bybit EARN positions for a specific category
async fn fetch_bybit_earn_positions(
    client: &Client,
    creds: &credentials::Credentials,
    category: &str,
) -> Vec<(String, f64)> {
    let recv_window = "5000";
    let timestamp = timestamp_ms();
    let query = format!("category={}", category);

    let signature = sign_bybit(
        timestamp,
        &creds.bybit.api_key,
        recv_window,
        &query,
        &creds.bybit.secret_key,
    );

    let resp = client
        .get(format!("https://api.bybit.com/v5/earn/position?{}", query))
        .header("X-BAPI-API-KEY", &creds.bybit.api_key)
        .header("X-BAPI-SIGN", &signature)
        .header("X-BAPI-TIMESTAMP", timestamp.to_string())
        .header("X-BAPI-RECV-WINDOW", recv_window)
        .send()
        .await;

    let mut positions = Vec::new();

    if let Ok(resp) = resp {
        if let Ok(body) = resp.text().await {
            if let Ok(response) = serde_json::from_str::<BybitEarnResponse>(&body) {
                if response.ret_code == 0 {
                    for pos in response.result.list {
                        let amount: f64 = pos.amount.parse().unwrap_or(0.0);
                        let yield_amount: f64 = pos.claimable_yield.parse().unwrap_or(0.0);
                        let total = amount + yield_amount;
                        if total > 0.0 {
                            positions.push((pos.coin, total));
                        }
                    }
                }
            }
        }
    }

    positions
}

/// Fetch Bybit balances (UNIFIED + FUND + EARN accounts)
async fn fetch_bybit_balances() -> Result<Vec<AssetBalance>, String> {
    let creds = credentials::load_credentials();

    if creds.bybit.api_key.is_empty() || creds.bybit.secret_key.is_empty() {
        info!("Bybit API keys not configured for balance query");
        return Ok(Vec::new());
    }

    let client = Client::new();
    let mut balances: Vec<AssetBalance> = Vec::new();

    // 1. Fetch UNIFIED account balance
    let timestamp = timestamp_ms();
    let recv_window = "5000";
    let query = "accountType=UNIFIED";

    let signature = sign_bybit(
        timestamp,
        &creds.bybit.api_key,
        recv_window,
        query,
        &creds.bybit.secret_key,
    );

    let resp = client
        .get(format!("https://api.bybit.com/v5/account/wallet-balance?{}", query))
        .header("X-BAPI-API-KEY", &creds.bybit.api_key)
        .header("X-BAPI-SIGN", &signature)
        .header("X-BAPI-TIMESTAMP", timestamp.to_string())
        .header("X-BAPI-RECV-WINDOW", recv_window)
        .send()
        .await
        .map_err(|e| format!("Bybit UNIFIED balance request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if status.is_success() {
        if let Ok(response) = serde_json::from_str::<BybitBalanceResponse>(&body) {
            if response.ret_code == 0 {
                for account in response.result.list {
                    for coin in account.coin {
                        let total: f64 = coin.wallet_balance.parse().unwrap_or(0.0);
                        let available: f64 = coin.available_to_withdraw.parse().unwrap_or(0.0);
                        let locked: f64 = coin.locked.parse().unwrap_or(0.0);

                        if total > 0.0 {
                            balances.push(AssetBalance {
                                asset: coin.coin,
                                free: available,
                                locked,
                                total,
                                usd_value: None,
                            });
                        }
                    }
                }
                info!("Bybit UNIFIED: {} balances", balances.len());
            }
        }
    }

    // 2. Fetch FUND account balance (funding wallet)
    let timestamp = timestamp_ms();
    let query = "accountType=FUND";

    let signature = sign_bybit(
        timestamp,
        &creds.bybit.api_key,
        recv_window,
        query,
        &creds.bybit.secret_key,
    );

    let resp = client
        .get(format!("https://api.bybit.com/v5/asset/transfer/query-account-coins-balance?{}", query))
        .header("X-BAPI-API-KEY", &creds.bybit.api_key)
        .header("X-BAPI-SIGN", &signature)
        .header("X-BAPI-TIMESTAMP", timestamp.to_string())
        .header("X-BAPI-RECV-WINDOW", recv_window)
        .send()
        .await
        .map_err(|e| format!("Bybit FUND balance request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if status.is_success() {
        if let Ok(response) = serde_json::from_str::<BybitFundBalanceResponse>(&body) {
            if response.ret_code == 0 {
                let mut fund_count = 0;
                for coin in response.result.balance {
                    let total: f64 = coin.wallet_balance.parse().unwrap_or(0.0);
                    if total > 0.0 {
                        // Check if already exists from UNIFIED
                        if !balances.iter().any(|b| b.asset == coin.coin) {
                            balances.push(AssetBalance {
                                asset: coin.coin,
                                free: total,
                                locked: 0.0,
                                total,
                                usd_value: None,
                            });
                            fund_count += 1;
                        }
                    }
                }
                info!("Bybit FUND: {} additional balances", fund_count);
            }
        }
    }

    // 3. Fetch EARN positions (FlexibleSaving + OnChain)
    let mut earn_count = 0;
    for category in ["FlexibleSaving", "OnChain"] {
        let positions = fetch_bybit_earn_positions(&client, &creds, category).await;
        for (coin, amount) in positions {
            // Add to existing balance or create new entry
            if let Some(existing) = balances.iter_mut().find(|b| b.asset == coin) {
                existing.locked += amount;
                existing.total += amount;
            } else {
                balances.push(AssetBalance {
                    asset: coin,
                    free: 0.0,
                    locked: amount,
                    total: amount,
                    usd_value: None,
                });
            }
            earn_count += 1;
        }
    }
    if earn_count > 0 {
        info!("Bybit EARN: {} positions added", earn_count);
    }

    info!("Bybit fetched {} total balances (UNIFIED + FUND + EARN)", balances.len());
    Ok(balances)
}

/// Fetch Bybit wallet with balances and status
pub async fn fetch_bybit_wallet() -> Result<ExchangeWalletInfo, String> {
    // Fetch wallet status first
    let wallet_status = fetch_bybit_wallet_status().await.unwrap_or_else(|e| {
        warn!("Failed to fetch Bybit wallet status: {}", e);
        Vec::new()
    });

    // Fetch balances
    let balances = fetch_bybit_balances().await.unwrap_or_else(|e| {
        warn!("Failed to fetch Bybit balances: {}", e);
        Vec::new()
    });

    info!(
        "Fetched Bybit wallet: {} balances, {} assets with status",
        balances.len(),
        wallet_status.len()
    );

    Ok(ExchangeWalletInfo {
        exchange: "Bybit".to_string(),
        balances,
        wallet_status,
        last_updated: timestamp_ms(),
    })
}

// ============================================================================
// Gate.io Client (API V4)
// ============================================================================

/// Sign Gate.io API request using HMAC-SHA512.
fn sign_gateio(timestamp: &str, method: &str, path: &str, query: &str, body: &str, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::{Sha512, Digest};

    // Body hash (SHA512 of body content)
    let body_hash = if body.is_empty() {
        // Empty body hash
        let mut hasher = Sha512::new();
        hasher.update(b"");
        hex::encode(hasher.finalize())
    } else {
        let mut hasher = Sha512::new();
        hasher.update(body.as_bytes());
        hex::encode(hasher.finalize())
    };

    // Signature string: METHOD\nPATH\nQUERY\nBODY_HASH\nTIMESTAMP
    let sign_string = format!("{}\n{}\n{}\n{}\n{}", method, path, query, body_hash, timestamp);

    let mut mac = Hmac::<Sha512>::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(sign_string.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Gate.io chain info
#[derive(Debug, Deserialize)]
struct GateIOChain {
    name: String,
    #[serde(default)]
    deposit_disabled: bool,
    #[serde(default)]
    withdraw_disabled: bool,
}

/// Gate.io wallet status response
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

/// Fetch Gate.io wallet status
pub async fn fetch_gateio_wallet_status() -> Result<Vec<AssetWalletStatus>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    // Public API - no auth required
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
                // Fallback if no chains info
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

            // Overall status: can if any network can
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

    info!("Fetched Gate.io wallet status: {} assets", wallet_status.len());
    Ok(wallet_status)
}

/// Gate.io balance response
#[derive(Debug, Deserialize)]
struct GateIOSpotAccount {
    currency: String,
    available: String,
    locked: String,
}

/// Fetch Gate.io balances
async fn fetch_gateio_balances() -> Result<Vec<AssetBalance>, String> {
    let creds = credentials::load_credentials();

    if creds.gateio.api_key.is_empty() || creds.gateio.secret_key.is_empty() {
        info!("Gate.io API keys not configured for balance query");
        return Ok(Vec::new());
    }

    let client = Client::new();
    let timestamp = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs())
    .to_string();

    let method = "GET";
    let path = "/api/v4/spot/accounts";
    let query = "";
    let body = "";

    let signature = sign_gateio(&timestamp, method, path, query, body, &creds.gateio.secret_key);

    let resp = client
        .get("https://api.gateio.ws/api/v4/spot/accounts")
        .header("KEY", &creds.gateio.api_key)
        .header("SIGN", signature)
        .header("Timestamp", &timestamp)
        .send()
        .await
        .map_err(|e| format!("Gate.io balance request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Gate.io balance API error: status={}, body={}", status, body));
    }

    let accounts: Vec<GateIOSpotAccount> = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Gate.io balance: {}", e))?;

    let balances: Vec<AssetBalance> = accounts
        .into_iter()
        .filter_map(|acc| {
            let available: f64 = acc.available.parse().unwrap_or(0.0);
            let locked: f64 = acc.locked.parse().unwrap_or(0.0);
            let total = available + locked;

            if total > 0.0 {
                Some(AssetBalance {
                    asset: acc.currency,
                    free: available,
                    locked,
                    total,
                    usd_value: None,
                })
            } else {
                None
            }
        })
        .collect();

    info!("Gate.io fetched {} balances with value", balances.len());
    Ok(balances)
}

/// Fetch Gate.io wallet with balances and status
pub async fn fetch_gateio_wallet() -> Result<ExchangeWalletInfo, String> {
    // Fetch wallet status first
    let wallet_status = fetch_gateio_wallet_status().await.unwrap_or_else(|e| {
        warn!("Failed to fetch Gate.io wallet status: {}", e);
        Vec::new()
    });

    // Fetch balances
    let balances = fetch_gateio_balances().await.unwrap_or_else(|e| {
        warn!("Failed to fetch Gate.io balances: {}", e);
        Vec::new()
    });

    info!(
        "Fetched Gate.io wallet: {} balances, {} assets with status",
        balances.len(),
        wallet_status.len()
    );

    Ok(ExchangeWalletInfo {
        exchange: "GateIO".to_string(),
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
    let (binance, upbit, bithumb, coinbase, bybit, gateio) = tokio::join!(
        fetch_binance_wallet(),
        fetch_upbit_wallet(),
        fetch_bithumb_wallet(),
        fetch_coinbase_wallet(),
        fetch_bybit_wallet(),
        fetch_gateio_wallet()
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
    if let Ok(info) = bybit {
        results.push(info);
    }
    if let Ok(info) = gateio {
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

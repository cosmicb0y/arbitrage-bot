//! USD/KRW exchange rate fetching.
//!
//! Fetches real-time exchange rate from Stockplus API (하나은행 기준).

use crate::ws_server::{self, BroadcastSender};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tracing::{info, warn};

/// Atomic storage for exchange rate (stored as rate * 100 for 2 decimal precision)
/// e.g., 1350.25 stored as 135025
static EXCHANGE_RATE: AtomicU64 = AtomicU64::new(0); // 0 means not loaded yet

/// Whether the exchange rate has been loaded from API at least once.
static RATE_LOADED: AtomicBool = AtomicBool::new(false);

/// API-based USD/KRW rate (from exchange rate API, not Upbit USDT)
static API_RATE: AtomicU64 = AtomicU64::new(0);

/// Check if exchange rate has been loaded from API.
pub fn is_rate_loaded() -> bool {
    RATE_LOADED.load(Ordering::Relaxed)
}

/// Get current USD/KRW exchange rate.
/// Returns None if rate hasn't been loaded yet.
pub fn get_usd_krw_rate() -> Option<f64> {
    if !is_rate_loaded() {
        return None;
    }
    let rate = EXCHANGE_RATE.load(Ordering::Relaxed);
    Some(rate as f64 / 100.0)
}

/// Get API-based USD/KRW rate (from exchange rate API).
/// Returns None if not yet fetched.
pub fn get_api_rate() -> Option<f64> {
    let rate = API_RATE.load(Ordering::Relaxed);
    if rate == 0 {
        None
    } else {
        Some(rate as f64 / 100.0)
    }
}

/// Stockplus API response structure.
#[derive(Debug, Deserialize)]
struct StockplusResponse {
    assets: Vec<ForexAsset>,
}

/// Individual forex asset from Stockplus API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForexAsset {
    id: String,
    base_price: f64,
}

/// Fetch exchange rate from Stockplus API (하나은행 기준).
pub async fn fetch_exchange_rate() -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let url = "https://mweb-api.stockplus.com/api/assets.json?ids=FOREX-KRWUSD";

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response: StockplusResponse = client.get(url).send().await?.json().await?;

    // Find USD/KRW rate from response
    let usd_asset = response
        .assets
        .iter()
        .find(|a| a.id == "FOREX-KRWUSD")
        .ok_or("FOREX-KRWUSD not found in response")?;

    let rate = usd_asset.base_price;

    // Store as integer (rate * 100)
    let rate_int = (rate * 100.0) as u64;
    EXCHANGE_RATE.store(rate_int, Ordering::Relaxed);
    API_RATE.store(rate_int, Ordering::Relaxed);
    RATE_LOADED.store(true, Ordering::Relaxed);

    Ok(rate)
}

/// Run exchange rate updater loop.
/// Updates rate every 5 minutes.
pub async fn run_exchange_rate_updater(broadcast_tx: BroadcastSender, state: crate::state::SharedState) {
    loop {
        match fetch_exchange_rate().await {
            Ok(rate) => {
                tracing::debug!("Updated USD/KRW exchange rate: {:.2}", rate);
                // Broadcast exchange rate to all clients
                ws_server::broadcast_exchange_rate(&broadcast_tx, &state, rate);
            }
            Err(e) => {
                warn!("Failed to fetch exchange rate: {}", e);
            }
        }

        // Update every 5 minutes
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_usd_krw_rate() {
        EXCHANGE_RATE.store(1400_50, Ordering::Relaxed);
        RATE_LOADED.store(true, Ordering::Relaxed);
        let rate = get_usd_krw_rate();
        assert!(rate.is_some());
        assert!((rate.unwrap() - 1400.50).abs() < 0.01);
    }

    #[test]
    fn test_rate_not_loaded() {
        RATE_LOADED.store(false, Ordering::Relaxed);
        let rate = get_usd_krw_rate();
        assert!(rate.is_none());
    }
}

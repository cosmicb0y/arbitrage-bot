//! WTS (Web Trading System) Module
//!
//! Bloomberg Terminal 스타일 트레이딩 시스템의 Rust 백엔드

pub mod types;
pub mod upbit;

pub use types::*;
pub use upbit::{BalanceEntry, WtsApiResult};

use std::time::Instant;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// WTS 창을 열거나 이미 열린 경우 포커스합니다.
///
/// # Returns
/// - `Ok(())` - 창이 성공적으로 열리거나 포커스됨
/// - `Err(String)` - 창 생성/포커스 실패
#[tauri::command]
pub async fn wts_open_window(app: tauri::AppHandle) -> Result<(), String> {
    // 이미 열린 창이 있으면 포커스
    if let Some(window) = app.get_webview_window("wts") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // 새 창 생성 (AC #2: 최소 크기 1280x720, 기본 크기 1440x900)
    WebviewWindowBuilder::new(&app, "wts", WebviewUrl::App("/wts".into()))
        .title("WTS - Trading System")
        .inner_size(1440.0, 900.0)
        .min_inner_size(1280.0, 720.0)
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 거래소 API 연결 상태를 확인합니다.
///
/// # Arguments
/// * `exchange` - 거래소 이름 (e.g., "upbit")
///
/// # Returns
/// * `ConnectionCheckResult` - 연결 성공 여부, 레이턴시, 에러 메시지
#[tauri::command]
pub async fn wts_check_connection(exchange: String) -> ConnectionCheckResult {
    match exchange.as_str() {
        "upbit" => {
            let start = Instant::now();
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default();

            match client.get("https://api.upbit.com/v1/market/all").send().await {
                Ok(response) if response.status().is_success() => {
                    let latency = start.elapsed().as_millis() as u64;
                    ConnectionCheckResult {
                        success: true,
                        latency: Some(latency),
                        error: None,
                    }
                }
                Ok(response) => ConnectionCheckResult {
                    success: false,
                    latency: None,
                    error: Some(format!("HTTP {}", response.status())),
                },
                Err(e) => ConnectionCheckResult {
                    success: false,
                    latency: None,
                    error: Some(e.to_string()),
                },
            }
        }
        _ => ConnectionCheckResult {
            success: false,
            latency: None,
            error: Some(format!("Unsupported exchange: {}", exchange)),
        },
    }
}

/// Upbit 잔고를 조회합니다.
///
/// # Returns
/// * `WtsApiResult<Vec<BalanceEntry>>` - 성공 시 잔고 목록, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_get_balance() -> WtsApiResult<Vec<BalanceEntry>> {
    match upbit::get_balance().await {
        Ok(balances) => WtsApiResult::ok(balances),
        Err(e) => WtsApiResult::err(e),
    }
}

//! WTS (Web Trading System) Module
//!
//! Bloomberg Terminal 스타일 트레이딩 시스템의 Rust 백엔드

pub mod types;

pub use types::*;

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

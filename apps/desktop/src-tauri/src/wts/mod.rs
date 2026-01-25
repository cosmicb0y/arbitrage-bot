//! WTS (Web Trading System) Module
//!
//! Bloomberg Terminal 스타일 트레이딩 시스템의 Rust 백엔드

pub mod types;
pub mod upbit;

pub use types::*;
pub use upbit::{
    BalanceEntry, DepositAddressParams, DepositAddressResponse, DepositChanceParams,
    DepositChanceResponse, GenerateAddressResponse, GetWithdrawParams, OrderParams, OrderResponse,
    UpbitMarket, WithdrawAddressResponse, WithdrawChanceParams, WithdrawChanceResponse,
    WithdrawParams, WithdrawResponse, WtsApiResult,
};

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
        "upbit" => match upbit::check_connection().await {
            Ok(latency) => ConnectionCheckResult {
                success: true,
                latency: Some(latency),
                error: None,
            },
            Err(e) => ConnectionCheckResult {
                success: false,
                latency: None,
                error: Some(e.to_korean_message()),
            },
        },
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

/// Upbit KRW 마켓 목록을 조회합니다.
///
/// # Returns
/// * `WtsApiResult<Vec<UpbitMarket>>` - 성공 시 마켓 목록, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_get_markets() -> WtsApiResult<Vec<UpbitMarket>> {
    match upbit::get_markets().await {
        Ok(markets) => WtsApiResult::ok(markets),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 주문을 실행합니다.
///
/// # Arguments
/// * `params` - 주문 파라미터 (market, side, volume, price, ord_type)
///
/// # Returns
/// * `WtsApiResult<OrderResponse>` - 성공 시 주문 결과, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_place_order(params: OrderParams) -> WtsApiResult<OrderResponse> {
    match upbit::place_order(params).await {
        Ok(order) => WtsApiResult::ok(order),
        Err(e) => WtsApiResult::err(e),
    }
}

// ============================================================================
// Deposit API Commands (WTS-4.1)
// ============================================================================

/// Upbit 입금 주소를 조회합니다.
///
/// # Arguments
/// * `params` - 입금 주소 조회 파라미터 (currency, net_type)
///
/// # Returns
/// * `WtsApiResult<DepositAddressResponse>` - 성공 시 입금 주소, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_get_deposit_address(
    params: DepositAddressParams,
) -> WtsApiResult<DepositAddressResponse> {
    match upbit::get_deposit_address(params).await {
        Ok(address) => WtsApiResult::ok(address),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 입금 주소를 생성합니다 (비동기).
///
/// # Arguments
/// * `params` - 입금 주소 생성 파라미터 (currency, net_type)
///
/// # Returns
/// * `WtsApiResult<GenerateAddressResponse>` - 성공 시 생성 상태/주소, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_generate_deposit_address(
    params: DepositAddressParams,
) -> WtsApiResult<GenerateAddressResponse> {
    match upbit::generate_deposit_address(params).await {
        Ok(response) => WtsApiResult::ok(response),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 입금 가능 정보를 조회합니다.
///
/// # Arguments
/// * `params` - 입금 가능 정보 조회 파라미터 (currency, net_type)
///
/// # Returns
/// * `WtsApiResult<DepositChanceResponse>` - 성공 시 입금 가능 정보, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_get_deposit_chance(
    params: DepositChanceParams,
) -> WtsApiResult<DepositChanceResponse> {
    match upbit::get_deposit_chance(params).await {
        Ok(chance) => WtsApiResult::ok(chance),
        Err(e) => WtsApiResult::err(e),
    }
}

// ============================================================================
// Withdraw API Commands (WTS-5.1)
// ============================================================================

/// Upbit 출금을 요청합니다.
///
/// # Arguments
/// * `params` - 출금 요청 파라미터 (currency, net_type, amount, address, ...)
///
/// # Returns
/// * `WtsApiResult<WithdrawResponse>` - 성공 시 출금 요청 결과, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_withdraw(params: WithdrawParams) -> WtsApiResult<WithdrawResponse> {
    match upbit::withdraw_coin(params).await {
        Ok(response) => WtsApiResult::ok(response),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 출금 가능 정보를 조회합니다.
///
/// # Arguments
/// * `params` - 출금 가능 정보 조회 파라미터 (currency, net_type)
///
/// # Returns
/// * `WtsApiResult<WithdrawChanceResponse>` - 성공 시 출금 가능 정보, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_get_withdraw_chance(
    params: WithdrawChanceParams,
) -> WtsApiResult<WithdrawChanceResponse> {
    match upbit::get_withdraw_chance(params).await {
        Ok(chance) => WtsApiResult::ok(chance),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit에 등록된 출금 허용 주소를 조회합니다.
///
/// # Arguments
/// * `params` - 출금 주소 조회 파라미터 (currency, net_type)
///
/// # Returns
/// * `WtsApiResult<Vec<WithdrawAddressResponse>>` - 성공 시 출금 주소 목록, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_get_withdraw_addresses(
    params: WithdrawChanceParams,
) -> WtsApiResult<Vec<WithdrawAddressResponse>> {
    match upbit::get_withdraw_addresses(params).await {
        Ok(addresses) => WtsApiResult::ok(addresses),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 출금 상태를 조회합니다.
///
/// # Arguments
/// * `params` - 출금 조회 파라미터 (uuid 또는 txid)
///
/// # Returns
/// * `WtsApiResult<WithdrawResponse>` - 성공 시 출금 상태, 실패 시 에러 정보
#[tauri::command]
pub async fn wts_get_withdraw(params: GetWithdrawParams) -> WtsApiResult<WithdrawResponse> {
    match upbit::get_withdraw(params).await {
        Ok(withdraw) => WtsApiResult::ok(withdraw),
        Err(e) => WtsApiResult::err(e),
    }
}

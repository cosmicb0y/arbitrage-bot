//! Upbit REST API Client
//!
//! Upbit 거래소 REST API 호출 클라이언트

use super::auth::{generate_jwt_token, generate_jwt_token_with_query};
use super::types::{
    BalanceEntry, DepositAddressParams, DepositAddressResponse, DepositChanceParams,
    DepositChanceResponse, GenerateAddressResponse, GetWithdrawParams, OrderParams, OrderResponse,
    UpbitApiError, UpbitMarket, WithdrawAddressResponse, WithdrawChanceParams,
    WithdrawChanceResponse, WithdrawParams, WithdrawResponse,
};
use std::collections::VecDeque;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Upbit API 기본 URL
const UPBIT_API_BASE: &str = "https://api.upbit.com/v1";
const ORDER_RATE_LIMIT: usize = 8;
const ORDER_RATE_WINDOW: Duration = Duration::from_secs(1);

fn order_rate_limiter() -> &'static tokio::sync::Mutex<VecDeque<Instant>> {
    static LIMITER: OnceLock<tokio::sync::Mutex<VecDeque<Instant>>> = OnceLock::new();
    LIMITER.get_or_init(|| tokio::sync::Mutex::new(VecDeque::new()))
}

async fn enforce_order_rate_limit() {
    loop {
        let wait = {
            let mut timestamps = order_rate_limiter().lock().await;
            let now = Instant::now();
            while let Some(front) = timestamps.front() {
                if now.duration_since(*front) >= ORDER_RATE_WINDOW {
                    timestamps.pop_front();
                } else {
                    break;
                }
            }

            if timestamps.len() < ORDER_RATE_LIMIT {
                timestamps.push_back(now);
                None
            } else {
                let oldest = *timestamps.front().unwrap();
                let elapsed = now.duration_since(oldest);
                Some(ORDER_RATE_WINDOW.saturating_sub(elapsed))
            }
        };

        match wait {
            None => break,
            Some(duration) => {
                if duration.is_zero() {
                    continue;
                }
                tokio::time::sleep(duration).await;
            }
        }
    }
}

fn parse_upbit_error(
    status: reqwest::StatusCode,
    error_body: serde_json::Value,
) -> UpbitApiError {
    let code = error_body["error"]["name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let message = error_body["error"]["message"]
        .as_str()
        .unwrap_or(&format!("HTTP {}", status))
        .to_string();

    let message_lower = message.to_lowercase();
    let code_lower = code.to_lowercase();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || code_lower.contains("rate")
        || code_lower.contains("too_many")
        || message_lower.contains("too many api requests")
        || message_lower.contains("too many requests")
        || message_lower.contains("rate limit")
    {
        return UpbitApiError::RateLimitExceeded {
            remaining_req: None,
        };
    }

    UpbitApiError::ApiError { code, message }
}

fn extract_remaining_req(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get("Remaining-Req")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

/// 환경 변수에서 API 키를 로드합니다.
fn load_api_keys() -> Result<(String, String), UpbitApiError> {
    load_env();
    let access_key =
        std::env::var("UPBIT_ACCESS_KEY").map_err(|_| UpbitApiError::MissingApiKey)?;
    let secret_key =
        std::env::var("UPBIT_SECRET_KEY").map_err(|_| UpbitApiError::MissingApiKey)?;

    if access_key.is_empty() || secret_key.is_empty() {
        return Err(UpbitApiError::MissingApiKey);
    }

    Ok((access_key, secret_key))
}

#[cfg(not(test))]
fn load_env() {
    let _ = crate::credentials::load_credentials();
}

#[cfg(test)]
fn load_env() {}

/// Upbit 마켓 목록을 조회합니다 (KRW 마켓만).
///
/// # Returns
/// * `Ok(Vec<UpbitMarket>)` - KRW 마켓 목록
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn get_markets() -> Result<Vec<UpbitMarket>, UpbitApiError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .get(format!("{}/market/all", UPBIT_API_BASE))
        .query(&[("isDetails", "true")])
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    if !response.status().is_success() {
        let status = response.status();
        return Err(UpbitApiError::ApiError {
            code: "http_error".to_string(),
            message: format!("HTTP {}", status),
        });
    }

    let all_markets: Vec<UpbitMarket> = response
        .json()
        .await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))?;

    // KRW 마켓만 필터링
    let krw_markets: Vec<UpbitMarket> = all_markets
        .into_iter()
        .filter(|m| m.market.starts_with("KRW-"))
        .collect();

    Ok(krw_markets)
}

/// Upbit 잔고를 조회합니다.
///
/// # Returns
/// * `Ok(Vec<BalanceEntry>)` - 자산별 잔고 목록
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn get_balance() -> Result<Vec<BalanceEntry>, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    let token =
        generate_jwt_token(&access_key, &secret_key).map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .get(format!("{}/accounts", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // Rate Limit 체크 (HTTP 429)
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 에러 응답 처리
    if !response.status().is_success() {
        let status = response.status();
        let error_body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    response
        .json::<Vec<BalanceEntry>>()
        .await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}

/// Upbit 주문을 실행합니다.
///
/// # Arguments
/// * `params` - 주문 파라미터 (market, side, volume, price, ord_type)
///
/// # Returns
/// * `Ok(OrderResponse)` - 주문 결과
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn place_order(params: OrderParams) -> Result<OrderResponse, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // JSON 바디 생성
    let body = serde_json::to_string(&params)
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))?;

    // Rate Limit(8회/초) 준수
    enforce_order_rate_limit().await;

    // 쿼리 해시 포함 JWT 생성
    let token = generate_jwt_token_with_query(&access_key, &secret_key, &body)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .post(format!("{}/orders", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // Rate Limit 체크 (HTTP 429)
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 에러 응답 처리
    if !response.status().is_success() {
        let status = response.status();
        let error_body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    response
        .json::<OrderResponse>()
        .await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}

// ============================================================================
// Deposit API Functions (WTS-4.1)
// ============================================================================

/// Upbit 입금 주소를 조회합니다.
///
/// # Arguments
/// * `params` - 입금 주소 조회 파라미터 (currency, net_type)
///
/// # Returns
/// * `Ok(DepositAddressResponse)` - 입금 주소 정보 (주소가 없으면 deposit_address: None)
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn get_deposit_address(
    params: DepositAddressParams,
) -> Result<DepositAddressResponse, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // 쿼리 문자열 생성
    let query = format!("currency={}&net_type={}", params.currency, params.net_type);

    eprintln!(
        "[DEBUG] get_deposit_address: currency={}, net_type={}",
        params.currency, params.net_type
    );

    // 쿼리 해시 포함 JWT 생성
    let token = generate_jwt_token_with_query(&access_key, &secret_key, &query)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let url = format!("{}/deposits/coin_address", UPBIT_API_BASE);
    eprintln!("[DEBUG] get_deposit_address: requesting URL={}", url);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("currency", &params.currency), ("net_type", &params.net_type)])
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let status = response.status();
    eprintln!("[DEBUG] get_deposit_address: HTTP status={}", status);

    // Rate Limit 체크 (HTTP 429)
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 응답 본문을 먼저 텍스트로 읽어서 디버깅
    let body_text = response
        .text()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    eprintln!(
        "[DEBUG] get_deposit_address: raw response body (first 1000 chars):\n{}",
        &body_text[..body_text.len().min(1000)]
    );

    // 에러 응답 처리
    if !status.is_success() {
        let error_body: serde_json::Value =
            serde_json::from_str(&body_text).unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    serde_json::from_str::<DepositAddressResponse>(&body_text).map_err(|e| {
        eprintln!(
            "[DEBUG] get_deposit_address: parse error: {}\nBody was: {}",
            e, body_text
        );
        UpbitApiError::ParseError(e.to_string())
    })
}

/// Upbit 입금 주소를 생성합니다 (비동기).
///
/// # Arguments
/// * `params` - 입금 주소 생성 파라미터 (currency, net_type)
///
/// # Returns
/// * `Ok(GenerateAddressResponse)` - 생성 중 상태 또는 이미 존재하는 주소
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn generate_deposit_address(
    params: DepositAddressParams,
) -> Result<GenerateAddressResponse, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // JSON 바디 생성
    let body = serde_json::to_string(&params)
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))?;

    // 쿼리 해시 포함 JWT 생성
    let token = generate_jwt_token_with_query(&access_key, &secret_key, &body)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .post(format!("{}/deposits/generate_coin_address", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // Rate Limit 체크 (HTTP 429)
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 에러 응답 처리
    if !response.status().is_success() {
        let status = response.status();
        let error_body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    response
        .json::<GenerateAddressResponse>()
        .await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}

/// Upbit 입금 가능 정보를 조회합니다.
///
/// # Arguments
/// * `params` - 입금 가능 정보 조회 파라미터 (currency, net_type)
///
/// # Returns
/// * `Ok(DepositChanceResponse)` - 입금 가능 정보
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn get_deposit_chance(
    params: DepositChanceParams,
) -> Result<DepositChanceResponse, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // 쿼리 문자열 생성
    let query = format!("currency={}&net_type={}", params.currency, params.net_type);

    eprintln!(
        "[DEBUG] get_deposit_chance: currency={}, net_type={}",
        params.currency, params.net_type
    );

    // 쿼리 해시 포함 JWT 생성
    let token = generate_jwt_token_with_query(&access_key, &secret_key, &query)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let url = format!("{}/deposits/chance/coin", UPBIT_API_BASE);
    eprintln!("[DEBUG] get_deposit_chance: requesting URL={}", url);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("currency", &params.currency), ("net_type", &params.net_type)])
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let status = response.status();
    eprintln!("[DEBUG] get_deposit_chance: HTTP status={}", status);

    // Rate Limit 체크 (HTTP 429)
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 응답 본문을 먼저 텍스트로 읽어서 디버깅
    let body_text = response
        .text()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    eprintln!(
        "[DEBUG] get_deposit_chance: raw response body (first 1000 chars):\n{}",
        &body_text[..body_text.len().min(1000)]
    );

    // 에러 응답 처리
    if !status.is_success() {
        let error_body: serde_json::Value =
            serde_json::from_str(&body_text).unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    serde_json::from_str::<DepositChanceResponse>(&body_text).map_err(|e| {
        eprintln!(
            "[DEBUG] get_deposit_chance: parse error: {}\nBody was: {}",
            e, body_text
        );
        UpbitApiError::ParseError(e.to_string())
    })
}

// ============================================================================
// Withdraw API Functions (WTS-5.1)
// ============================================================================

/// Upbit 출금을 요청합니다.
///
/// # Arguments
/// * `params` - 출금 요청 파라미터 (currency, net_type, amount, address, ...)
///
/// # Returns
/// * `Ok(WithdrawResponse)` - 출금 요청 결과 (uuid, state 등)
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn withdraw_coin(params: WithdrawParams) -> Result<WithdrawResponse, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // JSON 바디 생성
    let body = serde_json::to_string(&params)
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))?;

    // 쿼리 해시 포함 JWT 생성 (바디 해시)
    let token = generate_jwt_token_with_query(&access_key, &secret_key, &body)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .post(format!("{}/withdraws/coin", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // Rate Limit 체크 (HTTP 429)
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 에러 응답 처리
    if !response.status().is_success() {
        let status = response.status();
        let error_body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    response
        .json::<WithdrawResponse>()
        .await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}

/// Upbit 출금 가능 정보를 조회합니다.
///
/// # Arguments
/// * `params` - 출금 가능 정보 조회 파라미터 (currency, net_type)
///
/// # Returns
/// * `Ok(WithdrawChanceResponse)` - 출금 가능 정보 (수수료, 한도, 지갑 상태 등)
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn get_withdraw_chance(
    params: WithdrawChanceParams,
) -> Result<WithdrawChanceResponse, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // 쿼리 문자열 생성
    let query = format!("currency={}&net_type={}", params.currency, params.net_type);

    // 쿼리 해시 포함 JWT 생성
    let token = generate_jwt_token_with_query(&access_key, &secret_key, &query)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .get(format!("{}/withdraws/chance", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("currency", &params.currency), ("net_type", &params.net_type)])
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // Rate Limit 체크 (HTTP 429)
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 에러 응답 처리
    if !response.status().is_success() {
        let status = response.status();
        let error_body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    response
        .json::<WithdrawChanceResponse>()
        .await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}

/// Upbit에 등록된 출금 허용 주소 목록을 조회합니다.
///
/// # Arguments
/// * `params` - 출금 주소 조회 파라미터 (currency, net_type)
///
/// # Returns
/// * `Ok(Vec<WithdrawAddressResponse>)` - 등록된 출금 주소 목록
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn get_withdraw_addresses(
    params: WithdrawChanceParams,
) -> Result<Vec<WithdrawAddressResponse>, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // 쿼리 문자열 생성
    let query = format!("currency={}&net_type={}", params.currency, params.net_type);

    // 쿼리 해시 포함 JWT 생성
    let token = generate_jwt_token_with_query(&access_key, &secret_key, &query)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .get(format!("{}/withdraws/coin_addresses", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("currency", &params.currency), ("net_type", &params.net_type)])
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // Rate Limit 체크 (HTTP 429)
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 에러 응답 처리
    if !response.status().is_success() {
        let status = response.status();
        let error_body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    response
        .json::<Vec<WithdrawAddressResponse>>()
        .await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}

/// Upbit 출금 상태를 조회합니다.
///
/// # Arguments
/// * `params` - 출금 조회 파라미터 (uuid 또는 txid)
///
/// # Returns
/// * `Ok(WithdrawResponse)` - 출금 상태 정보
/// * `Err(UpbitApiError)` - API 호출 실패 시 에러
pub async fn get_withdraw(params: GetWithdrawParams) -> Result<WithdrawResponse, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // 쿼리 문자열 생성 (uuid 또는 txid 중 하나 사용)
    let query = if let Some(uuid) = &params.uuid {
        format!("uuid={}", uuid)
    } else if let Some(txid) = &params.txid {
        format!("txid={}", txid)
    } else {
        return Err(UpbitApiError::ApiError {
            code: "invalid_params".to_string(),
            message: "uuid or txid is required".to_string(),
        });
    };

    // 쿼리 해시 포함 JWT 생성
    let token = generate_jwt_token_with_query(&access_key, &secret_key, &query)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let mut request = client
        .get(format!("{}/withdraw", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token));

    // 쿼리 파라미터 추가
    if let Some(uuid) = &params.uuid {
        request = request.query(&[("uuid", uuid)]);
    } else if let Some(txid) = &params.txid {
        request = request.query(&[("txid", txid)]);
    }

    let response = request
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // Rate Limit 체크 (HTTP 429)
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(UpbitApiError::RateLimitExceeded {
            remaining_req: extract_remaining_req(response.headers()),
        });
    }

    // 에러 응답 처리
    if !response.status().is_success() {
        let status = response.status();
        let error_body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        return Err(parse_upbit_error(status, error_body));
    }

    // 성공 응답 파싱
    response
        .json::<WithdrawResponse>()
        .await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}

/// Upbit API 연결 상태를 확인합니다.
pub async fn check_connection() -> Result<u64, UpbitApiError> {
    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .get(format!("{}/market/all", UPBIT_API_BASE))
        .query(&[("isDetails", "false")])
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    if response.status().is_success() {
        Ok(start.elapsed().as_millis() as u64)
    } else {
        Err(UpbitApiError::ApiError {
            code: "connection_failed".to_string(),
            message: format!("HTTP {}", response.status()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<R>(
        access_key: Option<&str>,
        secret_key: Option<&str>,
        f: impl FnOnce() -> R,
    ) -> R {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev_access = std::env::var("UPBIT_ACCESS_KEY").ok();
        let prev_secret = std::env::var("UPBIT_SECRET_KEY").ok();

        match access_key {
            Some(value) => std::env::set_var("UPBIT_ACCESS_KEY", value),
            None => std::env::remove_var("UPBIT_ACCESS_KEY"),
        }
        match secret_key {
            Some(value) => std::env::set_var("UPBIT_SECRET_KEY", value),
            None => std::env::remove_var("UPBIT_SECRET_KEY"),
        }

        let result = f();

        match prev_access {
            Some(value) => std::env::set_var("UPBIT_ACCESS_KEY", value),
            None => std::env::remove_var("UPBIT_ACCESS_KEY"),
        }
        match prev_secret {
            Some(value) => std::env::set_var("UPBIT_SECRET_KEY", value),
            None => std::env::remove_var("UPBIT_SECRET_KEY"),
        }

        result
    }

    #[test]
    fn test_load_api_keys_missing() {
        // 환경 변수가 없을 때 MissingApiKey 에러 반환
        let result = with_env(None, None, load_api_keys);
        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    #[test]
    fn test_load_api_keys_success() {
        // 환경 변수 설정
        let result = with_env(Some("test_access"), Some("test_secret"), load_api_keys);
        assert!(result.is_ok());

        let (access, secret) = result.unwrap();
        assert_eq!(access, "test_access");
        assert_eq!(secret, "test_secret");
    }

    #[test]
    fn test_load_api_keys_empty() {
        // 빈 문자열은 MissingApiKey로 처리
        let result = with_env(Some(""), Some("test"), load_api_keys);
        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    #[test]
    fn test_parse_upbit_error_validation_error() {
        let error_body = serde_json::json!({
            "error": {
                "name": "validation_error",
                "message": "필수 파라미터가 누락되었습니다"
            }
        });

        let err = parse_upbit_error(reqwest::StatusCode::BAD_REQUEST, error_body);
        match err {
            UpbitApiError::ApiError { code, message } => {
                assert_eq!(code, "validation_error");
                assert_eq!(message, "필수 파라미터가 누락되었습니다");
            }
            _ => panic!("ApiError expected"),
        }
    }

    #[test]
    fn test_parse_upbit_error_rate_limit() {
        let error_body = serde_json::json!({
            "error": {
                "name": "too_many_requests",
                "message": "Too many API requests."
            }
        });

        let err = parse_upbit_error(reqwest::StatusCode::TOO_MANY_REQUESTS, error_body);
        assert!(matches!(err, UpbitApiError::RateLimitExceeded { .. }));
    }

    // ============================================================================
    // Place Order Tests (API 키 없을 때 에러)
    // ============================================================================

    use super::super::types::{OrderSide, UpbitOrderType};

    #[tokio::test]
    async fn test_place_order_missing_api_key() {
        // API 키가 없을 때 MissingApiKey 에러 반환
        // with_env 내에서 환경변수만 설정/해제하고, async 호출은 바깥에서 수행
        let _guard = ENV_LOCK.lock().unwrap();

        // 환경 변수 제거
        std::env::remove_var("UPBIT_ACCESS_KEY");
        std::env::remove_var("UPBIT_SECRET_KEY");

        let params = OrderParams {
            market: "KRW-BTC".to_string(),
            side: OrderSide::Bid,
            volume: Some("0.01".to_string()),
            price: Some("100000000".to_string()),
            ord_type: UpbitOrderType::Limit,
        };
        let result = place_order(params).await;

        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    // ============================================================================
    // Deposit API Tests (WTS-4.1)
    // ============================================================================

    use super::super::types::{DepositAddressParams, DepositChanceParams};

    #[tokio::test]
    async fn test_get_deposit_address_missing_api_key() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("UPBIT_ACCESS_KEY");
        std::env::remove_var("UPBIT_SECRET_KEY");

        let params = DepositAddressParams {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
        };
        let result = get_deposit_address(params).await;

        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    #[tokio::test]
    async fn test_generate_deposit_address_missing_api_key() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("UPBIT_ACCESS_KEY");
        std::env::remove_var("UPBIT_SECRET_KEY");

        let params = DepositAddressParams {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
        };
        let result = generate_deposit_address(params).await;

        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    #[tokio::test]
    async fn test_get_deposit_chance_missing_api_key() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("UPBIT_ACCESS_KEY");
        std::env::remove_var("UPBIT_SECRET_KEY");

        let params = DepositChanceParams {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
        };
        let result = get_deposit_chance(params).await;

        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    // ============================================================================
    // Withdraw API Tests (WTS-5.1)
    // ============================================================================

    use super::super::types::{
        GetWithdrawParams, WithdrawChanceParams as WithdrawChanceParamsType, WithdrawParams,
    };

    #[tokio::test]
    async fn test_withdraw_coin_missing_api_key() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("UPBIT_ACCESS_KEY");
        std::env::remove_var("UPBIT_SECRET_KEY");

        let params = WithdrawParams {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
            amount: "0.01".to_string(),
            address: "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string(),
            secondary_address: None,
            transaction_type: None,
        };
        let result = withdraw_coin(params).await;

        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    #[tokio::test]
    async fn test_get_withdraw_chance_missing_api_key() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("UPBIT_ACCESS_KEY");
        std::env::remove_var("UPBIT_SECRET_KEY");

        let params = WithdrawChanceParamsType {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
        };
        let result = get_withdraw_chance(params).await;

        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    #[tokio::test]
    async fn test_get_withdraw_addresses_missing_api_key() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("UPBIT_ACCESS_KEY");
        std::env::remove_var("UPBIT_SECRET_KEY");

        let params = WithdrawChanceParamsType {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
        };
        let result = get_withdraw_addresses(params).await;

        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }

    #[tokio::test]
    async fn test_get_withdraw_missing_api_key() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("UPBIT_ACCESS_KEY");
        std::env::remove_var("UPBIT_SECRET_KEY");

        let params = GetWithdrawParams {
            uuid: Some("9f432943-54e0-40b7-825f-b6fec8b42b79".to_string()),
            txid: None,
        };
        let result = get_withdraw(params).await;

        assert!(matches!(result, Err(UpbitApiError::MissingApiKey)));
    }
}

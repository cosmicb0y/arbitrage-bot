//! Upbit REST API Client
//!
//! Upbit 거래소 REST API 호출 클라이언트

use super::auth::{generate_jwt_token, generate_jwt_token_with_query};
use super::types::{BalanceEntry, OrderParams, OrderResponse, UpbitApiError, UpbitMarket};
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
        return UpbitApiError::RateLimitExceeded;
    }

    UpbitApiError::ApiError { code, message }
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
        return Err(UpbitApiError::RateLimitExceeded);
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
        return Err(UpbitApiError::RateLimitExceeded);
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
        return Err(UpbitApiError::RateLimitExceeded);
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
        assert!(matches!(err, UpbitApiError::RateLimitExceeded));
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
}

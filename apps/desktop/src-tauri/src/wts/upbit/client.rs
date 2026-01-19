//! Upbit REST API Client
//!
//! Upbit 거래소 REST API 호출 클라이언트

use super::auth::generate_jwt_token;
use super::types::{BalanceEntry, UpbitApiError, UpbitMarket};

/// Upbit API 기본 URL
const UPBIT_API_BASE: &str = "https://api.upbit.com/v1";

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
            return Err(UpbitApiError::RateLimitExceeded);
        }

        return Err(UpbitApiError::ApiError { code, message });
    }

    // 성공 응답 파싱
    response
        .json::<Vec<BalanceEntry>>()
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
}

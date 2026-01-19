//! Upbit API Types
//!
//! Upbit REST API 응답 및 에러 타입 정의

use serde::{Deserialize, Serialize};

/// Upbit 마켓 정보 (API 응답)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpbitMarket {
    /// 마켓 코드 (예: "KRW-BTC")
    pub market: String,
    /// 한글명 (예: "비트코인")
    pub korean_name: String,
    /// 영문명 (예: "Bitcoin")
    pub english_name: String,
    /// 유의 종목 여부 (CAUTION, NONE 등)
    #[serde(default)]
    pub market_warning: Option<String>,
}

/// 잔고 엔트리 (Upbit API 응답)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceEntry {
    /// 화폐 코드 (예: "BTC", "KRW")
    pub currency: String,
    /// 가용 잔고
    pub balance: String,
    /// 잠금 잔고 (미체결 주문)
    pub locked: String,
    /// 평균 매수가
    pub avg_buy_price: String,
    /// 평균 매수가 수정 여부
    pub avg_buy_price_modified: bool,
    /// 평가 기준 화폐 (예: "KRW")
    pub unit_currency: String,
}

/// Upbit API 에러
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UpbitApiError {
    #[serde(rename = "missing_api_key")]
    MissingApiKey,
    #[serde(rename = "jwt_error")]
    JwtError(String),
    #[serde(rename = "network_error")]
    NetworkError(String),
    #[serde(rename = "rate_limit")]
    RateLimitExceeded,
    #[serde(rename = "api_error")]
    ApiError { code: String, message: String },
    #[serde(rename = "parse_error")]
    ParseError(String),
}

impl UpbitApiError {
    /// 한국어 에러 메시지 반환
    pub fn to_korean_message(&self) -> String {
        match self {
            Self::MissingApiKey => "API 키가 설정되지 않았습니다".to_string(),
            Self::JwtError(_) => "JWT 토큰 생성에 실패했습니다".to_string(),
            Self::NetworkError(_) => "네트워크 연결에 실패했습니다".to_string(),
            Self::RateLimitExceeded => {
                "요청이 너무 많습니다. 잠시 후 다시 시도하세요".to_string()
            }
            Self::ApiError { code, message } => match code.as_str() {
                "jwt_verification" => "JWT 인증에 실패했습니다".to_string(),
                "no_authorization_ip" => "허용되지 않은 IP입니다".to_string(),
                "expired_access_key" => "만료된 API 키입니다".to_string(),
                "validation_error" => "잘못된 요청입니다".to_string(),
                _ => message.clone(),
            },
            Self::ParseError(_) => "응답 파싱에 실패했습니다".to_string(),
        }
    }

    /// 에러 코드 문자열 반환
    pub fn code(&self) -> String {
        match self {
            Self::MissingApiKey => "missing_api_key".to_string(),
            Self::JwtError(_) => "jwt_error".to_string(),
            Self::NetworkError(_) => "network_error".to_string(),
            Self::RateLimitExceeded => "rate_limit".to_string(),
            Self::ApiError { code, .. } => code.clone(),
            Self::ParseError(_) => "parse_error".to_string(),
        }
    }
}

/// WTS API 응답 래퍼
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WtsApiResult<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WtsApiErrorResponse>,
}

/// WTS API 에러 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WtsApiErrorResponse {
    pub code: String,
    pub message: String,
}

impl<T> WtsApiResult<T> {
    /// 성공 응답 생성
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// 에러 응답 생성
    pub fn err(error: UpbitApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(WtsApiErrorResponse {
                code: error.code(),
                message: error.to_korean_message(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_entry_deserialize() {
        let json = r#"{
            "currency": "BTC",
            "balance": "0.12345678",
            "locked": "0.00000000",
            "avg_buy_price": "50000000.00",
            "avg_buy_price_modified": false,
            "unit_currency": "KRW"
        }"#;

        let entry: BalanceEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.currency, "BTC");
        assert_eq!(entry.balance, "0.12345678");
        assert_eq!(entry.locked, "0.00000000");
        assert_eq!(entry.avg_buy_price, "50000000.00");
        assert!(!entry.avg_buy_price_modified);
        assert_eq!(entry.unit_currency, "KRW");
    }

    #[test]
    fn test_error_korean_message() {
        let err = UpbitApiError::RateLimitExceeded;
        assert_eq!(
            err.to_korean_message(),
            "요청이 너무 많습니다. 잠시 후 다시 시도하세요"
        );

        let err = UpbitApiError::MissingApiKey;
        assert_eq!(err.to_korean_message(), "API 키가 설정되지 않았습니다");

        let err = UpbitApiError::ApiError {
            code: "jwt_verification".to_string(),
            message: "JWT verification failed".to_string(),
        };
        assert_eq!(err.to_korean_message(), "JWT 인증에 실패했습니다");

        let err = UpbitApiError::ApiError {
            code: "no_authorization_ip".to_string(),
            message: "IP not allowed".to_string(),
        };
        assert_eq!(err.to_korean_message(), "허용되지 않은 IP입니다");
    }

    #[test]
    fn test_wts_api_result_ok() {
        let result = WtsApiResult::ok(vec!["test".to_string()]);
        assert!(result.success);
        assert!(result.data.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_wts_api_result_err() {
        let result: WtsApiResult<Vec<BalanceEntry>> =
            WtsApiResult::err(UpbitApiError::MissingApiKey);
        assert!(!result.success);
        assert!(result.data.is_none());
        assert!(result.error.is_some());
        let error = result.error.unwrap();
        assert_eq!(error.code, "missing_api_key");
        assert_eq!(error.message, "API 키가 설정되지 않았습니다");
    }
}

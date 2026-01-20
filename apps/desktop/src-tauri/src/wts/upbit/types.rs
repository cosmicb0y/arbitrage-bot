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
                // 인증 관련 에러
                "jwt_verification" => "JWT 인증에 실패했습니다".to_string(),
                "no_authorization_ip" => "허용되지 않은 IP입니다".to_string(),
                "expired_access_key" => "만료된 API 키입니다".to_string(),
                "validation_error" => "잘못된 요청입니다".to_string(),
                // 주문 관련 에러 (AC #6, #7, #8)
                "insufficient_funds_bid" => "매수 가능 금액이 부족합니다".to_string(),
                "insufficient_funds_ask" => "매도 가능 수량이 부족합니다".to_string(),
                "under_min_total_bid" | "under_min_total_ask" => {
                    "최소 주문금액(5,000원) 이상이어야 합니다".to_string()
                }
                "invalid_volume" => "주문 수량이 올바르지 않습니다".to_string(),
                "invalid_price" => "주문 가격이 올바르지 않습니다".to_string(),
                "market_does_not_exist" => "존재하지 않는 마켓입니다".to_string(),
                "invalid_side" => "주문 방향이 올바르지 않습니다".to_string(),
                "invalid_ord_type" => "주문 유형이 올바르지 않습니다".to_string(),
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

// ============================================================================
// Order API Types (Upbit)
// ============================================================================

/// 주문 방향
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    /// 매수
    Bid,
    /// 매도
    Ask,
}

/// 주문 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpbitOrderType {
    /// 지정가 주문
    Limit,
    /// 시장가 매수 (총액 지정)
    Price,
    /// 시장가 매도 (수량 지정)
    Market,
}

/// 주문 요청 파라미터 (Tauri 명령 인자)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderParams {
    /// 마켓 코드 (예: "KRW-BTC")
    pub market: String,
    /// 주문 방향: bid(매수) | ask(매도)
    pub side: OrderSide,
    /// 주문 수량 (시장가 매도 또는 지정가)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// 주문 가격 (시장가 매수: 총액, 지정가: 단가)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    /// 주문 유형
    pub ord_type: UpbitOrderType,
}

/// 주문 응답 (Upbit API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    /// 주문 고유 ID
    pub uuid: String,
    /// 주문 방향
    pub side: String,
    /// 주문 유형
    pub ord_type: String,
    /// 주문 가격
    pub price: Option<String>,
    /// 주문 상태: wait, watch, done, cancel
    pub state: String,
    /// 마켓 코드
    pub market: String,
    /// 주문 생성 시각
    pub created_at: String,
    /// 주문 수량
    pub volume: Option<String>,
    /// 미체결 수량
    pub remaining_volume: Option<String>,
    /// 예약 수수료
    pub reserved_fee: String,
    /// 미사용 수수료
    pub remaining_fee: String,
    /// 지불 수수료
    pub paid_fee: String,
    /// 잠금 금액/수량
    pub locked: String,
    /// 체결 수량
    pub executed_volume: String,
    /// 체결 횟수
    pub trades_count: i32,
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
    fn test_order_error_korean_messages() {
        // 잔고 부족 에러
        let err = UpbitApiError::ApiError {
            code: "insufficient_funds_bid".to_string(),
            message: "Insufficient balance".to_string(),
        };
        assert_eq!(err.to_korean_message(), "매수 가능 금액이 부족합니다");

        let err = UpbitApiError::ApiError {
            code: "insufficient_funds_ask".to_string(),
            message: "Insufficient balance".to_string(),
        };
        assert_eq!(err.to_korean_message(), "매도 가능 수량이 부족합니다");

        // 최소 주문금액 에러
        let err = UpbitApiError::ApiError {
            code: "under_min_total_bid".to_string(),
            message: "Under min total".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "최소 주문금액(5,000원) 이상이어야 합니다"
        );

        let err = UpbitApiError::ApiError {
            code: "under_min_total_ask".to_string(),
            message: "Under min total".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "최소 주문금액(5,000원) 이상이어야 합니다"
        );

        // 잘못된 파라미터 에러
        let err = UpbitApiError::ApiError {
            code: "invalid_volume".to_string(),
            message: "Invalid volume".to_string(),
        };
        assert_eq!(err.to_korean_message(), "주문 수량이 올바르지 않습니다");

        let err = UpbitApiError::ApiError {
            code: "invalid_price".to_string(),
            message: "Invalid price".to_string(),
        };
        assert_eq!(err.to_korean_message(), "주문 가격이 올바르지 않습니다");

        // 마켓 에러
        let err = UpbitApiError::ApiError {
            code: "market_does_not_exist".to_string(),
            message: "Market not found".to_string(),
        };
        assert_eq!(err.to_korean_message(), "존재하지 않는 마켓입니다");
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

    // ============================================================================
    // Order Types Tests
    // ============================================================================

    #[test]
    fn test_order_side_serialize() {
        assert_eq!(serde_json::to_string(&OrderSide::Bid).unwrap(), r#""bid""#);
        assert_eq!(serde_json::to_string(&OrderSide::Ask).unwrap(), r#""ask""#);
    }

    #[test]
    fn test_order_side_deserialize() {
        let bid: OrderSide = serde_json::from_str(r#""bid""#).unwrap();
        let ask: OrderSide = serde_json::from_str(r#""ask""#).unwrap();
        assert_eq!(bid, OrderSide::Bid);
        assert_eq!(ask, OrderSide::Ask);
    }

    #[test]
    fn test_order_type_serialize() {
        assert_eq!(
            serde_json::to_string(&UpbitOrderType::Limit).unwrap(),
            r#""limit""#
        );
        assert_eq!(
            serde_json::to_string(&UpbitOrderType::Price).unwrap(),
            r#""price""#
        );
        assert_eq!(
            serde_json::to_string(&UpbitOrderType::Market).unwrap(),
            r#""market""#
        );
    }

    #[test]
    fn test_order_type_deserialize() {
        let limit: UpbitOrderType = serde_json::from_str(r#""limit""#).unwrap();
        let price: UpbitOrderType = serde_json::from_str(r#""price""#).unwrap();
        let market: UpbitOrderType = serde_json::from_str(r#""market""#).unwrap();
        assert_eq!(limit, UpbitOrderType::Limit);
        assert_eq!(price, UpbitOrderType::Price);
        assert_eq!(market, UpbitOrderType::Market);
    }

    #[test]
    fn test_order_params_serialize_limit() {
        // 지정가 주문: market, side, volume, price, ord_type 모두 필요
        let params = OrderParams {
            market: "KRW-BTC".to_string(),
            side: OrderSide::Bid,
            volume: Some("0.01".to_string()),
            price: Some("100000000".to_string()),
            ord_type: UpbitOrderType::Limit,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["market"], "KRW-BTC");
        assert_eq!(json["side"], "bid");
        assert_eq!(json["volume"], "0.01");
        assert_eq!(json["price"], "100000000");
        assert_eq!(json["ord_type"], "limit");
    }

    #[test]
    fn test_order_params_serialize_market_buy() {
        // 시장가 매수: ord_type="price", volume 없음, price에 총액
        let params = OrderParams {
            market: "KRW-BTC".to_string(),
            side: OrderSide::Bid,
            volume: None,
            price: Some("100000".to_string()),
            ord_type: UpbitOrderType::Price,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["market"], "KRW-BTC");
        assert_eq!(json["side"], "bid");
        assert!(json.get("volume").is_none()); // skip_serializing_if 동작 확인
        assert_eq!(json["price"], "100000");
        assert_eq!(json["ord_type"], "price");
    }

    #[test]
    fn test_order_params_serialize_market_sell() {
        // 시장가 매도: ord_type="market", price 없음, volume에 수량
        let params = OrderParams {
            market: "KRW-BTC".to_string(),
            side: OrderSide::Ask,
            volume: Some("0.01".to_string()),
            price: None,
            ord_type: UpbitOrderType::Market,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["market"], "KRW-BTC");
        assert_eq!(json["side"], "ask");
        assert_eq!(json["volume"], "0.01");
        assert!(json.get("price").is_none()); // skip_serializing_if 동작 확인
        assert_eq!(json["ord_type"], "market");
    }

    #[test]
    fn test_order_response_deserialize() {
        let json = r#"{
            "uuid": "cdd92199-2897-4e14-9b66-51bd59fce35e",
            "side": "bid",
            "ord_type": "limit",
            "price": "100000000.0",
            "state": "wait",
            "market": "KRW-BTC",
            "created_at": "2023-01-01T00:00:00+09:00",
            "volume": "0.01",
            "remaining_volume": "0.01",
            "reserved_fee": "50.0",
            "remaining_fee": "50.0",
            "paid_fee": "0.0",
            "locked": "1000050.0",
            "executed_volume": "0.0",
            "trades_count": 0
        }"#;

        let response: OrderResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.uuid, "cdd92199-2897-4e14-9b66-51bd59fce35e");
        assert_eq!(response.side, "bid");
        assert_eq!(response.ord_type, "limit");
        assert_eq!(response.price, Some("100000000.0".to_string()));
        assert_eq!(response.state, "wait");
        assert_eq!(response.market, "KRW-BTC");
        assert_eq!(response.created_at, "2023-01-01T00:00:00+09:00");
        assert_eq!(response.volume, Some("0.01".to_string()));
        assert_eq!(response.remaining_volume, Some("0.01".to_string()));
        assert_eq!(response.reserved_fee, "50.0");
        assert_eq!(response.remaining_fee, "50.0");
        assert_eq!(response.paid_fee, "0.0");
        assert_eq!(response.locked, "1000050.0");
        assert_eq!(response.executed_volume, "0.0");
        assert_eq!(response.trades_count, 0);
    }

    #[test]
    fn test_order_response_deserialize_market_order() {
        // 시장가 주문 응답: price와 volume이 null일 수 있음
        let json = r#"{
            "uuid": "abc123",
            "side": "ask",
            "ord_type": "market",
            "price": null,
            "state": "done",
            "market": "KRW-BTC",
            "created_at": "2023-01-01T00:00:00+09:00",
            "volume": "0.01",
            "remaining_volume": "0.0",
            "reserved_fee": "0.0",
            "remaining_fee": "0.0",
            "paid_fee": "50.0",
            "locked": "0.0",
            "executed_volume": "0.01",
            "trades_count": 1
        }"#;

        let response: OrderResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.uuid, "abc123");
        assert_eq!(response.side, "ask");
        assert_eq!(response.ord_type, "market");
        assert!(response.price.is_none());
        assert_eq!(response.state, "done");
        assert_eq!(response.trades_count, 1);
    }
}

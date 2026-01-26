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
    RateLimitExceeded { remaining_req: Option<String> },
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
            Self::RateLimitExceeded { .. } => {
                "주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.".to_string()
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
                // 입금 관련 에러 (WTS-4.1)
                "coin_address_not_found" => "입금 주소가 아직 생성되지 않았습니다".to_string(),
                "deposit_address_not_found" => "입금 주소가 아직 생성되지 않았습니다".to_string(),
                "invalid_currency" => "지원하지 않는 자산입니다".to_string(),
                "invalid_net_type" => "지원하지 않는 네트워크입니다".to_string(),
                "deposit_paused" => "현재 입금이 일시 중단되었습니다".to_string(),
                "deposit_suspended" => "해당 자산의 입금이 중단되었습니다".to_string(),
                "address_generation_failed" => "입금 주소 생성에 실패했습니다".to_string(),
                // 출금 관련 에러 (WTS-5.1)
                "unregistered_withdraw_address" | "withdraw_address_not_registered" => {
                    "출금 주소를 Upbit 웹에서 먼저 등록해주세요".to_string()
                }
                "insufficient_funds_withdraw" => "출금 가능 잔고가 부족합니다".to_string(),
                "under_min_amount" => "최소 출금 수량 이상이어야 합니다".to_string(),
                "over_daily_limit" => "일일 출금 한도를 초과했습니다".to_string(),
                "withdraw_suspended" => "현재 출금이 일시 중단되었습니다".to_string(),
                "withdraw_disabled" => "해당 자산의 출금이 비활성화되었습니다".to_string(),
                "wallet_not_working" => {
                    "지갑 점검 중입니다. 잠시 후 다시 시도해주세요".to_string()
                }
                "two_factor_auth_required" => "Upbit 앱에서 2FA 인증이 필요합니다".to_string(),
                "invalid_withdraw_address" => "유효하지 않은 출금 주소입니다".to_string(),
                "invalid_secondary_address" => {
                    "유효하지 않은 보조 주소입니다 (태그/메모)".to_string()
                }
                "travel_rule_violation" => "트래블룰 검증에 실패했습니다".to_string(),
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
            Self::RateLimitExceeded { .. } => "rate_limit".to_string(),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
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
        let detail = match &error {
            UpbitApiError::RateLimitExceeded { remaining_req } => remaining_req
                .as_ref()
                .map(|value| serde_json::json!({ "remaining_req": value })),
            _ => None,
        };
        Self {
            success: false,
            data: None,
            error: Some(WtsApiErrorResponse {
                code: error.code(),
                message: error.to_korean_message(),
                detail,
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

// ============================================================================
// Deposit API Types (Upbit)
// ============================================================================

/// 입금 주소 조회 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddressParams {
    /// 자산 코드 (예: "BTC", "ETH")
    pub currency: String,
    /// 네트워크 타입 (예: "BTC", "ETH", "TRX" 등)
    pub net_type: String,
}

/// 입금 주소 조회 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddressResponse {
    /// 자산 코드
    pub currency: String,
    /// 네트워크 타입
    pub net_type: String,
    /// 입금 주소 (null일 수 있음 - 생성 중)
    pub deposit_address: Option<String>,
    /// 보조 주소 (일부 코인: XRP tag, EOS memo 등)
    pub secondary_address: Option<String>,
}

/// 입금 가능 정보 조회 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositChanceParams {
    /// 자산 코드
    pub currency: String,
    /// 네트워크 타입
    pub net_type: String,
}

/// 입금 가능 정보 응답 (실제 Upbit API 응답 형식)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositChanceResponse {
    /// 자산 코드
    pub currency: String,
    /// 네트워크 타입
    pub net_type: String,
    /// 입금 가능 여부
    pub is_deposit_possible: bool,
    /// 입금 불가능 사유 (가능하면 null)
    pub deposit_impossible_reason: Option<String>,
    /// 최소 입금 수량
    pub minimum_deposit_amount: f64,
    /// 최소 입금 확인 횟수
    pub minimum_deposit_confirmations: i32,
    /// 소수점 정밀도
    pub decimal_precision: i32,
}

/// 프론트엔드용 DepositNetwork (호환성 유지)
/// 실제 API에서 제공하지 않으므로 DepositChanceResponse에서 생성
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositNetwork {
    /// 네트워크 이름 (net_type 사용)
    pub name: String,
    /// 네트워크 타입
    pub net_type: String,
    /// 우선순위
    pub priority: i32,
    /// 입금 상태
    pub deposit_state: String,
    /// 확인 횟수
    pub confirm_count: i32,
}

impl DepositChanceResponse {
    /// 프론트엔드 호환용 네트워크 정보 생성
    pub fn to_network(&self) -> DepositNetwork {
        DepositNetwork {
            name: self.net_type.clone(),
            net_type: self.net_type.clone(),
            priority: 1,
            deposit_state: if self.is_deposit_possible {
                "normal".to_string()
            } else {
                "paused".to_string()
            },
            confirm_count: self.minimum_deposit_confirmations,
        }
    }

    /// 입금 상태 문자열 반환
    pub fn deposit_state(&self) -> &str {
        if self.is_deposit_possible {
            "normal"
        } else {
            "paused"
        }
    }

    /// 최소 입금 수량 문자열 반환
    pub fn minimum(&self) -> String {
        self.minimum_deposit_amount.to_string()
    }
}

/// 입금 주소 생성 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateAddressParams {
    /// 자산 코드 (예: "BTC", "ETH")
    pub currency: String,
    /// 네트워크 타입 (예: "BTC", "ETH", "TRX" 등)
    pub net_type: String,
}

/// 입금 주소 생성 응답 (비동기)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GenerateAddressResponse {
    /// 비동기 생성 중
    Creating {
        success: bool,
        message: String,
    },
    /// 이미 존재하는 주소
    Existing(DepositAddressResponse),
}

// ============================================================================
// Withdraw API Types (Upbit) - WTS-5.1
// ============================================================================

/// 출금 요청 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawParams {
    /// 자산 코드 (예: "BTC", "ETH")
    pub currency: String,
    /// 네트워크 타입 (예: "BTC", "ETH", "TRX" 등)
    pub net_type: String,
    /// 출금 수량
    pub amount: String,
    /// 출금 주소 (Upbit에 사전 등록 필수)
    pub address: String,
    /// 보조 주소 (XRP tag, EOS memo 등)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_address: Option<String>,
    /// 트래블룰 거래 유형 ("default" 또는 "internal")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<String>,
}

/// 출금 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawResponse {
    /// 응답 타입 (항상 "withdraw")
    #[serde(rename = "type")]
    pub response_type: String,
    /// 출금 고유 식별자
    pub uuid: String,
    /// 자산 코드
    pub currency: String,
    /// 네트워크 타입
    pub net_type: String,
    /// 트랜잭션 ID (블록체인 TXID, 처리 전에는 null)
    pub txid: Option<String>,
    /// 출금 상태
    pub state: String,
    /// 출금 생성 시각
    pub created_at: String,
    /// 출금 완료 시각 (완료 전에는 null)
    pub done_at: Option<String>,
    /// 출금 수량
    pub amount: String,
    /// 출금 수수료
    pub fee: String,
    /// 트래블룰 거래 유형
    pub transaction_type: String,
}

/// 출금 가능 정보 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawChanceParams {
    /// 자산 코드
    pub currency: String,
    /// 네트워크 타입
    pub net_type: String,
}

/// 출금 가능 정보 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawChanceResponse {
    /// 회원 레벨 정보
    pub member_level: WithdrawMemberLevel,
    /// 자산 정보 (API 필드명: "currency")
    #[serde(rename = "currency")]
    pub currency_info: WithdrawCurrencyInfo,
    /// 계좌 정보 (API 필드명: "account")
    #[serde(rename = "account")]
    pub account_info: WithdrawAccountInfo,
    /// 출금 한도 정보
    pub withdraw_limit: WithdrawLimitInfo,
}

/// 회원 레벨 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawMemberLevel {
    pub security_level: i32,
    pub fee_level: i32,
    pub email_verified: bool,
    pub identity_auth_verified: bool,
    pub bank_account_verified: bool,
    pub two_factor_auth_verified: bool,
    pub locked: bool,
    pub wallet_locked: bool,
}

/// 자산 정보 (출금용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawCurrencyInfo {
    pub code: String,
    pub withdraw_fee: String,
    pub is_coin: bool,
    pub wallet_state: String,
    pub wallet_support: Vec<String>,
}

/// 계좌 정보 (출금용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawAccountInfo {
    pub currency: String,
    pub balance: String,
    pub locked: String,
    pub avg_buy_price: String,
    pub avg_buy_price_modified: bool,
    pub unit_currency: String,
}

/// 출금 한도 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawLimitInfo {
    pub currency: String,
    pub minimum: String,
    pub onetime: String,
    pub daily: String,
    pub remaining_daily: String,
    pub remaining_daily_krw: String,
    pub fixed: i32,
    pub can_withdraw: bool,
}

/// 출금 허용 주소 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawAddressResponse {
    /// 자산 코드
    pub currency: String,
    /// 네트워크 타입
    pub net_type: String,
    /// 네트워크 이름
    pub network_name: String,
    /// 출금 주소
    pub withdraw_address: String,
    /// 보조 주소
    pub secondary_address: Option<String>,
}

/// 출금 조회 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWithdrawParams {
    /// 출금 UUID (uuid 또는 txid 중 하나 필수)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// 트랜잭션 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
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
        let err = UpbitApiError::RateLimitExceeded { remaining_req: None };
        assert_eq!(
            err.to_korean_message(),
            "주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요."
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

    // ============================================================================
    // Deposit Types Tests (WTS-4.1)
    // ============================================================================

    #[test]
    fn test_deposit_address_params_serialize() {
        let params = DepositAddressParams {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["currency"], "BTC");
        assert_eq!(json["net_type"], "BTC");
    }

    #[test]
    fn test_deposit_address_response_deserialize() {
        let json = r#"{
            "currency": "BTC",
            "net_type": "BTC",
            "deposit_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
            "secondary_address": null
        }"#;

        let response: DepositAddressResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.currency, "BTC");
        assert_eq!(response.net_type, "BTC");
        assert_eq!(
            response.deposit_address,
            Some("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string())
        );
        assert!(response.secondary_address.is_none());
    }

    #[test]
    fn test_deposit_address_response_with_secondary() {
        // XRP는 secondary_address (Destination Tag) 사용
        let json = r#"{
            "currency": "XRP",
            "net_type": "XRP",
            "deposit_address": "rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh",
            "secondary_address": "123456789"
        }"#;

        let response: DepositAddressResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.currency, "XRP");
        assert_eq!(response.secondary_address, Some("123456789".to_string()));
    }

    #[test]
    fn test_deposit_address_response_null_address() {
        // 주소가 아직 생성되지 않은 경우
        let json = r#"{
            "currency": "BTC",
            "net_type": "BTC",
            "deposit_address": null,
            "secondary_address": null
        }"#;

        let response: DepositAddressResponse = serde_json::from_str(json).unwrap();
        assert!(response.deposit_address.is_none());
    }

    #[test]
    fn test_deposit_chance_response_deserialize() {
        // 실제 Upbit API 응답 형식
        let json = r#"{
            "currency": "BTC",
            "net_type": "BTC",
            "is_deposit_possible": true,
            "deposit_impossible_reason": null,
            "minimum_deposit_amount": 0.001,
            "minimum_deposit_confirmations": 3,
            "decimal_precision": 8
        }"#;

        let response: DepositChanceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.currency, "BTC");
        assert_eq!(response.net_type, "BTC");
        assert!(response.is_deposit_possible);
        assert!(response.deposit_impossible_reason.is_none());
        assert!((response.minimum_deposit_amount - 0.001).abs() < f64::EPSILON);
        assert_eq!(response.minimum_deposit_confirmations, 3);
        assert_eq!(response.decimal_precision, 8);
    }

    #[test]
    fn test_deposit_chance_response_with_reason() {
        // 입금 불가능한 경우
        let json = r#"{
            "currency": "ETH",
            "net_type": "ETH",
            "is_deposit_possible": false,
            "deposit_impossible_reason": "네트워크 점검 중",
            "minimum_deposit_amount": 0.01,
            "minimum_deposit_confirmations": 12,
            "decimal_precision": 18
        }"#;

        let response: DepositChanceResponse = serde_json::from_str(json).unwrap();
        assert!(!response.is_deposit_possible);
        assert_eq!(
            response.deposit_impossible_reason,
            Some("네트워크 점검 중".to_string())
        );
    }

    #[test]
    fn test_deposit_chance_to_network() {
        let response = DepositChanceResponse {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
            is_deposit_possible: true,
            deposit_impossible_reason: None,
            minimum_deposit_amount: 0.001,
            minimum_deposit_confirmations: 3,
            decimal_precision: 8,
        };

        let network = response.to_network();
        assert_eq!(network.name, "BTC");
        assert_eq!(network.net_type, "BTC");
        assert_eq!(network.deposit_state, "normal");
        assert_eq!(network.confirm_count, 3);
    }

    #[test]
    fn test_generate_address_response_creating() {
        // 비동기 생성 중 응답
        let json = r#"{
            "success": true,
            "message": "creating"
        }"#;

        let response: GenerateAddressResponse = serde_json::from_str(json).unwrap();
        match response {
            GenerateAddressResponse::Creating { success, message } => {
                assert!(success);
                assert_eq!(message, "creating");
            }
            _ => panic!("Expected Creating variant"),
        }
    }

    #[test]
    fn test_generate_address_response_existing() {
        // 이미 존재하는 주소 응답
        let json = r#"{
            "currency": "BTC",
            "net_type": "BTC",
            "deposit_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
            "secondary_address": null
        }"#;

        let response: GenerateAddressResponse = serde_json::from_str(json).unwrap();
        match response {
            GenerateAddressResponse::Existing(addr) => {
                assert_eq!(addr.currency, "BTC");
                assert_eq!(
                    addr.deposit_address,
                    Some("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string())
                );
            }
            _ => panic!("Expected Existing variant"),
        }
    }

    #[test]
    fn test_deposit_error_korean_messages() {
        // 입금 관련 에러 메시지
        let err = UpbitApiError::ApiError {
            code: "deposit_address_not_found".to_string(),
            message: "Deposit address not found".to_string(),
        };
        assert_eq!(err.to_korean_message(), "입금 주소가 아직 생성되지 않았습니다");

        let err = UpbitApiError::ApiError {
            code: "invalid_currency".to_string(),
            message: "Invalid currency".to_string(),
        };
        assert_eq!(err.to_korean_message(), "지원하지 않는 자산입니다");

        let err = UpbitApiError::ApiError {
            code: "invalid_net_type".to_string(),
            message: "Invalid net type".to_string(),
        };
        assert_eq!(err.to_korean_message(), "지원하지 않는 네트워크입니다");

        let err = UpbitApiError::ApiError {
            code: "deposit_paused".to_string(),
            message: "Deposit paused".to_string(),
        };
        assert_eq!(err.to_korean_message(), "현재 입금이 일시 중단되었습니다");

        let err = UpbitApiError::ApiError {
            code: "deposit_suspended".to_string(),
            message: "Deposit suspended".to_string(),
        };
        assert_eq!(err.to_korean_message(), "해당 자산의 입금이 중단되었습니다");

        let err = UpbitApiError::ApiError {
            code: "address_generation_failed".to_string(),
            message: "Address generation failed".to_string(),
        };
        assert_eq!(err.to_korean_message(), "입금 주소 생성에 실패했습니다");
    }

    // ============================================================================
    // Withdraw Types Tests (WTS-5.1)
    // ============================================================================

    #[test]
    fn test_withdraw_params_serialize() {
        let params = WithdrawParams {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
            amount: "0.01".to_string(),
            address: "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string(),
            secondary_address: None,
            transaction_type: None,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["currency"], "BTC");
        assert_eq!(json["net_type"], "BTC");
        assert_eq!(json["amount"], "0.01");
        assert_eq!(json["address"], "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh");
        // skip_serializing_if should omit None values
        assert!(json.get("secondary_address").is_none());
        assert!(json.get("transaction_type").is_none());
    }

    #[test]
    fn test_withdraw_params_serialize_with_secondary() {
        // XRP는 secondary_address (Destination Tag) 사용
        let params = WithdrawParams {
            currency: "XRP".to_string(),
            net_type: "XRP".to_string(),
            amount: "100".to_string(),
            address: "rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh".to_string(),
            secondary_address: Some("123456789".to_string()),
            transaction_type: Some("default".to_string()),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["currency"], "XRP");
        assert_eq!(json["secondary_address"], "123456789");
        assert_eq!(json["transaction_type"], "default");
    }

    #[test]
    fn test_withdraw_response_deserialize() {
        let json = r#"{
            "type": "withdraw",
            "uuid": "9f432943-54e0-40b7-825f-b6fec8b42b79",
            "currency": "BTC",
            "net_type": "BTC",
            "txid": null,
            "state": "submitting",
            "created_at": "2026-01-24T10:30:00+09:00",
            "done_at": null,
            "amount": "0.01",
            "fee": "0.0005",
            "transaction_type": "default"
        }"#;

        let response: WithdrawResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.response_type, "withdraw");
        assert_eq!(response.uuid, "9f432943-54e0-40b7-825f-b6fec8b42b79");
        assert_eq!(response.currency, "BTC");
        assert_eq!(response.net_type, "BTC");
        assert!(response.txid.is_none());
        assert_eq!(response.state, "submitting");
        assert_eq!(response.created_at, "2026-01-24T10:30:00+09:00");
        assert!(response.done_at.is_none());
        assert_eq!(response.amount, "0.01");
        assert_eq!(response.fee, "0.0005");
        assert_eq!(response.transaction_type, "default");
    }

    #[test]
    fn test_withdraw_response_deserialize_with_txid() {
        // 출금 완료 후 txid가 있는 응답
        let json = r#"{
            "type": "withdraw",
            "uuid": "9f432943-54e0-40b7-825f-b6fec8b42b79",
            "currency": "BTC",
            "net_type": "BTC",
            "txid": "abc123def456...",
            "state": "done",
            "created_at": "2026-01-24T10:30:00+09:00",
            "done_at": "2026-01-24T11:00:00+09:00",
            "amount": "0.01",
            "fee": "0.0005",
            "transaction_type": "default"
        }"#;

        let response: WithdrawResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.txid, Some("abc123def456...".to_string()));
        assert_eq!(response.state, "done");
        assert_eq!(
            response.done_at,
            Some("2026-01-24T11:00:00+09:00".to_string())
        );
    }

    #[test]
    fn test_withdraw_chance_params_serialize() {
        let params = WithdrawChanceParams {
            currency: "BTC".to_string(),
            net_type: "BTC".to_string(),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["currency"], "BTC");
        assert_eq!(json["net_type"], "BTC");
    }

    #[test]
    fn test_withdraw_chance_response_deserialize() {
        // 실제 Upbit API 응답 구조에 맞춤
        let json = r#"{
            "member_level": {
                "security_level": 3,
                "fee_level": 1,
                "email_verified": true,
                "identity_auth_verified": true,
                "bank_account_verified": true,
                "two_factor_auth_verified": true,
                "locked": false,
                "wallet_locked": false
            },
            "currency": {
                "code": "BTC",
                "withdraw_fee": "0.0005",
                "is_coin": true,
                "wallet_state": "working",
                "wallet_support": ["deposit", "withdraw"]
            },
            "account": {
                "currency": "BTC",
                "balance": "1.0",
                "locked": "0.0",
                "avg_buy_price": "50000000",
                "avg_buy_price_modified": false,
                "unit_currency": "KRW"
            },
            "withdraw_limit": {
                "currency": "BTC",
                "minimum": "0.001",
                "onetime": "10.0",
                "daily": "100.0",
                "remaining_daily": "99.5",
                "remaining_daily_krw": "4950000000",
                "fixed": 8,
                "can_withdraw": true
            }
        }"#;

        let response: WithdrawChanceResponse = serde_json::from_str(json).unwrap();

        // member_level
        assert_eq!(response.member_level.security_level, 3);
        assert!(response.member_level.two_factor_auth_verified);
        assert!(!response.member_level.locked);
        assert!(!response.member_level.wallet_locked);

        // currency_info (API 필드명: "currency")
        assert_eq!(response.currency_info.code, "BTC");
        assert_eq!(response.currency_info.withdraw_fee, "0.0005");
        assert!(response.currency_info.is_coin);
        assert_eq!(response.currency_info.wallet_state, "working");

        // account_info (API 필드명: "account")
        assert_eq!(response.account_info.currency, "BTC");
        assert_eq!(response.account_info.balance, "1.0");
        assert_eq!(response.account_info.locked, "0.0");

        // withdraw_limit
        assert_eq!(response.withdraw_limit.minimum, "0.001");
        assert!(response.withdraw_limit.can_withdraw);
    }

    #[test]
    fn test_withdraw_address_response_deserialize() {
        let json = r#"{
            "currency": "BTC",
            "net_type": "BTC",
            "network_name": "Bitcoin",
            "withdraw_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
            "secondary_address": null
        }"#;

        let response: WithdrawAddressResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.currency, "BTC");
        assert_eq!(response.net_type, "BTC");
        assert_eq!(response.network_name, "Bitcoin");
        assert_eq!(
            response.withdraw_address,
            "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        );
        assert!(response.secondary_address.is_none());
    }

    #[test]
    fn test_withdraw_address_response_with_secondary() {
        // XRP는 secondary_address 사용
        let json = r#"{
            "currency": "XRP",
            "net_type": "XRP",
            "network_name": "Ripple",
            "withdraw_address": "rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh",
            "secondary_address": "123456789"
        }"#;

        let response: WithdrawAddressResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.currency, "XRP");
        assert_eq!(response.secondary_address, Some("123456789".to_string()));
    }

    #[test]
    fn test_get_withdraw_params_serialize_with_uuid() {
        let params = GetWithdrawParams {
            uuid: Some("9f432943-54e0-40b7-825f-b6fec8b42b79".to_string()),
            txid: None,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["uuid"], "9f432943-54e0-40b7-825f-b6fec8b42b79");
        assert!(json.get("txid").is_none());
    }

    #[test]
    fn test_get_withdraw_params_serialize_with_txid() {
        let params = GetWithdrawParams {
            uuid: None,
            txid: Some("abc123def456...".to_string()),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert!(json.get("uuid").is_none());
        assert_eq!(json["txid"], "abc123def456...");
    }

    #[test]
    fn test_withdraw_error_korean_messages() {
        // 출금 주소 미등록 에러
        let err = UpbitApiError::ApiError {
            code: "unregistered_withdraw_address".to_string(),
            message: "Unregistered withdraw address".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "출금 주소를 Upbit 웹에서 먼저 등록해주세요"
        );

        let err = UpbitApiError::ApiError {
            code: "withdraw_address_not_registered".to_string(),
            message: "Address not registered".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "출금 주소를 Upbit 웹에서 먼저 등록해주세요"
        );

        // 잔고 부족 에러
        let err = UpbitApiError::ApiError {
            code: "insufficient_funds_withdraw".to_string(),
            message: "Insufficient funds".to_string(),
        };
        assert_eq!(err.to_korean_message(), "출금 가능 잔고가 부족합니다");

        // 최소 출금 수량 에러
        let err = UpbitApiError::ApiError {
            code: "under_min_amount".to_string(),
            message: "Under minimum amount".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "최소 출금 수량 이상이어야 합니다"
        );

        // 일일 한도 초과 에러
        let err = UpbitApiError::ApiError {
            code: "over_daily_limit".to_string(),
            message: "Over daily limit".to_string(),
        };
        assert_eq!(err.to_korean_message(), "일일 출금 한도를 초과했습니다");

        // 출금 중단 에러
        let err = UpbitApiError::ApiError {
            code: "withdraw_suspended".to_string(),
            message: "Withdraw suspended".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "현재 출금이 일시 중단되었습니다"
        );

        let err = UpbitApiError::ApiError {
            code: "withdraw_disabled".to_string(),
            message: "Withdraw disabled".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "해당 자산의 출금이 비활성화되었습니다"
        );

        // 지갑 점검 에러
        let err = UpbitApiError::ApiError {
            code: "wallet_not_working".to_string(),
            message: "Wallet not working".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "지갑 점검 중입니다. 잠시 후 다시 시도해주세요"
        );

        // 2FA 필요 에러
        let err = UpbitApiError::ApiError {
            code: "two_factor_auth_required".to_string(),
            message: "2FA required".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "Upbit 앱에서 2FA 인증이 필요합니다"
        );

        // 유효하지 않은 주소 에러
        let err = UpbitApiError::ApiError {
            code: "invalid_withdraw_address".to_string(),
            message: "Invalid address".to_string(),
        };
        assert_eq!(err.to_korean_message(), "유효하지 않은 출금 주소입니다");

        let err = UpbitApiError::ApiError {
            code: "invalid_secondary_address".to_string(),
            message: "Invalid secondary address".to_string(),
        };
        assert_eq!(
            err.to_korean_message(),
            "유효하지 않은 보조 주소입니다 (태그/메모)"
        );

        // 트래블룰 에러
        let err = UpbitApiError::ApiError {
            code: "travel_rule_violation".to_string(),
            message: "Travel rule violation".to_string(),
        };
        assert_eq!(err.to_korean_message(), "트래블룰 검증에 실패했습니다");
    }
}

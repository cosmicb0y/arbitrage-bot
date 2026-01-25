# Story WTS-5.1: 출금 API Rust 백엔드 구현

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **Upbit 출금 API와 연동된 백엔드 기능**,
So that **자산을 외부 지갑으로 출금할 수 있다**.

## Acceptance Criteria

1. **Given** Upbit API 키가 설정되어 있을 때 **When** `wts_withdraw` Tauri 명령을 호출하면 **Then** 출금 요청이 Upbit API로 전송되어야 한다
2. **Given** 출금 요청이 성공했을 때 **When** 응답을 받으면 **Then** 출금 결과(uuid, txid, state, amount, fee 등)가 반환되어야 한다
3. **Given** 출금 요청이 실패했을 때 **When** 에러 응답이 수신되면 **Then** Upbit 에러 코드와 한국어 상세 메시지가 반환되어야 한다
4. **Given** API 호출 시 **When** Rate Limit을 준수해야 **Then** 30회/초 제한 내에서 동작해야 한다
5. **Given** Upbit API 키가 설정되지 않았을 때 **When** 출금 API를 호출하면 **Then** MissingApiKey 에러가 반환되어야 한다
6. **Given** 출금 가능 정보 조회 시 **When** `wts_get_withdraw_chance` 명령을 호출하면 **Then** 수수료, 한도, 지갑 상태가 반환되어야 한다
7. **Given** 출금 허용 주소 조회 시 **When** `wts_get_withdraw_addresses` 명령을 호출하면 **Then** Upbit에 등록된 출금 주소 목록이 반환되어야 한다
8. **Given** 출금 상태 조회 시 **When** `wts_get_withdraw` 명령을 uuid로 호출하면 **Then** 해당 출금 건의 상태가 반환되어야 한다
9. **Given** 등록되지 않은 출금 주소로 요청 시 **When** 에러가 발생하면 **Then** "출금 주소를 Upbit에서 먼저 등록해주세요" 메시지가 반환되어야 한다
10. **Given** 2FA 필요 시 **When** 관련 에러 코드가 반환되면 **Then** "Upbit 앱에서 2FA 인증이 필요합니다" 메시지가 반환되어야 한다

## Tasks / Subtasks

- [x] Task 1: 출금 관련 타입 정의 (AC: #1, #2, #6, #7, #8)
  - [x] Subtask 1.1: `WithdrawParams` 구조체 정의 (currency, net_type, amount, address, secondary_address, transaction_type)
  - [x] Subtask 1.2: `WithdrawResponse` 구조체 정의 (uuid, txid, currency, state, amount, fee, created_at 등)
  - [x] Subtask 1.3: `WithdrawChanceParams` 구조체 정의 (currency, net_type)
  - [x] Subtask 1.4: `WithdrawChanceResponse` 구조체 정의 (currency, net_type, withdraw_fee, minimum, withdraw_limit, withdraw_state 등)
  - [x] Subtask 1.5: `WithdrawAddressResponse` 구조체 정의 (currency, net_type, network_name, withdraw_address, secondary_address)
  - [x] Subtask 1.6: `GetWithdrawParams` 구조체 정의 (uuid, txid - WithdrawState는 응답의 state 필드로 처리)
  - [x] Subtask 1.7: 타입에 serde Serialize/Deserialize derive 추가

- [x] Task 2: 출금 요청 API 구현 (AC: #1, #2, #3, #4, #5)
  - [x] Subtask 2.1: `withdraw_coin` async 함수 구현 (client.rs)
  - [x] Subtask 2.2: POST `/v1/withdraws/coin` 엔드포인트 호출
  - [x] Subtask 2.3: JSON 바디 생성 (currency, net_type, amount, address, secondary_address, transaction_type)
  - [x] Subtask 2.4: 바디 해시 포함 JWT 생성 (`generate_jwt_token_with_query`)
  - [x] Subtask 2.5: 응답 파싱 및 UpbitApiError 변환
  - [x] Subtask 2.6: 에러 상태 코드별 분기 처리 (400, 401, 429)

- [x] Task 3: 출금 가능 정보 조회 API 구현 (AC: #6, #4)
  - [x] Subtask 3.1: `get_withdraw_chance` async 함수 구현 (client.rs)
  - [x] Subtask 3.2: GET `/v1/withdraws/chance` 엔드포인트 호출
  - [x] Subtask 3.3: 쿼리 파라미터 URL 인코딩 (currency, net_type)
  - [x] Subtask 3.4: 쿼리 해시 포함 JWT 생성
  - [x] Subtask 3.5: 응답 파싱 (수수료, 한도, 지갑 상태, 네트워크 정보)

- [x] Task 4: 출금 허용 주소 조회 API 구현 (AC: #7, #4)
  - [x] Subtask 4.1: `get_withdraw_addresses` async 함수 구현 (client.rs)
  - [x] Subtask 4.2: GET `/v1/withdraws/coin_addresses` 엔드포인트 호출
  - [x] Subtask 4.3: 쿼리 파라미터 URL 인코딩 (currency, net_type)
  - [x] Subtask 4.4: 응답 파싱 (등록된 주소 목록)

- [x] Task 5: 출금 상태 조회 API 구현 (AC: #8, #4)
  - [x] Subtask 5.1: `get_withdraw` async 함수 구현 (client.rs)
  - [x] Subtask 5.2: GET `/v1/withdraw` 엔드포인트 호출
  - [x] Subtask 5.3: 쿼리 파라미터 (uuid 또는 txid)
  - [x] Subtask 5.4: 응답 파싱 (출금 상태, 트랜잭션 정보)

- [x] Task 6: Tauri 명령 등록 (AC: #1-#10)
  - [x] Subtask 6.1: `wts_withdraw` Tauri 명령 함수 정의 (mod.rs)
  - [x] Subtask 6.2: `wts_get_withdraw_chance` Tauri 명령 함수 정의
  - [x] Subtask 6.3: `wts_get_withdraw_addresses` Tauri 명령 함수 정의
  - [x] Subtask 6.4: `wts_get_withdraw` Tauri 명령 함수 정의
  - [x] Subtask 6.5: main.rs에 명령 등록

- [x] Task 7: 에러 타입 확장 (AC: #3, #9, #10)
  - [x] Subtask 7.1: UpbitApiError.to_korean_message()에 출금 관련 에러 코드 추가
  - [x] Subtask 7.2: 출금 불가 상태 에러 메시지 추가 (insufficient_funds, withdraw_suspended 등)
  - [x] Subtask 7.3: 출금 주소 미등록 에러 메시지 추가 (unregistered_withdraw_address)
  - [x] Subtask 7.4: 2FA 필요 에러 메시지 추가 (two_factor_auth_required)
  - [x] Subtask 7.5: Rate Limit 에러 메시지 (이미 구현됨 - RateLimitExceeded enum)

- [x] Task 8: 단위 테스트 작성 (AC: #1-#10)
  - [x] Subtask 8.1: WithdrawParams 직렬화 테스트
  - [x] Subtask 8.2: WithdrawResponse 역직렬화 테스트
  - [x] Subtask 8.3: WithdrawChanceResponse 역직렬화 테스트
  - [x] Subtask 8.4: WithdrawAddressResponse 역직렬화 테스트
  - [x] Subtask 8.5: 에러 응답 파싱 테스트
  - [x] Subtask 8.6: API 키 누락 시 에러 테스트

- [x] Task 9: TypeScript 타입 동기화 (AC: #1, #2, #6, #7, #8)
  - [x] Subtask 9.1: types.ts에 WithdrawParams 인터페이스 추가
  - [x] Subtask 9.2: types.ts에 WithdrawResponse 인터페이스 추가
  - [x] Subtask 9.3: types.ts에 WithdrawChanceResponse 인터페이스 추가
  - [x] Subtask 9.4: types.ts에 WithdrawAddressResponse 인터페이스 추가
  - [x] Subtask 9.5: 출금 관련 에러 코드 매핑 추가 (UPBIT_ORDER_ERROR_MESSAGES)

## Dev Notes

### Upbit 출금 API 스펙

[Source: architecture.md#Upbit 출금 API]

**엔드포인트:**

| API | 엔드포인트 | 메서드 | Rate Limit |
|-----|-----------|--------|------------|
| 출금 가능 정보 | `/v1/withdraws/chance` | GET | 30/초 |
| 출금 허용 주소 | `/v1/withdraws/coin_addresses` | GET | 30/초 |
| 출금 요청 | `/v1/withdraws/coin` | POST | 30/초 |
| 출금 조회 | `/v1/withdraw` | GET | 30/초 |
| 출금 목록 | `/v1/withdraws` | GET | 30/초 |
| 출금 취소 | `/v1/withdraws/coin` | DELETE | 30/초 |

### 출금 요청 (POST /v1/withdraws/coin)

**요청 바디:**
```json
{
  "currency": "BTC",
  "net_type": "BTC",
  "amount": "0.01",
  "address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
  "secondary_address": null,
  "transaction_type": "default"
}
```

**파라미터 설명:**
- `currency` (필수): 자산 코드 (예: "BTC", "ETH", "XRP")
- `net_type` (필수): 네트워크 타입 (예: "BTC", "ETH", "TRX")
- `amount` (필수): 출금 수량 (문자열)
- `address` (필수): 출금 주소 (Upbit에 사전 등록 필수)
- `secondary_address` (선택): 보조 주소 (XRP tag, EOS memo 등)
- `transaction_type` (선택): 트래블룰 거래 유형 ("default", "internal")

**응답 필드:**
```json
{
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
}
```

### 출금 가능 정보 조회 (GET /v1/withdraws/chance)

**요청 파라미터:**
```
?currency=BTC&net_type=BTC
```

**응답 필드:**
```json
{
  "currency": "BTC",
  "net_type": "BTC",
  "member_level": {
    "security_level": 3,
    "fee_level": 1,
    "email_verified": true,
    "identity_auth_verified": true,
    "bank_account_verified": true,
    "two_factor_auth_verified": true,
    "locked": false
  },
  "currency_info": {
    "code": "BTC",
    "withdraw_fee": "0.0005",
    "is_coin": true,
    "wallet_state": "working",
    "wallet_support": ["deposit", "withdraw"]
  },
  "account_info": {
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
}
```

### 출금 허용 주소 조회 (GET /v1/withdraws/coin_addresses)

**요청 파라미터:**
```
?currency=BTC&net_type=BTC
```

**응답 필드:**
```json
[
  {
    "currency": "BTC",
    "net_type": "BTC",
    "network_name": "Bitcoin",
    "withdraw_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
    "secondary_address": null
  }
]
```

**중요:** Upbit에서 출금 주소는 사전 등록 필수. 미등록 주소로 출금 시 에러 발생.

### 출금 상태 조회 (GET /v1/withdraw)

**요청 파라미터:**
```
?uuid=9f432943-54e0-40b7-825f-b6fec8b42b79
# 또는
?txid=abc123...
```

**응답:** `WithdrawResponse`와 동일 구조

### 출금 상태 값 (WithdrawState)

| 상태 | 설명 |
|------|------|
| `submitting` | 처리 중 |
| `submitted` | 처리 완료 |
| `almost_accepted` | 출금대기중 |
| `rejected` | 거절 |
| `accepted` | 승인됨 |
| `processing` | 처리 중 |
| `done` | 완료 |
| `canceled` | 취소됨 |

### 기존 코드 패턴

**client.rs POST 요청 패턴 (WTS-3.1 참조):**

[Source: apps/desktop/src-tauri/src/wts/upbit/client.rs]

```rust
// POST 요청 (JSON 바디 + 해시)
pub async fn withdraw_coin(params: WithdrawParams) -> Result<WithdrawResponse, UpbitApiError> {
    let (access_key, secret_key) = load_api_keys()?;

    // JSON 바디 생성
    let body = serde_json::json!({
        "currency": params.currency,
        "net_type": params.net_type,
        "amount": params.amount,
        "address": params.address,
        "secondary_address": params.secondary_address,
        "transaction_type": params.transaction_type.unwrap_or("default".to_string()),
    });

    let body_string = serde_json::to_string(&body)
        .map_err(|e| UpbitApiError::InvalidResponse(e.to_string()))?;

    // 바디 해시 포함 JWT 생성
    let token = generate_jwt_token_with_hash(&access_key, &secret_key, &body_string)
        .map_err(UpbitApiError::JwtError)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    let response = client
        .post(format!("{}/withdraws/coin", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(body_string)
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // 에러 처리
    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(parse_upbit_error(status, &error_body));
    }

    // 성공 응답 파싱
    response
        .json::<WithdrawResponse>()
        .await
        .map_err(|e| UpbitApiError::InvalidResponse(e.to_string()))
}
```

**타입 정의 패턴 (types.rs):**

```rust
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
    /// 자산 코드
    pub currency: String,
    /// 네트워크 타입
    pub net_type: String,
    /// 회원 레벨 정보
    pub member_level: WithdrawMemberLevel,
    /// 자산 정보
    pub currency_info: WithdrawCurrencyInfo,
    /// 계좌 정보
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
}

/// 자산 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawCurrencyInfo {
    pub code: String,
    pub withdraw_fee: String,
    pub is_coin: bool,
    pub wallet_state: String,
    pub wallet_support: Vec<String>,
}

/// 계좌 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawAccountInfo {
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
```

**mod.rs Tauri 명령:**

```rust
/// Upbit 출금을 요청합니다.
#[tauri::command]
pub async fn wts_withdraw(params: WithdrawParams) -> WtsApiResult<WithdrawResponse> {
    match upbit::withdraw_coin(params).await {
        Ok(response) => WtsApiResult::ok(response),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 출금 가능 정보를 조회합니다.
#[tauri::command]
pub async fn wts_get_withdraw_chance(params: WithdrawChanceParams) -> WtsApiResult<WithdrawChanceResponse> {
    match upbit::get_withdraw_chance(params).await {
        Ok(chance) => WtsApiResult::ok(chance),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit에 등록된 출금 허용 주소를 조회합니다.
#[tauri::command]
pub async fn wts_get_withdraw_addresses(params: WithdrawChanceParams) -> WtsApiResult<Vec<WithdrawAddressResponse>> {
    match upbit::get_withdraw_addresses(params).await {
        Ok(addresses) => WtsApiResult::ok(addresses),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 출금 상태를 조회합니다.
#[tauri::command]
pub async fn wts_get_withdraw(params: GetWithdrawParams) -> WtsApiResult<WithdrawResponse> {
    match upbit::get_withdraw(params).await {
        Ok(withdraw) => WtsApiResult::ok(withdraw),
        Err(e) => WtsApiResult::err(e),
    }
}
```

### 에러 처리 확장

**출금 관련 Upbit 에러 코드:**

| 에러 코드 | 설명 | 한국어 메시지 |
|----------|------|--------------|
| `unregistered_withdraw_address` | 미등록 출금 주소 | "출금 주소를 Upbit 웹에서 먼저 등록해주세요" |
| `withdraw_address_not_registered` | 미등록 출금 주소 | "출금 주소를 Upbit 웹에서 먼저 등록해주세요" |
| `insufficient_funds_withdraw` | 잔고 부족 | "출금 가능 잔고가 부족합니다" |
| `under_min_amount` | 최소 출금 수량 미달 | "최소 출금 수량 이상이어야 합니다" |
| `over_daily_limit` | 일일 출금 한도 초과 | "일일 출금 한도를 초과했습니다" |
| `withdraw_suspended` | 출금 일시 중단 | "현재 출금이 일시 중단되었습니다" |
| `withdraw_disabled` | 출금 비활성화 | "해당 자산의 출금이 비활성화되었습니다" |
| `wallet_not_working` | 지갑 점검 중 | "지갑 점검 중입니다. 잠시 후 다시 시도해주세요" |
| `two_factor_auth_required` | 2FA 필요 | "Upbit 앱에서 2FA 인증이 필요합니다" |
| `invalid_withdraw_address` | 유효하지 않은 주소 | "유효하지 않은 출금 주소입니다" |
| `invalid_secondary_address` | 유효하지 않은 보조 주소 | "유효하지 않은 보조 주소입니다 (태그/메모)" |
| `travel_rule_violation` | 트래블룰 위반 | "트래블룰 검증에 실패했습니다" |

**types.rs 에러 메시지 확장:**

```rust
impl UpbitApiError {
    pub fn to_korean_message(&self) -> String {
        match self {
            // ... 기존 코드 ...
            Self::ApiError { code, message } => match code.as_str() {
                // 기존 에러 ...
                // 출금 관련 에러
                "unregistered_withdraw_address" | "withdraw_address_not_registered" =>
                    "출금 주소를 Upbit 웹에서 먼저 등록해주세요".to_string(),
                "insufficient_funds_withdraw" => "출금 가능 잔고가 부족합니다".to_string(),
                "under_min_amount" => "최소 출금 수량 이상이어야 합니다".to_string(),
                "over_daily_limit" => "일일 출금 한도를 초과했습니다".to_string(),
                "withdraw_suspended" => "현재 출금이 일시 중단되었습니다".to_string(),
                "withdraw_disabled" => "해당 자산의 출금이 비활성화되었습니다".to_string(),
                "wallet_not_working" => "지갑 점검 중입니다. 잠시 후 다시 시도해주세요".to_string(),
                "two_factor_auth_required" => "Upbit 앱에서 2FA 인증이 필요합니다".to_string(),
                "invalid_withdraw_address" => "유효하지 않은 출금 주소입니다".to_string(),
                "invalid_secondary_address" => "유효하지 않은 보조 주소입니다 (태그/메모)".to_string(),
                "travel_rule_violation" => "트래블룰 검증에 실패했습니다".to_string(),
                _ => message.clone(),
            },
            // ...
        }
    }
}
```

### TypeScript 타입 동기화

**types.ts 추가:**

```typescript
// ============================================================================
// Withdraw API Types (Upbit)
// ============================================================================

/** 출금 요청 파라미터 */
export interface WithdrawParams {
  /** 자산 코드 (예: "BTC", "ETH") */
  currency: string;
  /** 네트워크 타입 (예: "BTC", "ETH", "TRX" 등) */
  net_type: string;
  /** 출금 수량 */
  amount: string;
  /** 출금 주소 (Upbit에 사전 등록 필수) */
  address: string;
  /** 보조 주소 (XRP tag, EOS memo 등) */
  secondary_address?: string | null;
  /** 트래블룰 거래 유형 ("default" 또는 "internal") */
  transaction_type?: string;
}

/** 출금 응답 */
export interface WithdrawResponse {
  /** 응답 타입 (항상 "withdraw") */
  type: string;
  /** 출금 고유 식별자 */
  uuid: string;
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 트랜잭션 ID (블록체인 TXID, 처리 전에는 null) */
  txid: string | null;
  /** 출금 상태 */
  state: WithdrawState;
  /** 출금 생성 시각 */
  created_at: string;
  /** 출금 완료 시각 (완료 전에는 null) */
  done_at: string | null;
  /** 출금 수량 */
  amount: string;
  /** 출금 수수료 */
  fee: string;
  /** 트래블룰 거래 유형 */
  transaction_type: string;
}

/** 출금 상태 */
export type WithdrawState =
  | 'submitting'
  | 'submitted'
  | 'almost_accepted'
  | 'rejected'
  | 'accepted'
  | 'processing'
  | 'done'
  | 'canceled';

/** 출금 가능 정보 파라미터 */
export interface WithdrawChanceParams {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
}

/** 회원 레벨 정보 */
export interface WithdrawMemberLevel {
  security_level: number;
  fee_level: number;
  email_verified: boolean;
  identity_auth_verified: boolean;
  bank_account_verified: boolean;
  two_factor_auth_verified: boolean;
  locked: boolean;
}

/** 자산 정보 */
export interface WithdrawCurrencyInfo {
  code: string;
  withdraw_fee: string;
  is_coin: boolean;
  wallet_state: string;
  wallet_support: string[];
}

/** 계좌 정보 */
export interface WithdrawAccountInfo {
  balance: string;
  locked: string;
  avg_buy_price: string;
  avg_buy_price_modified: boolean;
  unit_currency: string;
}

/** 출금 한도 정보 */
export interface WithdrawLimitInfo {
  currency: string;
  minimum: string;
  onetime: string;
  daily: string;
  remaining_daily: string;
  remaining_daily_krw: string;
  fixed: number;
  can_withdraw: boolean;
}

/** 출금 가능 정보 응답 */
export interface WithdrawChanceResponse {
  currency: string;
  net_type: string;
  member_level: WithdrawMemberLevel;
  currency_info: WithdrawCurrencyInfo;
  account_info: WithdrawAccountInfo;
  withdraw_limit: WithdrawLimitInfo;
}

/** 출금 허용 주소 응답 */
export interface WithdrawAddressResponse {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 네트워크 이름 */
  network_name: string;
  /** 출금 주소 */
  withdraw_address: string;
  /** 보조 주소 */
  secondary_address: string | null;
}

/** 출금 조회 파라미터 */
export interface GetWithdrawParams {
  /** 출금 UUID (uuid 또는 txid 중 하나 필수) */
  uuid?: string;
  /** 트랜잭션 ID */
  txid?: string;
}

/** 출금 상태가 완료인지 확인 */
export function isWithdrawComplete(state: WithdrawState): boolean {
  return state === 'done';
}

/** 출금 상태가 진행 중인지 확인 */
export function isWithdrawPending(state: WithdrawState): boolean {
  return ['submitting', 'submitted', 'almost_accepted', 'accepted', 'processing'].includes(state);
}

/** 출금 상태가 실패인지 확인 */
export function isWithdrawFailed(state: WithdrawState): boolean {
  return ['rejected', 'canceled'].includes(state);
}
```

**출금 에러 메시지 추가 (upbitErrors.ts 확장):**

```typescript
// 출금 관련 에러
unregistered_withdraw_address: '출금 주소를 Upbit 웹에서 먼저 등록해주세요',
withdraw_address_not_registered: '출금 주소를 Upbit 웹에서 먼저 등록해주세요',
insufficient_funds_withdraw: '출금 가능 잔고가 부족합니다',
under_min_amount: '최소 출금 수량 이상이어야 합니다',
over_daily_limit: '일일 출금 한도를 초과했습니다',
withdraw_suspended: '현재 출금이 일시 중단되었습니다',
withdraw_disabled: '해당 자산의 출금이 비활성화되었습니다',
wallet_not_working: '지갑 점검 중입니다. 잠시 후 다시 시도해주세요',
two_factor_auth_required: 'Upbit 앱에서 2FA 인증이 필요합니다',
invalid_withdraw_address: '유효하지 않은 출금 주소입니다',
invalid_secondary_address: '유효하지 않은 보조 주소입니다 (태그/메모)',
travel_rule_violation: '트래블룰 검증에 실패했습니다',
```

### Project Structure Notes

**신규 타입 추가 위치:**
- `apps/desktop/src-tauri/src/wts/upbit/types.rs`
- `apps/desktop/src/wts/types.ts`

**수정 파일:**
- `apps/desktop/src-tauri/src/wts/upbit/client.rs` - 출금 관련 API 함수 4개 추가
- `apps/desktop/src-tauri/src/wts/upbit/types.rs` - 출금 관련 타입 추가
- `apps/desktop/src-tauri/src/wts/mod.rs` - Tauri 명령 함수 4개 추가
- `apps/desktop/src-tauri/src/main.rs` - 명령 등록
- `apps/desktop/src/wts/types.ts` - TypeScript 출금 타입 및 헬퍼 함수 추가
- `apps/desktop/src/wts/utils/upbitErrors.ts` - 출금 에러 메시지 추가

**아키텍처 정합성:**
- Tauri 명령 접두사 `wts_` 준수
- WtsApiResult 래퍼 패턴 준수
- UpbitApiError 에러 처리 패턴 준수
- 한국어 에러 메시지 매핑 패턴 준수
- Rate Limit(30회/초) 준수 (Exchange Default)

### 이전 스토리 참조 (매우 중요)

**WTS-4.1 (입금 API Rust 백엔드):**
- GET 요청 + 쿼리 해시 JWT 패턴 재사용 (출금 가능 정보, 출금 주소 조회)
- POST 요청은 바디 해시 JWT 패턴 사용 (출금 요청)
- 에러 처리 패턴 그대로 재사용
- API 키 로드 패턴 재사용 (`load_api_keys`)
- TypeScript 타입 동기화 패턴 참조

**WTS-3.1 (주문 API Rust 백엔드):**
- POST 요청 + JSON 바디 + 바디 해시 JWT 패턴 재사용
- `generate_jwt_token_with_hash` 함수 사용
- Rate Limit 처리 패턴 참조

### 주요 구현 고려사항

1. **출금 주소 사전 등록:**
   - Upbit 웹/앱에서 출금 주소 사전 등록 필수
   - API로는 등록된 주소만 사용 가능
   - 미등록 주소 시 친절한 안내 메시지 필요 (Story 5.5)

2. **보조 주소 (secondary_address):**
   - XRP (Destination Tag), EOS (Memo) 등은 필수
   - UI에서 필수 여부 표시 필요 (Story 5.2)

3. **2FA 인증:**
   - 출금 시 2FA 필요할 수 있음
   - 에러 발생 시 친절한 안내 필요 (Story 5.5)

4. **트래블룰 (Travel Rule):**
   - 일정 금액 이상 출금 시 상대 거래소 정보 필요
   - `transaction_type`으로 내부/외부 거래 구분

5. **출금 상태 추적:**
   - 출금은 비동기 처리
   - 상태 변화: submitting → submitted → processing → done
   - 실시간 상태 조회 필요 (프론트엔드에서 폴링)

### Git 인텔리전스

**최근 관련 커밋:**
- `53747fe feat(wts): implement deposit API Rust backend (WTS-4.1)` - 입금 API 패턴 참조
- `8dfa317 feat(wts): implement order error and rate limit handling (WTS-3.7)` - 에러 처리 패턴
- `6323801 feat(wts): implement deposit tab UI (WTS-4.2)` - UI 연동 패턴

### References

- [Architecture: Upbit 출금 API](/_bmad-output/planning-artifacts/architecture.md#Upbit 출금 API)
- [Architecture: WTS Backend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Backend Structure)
- [PRD: FR21-27 출금 기능](/_bmad-output/planning-artifacts/prd.md)
- [WTS Epics: Epic 5](/_bmad-output/planning-artifacts/wts-epics.md#Epic 5)
- [Previous Story: WTS-4.1 입금 API 백엔드](/_bmad-output/implementation-artifacts/wts-4-1-deposit-api-rust-backend.md)
- [Previous Story: WTS-3.1 주문 API 백엔드](/_bmad-output/implementation-artifacts/wts-3-1-order-api-rust-backend.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Code Review Fixes
- `withdraw_coin`: `transaction_type` now defaults to "default" if not provided, ensuring compatibility with Upbit API.
- `File List`: Removed reference to non-existent `upbitErrors.ts`.

### Debug Log References

- 없음 (모든 테스트 통과)

### Completion Notes List

- **Task 1**: 출금 관련 Rust 타입 정의 완료
  - WithdrawParams, WithdrawResponse, WithdrawChanceParams, WithdrawChanceResponse
  - WithdrawMemberLevel, WithdrawCurrencyInfo, WithdrawAccountInfo, WithdrawLimitInfo
  - WithdrawAddressResponse, GetWithdrawParams
  - 11개 타입 직렬화/역직렬화 테스트 작성

- **Tasks 2-5**: 4개 출금 API 함수 구현 완료
  - withdraw_coin(): POST /v1/withdraws/coin 출금 요청
  - get_withdraw_chance(): GET /v1/withdraws/chance 출금 가능 정보
  - get_withdraw_addresses(): GET /v1/withdraws/coin_addresses 등록된 출금 주소 목록
  - get_withdraw(): GET /v1/withdraw 출금 상태 조회
  - 모든 함수에 API 키 누락 테스트 추가

- **Task 6**: 4개 Tauri 명령 등록 완료
  - wts_withdraw, wts_get_withdraw_chance, wts_get_withdraw_addresses, wts_get_withdraw

- **Task 7**: 12개 출금 에러 코드 한국어 매핑 추가
  - 출금 주소 미등록, 잔고 부족, 한도 초과, 출금 중단, 2FA 필요 등

- **Task 8**: 56개 Rust WTS 테스트 전체 통과

- **Task 9**: TypeScript 타입 동기화 완료 (53개 테스트 통과)
  - 출금 관련 인터페이스 및 헬퍼 함수 추가
  - UPBIT_ORDER_ERROR_MESSAGES에 출금 에러 코드 추가

### File List

**수정된 파일:**
- apps/desktop/src-tauri/src/wts/upbit/types.rs (출금 타입 + 에러 코드 + 테스트)
- apps/desktop/src-tauri/src/wts/upbit/client.rs (출금 API 함수 4개 + 테스트)
- apps/desktop/src-tauri/src/wts/mod.rs (Tauri 명령 4개 + export)
- apps/desktop/src-tauri/src/main.rs (명령 등록)
- apps/desktop/src/wts/types.ts (TypeScript 타입 + 에러 코드)

### Change Log

- 2026-01-25: WTS-5.1 출금 API Rust 백엔드 구현 완료

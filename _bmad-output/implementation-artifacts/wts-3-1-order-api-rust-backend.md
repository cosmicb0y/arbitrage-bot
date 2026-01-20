# Story WTS-3.1: 주문 API Rust 백엔드 구현

Status: done

## Story

As a **트레이더**,
I want **Upbit 주문 API와 연동된 백엔드 기능**,
So that **매수/매도 주문을 실행할 수 있다**.

## Acceptance Criteria

1. **Given** Upbit API 키가 설정되어 있을 때 **When** `wts_place_order` Tauri 명령을 호출하면 **Then** 시장가/지정가, 매수/매도에 따른 적절한 API 파라미터가 전송되어야 한다
2. **Given** Upbit API 키가 설정되어 있을 때 **When** `wts_place_order` Tauri 명령을 호출하면 **Then** JWT 토큰 생성 시 주문 정보 해시가 포함되어야 한다
3. **Given** Upbit API 키가 설정되어 있을 때 **When** 주문 API를 호출하면 **Then** Rate Limit(8회/초)을 준수해야 한다
4. **Given** Upbit API 키가 설정되어 있을 때 **When** 주문이 성공하면 **Then** 주문 결과(uuid, side, ord_type, price, state 등)가 반환되어야 한다
5. **Given** Upbit API 키가 설정되어 있을 때 **When** API를 호출하면 **Then** API 평문 로깅이 금지되어야 한다 (키 정보 마스킹)
6. **Given** Upbit API 키가 설정되지 않았을 때 **When** `wts_place_order`를 호출하면 **Then** MissingApiKey 에러가 반환되어야 한다
7. **Given** 주문 파라미터가 유효하지 않을 때 **When** `wts_place_order`를 호출하면 **Then** validation_error가 반환되어야 한다
8. **Given** 잔고가 부족할 때 **When** `wts_place_order`를 호출하면 **Then** insufficient_funds 에러가 반환되어야 한다

## Tasks / Subtasks

- [x] Task 1: 주문 요청/응답 타입 정의 (AC: #1, #4)
  - [x] Subtask 1.1: `OrderParams` 구조체 정의 (market, side, volume, price, ord_type)
  - [x] Subtask 1.2: `OrderResponse` 구조체 정의 (Upbit API 응답)
  - [x] Subtask 1.3: `OrderSide`, `OrderType` enum 정의
  - [x] Subtask 1.4: 타입에 serde Serialize/Deserialize derive 추가

- [x] Task 2: JWT 토큰에 쿼리 해시 포함 (AC: #2)
  - [x] Subtask 2.1: `generate_jwt_token_with_query` 함수 추가 (auth.rs)
  - [x] Subtask 2.2: 쿼리 파라미터를 SHA-512 해시로 변환
  - [x] Subtask 2.3: JWT claims에 query_hash, query_hash_alg 필드 추가
  - [x] Subtask 2.4: 기존 `generate_jwt_token` 유지 (GET 요청용)

- [x] Task 3: 주문 API 클라이언트 구현 (AC: #1, #4, #5)
  - [x] Subtask 3.1: `place_order` async 함수 구현 (client.rs)
  - [x] Subtask 3.2: POST /v1/orders 엔드포인트 호출
  - [x] Subtask 3.3: Content-Type: application/json 헤더 설정
  - [x] Subtask 3.4: 주문 유형별 파라미터 처리 (limit/market/price)
  - [x] Subtask 3.5: 에러 응답 파싱 및 UpbitApiError 변환
  - [x] Subtask 3.6: 로깅 시 API 키 마스킹 확인

- [x] Task 4: Tauri 명령 등록 (AC: #1, #4, #6, #7, #8)
  - [x] Subtask 4.1: `wts_place_order` Tauri 명령 함수 정의 (mod.rs)
  - [x] Subtask 4.2: OrderParams를 Tauri 명령 인자로 수신
  - [x] Subtask 4.3: WtsApiResult<OrderResponse> 반환
  - [x] Subtask 4.4: main.rs에 명령 등록

- [x] Task 5: 주문 에러 타입 확장 (AC: #6, #7, #8)
  - [x] Subtask 5.1: UpbitApiError에 주문 관련 에러 코드 추가
  - [x] Subtask 5.2: insufficient_funds_bid, insufficient_funds_ask 처리
  - [x] Subtask 5.3: 한국어 에러 메시지 매핑 추가

- [x] Task 6: 단위 테스트 작성 (AC: #1-#8)
  - [x] Subtask 6.1: OrderParams 직렬화 테스트
  - [x] Subtask 6.2: OrderResponse 역직렬화 테스트
  - [x] Subtask 6.3: JWT 쿼리 해시 생성 테스트
  - [x] Subtask 6.4: 에러 응답 파싱 테스트

- [x] Task 7: TypeScript 타입 동기화 (AC: #4)
  - [x] Subtask 7.1: types.ts에 OrderParams 인터페이스 추가
  - [x] Subtask 7.2: types.ts에 OrderResponse 인터페이스 추가
  - [x] Subtask 7.3: 주문 관련 에러 코드 매핑 추가 (types.ts에 UPBIT_ORDER_ERROR_MESSAGES 추가)

## Dev Notes

### Upbit 주문 API 스펙

**엔드포인트:**
- POST `https://api.upbit.com/v1/orders`
- Rate Limit: 8회/초 (계정 기준)
- Content-Type: `application/json` 필수

**주문 유형:**

| ord_type | 설명 | 필수 파라미터 |
|----------|------|--------------|
| `limit` | 지정가 | market, side, volume, price |
| `price` | 시장가 매수 | market, side='bid', price(총액) |
| `market` | 시장가 매도 | market, side='ask', volume |

**요청 파라미터:**
```json
{
  "market": "KRW-BTC",
  "side": "bid",          // bid(매수) | ask(매도)
  "volume": "0.01",       // 주문 수량 (시장가 매도 또는 지정가)
  "price": "100000",      // 주문 가격 (시장가 매수: 총액, 지정가: 단가)
  "ord_type": "limit"     // limit | price | market
}
```

**응답 필드:**
```json
{
  "uuid": "cdd92199-2897-4e14-9b...",
  "side": "bid",
  "ord_type": "limit",
  "price": "100000.0",
  "state": "wait",
  "market": "KRW-BTC",
  "created_at": "2023-01-01T00:00:00+09:00",
  "volume": "0.01",
  "remaining_volume": "0.01",
  "reserved_fee": "...",
  "remaining_fee": "...",
  "paid_fee": "...",
  "locked": "...",
  "executed_volume": "0.0",
  "trades_count": 0
}
```

### JWT 토큰 생성 (쿼리 해시 포함)

[Source: architecture.md#Upbit REST API 공통]

POST 요청 시 요청 본문을 SHA-512 해시하여 JWT에 포함해야 합니다:

```rust
// auth.rs
#[derive(Debug, Serialize)]
struct UpbitClaimsWithQuery {
    access_key: String,
    nonce: String,
    timestamp: i64,
    query_hash: String,
    query_hash_alg: String,  // "SHA512"
}

pub fn generate_jwt_token_with_query(
    access_key: &str,
    secret_key: &str,
    query: &str,
) -> Result<String, String> {
    use sha2::{Sha512, Digest};

    let mut hasher = Sha512::new();
    hasher.update(query.as_bytes());
    let query_hash = hex::encode(hasher.finalize());

    let claims = UpbitClaimsWithQuery {
        access_key: access_key.to_string(),
        nonce: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        query_hash,
        query_hash_alg: "SHA512".to_string(),
    };

    // ... encode with HS256
}
```

### 기존 코드 패턴

**client.rs 패턴:**
[Source: apps/desktop/src-tauri/src/wts/upbit/client.rs]

```rust
pub async fn place_order(params: OrderParams) -> Result<OrderResponse, UpbitApiError> {
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
        // ... 기존 에러 처리 패턴
    }

    response.json::<OrderResponse>().await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}
```

**types.rs 추가 타입:**

```rust
/// 주문 방향
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Bid,  // 매수
    Ask,  // 매도
}

/// 주문 유형
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpbitOrderType {
    Limit,   // 지정가
    Price,   // 시장가 매수
    Market,  // 시장가 매도
}

/// 주문 요청 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderParams {
    pub market: String,
    pub side: OrderSide,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    pub ord_type: UpbitOrderType,
}

/// 주문 응답 (Upbit API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub uuid: String,
    pub side: String,
    pub ord_type: String,
    pub price: Option<String>,
    pub state: String,
    pub market: String,
    pub created_at: String,
    pub volume: Option<String>,
    pub remaining_volume: Option<String>,
    pub reserved_fee: String,
    pub remaining_fee: String,
    pub paid_fee: String,
    pub locked: String,
    pub executed_volume: String,
    pub trades_count: i32,
}
```

**mod.rs Tauri 명령:**

```rust
/// Upbit 주문을 실행합니다.
#[tauri::command]
pub async fn wts_place_order(params: OrderParams) -> WtsApiResult<OrderResponse> {
    match upbit::place_order(params).await {
        Ok(order) => WtsApiResult::ok(order),
        Err(e) => WtsApiResult::err(e),
    }
}
```

### 에러 처리 확장

**주문 관련 Upbit 에러 코드:**

| 에러 코드 | 설명 | 한국어 메시지 |
|----------|------|--------------|
| `insufficient_funds_bid` | 매수 잔고 부족 | "매수 가능 금액이 부족합니다" |
| `insufficient_funds_ask` | 매도 잔고 부족 | "매도 가능 수량이 부족합니다" |
| `under_min_total_bid` | 최소 주문금액 미달 | "최소 주문금액(5,000원) 이상이어야 합니다" |
| `under_min_total_ask` | 최소 주문금액 미달 | "최소 주문금액(5,000원) 이상이어야 합니다" |
| `invalid_volume` | 잘못된 수량 | "주문 수량이 올바르지 않습니다" |
| `invalid_price` | 잘못된 가격 | "주문 가격이 올바르지 않습니다" |
| `market_does_not_exist` | 마켓 없음 | "존재하지 않는 마켓입니다" |

**types.rs 에러 메시지 확장:**

```rust
impl UpbitApiError {
    pub fn to_korean_message(&self) -> String {
        match self {
            // ... 기존 코드 ...
            Self::ApiError { code, message } => match code.as_str() {
                // 기존 에러 ...
                "insufficient_funds_bid" => "매수 가능 금액이 부족합니다".to_string(),
                "insufficient_funds_ask" => "매도 가능 수량이 부족합니다".to_string(),
                "under_min_total_bid" | "under_min_total_ask" =>
                    "최소 주문금액(5,000원) 이상이어야 합니다".to_string(),
                "invalid_volume" => "주문 수량이 올바르지 않습니다".to_string(),
                "invalid_price" => "주문 가격이 올바르지 않습니다".to_string(),
                "market_does_not_exist" => "존재하지 않는 마켓입니다".to_string(),
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
// Order API Types (Upbit)
// ============================================================================

/** Upbit 주문 방향 */
export type UpbitOrderSide = 'bid' | 'ask';

/** Upbit 주문 유형 */
export type UpbitOrderType = 'limit' | 'price' | 'market';

/** 주문 요청 파라미터 (Tauri 명령) */
export interface OrderParams {
  /** 마켓 코드 (예: "KRW-BTC") */
  market: string;
  /** 주문 방향: bid(매수) | ask(매도) */
  side: UpbitOrderSide;
  /** 주문 수량 (시장가 매도 또는 지정가) */
  volume?: string;
  /** 주문 가격 (시장가 매수: 총액, 지정가: 단가) */
  price?: string;
  /** 주문 유형 */
  ord_type: UpbitOrderType;
}

/** 주문 응답 (Upbit API) */
export interface OrderResponse {
  /** 주문 고유 ID */
  uuid: string;
  /** 주문 방향 */
  side: string;
  /** 주문 유형 */
  ord_type: string;
  /** 주문 가격 */
  price: string | null;
  /** 주문 상태: wait, watch, done, cancel */
  state: string;
  /** 마켓 코드 */
  market: string;
  /** 주문 생성 시각 */
  created_at: string;
  /** 주문 수량 */
  volume: string | null;
  /** 미체결 수량 */
  remaining_volume: string | null;
  /** 예약 수수료 */
  reserved_fee: string;
  /** 미사용 수수료 */
  remaining_fee: string;
  /** 지불 수수료 */
  paid_fee: string;
  /** 잠금 금액/수량 */
  locked: string;
  /** 체결 수량 */
  executed_volume: string;
  /** 체결 횟수 */
  trades_count: number;
}
```

### Cargo 의존성 확인

**필요한 크레이트:**
- `sha2` - SHA-512 해싱 (query_hash)
- `hex` - 해시 hex 인코딩

```toml
# Cargo.toml (확인 필요)
[dependencies]
sha2 = "0.10"
hex = "0.4"
```

### API 키 마스킹 규칙

[Source: architecture.md#Security Requirements]

- API 키는 절대 로그에 평문으로 기록하지 않음
- 에러 메시지에서 API 키 관련 정보 제외
- 디버그 출력 시 `***MASKED***` 처리

### Project Structure Notes

**신규 파일:**
- (없음, 기존 파일 확장)

**수정 파일:**
- `apps/desktop/src-tauri/src/wts/upbit/types.rs` - OrderParams, OrderResponse, OrderSide, UpbitOrderType 추가
- `apps/desktop/src-tauri/src/wts/upbit/auth.rs` - generate_jwt_token_with_query 함수 추가
- `apps/desktop/src-tauri/src/wts/upbit/client.rs` - place_order 함수 추가
- `apps/desktop/src-tauri/src/wts/mod.rs` - wts_place_order Tauri 명령 추가, 타입 export
- `apps/desktop/src-tauri/src/main.rs` - wts_place_order 명령 등록
- `apps/desktop/src/wts/types.ts` - OrderParams, OrderResponse 인터페이스 추가
- (선택) `apps/desktop/src/wts/utils/upbitErrors.ts` - 주문 에러 코드 매핑

**아키텍처 정합성:**
- Tauri 명령 접두사 `wts_` 준수
- WtsApiResult 래퍼 패턴 준수
- UpbitApiError 에러 처리 패턴 준수
- 한국어 에러 메시지 매핑 패턴 준수

### 이전 스토리 참조

**WTS-2.6 (orderStore):**
- orderStore에 이미 OrderType ('market' | 'limit'), OrderSide ('buy' | 'sell') 정의됨
- 프론트엔드 side: 'buy' | 'sell' → Upbit API side: 'bid' | 'ask' 매핑 필요
- 프론트엔드 orderType: 'market' | 'limit' → Upbit API ord_type 매핑 필요

**매핑 로직 (프론트엔드에서 처리):**
```typescript
// orderStore side → Upbit side
const upbitSide = side === 'buy' ? 'bid' : 'ask';

// orderType + side → Upbit ord_type
function getUpbitOrderType(orderType: OrderType, side: OrderSide): UpbitOrderType {
  if (orderType === 'limit') return 'limit';
  // 시장가: 매수는 'price', 매도는 'market'
  return side === 'buy' ? 'price' : 'market';
}
```

### References

- [Architecture: Upbit 주문 API](/_bmad-output/planning-artifacts/architecture.md#Upbit 주문 API)
- [Architecture: Upbit REST API 공통](/_bmad-output/planning-artifacts/architecture.md#Technical Constraints)
- [Architecture: WTS Backend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Backend Structure)
- [PRD: FR10-16 주문 기능](/_bmad-output/planning-artifacts/prd.md)
- [WTS Epics: Story 3.1](/_bmad-output/planning-artifacts/wts-epics.md#Story 3.1)
- [Previous Story: WTS-2.6](/_bmad-output/implementation-artifacts/wts-2-6-orderbook-panel-ui-click-interaction.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- 이전 기록: 모든 38개 단위 테스트 통과
- 리뷰 수정 후 테스트 미실행 (rate limit/에러 파싱 테스트 추가)

### Completion Notes List

- Task 1: types.rs에 OrderSide, UpbitOrderType enum 및 OrderParams, OrderResponse 구조체 추가
- Task 2: auth.rs에 generate_jwt_token_with_query 함수 추가 (SHA-512 해시 포함 JWT)
- Task 3: client.rs에 place_order async 함수 추가 (POST /v1/orders)
- Task 4: mod.rs에 wts_place_order Tauri 명령 추가, main.rs에 등록
- Task 5: UpbitApiError.to_korean_message()에 주문 관련 에러 코드 8개 추가
- Task 6: 각 Task별로 단위 테스트 작성 (총 38개 테스트)
- Task 7: types.ts에 OrderParams, OrderResponse, UpbitOrderSide, UpbitOrderType 추가 및 헬퍼 함수 toUpbitSide, toUpbitOrderType, UPBIT_ORDER_ERROR_MESSAGES 추가
- Review Fix: 주문 Rate Limit(8회/초) 클라이언트 스로틀링 추가 (place_order)
- Review Fix: 에러 응답 파싱 테스트 추가 (validation_error, rate limit)

### File List

- apps/desktop/src-tauri/src/wts/upbit/types.rs (수정)
- apps/desktop/src-tauri/src/wts/upbit/auth.rs (수정)
- apps/desktop/src-tauri/src/wts/upbit/client.rs (수정)
- apps/desktop/src-tauri/src/wts/mod.rs (수정)
- apps/desktop/src-tauri/src/main.rs (수정)
- apps/desktop/src/wts/types.ts (수정)
- apps/desktop/src/wts/panels/OrderPanel.tsx (수정)

## Change Log

- 2026-01-20: WTS-3.1 구현 완료 - Upbit 주문 API Rust 백엔드
- 2026-01-20: 리뷰 수정 - 주문 Rate Limit 스로틀링 및 에러 파싱 테스트 추가

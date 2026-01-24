# Story WTS-4.1: 입금 API Rust 백엔드 구현

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **Upbit 입금 주소 조회/생성 API와 연동된 백엔드**,
So that **입금 주소를 받아볼 수 있다**.

## Acceptance Criteria

1. **Given** Upbit API 키가 설정되어 있을 때 **When** `wts_get_deposit_address` Tauri 명령을 호출하면 **Then** 해당 자산/네트워크의 입금 주소가 반환되어야 한다
2. **Given** Upbit API 키가 설정되어 있을 때 **When** 입금 주소가 없는 경우 **Then** `wts_generate_deposit_address` 명령으로 생성 요청을 할 수 있어야 한다
3. **Given** 입금 주소 생성이 요청되었을 때 **When** 비동기 생성이 진행 중이면 **Then** 생성 중 상태(`is_generating: true`)가 반환되어야 한다
4. **Given** 비동기 생성 상태일 때 **When** 재조회하면 **Then** 생성 완료된 주소 또는 계속 생성 중 상태가 반환되어야 한다
5. **Given** Upbit API 키가 설정되지 않았을 때 **When** 입금 주소 API를 호출하면 **Then** MissingApiKey 에러가 반환되어야 한다
6. **Given** 지원하지 않는 자산/네트워크일 때 **When** 입금 주소를 조회하면 **Then** 적절한 에러 메시지가 반환되어야 한다
7. **Given** 입금 가능 정보 조회 시 **When** `wts_get_deposit_chance` 명령을 호출하면 **Then** 입금 가능 여부, 최소 수량, 네트워크 목록이 반환되어야 한다
8. **Given** API 호출 시 **When** Rate Limit(30회/초)을 준수해야 한다

## Tasks / Subtasks

- [x] Task 1: 입금 관련 타입 정의 (AC: #1, #3, #7)
  - [x] Subtask 1.1: `DepositAddressParams` 구조체 정의 (currency, net_type)
  - [x] Subtask 1.2: `DepositAddressResponse` 구조체 정의 (currency, net_type, deposit_address, secondary_address)
  - [x] Subtask 1.3: `DepositChanceParams` 구조체 정의 (currency, net_type)
  - [x] Subtask 1.4: `DepositChanceResponse` 구조체 정의 (currency, net_type, network, deposit_state, minimum 등)
  - [x] Subtask 1.5: `GenerateAddressParams` 구조체 정의 (currency, net_type)
  - [x] Subtask 1.6: `GenerateAddressResponse` 구조체 정의 (success, message)
  - [x] Subtask 1.7: 타입에 serde Serialize/Deserialize derive 추가

- [x] Task 2: 입금 주소 조회 API 구현 (AC: #1, #5, #6, #8)
  - [x] Subtask 2.1: `get_deposit_address` async 함수 구현 (client.rs)
  - [x] Subtask 2.2: GET `/v1/deposits/coin_address` 엔드포인트 호출
  - [x] Subtask 2.3: 쿼리 파라미터 URL 인코딩 (currency, net_type)
  - [x] Subtask 2.4: 쿼리 해시 포함 JWT 생성
  - [x] Subtask 2.5: 응답 파싱 및 UpbitApiError 변환
  - [x] Subtask 2.6: 주소 미존재 시 null 반환 처리

- [x] Task 3: 입금 주소 생성 API 구현 (AC: #2, #3, #4, #8)
  - [x] Subtask 3.1: `generate_deposit_address` async 함수 구현 (client.rs)
  - [x] Subtask 3.2: POST `/v1/deposits/generate_coin_address` 엔드포인트 호출
  - [x] Subtask 3.3: JSON 바디로 currency, net_type 전송
  - [x] Subtask 3.4: 비동기 생성 응답 처리 (success=true, message="creating")
  - [x] Subtask 3.5: 이미 존재하는 주소 응답 처리

- [x] Task 4: 입금 가능 정보 조회 API 구현 (AC: #7, #8)
  - [x] Subtask 4.1: `get_deposit_chance` async 함수 구현 (client.rs)
  - [x] Subtask 4.2: GET `/v1/deposits/chance/coin` 엔드포인트 호출
  - [x] Subtask 4.3: 쿼리 파라미터 URL 인코딩 (currency, net_type)
  - [x] Subtask 4.4: 응답 파싱 (입금 가능 상태, 네트워크 정보, 최소 수량)

- [x] Task 5: Tauri 명령 등록 (AC: #1-#7)
  - [x] Subtask 5.1: `wts_get_deposit_address` Tauri 명령 함수 정의 (mod.rs)
  - [x] Subtask 5.2: `wts_generate_deposit_address` Tauri 명령 함수 정의
  - [x] Subtask 5.3: `wts_get_deposit_chance` Tauri 명령 함수 정의
  - [x] Subtask 5.4: main.rs에 명령 등록

- [x] Task 6: 에러 타입 확장 (AC: #5, #6)
  - [x] Subtask 6.1: UpbitApiError.to_korean_message()에 입금 관련 에러 코드 추가
  - [x] Subtask 6.2: 입금 불가 상태 에러 메시지 추가
  - [x] Subtask 6.3: 지원하지 않는 네트워크 에러 메시지 추가

- [x] Task 7: 단위 테스트 작성 (AC: #1-#8)
  - [x] Subtask 7.1: DepositAddressParams 직렬화 테스트
  - [x] Subtask 7.2: DepositAddressResponse 역직렬화 테스트
  - [x] Subtask 7.3: DepositChanceResponse 역직렬화 테스트
  - [x] Subtask 7.4: 에러 응답 파싱 테스트
  - [x] Subtask 7.5: API 키 누락 시 에러 테스트

- [x] Task 8: TypeScript 타입 동기화 (AC: #1, #3, #7)
  - [x] Subtask 8.1: types.ts에 DepositAddressParams 인터페이스 추가
  - [x] Subtask 8.2: types.ts에 DepositAddressResponse 인터페이스 추가
  - [x] Subtask 8.3: types.ts에 DepositChanceResponse 인터페이스 추가
  - [x] Subtask 8.4: 입금 관련 에러 코드 매핑 추가

- [x] Task 9: 코드 리뷰 후속 조치 (Senior Developer Review)
  - [x] Subtask 9.1: `upbit::client::check_connection` 추가 및 `mod.rs` 통합 (중복 제거)
  - [x] Subtask 9.2: 사용되지 않는 임포트 및 경고 정리

## Dev Notes

### Upbit 입금 API 스펙

[Source: architecture.md#Upbit 입금 API]

**엔드포인트:**

| API | 엔드포인트 | 메서드 | Rate Limit |
|-----|-----------|--------|------------|
| 입금 가능 정보 | `/v1/deposits/chance/coin` | GET | 30/초 |
| 입금 주소 생성 | `/v1/deposits/generate_coin_address` | POST | 30/초 |
| 개별 주소 조회 | `/v1/deposits/coin_address` | GET | 30/초 |
| 주소 목록 조회 | `/v1/deposits/coin_addresses` | GET | 30/초 |

### 입금 가능 정보 조회 (GET /v1/deposits/chance/coin)

**요청 파라미터:**
```
?currency=BTC&net_type=BTC
```

**응답 필드:**
```json
{
  "currency": "BTC",
  "net_type": "BTC",
  "network": {
    "name": "Bitcoin",
    "net_type": "BTC",
    "priority": 1,
    "deposit_state": "normal",
    "confirm_count": 3
  },
  "deposit_state": "normal",
  "member_level": {
    "security_level": 3,
    "fee_level": 1,
    "deposit_limit": "10000000000",
    "withdraw_limit": "10000000000"
  },
  "minimum": "0.001"
}
```

### 입금 주소 조회 (GET /v1/deposits/coin_address)

**요청 파라미터:**
```
?currency=BTC&net_type=BTC
```

**응답 필드:**
```json
{
  "currency": "BTC",
  "net_type": "BTC",
  "deposit_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
  "secondary_address": null
}
```

**주의:** 주소가 생성되지 않은 경우 `deposit_address`가 `null`로 반환됩니다.

### 입금 주소 생성 (POST /v1/deposits/generate_coin_address)

**요청 바디:**
```json
{
  "currency": "BTC",
  "net_type": "BTC"
}
```

**응답 필드 (비동기 생성 시):**
```json
{
  "success": true,
  "message": "creating"
}
```

**응답 필드 (이미 존재 시):**
```json
{
  "currency": "BTC",
  "net_type": "BTC",
  "deposit_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
  "secondary_address": null
}
```

### 비동기 주소 생성 처리 패턴

[Source: architecture.md#Upbit 입금 제약]

Upbit 입금 주소 생성은 비동기로 처리됩니다:

1. `generate_deposit_address` 호출 → `success: true, message: "creating"` 반환
2. 클라이언트에서 일정 시간(3초) 후 `get_deposit_address` 재조회
3. 주소 생성 완료되면 `deposit_address` 필드에 주소 반환
4. 최대 5회 재시도 후 실패 처리

**프론트엔드 재조회 로직:**
```typescript
// 재조회 로직은 프론트엔드(Story 4.4)에서 구현
async function pollDepositAddress(currency: string, netType: string) {
  const MAX_RETRIES = 5;
  const RETRY_DELAY = 3000; // 3초

  for (let i = 0; i < MAX_RETRIES; i++) {
    const result = await invoke('wts_get_deposit_address', { params: { currency, net_type: netType } });
    if (result.data?.deposit_address) {
      return result.data;
    }
    await new Promise(resolve => setTimeout(resolve, RETRY_DELAY));
  }
  throw new Error('입금 주소 생성 시간이 초과되었습니다');
}
```

### 기존 코드 패턴

**client.rs 패턴 (WTS-3.1 참조):**

[Source: apps/desktop/src-tauri/src/wts/upbit/client.rs]

```rust
// GET 요청 (쿼리 파라미터 포함)
pub async fn get_deposit_address(params: DepositAddressParams) -> Result<DepositAddressResponse, UpbitApiError> {
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
        .get(format!("{}/deposits/coin_address", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("currency", &params.currency), ("net_type", &params.net_type)])
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    // 에러 처리는 기존 패턴 따름
    // ...
}
```

**타입 정의 패턴 (types.rs):**

```rust
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

/// 입금 가능 정보 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositChanceResponse {
    /// 자산 코드
    pub currency: String,
    /// 네트워크 타입
    pub net_type: String,
    /// 네트워크 정보
    pub network: DepositNetwork,
    /// 입금 상태 (normal, paused 등)
    pub deposit_state: String,
    /// 최소 입금 수량
    pub minimum: String,
}

/// 네트워크 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositNetwork {
    /// 네트워크 이름
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
```

**mod.rs Tauri 명령:**

```rust
/// Upbit 입금 주소를 조회합니다.
#[tauri::command]
pub async fn wts_get_deposit_address(params: DepositAddressParams) -> WtsApiResult<DepositAddressResponse> {
    match upbit::get_deposit_address(params).await {
        Ok(address) => WtsApiResult::ok(address),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 입금 주소를 생성합니다 (비동기).
#[tauri::command]
pub async fn wts_generate_deposit_address(params: DepositAddressParams) -> WtsApiResult<GenerateAddressResponse> {
    match upbit::generate_deposit_address(params).await {
        Ok(response) => WtsApiResult::ok(response),
        Err(e) => WtsApiResult::err(e),
    }
}

/// Upbit 입금 가능 정보를 조회합니다.
#[tauri::command]
pub async fn wts_get_deposit_chance(params: DepositChanceParams) -> WtsApiResult<DepositChanceResponse> {
    match upbit::get_deposit_chance(params).await {
        Ok(chance) => WtsApiResult::ok(chance),
        Err(e) => WtsApiResult::err(e),
    }
}
```

### 에러 처리 확장

**입금 관련 Upbit 에러 코드:**

| 에러 코드 | 설명 | 한국어 메시지 |
|----------|------|--------------|
| `deposit_address_not_found` | 입금 주소 없음 | "입금 주소가 아직 생성되지 않았습니다" |
| `invalid_currency` | 잘못된 자산 코드 | "지원하지 않는 자산입니다" |
| `invalid_net_type` | 잘못된 네트워크 | "지원하지 않는 네트워크입니다" |
| `deposit_paused` | 입금 일시 중단 | "현재 입금이 일시 중단되었습니다" |
| `deposit_suspended` | 입금 중단 | "해당 자산의 입금이 중단되었습니다" |
| `address_generation_failed` | 주소 생성 실패 | "입금 주소 생성에 실패했습니다" |

**types.rs 에러 메시지 확장:**

```rust
impl UpbitApiError {
    pub fn to_korean_message(&self) -> String {
        match self {
            // ... 기존 코드 ...
            Self::ApiError { code, message } => match code.as_str() {
                // 기존 에러 ...
                // 입금 관련 에러
                "deposit_address_not_found" => "입금 주소가 아직 생성되지 않았습니다".to_string(),
                "invalid_currency" => "지원하지 않는 자산입니다".to_string(),
                "invalid_net_type" => "지원하지 않는 네트워크입니다".to_string(),
                "deposit_paused" => "현재 입금이 일시 중단되었습니다".to_string(),
                "deposit_suspended" => "해당 자산의 입금이 중단되었습니다".to_string(),
                "address_generation_failed" => "입금 주소 생성에 실패했습니다".to_string(),
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
// Deposit API Types (Upbit)
// ============================================================================

/** 입금 주소 조회 파라미터 */
export interface DepositAddressParams {
  /** 자산 코드 (예: "BTC", "ETH") */
  currency: string;
  /** 네트워크 타입 (예: "BTC", "ETH", "TRX" 등) */
  net_type: string;
}

/** 입금 주소 조회 응답 */
export interface DepositAddressResponse {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 입금 주소 (null일 수 있음 - 생성 중) */
  deposit_address: string | null;
  /** 보조 주소 (XRP tag, EOS memo 등) */
  secondary_address: string | null;
}

/** 입금 가능 정보 파라미터 */
export interface DepositChanceParams {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
}

/** 네트워크 정보 */
export interface DepositNetwork {
  /** 네트워크 이름 */
  name: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 우선순위 */
  priority: number;
  /** 입금 상태 */
  deposit_state: string;
  /** 확인 횟수 */
  confirm_count: number;
}

/** 입금 가능 정보 응답 */
export interface DepositChanceResponse {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 네트워크 정보 */
  network: DepositNetwork;
  /** 입금 상태 (normal, paused 등) */
  deposit_state: string;
  /** 최소 입금 수량 */
  minimum: string;
}

/** 입금 주소 생성 응답 (비동기) */
export type GenerateAddressResponse =
  | { success: true; message: 'creating' }  // 비동기 생성 중
  | DepositAddressResponse;                  // 이미 존재하는 주소

/** 입금 상태 타입 */
export type DepositState = 'normal' | 'paused' | 'suspended';

/** 입금 상태가 정상인지 확인 */
export function isDepositAvailable(state: string): boolean {
  return state === 'normal';
}
```

**입금 에러 메시지 추가 (UPBIT_ORDER_ERROR_MESSAGES 확장):**

```typescript
// 입금 관련 에러
deposit_address_not_found: '입금 주소가 아직 생성되지 않았습니다',
invalid_currency: '지원하지 않는 자산입니다',
invalid_net_type: '지원하지 않는 네트워크입니다',
deposit_paused: '현재 입금이 일시 중단되었습니다',
deposit_suspended: '해당 자산의 입금이 중단되었습니다',
address_generation_failed: '입금 주소 생성에 실패했습니다',
```

### Project Structure Notes

**신규 타입 추가 위치:**
- `apps/desktop/src-tauri/src/wts/upbit/types.rs`
- `apps/desktop/src/wts/types.ts`

**수정 파일:**
- `apps/desktop/src-tauri/src/wts/upbit/client.rs` - 입금 관련 API 함수 추가 및 `check_connection` 구현
- `apps/desktop/src-tauri/src/wts/upbit/types.rs` - 입금 관련 타입 추가
- `apps/desktop/src-tauri/src/wts/upbit/mod.rs` - 사용되지 않는 re-export 정리
- `apps/desktop/src-tauri/src/wts/mod.rs` - Tauri 명령 추가, 타입 export, 연결 체크 통합
- `apps/desktop/src-tauri/src/main.rs` - 명령 등록
- `apps/desktop/src/wts/types.ts` - TypeScript 타입 추가

**아키텍처 정합성:**
- Tauri 명령 접두사 `wts_` 준수
- WtsApiResult 래퍼 패턴 준수
- UpbitApiError 에러 처리 패턴 준수
- 한국어 에러 메시지 매핑 패턴 준수
- Rate Limit(30회/초) 준수 (Exchange Default)

### 이전 스토리 참조

**WTS-3.1 (주문 API Rust 백엔드):**
- JWT 토큰 생성 패턴 재사용 (`generate_jwt_token_with_query`)
- 에러 처리 패턴 재사용 (`parse_upbit_error`)
- Rate Limit 처리 패턴 참조 (입금은 30회/초)

**WTS-2.1 (잔고 조회 백엔드):**
- GET 요청 + JWT 인증 패턴 재사용
- API 키 로드 패턴 재사용 (`load_api_keys`)

### 주요 구현 고려사항

1. **비동기 주소 생성 처리:**
   - 백엔드는 응답만 반환하고, 재조회 로직은 프론트엔드(Story 4.4)에서 처리
   - `GenerateAddressResponse`는 `untagged` enum으로 두 가지 응답 형태 처리

2. **네트워크 타입:**
   - 같은 자산도 여러 네트워크 지원 (예: USDT → ERC20, TRC20, BEP20)
   - `net_type` 파라미터 필수

3. **보조 주소:**
   - XRP (Destination Tag), EOS (Memo) 등은 `secondary_address` 사용
   - UI에서 표시 필요 (Story 4.3)

4. **입금 상태 확인:**
   - `deposit_state`가 `normal`이 아니면 입금 불가
   - UI에서 상태 표시 필요 (Story 4.2)

### References

- [Architecture: Upbit 입금 API](/_bmad-output/planning-artifacts/architecture.md#Upbit 입금 API)
- [Architecture: WTS Backend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Backend Structure)
- [PRD: FR17-20 입금 기능](/_bmad-output/planning-artifacts/prd.md)
- [WTS Epics: Epic 4](/_bmad-output/planning-artifacts/wts-epics.md#Epic 4)
- [Previous Story: WTS-3.1 주문 API 백엔드](/_bmad-output/implementation-artifacts/wts-3-1-order-api-rust-backend.md)
- [Previous Story: WTS-2.1 잔고 조회 백엔드](/_bmad-output/implementation-artifacts/wts-2-1-upbit-api-auth-balance-backend.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Rust 51개 테스트 통과 (types.rs, client.rs)
- TypeScript types.test.ts 52개 테스트 통과 (새 입금 타입 포함)
- Senior Developer Review 후속 조치 테스트 통과 (9개 client 테스트, 22개 types 테스트)

### Completion Notes List

- **Task 1-8**: 초기 구현 완료 및 테스트 통과
- **Task 9**: 코드 리뷰 피드백 반영. Upbit 연결 체크 로직을 `upbit::client`로 일원화하여 중복을 제거하고 유지보수성 향상. 불필요한 경고 정리 완료.

### File List

**Modified:**
- `apps/desktop/src-tauri/src/wts/upbit/types.rs` - 입금 API 타입 정의 및 에러 메시지 추가
- `apps/desktop/src-tauri/src/wts/upbit/client.rs` - 입금 API 함수 3개 및 `check_connection` 구현
- `apps/desktop/src-tauri/src/wts/upbit/mod.rs` - 사용되지 않는 re-export 정리
- `apps/desktop/src-tauri/src/wts/mod.rs` - Tauri 명령 함수 3개 추가 및 연결 체크 로직 통합
- `apps/desktop/src-tauri/src/main.rs` - Tauri 명령 등록
- `apps/desktop/src/wts/types.ts` - TypeScript 입금 타입 및 헬퍼 함수 추가
- `apps/desktop/src/wts/__tests__/types.test.ts` - TypeScript 입금 타입 테스트 추가
- `_bmad-output/implementation-artifacts/sprint-status.yaml` - 스토리 상태 업데이트
# Story WTS-2.1: Upbit API 인증 및 잔고 조회 Rust 백엔드

Status: done

## Story

As a **트레이더**,
I want **Upbit API를 통해 내 잔고를 조회하는 백엔드 기능**,
So that **보유 자산 현황을 확인할 수 있다**.

## Acceptance Criteria

1. **Given** Upbit API 키가 환경 변수에 설정되어 있을 때 **When** `wts_get_balance` Tauri 명령을 호출하면 **Then** JWT 토큰이 생성되어 Upbit API에 인증 요청이 전송되어야 한다
2. **Given** Upbit API 키가 환경 변수에 설정되어 있을 때 **When** `wts_get_balance` 호출이 성공하면 **Then** 자산별 잔고(currency, balance, locked, avg_buy_price)가 반환되어야 한다
3. **Given** Upbit API 호출이 실패했을 때 **When** API 에러가 발생하면 **Then** 에러 코드와 메시지가 반환되어야 한다
4. **Given** API 호출 시 **When** Rate Limit(30회/초)을 초과하면 **Then** 적절한 에러 메시지가 반환되어야 한다
5. **Given** 환경 변수에 API 키가 없을 때 **When** `wts_get_balance`를 호출하면 **Then** 설정 오류 메시지가 반환되어야 한다

## Tasks / Subtasks

- [x] Task 1: Upbit API 인증 모듈 구현 (AC: #1)
  - [x] Subtask 1.1: `apps/desktop/src-tauri/src/wts/upbit/mod.rs` 모듈 구조 생성
  - [x] Subtask 1.2: `apps/desktop/src-tauri/src/wts/upbit/auth.rs` JWT 토큰 생성 로직 구현
  - [x] Subtask 1.3: HMAC-SHA256 서명 생성 (jsonwebtoken 크레이트 사용)
  - [x] Subtask 1.4: 환경 변수에서 API 키/시크릿 로드 (UPBIT_ACCESS_KEY, UPBIT_SECRET_KEY)

- [x] Task 2: Upbit 잔고 조회 API 클라이언트 구현 (AC: #1, #2)
  - [x] Subtask 2.1: `apps/desktop/src-tauri/src/wts/upbit/client.rs` HTTP 클라이언트 구현
  - [x] Subtask 2.2: `GET /v1/accounts` 엔드포인트 호출 구현
  - [x] Subtask 2.3: Authorization Bearer 헤더 설정
  - [x] Subtask 2.4: 응답 파싱 및 BalanceEntry 타입 반환

- [x] Task 3: Tauri 명령 정의 (AC: #1, #2, #3, #4, #5)
  - [x] Subtask 3.1: `wts_get_balance` 명령 정의 (apps/desktop/src-tauri/src/wts/mod.rs)
  - [x] Subtask 3.2: main.rs에 명령 등록
  - [x] Subtask 3.3: 에러 응답 타입 정의 (WtsApiError)

- [x] Task 4: 타입 정의 확장 (AC: #2)
  - [x] Subtask 4.1: Rust BalanceEntry 타입 (apps/desktop/src-tauri/src/wts/upbit/types.rs)
  - [x] Subtask 4.2: TypeScript BalanceEntry 타입 (apps/desktop/src/wts/types.ts)
  - [x] Subtask 4.3: WtsApiResult 래퍼 타입 정의

- [x] Task 5: 에러 처리 구현 (AC: #3, #4, #5)
  - [x] Subtask 5.1: Upbit 에러 코드 → 한국어 메시지 매핑
  - [x] Subtask 5.2: Rate Limit 에러 감지 (HTTP 429 또는 Too many API requests)
  - [x] Subtask 5.3: API 키 미설정 에러 처리

- [x] Task 6: 테스트 작성 (AC: #1, #2, #3, #4, #5)
  - [x] Subtask 6.1: JWT 토큰 생성 단위 테스트
  - [x] Subtask 6.2: 잔고 응답 파싱 테스트
  - [x] Subtask 6.3: 에러 응답 처리 테스트

## Dev Notes

### Architecture 준수사항

**Tauri Command Naming:**
[Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]

- Rust 명령: snake_case (예: `wts_get_balance`)
- 접두사: `wts_` (WTS 전용 명령 구분)
- TypeScript invoke: 동일 snake_case

**WTS Backend Structure:**
[Source: _bmad-output/planning-artifacts/architecture.md#Project Structure]

```
apps/desktop/src-tauri/src/wts/
├── mod.rs              # 모듈 선언 + wts_get_balance 명령
├── types.rs            # 기존 타입 정의
├── upbit/
│   ├── mod.rs          # Upbit 모듈 선언 (신규)
│   ├── auth.rs         # JWT 토큰 생성 (신규)
│   ├── client.rs       # REST API 클라이언트 (신규)
│   └── types.rs        # Upbit API 응답 타입 (신규)
```

### Upbit API 상세

**잔고 조회 API:**
[Source: _bmad-output/planning-artifacts/architecture.md#Technical Constraints]

| 항목 | 값 |
|------|-----|
| Endpoint | `GET https://api.upbit.com/v1/accounts` |
| Rate Limit | 30회/초 (Exchange Default) |
| 인증 | `Authorization: Bearer {JWT_TOKEN}` |

**JWT 토큰 생성:**
[Source: Upbit API 공식 문서]

```rust
// JWT Payload
{
  "access_key": "{ACCESS_KEY}",
  "nonce": "{UUID v4}",
  "timestamp": {Unix ms}
}

// JWT 서명: HMAC-SHA256 with SECRET_KEY
// 헤더: {"alg": "HS256", "typ": "JWT"}
```

**잔고 조회 응답:**

```json
[
  {
    "currency": "BTC",
    "balance": "0.12345678",
    "locked": "0.00000000",
    "avg_buy_price": "50000000.00",
    "avg_buy_price_modified": false,
    "unit_currency": "KRW"
  },
  {
    "currency": "KRW",
    "balance": "1000000.00",
    "locked": "0.00",
    "avg_buy_price": "0",
    "avg_buy_price_modified": true,
    "unit_currency": "KRW"
  }
]
```

**Upbit 에러 응답:**
[Source: _bmad-output/planning-artifacts/architecture.md#Technical Constraints]

| 상태 코드 | 에러 코드 | 한국어 메시지 |
|---------|---------|------------|
| 400 | `validation_error` | 잘못된 요청입니다 |
| 401 | `jwt_verification` | JWT 인증에 실패했습니다 |
| 401 | `no_authorization_ip` | 허용되지 않은 IP입니다 |
| 401 | `expired_access_key` | 만료된 API 키입니다 |
| 429 | (Rate Limit) | 요청이 너무 많습니다. 잠시 후 다시 시도하세요 |
| 500 | (Server Error) | 서버 오류가 발생했습니다 |

### 이전 스토리에서 학습한 사항

**WTS Epic 1 완료:**
- wts_open_window, wts_check_connection 명령 구현됨
- reqwest 클라이언트 패턴 확립
- ConnectionCheckResult 타입 정의됨
- 기존 mod.rs에 명령 추가하는 패턴 확립

**기존 코드 현황:**

```rust
// apps/desktop/src-tauri/src/wts/mod.rs (현재)
pub mod types;
pub use types::*;

#[tauri::command]
pub async fn wts_open_window(app: tauri::AppHandle) -> Result<(), String> { ... }

#[tauri::command]
pub async fn wts_check_connection(exchange: String) -> ConnectionCheckResult { ... }
```

**main.rs 명령 등록 패턴:**

```rust
// 기존 invoke_handler에 wts 명령 추가
.invoke_handler(tauri::generate_handler![
    // ... 기존 명령들
    wts::wts_open_window,
    wts::wts_check_connection,
    wts::wts_get_balance,  // 신규 추가
])
```

### 구현 가이드

**1. Upbit 모듈 구조:**

```rust
// apps/desktop/src-tauri/src/wts/upbit/mod.rs
pub mod auth;
pub mod client;
pub mod types;

pub use auth::*;
pub use client::*;
pub use types::*;
```

**2. JWT 토큰 생성:**

```rust
// apps/desktop/src-tauri/src/wts/upbit/auth.rs
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, Header, AlgorithmType};
use sha2::Sha256;
use std::collections::BTreeMap;
use uuid::Uuid;

pub fn generate_jwt_token(access_key: &str, secret_key: &str) -> Result<String, String> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut claims = BTreeMap::new();
    claims.insert("access_key", access_key.to_string());
    claims.insert("nonce", Uuid::new_v4().to_string());
    claims.insert("timestamp", chrono::Utc::now().timestamp_millis().to_string());

    let header = Header {
        algorithm: AlgorithmType::Hs256,
        ..Default::default()
    };

    claims.sign_with_key(&key)
        .map_err(|e| e.to_string())
}
```

**3. 잔고 조회 클라이언트:**

```rust
// apps/desktop/src-tauri/src/wts/upbit/client.rs
use super::{generate_jwt_token, BalanceEntry, UpbitApiError};

const UPBIT_API_BASE: &str = "https://api.upbit.com/v1";

pub async fn get_balance() -> Result<Vec<BalanceEntry>, UpbitApiError> {
    let access_key = std::env::var("UPBIT_ACCESS_KEY")
        .map_err(|_| UpbitApiError::MissingApiKey)?;
    let secret_key = std::env::var("UPBIT_SECRET_KEY")
        .map_err(|_| UpbitApiError::MissingApiKey)?;

    let token = generate_jwt_token(&access_key, &secret_key)
        .map_err(|e| UpbitApiError::JwtError(e))?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/accounts", UPBIT_API_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| UpbitApiError::NetworkError(e.to_string()))?;

    if response.status() == 429 {
        return Err(UpbitApiError::RateLimitExceeded);
    }

    if !response.status().is_success() {
        let error_body: serde_json::Value = response.json().await
            .unwrap_or_default();
        return Err(UpbitApiError::ApiError {
            code: error_body["error"]["name"].as_str()
                .unwrap_or("unknown").to_string(),
            message: error_body["error"]["message"].as_str()
                .unwrap_or("알 수 없는 오류").to_string(),
        });
    }

    response.json::<Vec<BalanceEntry>>().await
        .map_err(|e| UpbitApiError::ParseError(e.to_string()))
}
```

**4. 타입 정의:**

```rust
// apps/desktop/src-tauri/src/wts/upbit/types.rs
use serde::{Deserialize, Serialize};

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
            Self::RateLimitExceeded => "요청이 너무 많습니다. 잠시 후 다시 시도하세요".to_string(),
            Self::ApiError { code, message } => {
                match code.as_str() {
                    "jwt_verification" => "JWT 인증에 실패했습니다".to_string(),
                    "no_authorization_ip" => "허용되지 않은 IP입니다".to_string(),
                    "expired_access_key" => "만료된 API 키입니다".to_string(),
                    _ => message.clone(),
                }
            }
            Self::ParseError(_) => "응답 파싱에 실패했습니다".to_string(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WtsApiErrorResponse {
    pub code: String,
    pub message: String,
}
```

**5. Tauri 명령:**

```rust
// apps/desktop/src-tauri/src/wts/mod.rs (확장)
pub mod upbit;

use upbit::{get_balance, BalanceEntry, UpbitApiError, WtsApiResult, WtsApiErrorResponse};

#[tauri::command]
pub async fn wts_get_balance() -> WtsApiResult<Vec<BalanceEntry>> {
    match get_balance().await {
        Ok(balances) => WtsApiResult {
            success: true,
            data: Some(balances),
            error: None,
        },
        Err(e) => WtsApiResult {
            success: false,
            data: None,
            error: Some(WtsApiErrorResponse {
                code: format!("{:?}", e),
                message: e.to_korean_message(),
            }),
        },
    }
}
```

**6. TypeScript 타입 확장:**

```typescript
// apps/desktop/src/wts/types.ts (확장)

/** 잔고 엔트리 */
export interface BalanceEntry {
  /** 화폐 코드 (예: "BTC", "KRW") */
  currency: string;
  /** 가용 잔고 */
  balance: string;
  /** 잠금 잔고 (미체결 주문) */
  locked: string;
  /** 평균 매수가 */
  avg_buy_price: string;
  /** 평균 매수가 수정 여부 */
  avg_buy_price_modified: boolean;
  /** 평가 기준 화폐 (예: "KRW") */
  unit_currency: string;
}

/** WTS API 응답 래퍼 */
export interface WtsApiResult<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
  };
}
```

**7. Cargo.toml 의존성:**

```toml
# apps/desktop/src-tauri/Cargo.toml (확장)
[dependencies]
# 기존 의존성...
jsonwebtoken = "9"  # JWT 생성
hmac = "0.12"       # HMAC-SHA256
sha2 = "0.10"       # SHA256
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"      # timestamp
```

### 환경 변수 설정

**.env 파일:**

```env
# Upbit API Keys
UPBIT_ACCESS_KEY=your_access_key_here
UPBIT_SECRET_KEY=your_secret_key_here
```

**참고:** API 키는 Upbit 개발자 센터에서 발급
- 필요 권한: 자산 조회 (accounts)
- IP 화이트리스트 설정 필요

### 테스트 예시

```rust
// apps/desktop/src-tauri/src/wts/upbit/tests.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_token_format() {
        // JWT는 header.payload.signature 형식
        let token = generate_jwt_token("test_key", "test_secret").unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

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
    }

    #[test]
    fn test_error_korean_message() {
        let err = UpbitApiError::RateLimitExceeded;
        assert_eq!(err.to_korean_message(), "요청이 너무 많습니다. 잠시 후 다시 시도하세요");

        let err = UpbitApiError::MissingApiKey;
        assert_eq!(err.to_korean_message(), "API 키가 설정되지 않았습니다");
    }
}
```

### Project Structure Notes

**신규 파일:**
- `apps/desktop/src-tauri/src/wts/upbit/mod.rs`
- `apps/desktop/src-tauri/src/wts/upbit/auth.rs`
- `apps/desktop/src-tauri/src/wts/upbit/client.rs`
- `apps/desktop/src-tauri/src/wts/upbit/types.rs`

**변경 파일:**
- `apps/desktop/src-tauri/src/wts/mod.rs` - upbit 모듈 추가, wts_get_balance 명령
- `apps/desktop/src-tauri/src/main.rs` - wts_get_balance 명령 등록
- `apps/desktop/src-tauri/Cargo.toml` - JWT/HMAC 의존성 추가
- `apps/desktop/src/wts/types.ts` - BalanceEntry, WtsApiResult 타입 추가

**디렉토리 구조 변경:**

```
apps/desktop/src-tauri/src/wts/
├── mod.rs              # (변경) upbit 모듈 추가, wts_get_balance
├── types.rs            # (기존)
└── upbit/              # (신규)
    ├── mod.rs          # 모듈 선언
    ├── auth.rs         # JWT 토큰 생성
    ├── client.rs       # REST API 클라이언트
    └── types.rs        # Upbit API 타입
```

### 보안 고려사항

**API 키 관리:**
- 환경 변수로만 API 키 로드 (.env 파일)
- 로그에 API 키/시크릿 절대 출력 금지
- JWT 토큰은 매 요청마다 새로 생성 (nonce + timestamp)

**Rate Limit 준수:**
- Exchange Default: 30회/초
- 429 응답 시 즉시 에러 반환 (재시도 로직은 프론트엔드에서)

### Git 최근 커밋 패턴

**커밋 메시지 형식:** `feat(wts): 설명`

예시:
- `feat(wts): add Upbit API authentication module`
- `feat(wts): implement wts_get_balance command`

### References

- [Architecture Document: Backend Structure](_bmad-output/planning-artifacts/architecture.md#WTS Backend Structure)
- [Architecture Document: Upbit API Constraints](_bmad-output/planning-artifacts/architecture.md#Technical Constraints)
- [WTS Epics: Story 2.1](_bmad-output/planning-artifacts/wts-epics.md#Story 2.1)
- [Previous Story: WTS-1.6](_bmad-output/implementation-artifacts/wts-1-6-console-panel-basic-structure.md)
- [Upbit API 공식 문서](https://docs.upbit.com/)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Rust 빌드: `cargo check` 성공 (경고만, 미사용 타입)
- Rust 테스트: 22개 전체 통과 (신규 9개 포함)
- TypeScript: `tsc --noEmit` 통과
- Frontend 테스트: 120/121 통과 (기존 테스트 1개 타임아웃, 이 스토리와 무관)

### Completion Notes List

- JWT 토큰 생성: `jsonwebtoken` 크레이트 사용, HS256 알고리즘
- 환경 변수: `UPBIT_ACCESS_KEY`, `UPBIT_SECRET_KEY`에서 로드
- 에러 처리: 한국어 메시지 매핑 완료 (MissingApiKey, RateLimitExceeded, ApiError 등)
- Tauri 명령: `wts_get_balance` 등록 및 테스트 완료
- 타입: Rust `BalanceEntry`, `WtsApiResult`, `UpbitApiError` + TypeScript 동일 타입
- Rate Limit 감지: 429 + 메시지/코드 기반 감지 보완
- .env 로딩: 런타임에서 Upbit 키 로딩 보완 (테스트에서는 비활성)
- 테스트 안정성: 환경 변수 변경 테스트 직렬화
- 연결 상태 체크: 오래된 응답 무시로 레이스 방지
- 테스트는 재실행하지 않음

### File List

**신규 파일:**
- apps/desktop/src-tauri/src/wts/upbit/mod.rs
- apps/desktop/src-tauri/src/wts/upbit/auth.rs
- apps/desktop/src-tauri/src/wts/upbit/client.rs
- apps/desktop/src-tauri/src/wts/upbit/types.rs

**변경 파일:**
- apps/desktop/src-tauri/src/wts/mod.rs
- apps/desktop/src-tauri/src/main.rs
- apps/desktop/src-tauri/Cargo.toml
- apps/desktop/src/wts/types.ts
- apps/desktop/src/wts/hooks/useConnectionCheck.ts

## Change Log

- 2026-01-19: Upbit API 인증 및 잔고 조회 백엔드 구현 완료
- 2026-01-19: Code review fixes (rate limit 감지, .env 로딩, 테스트 안정성, 연결 레이스 보완)

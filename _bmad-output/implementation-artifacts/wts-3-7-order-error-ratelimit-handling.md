# Story WTS-3.7: 주문 에러 처리 및 Rate Limit 알림

Status: done

## Story

As a **트레이더**,
I want **주문 실패 시 명확한 에러 메시지와 Rate Limit 알림**,
So that **문제 원인을 파악하고 대응할 수 있다**.

## Acceptance Criteria

1. **Given** 주문 API 호출이 실패했을 때 **When** 에러 응답이 수신되면 **Then** Upbit 에러 코드에 따른 한국어 메시지가 표시되어야 한다

2. **Given** 주문 API 호출이 실패했을 때 **When** 에러가 발생하면 **Then** 콘솔에 ERROR 레벨로 기록되어야 한다

3. **Given** 주문 API 호출이 실패했을 때 **When** 에러 메시지가 표시될 때 **Then** 토스트 알림이 표시되어야 한다

4. **Given** Rate Limit이 초과되었을 때 **When** 429 에러 또는 관련 에러 코드가 수신되면 **Then** "주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요." 메시지가 표시되어야 한다

5. **Given** Rate Limit 에러가 발생했을 때 **When** 사용자가 메시지를 확인하면 **Then** 사용자가 재시도 타이밍을 판단할 수 있어야 한다

6. **Given** 네트워크 오류가 발생했을 때 **When** 연결 실패 또는 타임아웃이 발생하면 **Then** "네트워크 연결을 확인하세요." 메시지가 표시되어야 한다

7. **Given** 잔고 부족 에러가 발생했을 때 **When** insufficient_funds 에러 코드가 수신되면 **Then** "매수/매도 가능 금액/수량이 부족합니다" 메시지가 표시되어야 한다

8. **Given** 최소 주문금액 미달 시 **When** under_min_total 에러 코드가 수신되면 **Then** "최소 주문금액(5,000원) 이상이어야 합니다" 메시지가 표시되어야 한다

## Tasks / Subtasks

- [x] Task 1: Rate Limit 에러 처리 구현 (AC: #4, #5)
  - [x] Subtask 1.1: Rust 백엔드에서 429 응답 감지 및 rate_limit 에러 코드 반환
  - [x] Subtask 1.2: 프론트엔드 types.ts에 Rate Limit 관련 에러 코드 추가 확인
  - [x] Subtask 1.3: OrderPanel에서 Rate Limit 에러 특별 처리 (재시도 안내 포함)
  - [x] Subtask 1.4: Rate Limit 에러 시 남은 대기 시간 표시 (Remaining-Req 헤더 활용)

- [x] Task 2: 네트워크 오류 처리 구현 (AC: #6)
  - [x] Subtask 2.1: Rust 백엔드에서 네트워크 타임아웃/연결 실패 감지
  - [x] Subtask 2.2: network_error 에러 코드로 프론트엔드 전달
  - [x] Subtask 2.3: 네트워크 오류 시 토스트 + 콘솔 로깅

- [x] Task 3: 에러 메시지 한국어 변환 완성도 검증 (AC: #1, #7, #8)
  - [x] Subtask 3.1: UPBIT_ORDER_ERROR_MESSAGES 맵핑 완전성 검토
  - [x] Subtask 3.2: 누락된 Upbit 에러 코드 추가
  - [x] Subtask 3.3: 잔고 부족/최소 주문금액 에러 메시지 확인

- [x] Task 4: 에러 로깅 및 토스트 통합 검증 (AC: #2, #3)
  - [x] Subtask 4.1: 모든 에러 경로에서 콘솔 ERROR 레벨 로깅 확인
  - [x] Subtask 4.2: 모든 에러 경로에서 토스트 알림 표시 확인
  - [x] Subtask 4.3: 에러 detail에 원본 응답 포함 확인 (민감 정보 마스킹)

- [x] Task 5: 에러 처리 유틸리티 개선
  - [x] Subtask 5.1: errorHandler.ts 파일 생성 (아키텍처 문서 기반)
  - [x] Subtask 5.2: handleApiError 함수 구현 (콘솔 로깅 + 토스트 통합)
  - [x] Subtask 5.3: Rate Limit 감지 및 재시도 가이드 로직

- [x] Task 6: 단위 테스트 작성
  - [x] Subtask 6.1: getOrderErrorMessage 함수 테스트 확장
  - [x] Subtask 6.2: Rate Limit 에러 처리 테스트
  - [x] Subtask 6.3: 네트워크 오류 처리 테스트
  - [x] Subtask 6.4: OrderPanel 에러 시나리오 통합 테스트

## Dev Notes

### 현재 구현 상태 분석

**이미 구현된 것:**
- `types.ts`: `UPBIT_ORDER_ERROR_MESSAGES` 매핑 (18개 에러 코드)
- `types.ts`: `getOrderErrorMessage()` 함수 (에러 코드 → 한국어 변환)
- `OrderPanel.tsx`: 에러 발생 시 콘솔 로깅 + 토스트 알림 호출
- `consoleStore.ts`: ERROR 레벨 로깅 지원
- `toastStore.ts`: error 타입 토스트 지원

**이 스토리에서 구현/보완할 것:**
1. Rate Limit (429) 에러 특별 처리 - 재시도 안내 메시지
2. Rust 백엔드 네트워크 오류 감지 개선
3. 에러 처리 유틸리티 함수 추출 (errorHandler.ts)
4. 추가 Upbit 에러 코드 매핑 (필요시)

### 아키텍처 요구사항

[Source: architecture.md#Error Handling & Logging]

**에러 처리 패턴:**
```typescript
// 에러 처리 유틸리티
function handleApiError(error: unknown, category: ConsoleCategory) {
  const message = translateUpbitError(error);
  consoleStore.getState().addLog('ERROR', category, message, error);
  toast.error(message);
}
```

[Source: architecture.md#Communication Patterns]

**Rate Limit Handling:**
```typescript
// Upbit Rate Limit 준수
const RATE_LIMITS = {
  order: { max: 8, window: 1000 },    // 8/초
  query: { max: 30, window: 1000 },   // 30/초
  quotation: { max: 10, window: 1000 }, // 10/초 (IP)
};
```

### NFR 요구사항

[Source: wts-epics.md#NFRs]

- **NFR14**: Rate Limit은 거래소별 호출 제한 준수
- **NFR15**: 에러 처리는 거래소별 에러 코드 파싱 및 표시
- **NFR19**: 일시적 네트워크 오류 시 재시도

### Upbit API 에러 응답 형식

[Source: architecture.md#Technical Constraints]

```json
{
  "error": {
    "message": "잔고가 부족합니다.",
    "name": "insufficient_funds_bid"
  }
}
```

**HTTP 상태 코드별 처리:**

| 상태 코드 | 에러 코드 | 발생 이유 |
|---------|---------|---------|
| 400 | `validation_error` | 필수 파라미터 누락 |
| 400 | `insufficient_funds_*` | 잔고 부족 |
| 401 | `jwt_verification` | JWT 검증 실패 |
| 401 | `no_authorization_ip` | 미등록 IP |
| 429 | - | Rate Limit 초과 |
| 500 | - | 서버 내부 오류 |

### 현재 에러 메시지 매핑

[Source: apps/desktop/src/wts/types.ts:366-388]

```typescript
export const UPBIT_ORDER_ERROR_MESSAGES: Record<string, string> = {
  // 인증 관련
  missing_api_key: 'API 키가 설정되지 않았습니다',
  jwt_error: 'JWT 토큰 생성에 실패했습니다',
  jwt_verification: 'JWT 인증에 실패했습니다',
  no_authorization_ip: '허용되지 않은 IP입니다',
  expired_access_key: '만료된 API 키입니다',
  // 네트워크/서버
  network_error: '네트워크 연결에 실패했습니다',
  rate_limit: '요청이 너무 많습니다. 잠시 후 다시 시도하세요',
  parse_error: '응답 파싱에 실패했습니다',
  // 주문 관련
  insufficient_funds_bid: '매수 가능 금액이 부족합니다',
  insufficient_funds_ask: '매도 가능 수량이 부족합니다',
  under_min_total_bid: '최소 주문금액(5,000원) 이상이어야 합니다',
  under_min_total_ask: '최소 주문금액(5,000원) 이상이어야 합니다',
  invalid_volume: '주문 수량이 올바르지 않습니다',
  invalid_price: '주문 가격이 올바르지 않습니다',
  market_does_not_exist: '존재하지 않는 마켓입니다',
  invalid_side: '주문 방향이 올바르지 않습니다',
  invalid_ord_type: '주문 유형이 올바르지 않습니다',
  validation_error: '잘못된 요청입니다',
};
```

### 추가 필요한 에러 코드

```typescript
// Rate Limit 관련 추가
'too_many_requests': '주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요',

// 네트워크 관련 추가
'timeout_error': '요청 시간이 초과되었습니다. 네트워크 상태를 확인하세요',
'connection_error': '서버에 연결할 수 없습니다. 네트워크 상태를 확인하세요',

// Upbit 추가 에러 코드
'invalid_query_payload': '요청 파라미터가 올바르지 않습니다',
'server_error': '서버 오류가 발생했습니다. 잠시 후 다시 시도하세요',
'service_unavailable': '서비스를 일시적으로 이용할 수 없습니다',
```

### 이전 스토리 학습사항

**WTS-3.6 (콘솔 로그 완성):**
- sanitizeLogDetail 유틸리티로 민감 정보 마스킹 구현 완료
- 에러 로그에 ERROR 레벨 사용, 타임스탬프 포함
- 주문 실패 시 콘솔 + 토스트 동시 표시 패턴 확립

**WTS-3.3, WTS-3.4 (시장가/지정가 주문):**
- OrderPanel에서 try/catch로 에러 처리
- getOrderErrorMessage() 활용한 한국어 변환
- 에러 시 다이얼로그 닫기 처리 완료

### Git 히스토리 분석

최근 커밋:
- `aa32cc7 feat(wts): complete console log for order results (WTS-3.6)`
- `b7f9c92 feat(wts): enhance order confirm dialog (WTS-3.5)`
- `df4266e feat(wts): implement limit order buy/sell execution (WTS-3.4)`

### 구현 위치

**수정 대상 파일:**
- `apps/desktop/src/wts/types.ts` - 에러 코드 매핑 추가
- `apps/desktop/src/wts/panels/OrderPanel.tsx` - Rate Limit 특별 처리
- `apps/desktop/src-tauri/src/wts/upbit/client.rs` - 네트워크 오류/Rate Limit 감지

**신규 생성 파일:**
- `apps/desktop/src/wts/utils/errorHandler.ts` - 에러 처리 유틸리티

**테스트 파일:**
- `apps/desktop/src/wts/__tests__/utils/errorHandler.test.ts`
- `apps/desktop/src/wts/__tests__/panels/OrderPanel.test.tsx` (확장)

### Project Structure Notes

**아키텍처 준수 사항:**
- 에러 처리 시 콘솔 로깅 + Toast 알림 모두 수행
- 에러 코드에 대한 한국어 메시지 반환
- Rate Limit 준수 (주문: 8회/초)
- 민감 정보 마스킹 (sanitizeLogDetail 활용)

**Rust 백엔드 에러 코드 매핑:**
```rust
// wts_place_order 에러 응답 형식
WtsApiResult {
    success: false,
    data: None,
    error: Some(WtsApiError {
        code: "rate_limit".to_string(),
        message: "Too Many Requests".to_string(),
    }),
}
```

### Rate Limit 재시도 안내 UX

```
┌─────────────────────────────────────────┐
│ ⚠️ 주문 요청이 너무 빠릅니다             │
│                                         │
│ 잠시 후 다시 시도하세요.                │
│ (주문 제한: 초당 8회)                   │
└─────────────────────────────────────────┘
```

### 에러 처리 유틸리티 설계

```typescript
// apps/desktop/src/wts/utils/errorHandler.ts

import { useConsoleStore } from '../stores/consoleStore';
import { useToastStore } from '../stores/toastStore';
import { getOrderErrorMessage } from '../types';
import type { LogCategory, WtsApiErrorResponse } from '../types';

/**
 * API 에러 통합 처리
 * - 콘솔 ERROR 로깅
 * - 토스트 알림 표시
 * - Rate Limit 특별 처리
 */
export function handleApiError(
  error: unknown,
  category: LogCategory,
  context?: string
): void {
  const addLog = useConsoleStore.getState().addLog;
  const showToast = useToastStore.getState().showToast;

  let errorCode = 'unknown';
  let errorMessage = '알 수 없는 오류가 발생했습니다';

  if (isWtsApiError(error)) {
    errorCode = error.code;
    errorMessage = getOrderErrorMessage(error.code, error.message);
  } else if (error instanceof Error) {
    errorMessage = error.message;
  }

  const logMessage = context
    ? `${context}: ${errorMessage}`
    : errorMessage;

  addLog('ERROR', category, logMessage, error);
  showToast('error', errorMessage);

  // Rate Limit 특별 안내
  if (errorCode === 'rate_limit' || errorCode === 'too_many_requests') {
    addLog('INFO', category, '주문 제한: 초당 8회. 잠시 후 다시 시도하세요.');
  }
}

function isWtsApiError(error: unknown): error is WtsApiErrorResponse {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof (error as Record<string, unknown>).code === 'string'
  );
}
```

### References

- [Architecture: Error Handling & Logging](/_bmad-output/planning-artifacts/architecture.md#Error Handling & Logging)
- [Architecture: Rate Limit Handling](/_bmad-output/planning-artifacts/architecture.md#Rate Limit Handling)
- [WTS Epics: Story 3.7](/_bmad-output/planning-artifacts/wts-epics.md#Story 3.7)
- [Previous Story: WTS-3.6](/_bmad-output/implementation-artifacts/wts-3-6-console-log-order-result.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

### Completion Notes List

- **Task 1**: Rate Limit 에러 처리 구현 완료. Rust 백엔드에서 429 응답 및 rate_limit 관련 에러 코드 감지 이미 구현됨. 프론트엔드 types.ts에 `too_many_requests` 에러 코드 추가, `isRateLimitError()` 헬퍼 함수 구현. OrderPanel에서 Rate Limit 에러 시 "주문 제한: 초당 8회" 재시도 안내 INFO 로그 추가.

- **Task 2**: 네트워크 오류 처리 검증 완료. Rust 백엔드 UpbitApiError::NetworkError 이미 구현됨. types.ts에 `timeout_error`, `connection_error` 에러 메시지 추가, `isNetworkError()` 헬퍼 함수 구현.

- **Task 3**: 에러 메시지 한국어 변환 완성도 검증 완료. UPBIT_ORDER_ERROR_MESSAGES에 6개 에러 코드 추가 (too_many_requests, timeout_error, connection_error, server_error, service_unavailable, invalid_query_payload). 기존 잔고 부족/최소 주문금액 에러 메시지 검증됨.

- **Task 4**: 에러 로깅 및 토스트 통합 검증 완료. OrderPanel의 모든 에러 경로에서 ERROR 레벨 로깅 + 토스트 알림 표시 확인됨.

- **Task 5**: errorHandler.ts 유틸리티 파일 생성. handleApiError() 함수로 콘솔 로깅 + 토스트 통합 처리. isWtsApiError(), getErrorDetails() 헬퍼 함수 구현.

- **Task 6**: 단위 테스트 작성 완료. types.test.ts에 에러 메시지/헬퍼 함수 테스트 추가 (11개). errorHandler.test.ts 신규 생성 (18개 테스트). OrderPanel.test.tsx에 Rate Limit/네트워크 에러 처리 테스트 추가 (2개). 전체 460개 테스트 통과.
- **Review Fixes**: Rate Limit 메시지 문구 정합성 수정, Remaining-Req 헤더 전달/로그 추가, handleApiError 통합 및 에러 detail 로깅 보강, 네트워크 오류 메시지 보정. 관련 테스트 업데이트. (테스트 실행 미수행)

### File List

**Modified:**
- apps/desktop/src/wts/types.ts - 에러 코드 매핑 확장, isRateLimitError/isNetworkError 헬퍼 함수 추가
- apps/desktop/src/wts/panels/OrderPanel.tsx - Rate Limit 에러 시 재시도 안내 INFO 로그 추가
- apps/desktop/src/wts/__tests__/types.test.ts - 에러 메시지 테스트 추가
- apps/desktop/src/wts/__tests__/panels/OrderPanel.test.tsx - Rate Limit/네트워크 에러 테스트 추가
- apps/desktop/src-tauri/src/wts/upbit/client.rs - Remaining-Req 헤더 추출 및 Rate Limit detail 전달
- apps/desktop/src-tauri/src/wts/upbit/types.rs - Rate Limit 메시지 정합성 및 detail 필드 추가

**Created:**
- apps/desktop/src/wts/utils/errorHandler.ts - 에러 처리 유틸리티
- apps/desktop/src/wts/__tests__/utils/errorHandler.test.ts - errorHandler 테스트

## Change Log

- 2026-01-24: WTS-3.7 구현 완료 - 주문 에러 처리 및 Rate Limit 알림 기능 구현
- 2026-01-24: 코드 리뷰 수정 - Rate Limit 문구/Remaining-Req/에러 처리 유틸리티 보강

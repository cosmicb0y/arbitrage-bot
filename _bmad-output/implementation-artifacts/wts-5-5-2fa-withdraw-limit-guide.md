# Story WTS-5.5: 2FA 및 출금 제한 안내

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **2FA 필요 시 명확한 안내 메시지**,
So that **출금 실패 원인을 알고 대응할 수 있다**.

## Acceptance Criteria

1. **Given** 출금 요청이 2FA를 요구할 때 **When** `two_factor_auth_required` 에러 코드가 반환되면 **Then** "Upbit 앱에서 2FA 인증이 필요합니다." 안내가 토스트로 표시되어야 한다
2. **Given** 2FA 에러가 발생했을 때 **When** 에러가 처리되면 **Then** 콘솔에 WARN 레벨로 기록되어야 한다 (ERROR가 아닌 WARN - 사용자 액션 필요)
3. **Given** 2FA 에러가 발생했을 때 **When** 에러가 처리되면 **Then** 추가 안내 메시지 "Upbit 모바일 앱에서 출금 인증을 완료한 후 다시 시도하세요."가 INFO 레벨로 기록되어야 한다
4. **Given** 출금 주소가 사전 등록되지 않았을 때 **When** `unregistered_withdraw_address` 또는 `withdraw_address_not_registered` 에러 코드가 반환되면 **Then** "출금 주소를 Upbit 웹에서 먼저 등록해주세요." 안내가 표시되어야 한다
5. **Given** 미등록 출금 주소 에러가 발생했을 때 **When** 에러가 처리되면 **Then** 콘솔에 WARN 레벨로 기록되어야 한다
6. **Given** 미등록 출금 주소 에러가 발생했을 때 **When** 에러가 처리되면 **Then** Upbit 출금 주소 등록 안내 정보가 INFO 레벨로 기록되어야 한다 ("https://upbit.com > 입출금 > 출금 > 출금주소관리에서 주소를 등록하세요")
7. **Given** 일일 출금 한도 초과 에러가 발생했을 때 **When** `over_daily_limit` 에러 코드가 반환되면 **Then** "일일 출금 한도를 초과했습니다." 안내가 WARN 레벨로 기록되어야 한다
8. **Given** 최소 출금 수량 미만 에러가 발생했을 때 **When** `under_min_amount` 에러 코드가 반환되면 **Then** "최소 출금 수량 이상이어야 합니다." 안내가 ERROR 레벨로 기록되어야 한다

## Tasks / Subtasks

- [x] Task 1: 출금 특화 에러 분류 상수 정의 (AC: #1-#8)
  - [x] Subtask 1.1: types.ts에 WITHDRAW_ACTION_REQUIRED_ERRORS 상수 추가 (2FA, 미등록 주소 - WARN 레벨)
  - [x] Subtask 1.2: types.ts에 WITHDRAW_LIMIT_ERRORS 상수 추가 (한도 초과, 최소 수량 미만)
  - [x] Subtask 1.3: isWithdrawActionRequiredError(code) 헬퍼 함수 추가
  - [x] Subtask 1.4: isWithdrawLimitError(code) 헬퍼 함수 추가

- [x] Task 2: 출금 에러별 추가 안내 메시지 정의 (AC: #3, #6)
  - [x] Subtask 2.1: types.ts에 WITHDRAW_ERROR_GUIDANCE 상수 추가 (에러 코드별 추가 안내)
  - [x] Subtask 2.2: two_factor_auth_required 안내: "Upbit 모바일 앱에서 출금 인증을 완료한 후 다시 시도하세요."
  - [x] Subtask 2.3: unregistered_withdraw_address 안내: "https://upbit.com > 입출금 > 출금 > 출금주소관리에서 주소를 등록하세요"

- [x] Task 3: errorHandler.ts 출금 에러 처리 개선 (AC: #1-#8)
  - [x] Subtask 3.1: handleWithdrawError 전용 함수 추가
  - [x] Subtask 3.2: 액션 필요 에러(2FA, 미등록 주소)는 WARN 레벨로 기록
  - [x] Subtask 3.3: 추가 안내 메시지 INFO 레벨로 자동 기록
  - [x] Subtask 3.4: 기존 handleApiError에서 WITHDRAW 카테고리일 때 handleWithdrawError 위임

- [x] Task 4: WtsWindow 출금 에러 핸들링 통합 (AC: #1-#8)
  - [x] Subtask 4.1: handleWithdrawConfirm에서 에러 응답 시 handleWithdrawError 호출
  - [x] Subtask 4.2: WithdrawConfirmDialog 닫지 않고 에러 표시 (재시도 가능하도록)

- [x] Task 5: 단위 테스트 작성 (AC: #1-#8)
  - [x] Subtask 5.1: WITHDRAW_ACTION_REQUIRED_ERRORS 상수 테스트
  - [x] Subtask 5.2: WITHDRAW_ERROR_GUIDANCE 상수 테스트
  - [x] Subtask 5.3: isWithdrawActionRequiredError 함수 테스트
  - [x] Subtask 5.4: handleWithdrawError 함수 테스트 (WARN 레벨 검증)
  - [x] Subtask 5.5: 2FA 에러 시 추가 안내 메시지 기록 테스트
  - [x] Subtask 5.6: 미등록 주소 에러 시 등록 안내 기록 테스트

## Dev Notes

### 기존 에러 메시지 정의 (이미 구현됨)

[Source: apps/desktop/src/wts/types.ts:403-415]

```typescript
// 출금 관련 (WTS-5.1)
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

### 에러 분류 기준

**액션 필요 에러 (WARN 레벨)** - 사용자가 외부에서 조치 후 재시도 가능:
- `two_factor_auth_required`: Upbit 앱에서 2FA 인증 필요
- `unregistered_withdraw_address`: Upbit 웹에서 주소 등록 필요
- `withdraw_address_not_registered`: 동일

**한도 에러 (WARN/ERROR 레벨)** - 조건 변경 필요:
- `over_daily_limit`: 일일 한도 초과 (WARN - 다음 날 재시도)
- `under_min_amount`: 최소 수량 미만 (ERROR - 수량 수정 필요)
- `insufficient_funds_withdraw`: 잔고 부족 (ERROR - 입금 필요)

**시스템 에러 (ERROR 레벨)** - 일시적 또는 설정 문제:
- `withdraw_suspended`: 출금 중단
- `withdraw_disabled`: 출금 비활성화
- `wallet_not_working`: 지갑 점검 중

### 추가 안내 메시지 구조

```typescript
// types.ts에 추가
export const WITHDRAW_ERROR_GUIDANCE: Record<string, string> = {
  two_factor_auth_required: 'Upbit 모바일 앱에서 출금 인증을 완료한 후 다시 시도하세요.',
  unregistered_withdraw_address: 'https://upbit.com > 입출금 > 출금 > 출금주소관리에서 주소를 등록하세요.',
  withdraw_address_not_registered: 'https://upbit.com > 입출금 > 출금 > 출금주소관리에서 주소를 등록하세요.',
  over_daily_limit: '출금 한도는 매일 00:00(KST)에 초기화됩니다.',
};

export const WITHDRAW_ACTION_REQUIRED_ERRORS = [
  'two_factor_auth_required',
  'unregistered_withdraw_address',
  'withdraw_address_not_registered',
] as const;

export const WITHDRAW_LIMIT_ERRORS = [
  'over_daily_limit',
  'under_min_amount',
  'insufficient_funds_withdraw',
] as const;

export function isWithdrawActionRequiredError(code: string): boolean {
  return WITHDRAW_ACTION_REQUIRED_ERRORS.includes(code as typeof WITHDRAW_ACTION_REQUIRED_ERRORS[number]);
}

export function isWithdrawLimitError(code: string): boolean {
  return WITHDRAW_LIMIT_ERRORS.includes(code as typeof WITHDRAW_LIMIT_ERRORS[number]);
}
```

### handleWithdrawError 구현

```typescript
// utils/errorHandler.ts에 추가

import {
  isWithdrawActionRequiredError,
  WITHDRAW_ERROR_GUIDANCE,
  getOrderErrorMessage,
} from '../types';

/**
 * 출금 에러 전용 처리
 * - 액션 필요 에러: WARN 레벨 + 추가 안내
 * - 한도 에러: WARN/ERROR 레벨
 * - 기타: ERROR 레벨
 */
export function handleWithdrawError(
  error: unknown,
  context?: string
): void {
  const addLog = useConsoleStore.getState().addLog;
  const showToast = useToastStore.getState().showToast;

  let errorCode = 'unknown';
  let errorMessage = '알 수 없는 오류가 발생했습니다';
  let logDetail: unknown = error;

  if (isWtsApiError(error)) {
    errorCode = error.code;
    errorMessage = getOrderErrorMessage(error.code, error.message);
    logDetail = error.detail ? { ...error.detail, code: error.code, message: error.message } : error;
  } else if (error instanceof Error) {
    errorMessage = error.message;
    logDetail = { name: error.name, message: error.message };
  } else if (typeof error === 'string') {
    errorMessage = error;
    logDetail = error;
  }

  const logMessage = context ? `${context}: ${errorMessage}` : errorMessage;

  // 액션 필요 에러: WARN 레벨
  if (isWithdrawActionRequiredError(errorCode)) {
    addLog('WARN', 'WITHDRAW', logMessage, logDetail);
    showToast('warning', errorMessage);

    // 추가 안내 메시지
    const guidance = WITHDRAW_ERROR_GUIDANCE[errorCode];
    if (guidance) {
      addLog('INFO', 'WITHDRAW', guidance);
    }
    return;
  }

  // 한도 에러: over_daily_limit은 WARN, 나머지는 ERROR
  if (errorCode === 'over_daily_limit') {
    addLog('WARN', 'WITHDRAW', logMessage, logDetail);
    showToast('warning', errorMessage);

    const guidance = WITHDRAW_ERROR_GUIDANCE[errorCode];
    if (guidance) {
      addLog('INFO', 'WITHDRAW', guidance);
    }
    return;
  }

  // 기타 에러: ERROR 레벨
  addLog('ERROR', 'WITHDRAW', logMessage, logDetail);
  showToast('error', errorMessage);

  // Rate Limit 처리 (기존 로직)
  if (isRateLimitError(errorCode)) {
    addLog('INFO', 'WITHDRAW', '요청 제한으로 잠시 후 다시 시도하세요.');
  }

  // 네트워크 에러 처리
  if (isNetworkError(errorCode)) {
    addLog('INFO', 'WITHDRAW', '네트워크 연결을 확인하고 다시 시도하세요.');
  }
}
```

### WtsWindow 에러 처리 수정

[Source: apps/desktop/src/wts/WtsWindow.tsx:handleWithdrawConfirm]

현재 코드에서 `handleApiError`를 호출하는 부분을 수정:

```typescript
// 기존
if (!result.success && result.error) {
  handleApiError(result.error, 'WITHDRAW', '출금 요청 실패');
  return;
}

// 수정 후
if (!result.success && result.error) {
  handleWithdrawError(result.error, '출금 요청 실패');
  // 다이얼로그 닫지 않음 - 사용자가 조치 후 재시도 가능
  return;
}
```

### 에러 발생 시 다이얼로그 동작

**현재 동작:**
- 에러 발생 시 다이얼로그 닫기
- 에러 토스트 표시

**개선 동작:**
- 액션 필요 에러 (2FA, 미등록 주소): 다이얼로그 유지, 재시도 가능
- 한도/잔고 에러: 다이얼로그 닫기 (수량 수정 필요)
- 시스템 에러: 다이얼로그 닫기

### 테스트 케이스

```typescript
// __tests__/utils/errorHandler.test.ts 추가

describe('handleWithdrawError', () => {
  it('should log 2FA error as WARN level', () => {
    const error = { code: 'two_factor_auth_required', message: 'test' };
    handleWithdrawError(error);

    expect(mockAddLog).toHaveBeenCalledWith(
      'WARN',
      'WITHDRAW',
      expect.stringContaining('2FA'),
      expect.anything()
    );
  });

  it('should add guidance message for 2FA error', () => {
    const error = { code: 'two_factor_auth_required', message: 'test' };
    handleWithdrawError(error);

    expect(mockAddLog).toHaveBeenCalledWith(
      'INFO',
      'WITHDRAW',
      expect.stringContaining('Upbit 모바일 앱')
    );
  });

  it('should log unregistered address error as WARN level', () => {
    const error = { code: 'unregistered_withdraw_address', message: 'test' };
    handleWithdrawError(error);

    expect(mockAddLog).toHaveBeenCalledWith(
      'WARN',
      'WITHDRAW',
      expect.stringContaining('등록'),
      expect.anything()
    );
  });

  it('should add registration URL for unregistered address error', () => {
    const error = { code: 'unregistered_withdraw_address', message: 'test' };
    handleWithdrawError(error);

    expect(mockAddLog).toHaveBeenCalledWith(
      'INFO',
      'WITHDRAW',
      expect.stringContaining('upbit.com')
    );
  });

  it('should log under_min_amount as ERROR level', () => {
    const error = { code: 'under_min_amount', message: 'test' };
    handleWithdrawError(error);

    expect(mockAddLog).toHaveBeenCalledWith(
      'ERROR',
      'WITHDRAW',
      expect.anything(),
      expect.anything()
    );
  });
});

// __tests__/types.test.ts 추가

describe('Withdraw error helpers', () => {
  describe('isWithdrawActionRequiredError', () => {
    it('should return true for 2FA error', () => {
      expect(isWithdrawActionRequiredError('two_factor_auth_required')).toBe(true);
    });

    it('should return true for unregistered address', () => {
      expect(isWithdrawActionRequiredError('unregistered_withdraw_address')).toBe(true);
    });

    it('should return false for limit errors', () => {
      expect(isWithdrawActionRequiredError('under_min_amount')).toBe(false);
    });
  });

  describe('WITHDRAW_ERROR_GUIDANCE', () => {
    it('should have guidance for 2FA error', () => {
      expect(WITHDRAW_ERROR_GUIDANCE['two_factor_auth_required']).toContain('Upbit 모바일 앱');
    });

    it('should have URL for unregistered address', () => {
      expect(WITHDRAW_ERROR_GUIDANCE['unregistered_withdraw_address']).toContain('upbit.com');
    });
  });
});
```

### Project Structure Notes

**수정 파일:**
- `apps/desktop/src/wts/types.ts` - 출금 에러 분류 상수 및 헬퍼 함수 추가
- `apps/desktop/src/wts/utils/errorHandler.ts` - handleWithdrawError 함수 추가
- `apps/desktop/src/wts/WtsWindow.tsx` - handleWithdrawConfirm 에러 처리 개선

**테스트 수정:**
- `apps/desktop/src/wts/__tests__/types.test.ts` - 출금 에러 헬퍼 테스트 추가
- `apps/desktop/src/wts/__tests__/utils/errorHandler.test.ts` - handleWithdrawError 테스트 추가

**아키텍처 정합성:**
- 에러 처리 패턴 준수: 콘솔 로깅 + 토스트 알림
- 로그 레벨 활용: WARN (액션 필요), ERROR (오류), INFO (안내)
- WTS 컴포넌트 구조 준수 (`wts/utils/`)

### 이전 스토리 참조

**WTS-5.1 (출금 API Rust 백엔드):**
- Upbit 출금 에러 코드 매핑 완료
- two_factor_auth_required, unregistered_withdraw_address 등 정의

**WTS-5.3 (출금 확인 다이얼로그):**
- handleWithdrawConfirm에서 기본 에러 처리 구현
- handleApiError 사용 중

**WTS-5.4 (출금 실행 및 결과 처리):**
- 출금 성공 플로우 구현 완료
- 에러 시 기본 토스트 표시

### 다음 스토리 연결 (WTS-5.6)

**WTS-5.6 (출금 에러 처리 및 네트워크 오류 대응):**
- 네트워크 오류 시 자동 재시도 로직
- 이 스토리에서는 네트워크 에러 안내만 구현, 5.6에서 재시도 로직 추가

### References

- [Architecture: Error Handling Flow](/_bmad-output/planning-artifacts/architecture.md#Error Handling Flow)
- [PRD: FR27 2FA 안내 메시지](/_bmad-output/planning-artifacts/prd.md)
- [PRD: FR34 API 에러 코드별 메시지](/_bmad-output/planning-artifacts/prd.md)
- [WTS Epics: Epic 5 Story 5.5](/_bmad-output/planning-artifacts/wts-epics.md#Story 5.5)
- [Previous Story: WTS-5.4 출금 실행 및 결과 처리](/_bmad-output/implementation-artifacts/wts-5-4-withdraw-execute-result.md)
- [TypeScript Types: UPBIT_ORDER_ERROR_MESSAGES](apps/desktop/src/wts/types.ts:366-416)
- [Error Handler: handleApiError](apps/desktop/src/wts/utils/errorHandler.ts:73-134)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A

### Completion Notes List

- 모든 5개 Task 완료 (RED-GREEN-REFACTOR TDD 사이클 적용)
- AC #1-#8 구현 완료:
  - 2FA 에러 WARN 레벨 로깅 + 추가 안내 메시지 (AC #1-#3)
  - 미등록 주소 에러 WARN 레벨 로깅 + URL 안내 (AC #4-#6)
  - 일일 한도 초과 WARN 레벨 로깅 (AC #7)
  - 최소 수량 미만 ERROR 레벨 로깅 (AC #8)
- 출금 에러 처리 위임 보완 (handleApiError → handleWithdrawError), 출금 Rate Limit 메시지 보정
- 출금 상태 조회 오류 처리 import 누락 수정
- withdraw_address_not_registered 테스트 추가
- Integration test 회귀 테스트 수정 (limitOrder.integration.test.tsx)
- 총 702개 테스트 중 1개 flaky test (useConnectionCheck timeout - 기존 이슈) 제외 모두 통과

### File List

**Modified Files:**
- `apps/desktop/src/wts/types.ts` - WITHDRAW_ACTION_REQUIRED_ERRORS, WITHDRAW_LIMIT_ERRORS, WITHDRAW_ERROR_GUIDANCE 상수 및 헬퍼 함수 추가
- `apps/desktop/src/wts/utils/errorHandler.ts` - handleApiError의 WITHDRAW 위임, Rate Limit 안내 보완
- `apps/desktop/src/wts/WtsWindow.tsx` - 상태 조회 오류 처리 import 보완
- `apps/desktop/src/wts/__tests__/types.test.ts` - 출금 에러 상수/헬퍼 테스트 추가
- `apps/desktop/src/wts/__tests__/utils/errorHandler.test.ts` - withdraw_address_not_registered 케이스 테스트 추가
- `apps/desktop/src/wts/__tests__/WtsWindow.withdraw.test.tsx` - WTS-5.5 에러 처리 테스트 추가 (5개 테스트)
- `apps/desktop/src/wts/__tests__/integration/limitOrder.integration.test.tsx` - 회귀 테스트 수정 (getState 모킹)
- `_bmad-output/implementation-artifacts/wts-5-5-2fa-withdraw-limit-guide.md` - 리뷰 수정 내역 및 상태 업데이트
- `_bmad-output/implementation-artifacts/sprint-status.yaml` - 스프린트 상태 동기화 (wts-5-5)

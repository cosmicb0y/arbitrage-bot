# Story WTS-5.6: 출금 에러 처리 및 네트워크 오류 대응

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **출금 실패 시 명확한 에러 메시지와 네트워크 오류 안내**,
So that **문제를 파악하고 재시도할 수 있다**.

## Acceptance Criteria

1. **Given** 출금 API 호출이 실패했을 때 **When** 에러 응답이 수신되면 **Then** Upbit 에러 코드에 따른 한국어 메시지가 표시되어야 한다
2. **Given** 출금 에러가 발생했을 때 **When** 에러가 처리되면 **Then** 콘솔에 ERROR 레벨로 기록되어야 한다 (이미 WTS-5.5에서 구현된 WARN 케이스 제외)
3. **Given** 출금 에러가 발생했을 때 **When** 에러가 처리되면 **Then** 토스트 알림이 표시되어야 한다
4. **Given** 네트워크 오류가 발생했을 때 **When** 연결 실패 또는 타임아웃이 발생하면 **Then** "네트워크 연결을 확인하세요." 메시지가 표시되어야 한다
5. **Given** 네트워크 오류가 발생했을 때 **When** 출금 요청이 실패하면 **Then** 자동 재시도가 1회 수행되어야 한다
6. **Given** 자동 재시도가 실패했을 때 **When** 재시도도 실패하면 **Then** 사용자에게 수동 재시도 옵션이 제공되어야 한다
7. **Given** 재시도 중일 때 **When** 재시도가 진행되면 **Then** 콘솔에 "재시도 중... (1/1)" INFO 레벨로 기록되어야 한다
8. **Given** 재시도 후 성공했을 때 **When** 재시도가 성공하면 **Then** 콘솔에 "재시도 성공" SUCCESS 레벨로 기록되어야 한다

## Tasks / Subtasks

- [x] Task 1: 네트워크 에러 감지 및 분류 강화 (AC: #4)
  - [x] Subtask 1.1: types.ts에 NETWORK_ERROR_CODES 상수 추가 (timeout, connection_refused, network_error, etc.)
  - [x] Subtask 1.2: isNetworkRetryableError(code) 헬퍼 함수 추가 (재시도 가능한 네트워크 에러 판별)
  - [x] Subtask 1.3: NETWORK_ERROR_MESSAGES 상수 추가 (네트워크 에러별 한국어 메시지)

- [x] Task 2: 출금 자동 재시도 로직 구현 (AC: #5, #7, #8)
  - [x] Subtask 2.1: WtsWindow.tsx에 withdrawWithRetry 함수 추가 (재시도 래퍼)
  - [x] Subtask 2.2: 네트워크 에러 시 1회 자동 재시도 구현 (3초 딜레이)
  - [x] Subtask 2.3: 재시도 중 상태 표시 (콘솔 INFO 로그)
  - [x] Subtask 2.4: 재시도 성공 시 SUCCESS 로그 기록
  - [x] Subtask 2.5: 재시도 실패 시 원래 에러 처리 흐름으로 전달

- [x] Task 3: 수동 재시도 옵션 UI 구현 (AC: #6)
  - [x] Subtask 3.1: WithdrawConfirmDialog에 retryable 상태 prop 추가
  - [x] Subtask 3.2: 네트워크 에러 후 "다시 시도" 버튼 표시
  - [x] Subtask 3.3: 수동 재시도 시 동일한 출금 요청 재전송
  - [x] Subtask 3.4: 재시도 버튼 클릭 시 로딩 상태 표시

- [x] Task 4: 에러 메시지 개선 및 통합 (AC: #1, #2, #3)
  - [x] Subtask 4.1: handleWithdrawError에 네트워크 재시도 로직 통합
  - [x] Subtask 4.2: 기존 에러 메시지와 일관성 유지
  - [x] Subtask 4.3: 에러 발생 시 다이얼로그 상태 관리 (retryable 여부에 따라)

- [x] Task 5: 단위 테스트 작성 (AC: #1-#8)
  - [x] Subtask 5.1: NETWORK_ERROR_CODES 상수 테스트
  - [x] Subtask 5.2: isNetworkRetryableError 함수 테스트
  - [x] Subtask 5.3: withdrawWithRetry 함수 테스트 (네트워크 에러 시 재시도)
  - [x] Subtask 5.4: 재시도 성공 시 SUCCESS 로그 검증
  - [x] Subtask 5.5: 재시도 실패 시 ERROR 로그 검증
  - [x] Subtask 5.6: WithdrawConfirmDialog 재시도 버튼 렌더링 테스트
  - [x] Subtask 5.7: 수동 재시도 클릭 시 요청 재전송 테스트

## Dev Notes

### 기존 네트워크 에러 처리 (참조)

[Source: apps/desktop/src/wts/utils/errorHandler.ts:17-27]

```typescript
function isNetworkMessage(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    lower.includes('network') ||
    lower.includes('timeout') ||
    lower.includes('timed out') ||
    lower.includes('connection') ||
    lower.includes('econn') ||
    message.includes('네트워크')
  );
}
```

[Source: apps/desktop/src/wts/utils/errorHandler.ts:245-248]

```typescript
// 네트워크 에러 처리
if (isNetworkError(errorCode)) {
  addLog('INFO', 'WITHDRAW', '네트워크 연결을 확인하고 다시 시도하세요.');
}
```

### 네트워크 에러 코드 정의 추가

```typescript
// types.ts에 추가
export const NETWORK_ERROR_CODES = [
  'network_error',
  'timeout',
  'connection_refused',
  'connection_reset',
  'econnrefused',
  'econnreset',
  'etimedout',
  'enetunreach',
] as const;

export const NETWORK_ERROR_MESSAGES: Record<string, string> = {
  network_error: '네트워크 연결을 확인하세요.',
  timeout: '요청 시간이 초과되었습니다. 다시 시도하세요.',
  connection_refused: '서버에 연결할 수 없습니다.',
  connection_reset: '연결이 끊어졌습니다. 다시 시도하세요.',
  econnrefused: '서버에 연결할 수 없습니다.',
  econnreset: '연결이 끊어졌습니다.',
  etimedout: '요청 시간이 초과되었습니다.',
  enetunreach: '네트워크에 연결할 수 없습니다.',
};

/**
 * 재시도 가능한 네트워크 에러인지 확인
 */
export function isNetworkRetryableError(code: string): boolean {
  return NETWORK_ERROR_CODES.includes(code.toLowerCase() as typeof NETWORK_ERROR_CODES[number]);
}
```

### withdrawWithRetry 함수 구현

```typescript
// WtsWindow.tsx에 추가

const WITHDRAW_RETRY_DELAY = 3000; // 3초
const MAX_WITHDRAW_RETRIES = 1;

/**
 * 출금 요청 with 자동 재시도
 * 네트워크 에러 시 1회 자동 재시도
 */
async function withdrawWithRetry(
  params: WithdrawParams,
  retryCount: number = 0
): Promise<WtsApiResult<WithdrawResponse>> {
  const addLog = useConsoleStore.getState().addLog;

  try {
    const result = await invoke<WtsApiResult<WithdrawResponse>>('wts_withdraw', { params });

    // 성공 또는 비-네트워크 에러
    if (result.success || !result.error) {
      return result;
    }

    // 네트워크 에러이고 재시도 횟수가 남았을 때
    if (isNetworkRetryableError(result.error.code) && retryCount < MAX_WITHDRAW_RETRIES) {
      addLog('INFO', 'WITHDRAW', `네트워크 오류로 재시도 중... (${retryCount + 1}/${MAX_WITHDRAW_RETRIES})`);

      // 재시도 딜레이
      await new Promise(resolve => setTimeout(resolve, WITHDRAW_RETRY_DELAY));

      // 재귀 호출로 재시도
      const retryResult = await withdrawWithRetry(params, retryCount + 1);

      // 재시도 성공 시 로그
      if (retryResult.success) {
        addLog('SUCCESS', 'WITHDRAW', '재시도 성공');
      }

      return retryResult;
    }

    // 재시도 불가 또는 재시도 횟수 초과
    return result;
  } catch (error) {
    // invoke 자체에서 에러 발생 (네트워크 레벨)
    const errorCode = isNetworkMessage(String(error)) ? 'network_error' : 'unknown';

    if (isNetworkRetryableError(errorCode) && retryCount < MAX_WITHDRAW_RETRIES) {
      addLog('INFO', 'WITHDRAW', `네트워크 오류로 재시도 중... (${retryCount + 1}/${MAX_WITHDRAW_RETRIES})`);

      await new Promise(resolve => setTimeout(resolve, WITHDRAW_RETRY_DELAY));

      const retryResult = await withdrawWithRetry(params, retryCount + 1);

      if (retryResult.success) {
        addLog('SUCCESS', 'WITHDRAW', '재시도 성공');
      }

      return retryResult;
    }

    // 에러 응답 형식으로 반환
    return {
      success: false,
      error: {
        code: errorCode,
        message: error instanceof Error ? error.message : String(error),
      },
    };
  }
}
```

### WithdrawConfirmDialog 수동 재시도 UI

```typescript
// WithdrawConfirmDialog props 확장
interface WithdrawConfirmDialogProps {
  // ... 기존 props
  retryable?: boolean;        // 재시도 가능 상태
  onRetry?: () => void;       // 재시도 콜백
  retryLoading?: boolean;     // 재시도 중 로딩 상태
}

// 다이얼로그 내부 버튼 영역
{retryable && (
  <Button
    onClick={onRetry}
    disabled={retryLoading}
    variant="secondary"
    className="w-full"
  >
    {retryLoading ? (
      <>
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        재시도 중...
      </>
    ) : (
      '다시 시도'
    )}
  </Button>
)}
```

### WtsWindow 상태 관리 확장

```typescript
// WtsWindow.tsx 상태 추가
const [withdrawRetryable, setWithdrawRetryable] = useState(false);
const [withdrawRetryLoading, setWithdrawRetryLoading] = useState(false);
const [lastWithdrawParams, setLastWithdrawParams] = useState<WithdrawParams | null>(null);

// handleWithdrawConfirm 수정
const handleWithdrawConfirm = async () => {
  if (!withdrawRequest) return;

  setWithdrawLoading(true);
  setWithdrawRetryable(false);

  // 마지막 요청 파라미터 저장 (재시도용)
  setLastWithdrawParams(withdrawRequest);

  const result = await withdrawWithRetry(withdrawRequest);

  setWithdrawLoading(false);

  if (result.success) {
    // 성공 처리
    handleWithdrawSuccess(result.data!);
    setWithdrawDialogOpen(false);
  } else if (result.error) {
    handleWithdrawError(result.error, '출금 요청 실패');

    // 네트워크 에러일 경우 재시도 버튼 활성화
    if (isNetworkRetryableError(result.error.code)) {
      setWithdrawRetryable(true);
    } else {
      // 비-네트워크 에러는 다이얼로그 닫기
      setWithdrawDialogOpen(false);
    }
  }
};

// 수동 재시도 핸들러
const handleWithdrawRetry = async () => {
  if (!lastWithdrawParams) return;

  setWithdrawRetryLoading(true);

  const result = await invoke<WtsApiResult<WithdrawResponse>>('wts_withdraw', {
    params: lastWithdrawParams
  });

  setWithdrawRetryLoading(false);

  if (result.success) {
    useConsoleStore.getState().addLog('SUCCESS', 'WITHDRAW', '수동 재시도 성공');
    handleWithdrawSuccess(result.data!);
    setWithdrawDialogOpen(false);
    setWithdrawRetryable(false);
  } else if (result.error) {
    handleWithdrawError(result.error, '재시도 실패');
  }
};
```

### 에러 플로우 다이어그램

```
출금 요청
    │
    ▼
withdrawWithRetry(params, 0)
    │
    ├─── 성공 ──► 완료
    │
    └─── 실패
           │
           ├─── 네트워크 에러? ──► 예 ──► 재시도 횟수 < 1?
           │                                    │
           │                                    ├─── 예 ──► INFO 로그 "재시도 중..."
           │                                    │              │
           │                                    │              ▼
           │                                    │         3초 대기
           │                                    │              │
           │                                    │              ▼
           │                                    │    withdrawWithRetry(params, 1)
           │                                    │              │
           │                                    │              ├─── 성공 ──► SUCCESS 로그
           │                                    │              │
           │                                    │              └─── 실패 ──► retryable=true
           │                                    │                           (재시도 버튼 표시)
           │                                    │
           │                                    └─── 아니오 ──► retryable=true
           │
           └─── 아니오 ──► handleWithdrawError (기존 로직)
                          다이얼로그 닫기
```

### 테스트 케이스

```typescript
// __tests__/types.test.ts 추가

describe('Network error helpers', () => {
  describe('isNetworkRetryableError', () => {
    it('should return true for network_error', () => {
      expect(isNetworkRetryableError('network_error')).toBe(true);
    });

    it('should return true for timeout', () => {
      expect(isNetworkRetryableError('timeout')).toBe(true);
    });

    it('should return true for connection_refused', () => {
      expect(isNetworkRetryableError('connection_refused')).toBe(true);
    });

    it('should return false for insufficient_funds', () => {
      expect(isNetworkRetryableError('insufficient_funds_withdraw')).toBe(false);
    });

    it('should be case insensitive', () => {
      expect(isNetworkRetryableError('TIMEOUT')).toBe(true);
      expect(isNetworkRetryableError('Network_Error')).toBe(true);
    });
  });
});

// __tests__/WtsWindow.withdraw.test.tsx 추가

describe('withdrawWithRetry', () => {
  it('should return success without retry on successful request', async () => {
    mockInvoke.mockResolvedValueOnce({ success: true, data: mockWithdrawResponse });

    const result = await withdrawWithRetry(mockParams);

    expect(result.success).toBe(true);
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it('should retry once on network error', async () => {
    mockInvoke
      .mockResolvedValueOnce({ success: false, error: { code: 'network_error', message: 'test' } })
      .mockResolvedValueOnce({ success: true, data: mockWithdrawResponse });

    const result = await withdrawWithRetry(mockParams);

    expect(result.success).toBe(true);
    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(mockAddLog).toHaveBeenCalledWith('INFO', 'WITHDRAW', expect.stringContaining('재시도 중'));
    expect(mockAddLog).toHaveBeenCalledWith('SUCCESS', 'WITHDRAW', '재시도 성공');
  });

  it('should not retry on non-network error', async () => {
    mockInvoke.mockResolvedValueOnce({
      success: false,
      error: { code: 'insufficient_funds_withdraw', message: 'test' }
    });

    const result = await withdrawWithRetry(mockParams);

    expect(result.success).toBe(false);
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it('should return error after max retries exceeded', async () => {
    mockInvoke.mockResolvedValue({
      success: false,
      error: { code: 'network_error', message: 'test' }
    });

    const result = await withdrawWithRetry(mockParams);

    expect(result.success).toBe(false);
    expect(mockInvoke).toHaveBeenCalledTimes(2); // 원래 + 1회 재시도
  });
});

describe('WithdrawConfirmDialog retry button', () => {
  it('should show retry button when retryable is true', () => {
    render(
      <WithdrawConfirmDialog
        open={true}
        retryable={true}
        onRetry={jest.fn()}
      />
    );

    expect(screen.getByText('다시 시도')).toBeInTheDocument();
  });

  it('should not show retry button when retryable is false', () => {
    render(
      <WithdrawConfirmDialog
        open={true}
        retryable={false}
      />
    );

    expect(screen.queryByText('다시 시도')).not.toBeInTheDocument();
  });

  it('should show loading state during retry', () => {
    render(
      <WithdrawConfirmDialog
        open={true}
        retryable={true}
        retryLoading={true}
        onRetry={jest.fn()}
      />
    );

    expect(screen.getByText('재시도 중...')).toBeInTheDocument();
  });

  it('should call onRetry when retry button clicked', async () => {
    const onRetry = jest.fn();
    render(
      <WithdrawConfirmDialog
        open={true}
        retryable={true}
        onRetry={onRetry}
      />
    );

    await userEvent.click(screen.getByText('다시 시도'));

    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});
```

### Project Structure Notes

**수정 파일:**
- `apps/desktop/src/wts/types.ts` - NETWORK_ERROR_CODES, NETWORK_ERROR_MESSAGES, isNetworkRetryableError 추가
- `apps/desktop/src/wts/WtsWindow.tsx` - withdrawWithRetry 함수, 재시도 상태 관리, handleWithdrawRetry 추가
- `apps/desktop/src/wts/components/WithdrawConfirmDialog.tsx` - retryable, onRetry, retryLoading props 추가

**테스트 수정:**
- `apps/desktop/src/wts/__tests__/types.test.ts` - 네트워크 에러 헬퍼 테스트 추가
- `apps/desktop/src/wts/__tests__/WtsWindow.withdraw.test.tsx` - withdrawWithRetry 테스트, 재시도 UI 테스트 추가

**아키텍처 정합성:**
- 에러 처리 패턴 준수: 콘솔 로깅 + 토스트 알림
- 재시도 로직: 1회 자동 재시도 + 수동 재시도 옵션
- WTS 컴포넌트 구조 준수 (`wts/components/`)

### 이전 스토리 참조 (WTS-5.5)

**이미 구현된 기능:**
- handleWithdrawError 함수 (WARN/ERROR 레벨 분기)
- 2FA, 미등록 주소 에러 처리 (WARN + 추가 안내)
- 한도 에러 처리 (WARN/ERROR)
- 기본 네트워크 에러 안내 메시지 ("네트워크 연결을 확인하고 다시 시도하세요.")

**이 스토리에서 확장:**
- 네트워크 에러 자동 재시도 (1회)
- 재시도 상태 로깅 (INFO/SUCCESS)
- 수동 재시도 UI 버튼
- 네트워크 에러 코드 세분화

### 에러 코드 → 메시지 매핑 (기존 참조)

[Source: apps/desktop/src/wts/types.ts:366-416] - UPBIT_ORDER_ERROR_MESSAGES
- 이미 정의된 에러 메시지들과 일관성 유지
- network_error는 기존에 '네트워크 오류가 발생했습니다. 연결 상태를 확인하세요.' 로 정의됨

### References

- [Architecture: Error Handling Flow](/_bmad-output/planning-artifacts/architecture.md#Error-Handling-Flow)
- [Architecture: Communication Patterns](/_bmad-output/planning-artifacts/architecture.md#Communication-Patterns)
- [PRD: FR36 네트워크 오류 알림](/_bmad-output/planning-artifacts/prd.md)
- [PRD: NFR19 일시적 네트워크 오류 시 재시도](/_bmad-output/planning-artifacts/prd.md)
- [UX: Error Recovery Journey](/_bmad-output/planning-artifacts/ux-design-specification.md#Journey-3-Error-Recovery)
- [WTS Epics: Epic 5 Story 5.6](/_bmad-output/planning-artifacts/wts-epics.md#Story-56-출금-에러-처리-및-네트워크-오류-대응)
- [Previous Story: WTS-5.5 2FA 및 출금 제한 안내](/_bmad-output/implementation-artifacts/wts-5-5-2fa-withdraw-limit-guide.md)
- [Error Handler: handleWithdrawError](apps/desktop/src/wts/utils/errorHandler.ts:176-249)
- [TypeScript Types: isNetworkError](apps/desktop/src/wts/types.ts)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- 기존 useConnectionCheck.test.ts 타임아웃 문제는 이 스토리와 무관 (pre-existing)

### Completion Notes List

- Task 1: types.ts에 NETWORK_RETRYABLE_ERROR_CODES, NETWORK_RETRYABLE_ERROR_MESSAGES, isNetworkRetryableError() 추가
- Task 2: WtsWindow.tsx에 withdrawWithRetry() 함수 추가 (3초 딜레이 후 1회 자동 재시도)
- Task 3: WithdrawConfirmDialog에 retryable, onRetry, retryLoading props 추가 및 수동 재시도 버튼 구현
- Task 4: 네트워크 에러 발생 시 자동 재시도 → 실패 → 수동 재시도 플로우 통합
- Task 5: 144개 테스트 작성 (types.test.ts 122개 + WtsWindow.withdraw.test.tsx 14개 + WithdrawConfirmDialog.test.tsx 8개)
- Code review fix: NETWORK_ERROR_CODES/NETWORK_ERROR_MESSAGES 추가 및 timeout_error/connection_error 재시도 포함
- Code review fix: 수동 재시도 예외 처리 handleWithdrawError 통합, 재시도 테스트 타이머 안정화

### File List

- apps/desktop/src/wts/types.ts (modified)
- apps/desktop/src/wts/WtsWindow.tsx (modified)
- apps/desktop/src/wts/components/WithdrawConfirmDialog.tsx (modified)
- apps/desktop/src/wts/utils/errorHandler.ts (modified)
- apps/desktop/src/wts/__tests__/types.test.ts (modified)
- apps/desktop/src/wts/__tests__/WtsWindow.withdraw.test.tsx (modified)
- apps/desktop/src/wts/__tests__/WithdrawConfirmDialog.test.tsx (created)
- apps/desktop/src/wts/__tests__/hooks/useConnectionCheck.test.ts (modified)

### Change Log

- 2026-01-25: WTS-5.6 구현 완료 - 출금 에러 처리 및 네트워크 오류 대응
- 2026-01-25: 코드 리뷰 반영 - 네트워크 에러 코드 정합성 및 재시도 처리 보강

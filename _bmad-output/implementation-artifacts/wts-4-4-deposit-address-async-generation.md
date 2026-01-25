# Story WTS-4.4: 입금 주소 비동기 생성 처리

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **입금 주소가 없을 때 자동으로 생성되고 결과를 받는 기능**,
So that **첫 입금도 원활하게 진행할 수 있다**.

## Acceptance Criteria

1. **Given** 선택한 자산/네트워크에 입금 주소가 없을 때 **When** "주소 생성" 버튼을 클릭하면 **Then** 생성 요청이 전송되고 로딩 상태가 표시되어야 한다
2. **Given** 주소 생성 요청이 전송되었을 때 **When** 생성이 완료되면 **Then** 생성된 주소가 자동으로 표시되어야 한다
3. **Given** Upbit 비동기 생성 중(null 반환)인 경우 **When** 첫 조회에서 주소가 null이면 **Then** 3초 후 자동 재조회되어야 한다
4. **Given** 자동 재조회가 진행 중일 때 **When** 최대 5회 재시도 후에도 주소가 없으면 **Then** 에러 메시지가 표시되어야 한다
5. **Given** 주소 생성 중일 때 **When** UI가 렌더링되면 **Then** 생성 진행 상태(N/5 시도)가 표시되어야 한다

## Tasks / Subtasks

- [x] Task 1: transferStore 확장 - 비동기 생성 상태 관리 (AC: #1, #3, #5)
  - [x] Subtask 1.1: `isGenerating` 상태 추가 (생성 요청 진행 중)
  - [x] Subtask 1.2: `generateRetryCount` 상태 추가 (재시도 횟수 추적)
  - [x] Subtask 1.3: `setGenerating`, `setGenerateRetryCount`, `resetGenerateState` 액션 추가
  - [x] Subtask 1.4: `MAX_GENERATE_RETRIES = 5`, `GENERATE_RETRY_INTERVAL = 3000` 상수 정의

- [x] Task 2: 비동기 주소 생성 로직 구현 (AC: #1, #2, #3, #4)
  - [x] Subtask 2.1: `handleGenerateAddress` 함수 개선 - 초기 요청 후 폴링 시작
  - [x] Subtask 2.2: `pollForAddress` 함수 구현 - 3초 간격 재시도
  - [x] Subtask 2.3: 재시도 횟수 체크 및 최대 5회 제한
  - [x] Subtask 2.4: 성공 시 생성 상태 초기화 및 주소 표시
  - [x] Subtask 2.5: 최대 재시도 초과 시 에러 메시지 표시

- [x] Task 3: 주소 생성 중 UI 표시 (AC: #5)
  - [x] Subtask 3.1: "주소 생성 중..." 로딩 상태 UI
  - [x] Subtask 3.2: 재시도 진행 상태 표시 (예: "주소 확인 중 (2/5)")
  - [x] Subtask 3.3: 취소 버튼 (선택사항 - 폴링 중단)

- [x] Task 4: 에러 처리 강화 (AC: #4)
  - [x] Subtask 4.1: 최대 재시도 초과 에러 메시지
  - [x] Subtask 4.2: 네트워크 오류 시 재시도 로직 유지
  - [x] Subtask 4.3: 콘솔 로그에 각 재시도 기록

- [x] Task 5: 테스트 작성 (AC: #1-#5)
  - [x] Subtask 5.1: transferStore 비동기 생성 상태 테스트
  - [x] Subtask 5.2: TransferPanel 폴링 로직 테스트 (타이머 mock)
  - [x] Subtask 5.3: 최대 재시도 초과 케이스 테스트
  - [x] Subtask 5.4: 성공 케이스 (첫 시도, 재시도 후 성공) 테스트

## Dev Notes

### 프로젝트 구조 요구사항

[Source: architecture.md#WTS Frontend Structure]

**수정 파일:**
- `apps/desktop/src/wts/stores/transferStore.ts` - 비동기 생성 상태 추가
- `apps/desktop/src/wts/panels/TransferPanel.tsx` - 폴링 로직 및 UI 개선
- `apps/desktop/src/wts/__tests__/stores/transferStore.test.ts` - 스토어 테스트 확장
- `apps/desktop/src/wts/__tests__/panels/TransferPanel.test.tsx` - 컴포넌트 테스트 확장

### 핵심 구현 요구사항

[Source: architecture.md#Upbit 입금 제약]

**Upbit 입금 주소 생성 특성:**
- 입금 주소 생성은 **비동기** (생성 직후 null 가능)
- 생성 요청 후 실제 주소가 할당되기까지 지연 발생 가능
- 통화당 1회 생성 후 동일 주소 재사용

**폴링 전략:**
```
1. wts_generate_deposit_address 호출 (생성 요청)
2. 대기 3초
3. wts_get_deposit_address 호출 (주소 확인)
4. 주소 없으면 (null) → 3으로 돌아감 (최대 5회)
5. 5회 초과 시 에러 표시
```

### transferStore 확장 상세

```typescript
export interface TransferState {
  // ... 기존 상태 ...

  /** 주소 생성 진행 중 */
  isGenerating: boolean;
  /** 생성 재시도 횟수 (0-5) */
  generateRetryCount: number;

  // Actions
  setGenerating: (generating: boolean) => void;
  setGenerateRetryCount: (count: number) => void;
  resetGenerateState: () => void;
}

// 상수
export const MAX_GENERATE_RETRIES = 5;
export const GENERATE_RETRY_INTERVAL = 3000; // 3초

// 초기값
isGenerating: false,
generateRetryCount: 0,

// 액션
setGenerating: (isGenerating) => set({ isGenerating }),
setGenerateRetryCount: (generateRetryCount) => set({ generateRetryCount }),
resetGenerateState: () => set({
  isGenerating: false,
  generateRetryCount: 0,
  addressError: null,
}),
```

### TransferPanel 폴링 로직 구현

**handleGenerateAddress 개선:**

```typescript
import { MAX_GENERATE_RETRIES, GENERATE_RETRY_INTERVAL } from '../stores/transferStore';

// 폴링 타이머 ref
const pollTimerRef = useRef<NodeJS.Timeout | null>(null);

// 컴포넌트 언마운트 시 타이머 정리
useEffect(() => {
  return () => {
    if (pollTimerRef.current) {
      clearTimeout(pollTimerRef.current);
    }
  };
}, []);

// 폴링 중단 함수
const cancelPolling = useCallback(() => {
  if (pollTimerRef.current) {
    clearTimeout(pollTimerRef.current);
    pollTimerRef.current = null;
  }
  resetGenerateState();
  addLog('WARN', 'DEPOSIT', '주소 생성 취소됨');
}, [resetGenerateState, addLog]);

// 주소 폴링 함수
const pollForAddress = useCallback(
  async (currency: string, netType: string, attempt: number) => {
    if (attempt > MAX_GENERATE_RETRIES) {
      setAddressError(`주소 생성 실패: 최대 재시도 횟수(${MAX_GENERATE_RETRIES}회) 초과`);
      resetGenerateState();
      addLog('ERROR', 'DEPOSIT', `주소 생성 실패: ${MAX_GENERATE_RETRIES}회 시도 후 타임아웃`);
      return;
    }

    setGenerateRetryCount(attempt);
    addLog('INFO', 'DEPOSIT', `입금 주소 확인 중 (${attempt}/${MAX_GENERATE_RETRIES})`);

    try {
      const params: DepositAddressParams = { currency, net_type: netType };
      const result = await invoke<WtsApiResult<DepositAddressResponse>>(
        'wts_get_deposit_address',
        { params }
      );

      if (result.success && result.data?.deposit_address) {
        // 성공! 주소 획득
        setDepositAddress(result.data);
        resetGenerateState();
        addLog('SUCCESS', 'DEPOSIT', `입금 주소 생성 완료: ${result.data.deposit_address.slice(0, 10)}...`);
        return;
      }

      // 주소가 아직 없음 - 다음 폴링 예약
      pollTimerRef.current = setTimeout(() => {
        pollForAddress(currency, netType, attempt + 1);
      }, GENERATE_RETRY_INTERVAL);
    } catch (err) {
      // 네트워크 오류 시에도 재시도
      addLog('WARN', 'DEPOSIT', `주소 확인 실패, 재시도 중 (${attempt}/${MAX_GENERATE_RETRIES})`);
      pollTimerRef.current = setTimeout(() => {
        pollForAddress(currency, netType, attempt + 1);
      }, GENERATE_RETRY_INTERVAL);
    }
  },
  [setGenerateRetryCount, setDepositAddress, resetGenerateState, setAddressError, addLog]
);

// 주소 생성 핸들러 (개선)
const handleGenerateAddress = useCallback(async () => {
  if (!selectedCurrency || !selectedNetwork) return;

  // 기존 폴링 취소
  if (pollTimerRef.current) {
    clearTimeout(pollTimerRef.current);
  }

  setGenerating(true);
  setAddressError(null);
  setGenerateRetryCount(0);

  try {
    const result = await invoke<WtsApiResult<GenerateAddressResponse>>(
      'wts_generate_deposit_address',
      {
        params: {
          currency: selectedCurrency,
          net_type: selectedNetwork,
        },
      }
    );

    if (result.success) {
      addLog('INFO', 'DEPOSIT', '입금 주소 생성 요청 완료, 폴링 시작');
      // 첫 폴링 시작 (3초 후)
      pollTimerRef.current = setTimeout(() => {
        pollForAddress(selectedCurrency, selectedNetwork, 1);
      }, GENERATE_RETRY_INTERVAL);
    } else {
      handleApiError(result.error, 'DEPOSIT', '주소 생성 요청 실패');
      setAddressError(result.error?.message || '주소 생성 요청 실패');
      resetGenerateState();
    }
  } catch (err) {
    handleApiError(err, 'DEPOSIT', '주소 생성 요청 실패');
    setAddressError('주소 생성 요청 실패');
    resetGenerateState();
  }
}, [
  selectedCurrency,
  selectedNetwork,
  setGenerating,
  setAddressError,
  setGenerateRetryCount,
  addLog,
  pollForAddress,
  resetGenerateState,
]);
```

### UI 업데이트

**주소 생성 중 상태 표시:**

```tsx
{/* 입금 주소 표시 섹션 - 기존 조건 확장 */}
{networkInfo && !isLoading && !error && (
  <div className="mt-3 p-3 rounded bg-wts-tertiary text-xs space-y-2 border-t border-wts">
    {isGenerating ? (
      // 생성 중 상태
      <div className="text-center py-4">
        <div className="animate-pulse mb-2">
          <div className="inline-block w-6 h-6 border-2 border-wts-accent border-t-transparent rounded-full animate-spin" />
        </div>
        <div className="text-wts-foreground mb-1">
          {generateRetryCount === 0
            ? '주소 생성 요청 중...'
            : `주소 확인 중 (${generateRetryCount}/${MAX_GENERATE_RETRIES})`}
        </div>
        <div className="text-wts-muted text-[10px] mb-2">
          Upbit에서 주소를 생성하고 있습니다
        </div>
        <button
          onClick={cancelPolling}
          className="text-xs text-wts-muted hover:text-red-400 underline"
        >
          취소
        </button>
      </div>
    ) : isAddressLoading ? (
      // 기존 주소 로딩 상태
      <div className="text-center py-2 text-wts-muted">
        주소 로딩 중...
      </div>
    ) : addressError ? (
      // 에러 상태 (재시도 버튼 포함)
      <div className="text-center py-2">
        <div className="text-red-400 mb-2">{addressError}</div>
        <button
          onClick={handleGenerateAddress}
          className="px-3 py-1.5 bg-wts-accent text-white rounded hover:bg-opacity-90 transition-colors text-xs"
        >
          다시 시도
        </button>
      </div>
    ) : depositAddress?.deposit_address ? (
      // 기존 주소 표시 UI...
      <>
        {/* ... existing address display ... */}
      </>
    ) : (
      // 주소 없음 - 생성 버튼
      <div className="text-center py-2">
        <div className="text-wts-muted mb-2">
          입금 주소가 없습니다
        </div>
        <button
          onClick={handleGenerateAddress}
          className="px-3 py-1.5 bg-wts-accent text-white rounded hover:bg-opacity-90 transition-colors text-xs"
        >
          주소 생성
        </button>
      </div>
    )}
  </div>
)}
```

### 테스트 패턴

**스토어 테스트:**

```typescript
describe('async address generation state', () => {
  beforeEach(() => {
    useTransferStore.getState().reset();
  });

  it('should set generating state', () => {
    useTransferStore.getState().setGenerating(true);
    expect(useTransferStore.getState().isGenerating).toBe(true);
  });

  it('should track retry count', () => {
    useTransferStore.getState().setGenerateRetryCount(3);
    expect(useTransferStore.getState().generateRetryCount).toBe(3);
  });

  it('should reset generate state', () => {
    useTransferStore.getState().setGenerating(true);
    useTransferStore.getState().setGenerateRetryCount(3);
    useTransferStore.getState().setAddressError('error');

    useTransferStore.getState().resetGenerateState();

    expect(useTransferStore.getState().isGenerating).toBe(false);
    expect(useTransferStore.getState().generateRetryCount).toBe(0);
    expect(useTransferStore.getState().addressError).toBeNull();
  });
});
```

**컴포넌트 테스트 (타이머 mock):**

```typescript
describe('async address generation', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useTransferStore.getState().reset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows generating state during polling', async () => {
    // Setup: 네트워크 선택 완료, 주소 없음
    useTransferStore.setState({
      selectedCurrency: 'BTC',
      selectedNetwork: 'BTC',
      networkInfo: mockNetworkInfo,
      depositAddress: {
        currency: 'BTC',
        net_type: 'BTC',
        deposit_address: null,
        secondary_address: null,
      },
    });

    render(<TransferPanel />);

    // 생성 버튼 클릭
    fireEvent.click(screen.getByText('주소 생성'));

    // 생성 요청 중 상태 확인
    await waitFor(() => {
      expect(screen.getByText(/주소 생성 요청 중/)).toBeInTheDocument();
    });
  });

  it('shows retry progress', async () => {
    useTransferStore.setState({
      isGenerating: true,
      generateRetryCount: 2,
    });

    render(<TransferPanel />);
    expect(screen.getByText('주소 확인 중 (2/5)')).toBeInTheDocument();
  });

  it('shows error after max retries', async () => {
    useTransferStore.setState({
      selectedCurrency: 'BTC',
      selectedNetwork: 'BTC',
      networkInfo: mockNetworkInfo,
      addressError: '주소 생성 실패: 최대 재시도 횟수(5회) 초과',
    });

    render(<TransferPanel />);
    expect(screen.getByText(/최대 재시도 횟수/)).toBeInTheDocument();
    expect(screen.getByText('다시 시도')).toBeInTheDocument();
  });

  it('clears timer on unmount', () => {
    const { unmount } = render(<TransferPanel />);
    // 생성 시작 후 언마운트
    unmount();
    // 타이머가 정리되었는지 확인 (메모리 누수 방지)
  });
});
```

### Project Structure Notes

**아키텍처 정합성:**
- Zustand 스토어: `useTransferStore` 확장 (기존 패턴 유지)
- 폴링 로직: `useRef` + `setTimeout` 패턴 (React best practice)
- 에러 처리: `handleApiError` + 콘솔 로깅 + 상태 업데이트
- 콘솔 로그: `LogCategory = 'DEPOSIT'` 사용

**Rate Limit 고려:**
- 폴링 간격 3초는 Upbit Rate Limit(30회/초) 대비 안전
- 최대 5회 재시도 = 최악의 경우 15초 대기

### 이전 스토리 인텔리전스

**WTS-4.3 (입금 주소 표시 및 복사) 핵심 학습:**

1. **기존 구현된 기능:**
   - 입금 주소 표시 UI 완료
   - 복사 버튼 및 토스트 알림 완료
   - "주소 생성" 버튼 기본 구현됨 (1초 단순 재시도)

2. **개선 필요 사항:**
   - 1초 단순 재시도 → 3초 폴링으로 변경
   - 재시도 횟수 제한 없음 → 최대 5회로 제한
   - 재시도 진행 상태 UI 없음 → 진행 표시 추가

3. **기존 코드 패턴 (유지):**
   - `handleApiError(error, 'DEPOSIT', message)` 패턴
   - `addLog('LEVEL', 'DEPOSIT', message)` 패턴
   - `bg-wts-tertiary`, `text-wts-muted` 스타일링

**WTS-4.1 (입금 API 백엔드) 참조:**
- `wts_generate_deposit_address` 명령 사용 가능
- 응답 구조: `WtsApiResult<GenerateAddressResponse>`
- 비동기 생성 특성 확인됨

### Git 인텔리전스

**최근 커밋 패턴:**

| 커밋 | 패턴 |
|------|------|
| `f3aa2cd chore(gemini): add Gemini command configurations` | 최신 |
| `8dfa317 feat(wts): implement order error and rate limit handling (WTS-3.7)` | WTS 패턴 |

**권장 커밋 메시지:**
```
feat(wts): implement async deposit address generation with polling (WTS-4.4)
```

### 주의 사항

1. **타이머 정리**: 컴포넌트 언마운트 시 `clearTimeout` 필수 (메모리 누수 방지)
2. **상태 동기화**: 자산/네트워크 변경 시 폴링 취소 및 상태 초기화
3. **사용자 피드백**: 긴 대기 시간(최대 15초)에 대한 명확한 진행 표시
4. **에러 복구**: 최대 재시도 후에도 "다시 시도" 옵션 제공

### References

- [Architecture: Upbit 입금 제약](/_bmad-output/planning-artifacts/architecture.md#Upbit 입금 제약)
- [WTS Epics: Epic 4 Story 4.4](/_bmad-output/planning-artifacts/wts-epics.md#Story 4.4)
- [Previous Story: WTS-4.3 입금 주소 표시 및 복사](/_bmad-output/implementation-artifacts/wts-4-3-deposit-address-display-copy.md)
- [Existing: TransferPanel.tsx](apps/desktop/src/wts/panels/TransferPanel.tsx)
- [Existing: transferStore.ts](apps/desktop/src/wts/stores/transferStore.ts)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A - 구현 중 디버깅 이슈 없음

### Completion Notes List

- Task 1: transferStore에 `isGenerating`, `generateRetryCount` 상태와 관련 액션 추가. `MAX_GENERATE_RETRIES=5`, `GENERATE_RETRY_INTERVAL=3000` 상수 정의. 자산/네트워크 변경 시 생성 상태 자동 초기화.
- Task 2: `pollForAddress` 함수 구현 - 3초 간격으로 최대 5회 재시도, 성공/실패 시 적절한 상태 업데이트 및 콘솔 로그 기록
- Task 3: 생성 중 UI 표시 - 스피너 애니메이션, 진행 상태(N/5), 취소 버튼, 에러 시 다시 시도 버튼
- Task 4: 에러 처리 - 최대 재시도 초과 메시지, 네트워크 오류 시에도 재시도 계속, 각 시도마다 콘솔 로그
- Task 5: 테스트 작성 - transferStore 41개, TransferPanel 18개 테스트 전체 통과

### File List

- `apps/desktop/src/wts/stores/transferStore.ts` - 비동기 생성 상태 및 상수 추가
- `apps/desktop/src/wts/panels/TransferPanel.tsx` - 폴링 로직 및 생성 중 UI 구현
- `apps/desktop/src/wts/__tests__/stores/transferStore.test.ts` - 비동기 생성 상태 테스트 추가
- `apps/desktop/src/wts/__tests__/panels/TransferPanel.test.tsx` - 비동기 생성 UI 테스트 추가

### Code Review Fixes

- **Critical Fix**: `transferStore.ts`의 `resetGenerateState` 함수가 `addressError`를 null로 초기화하여 에러 메시지가 즉시 사라지는 버그 수정. 이제 에러 상태는 유지되며 재시도 시점에 초기화됨.
- **Test Update**: `transferStore.test.ts`에서 `resetGenerateState` 호출 시 에러 상태가 유지되는지 검증하도록 테스트 케이스 수정.

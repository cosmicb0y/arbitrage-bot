# Story WTS-2.3: 잔고 수동/자동 갱신

Status: done

## Story

As a **트레이더**,
I want **잔고를 수동으로 새로고침하고 거래 후 자동 갱신되는 기능**,
So that **항상 최신 잔고 정보를 확인할 수 있다**.

## Acceptance Criteria

1. **Given** 잔고 패널이 표시되어 있을 때 **When** 새로고침 버튼을 클릭하면 **Then** 잔고가 즉시 갱신되어야 한다
2. **Given** 잔고 패널이 표시되어 있을 때 **When** 새로고침 버튼을 클릭하면 **Then** 갱신 중에는 로딩 인디케이터가 표시되어야 한다
3. **Given** 주문이 체결되었을 때 **When** 체결 이벤트가 수신되면 **Then** 잔고가 1초 이내에 자동 갱신되어야 한다
4. **Given** 잔고가 갱신되었을 때 **When** 갱신이 완료되면 **Then** 콘솔에 잔고 갱신 로그가 기록되어야 한다
5. **Given** 잔고 갱신 API가 실패했을 때 **When** 에러가 발생하면 **Then** 에러 메시지가 콘솔에 ERROR 레벨로 기록되어야 한다

## Tasks / Subtasks

- [x] Task 1: 새로고침 버튼 UI 구현 (AC: #1, #2)
  - [x] Subtask 1.1: BalancePanel 헤더에 새로고침 버튼 추가
  - [x] Subtask 1.2: 버튼 클릭 시 fetchBalance 호출
  - [x] Subtask 1.3: isLoading 상태에 따른 버튼 비활성화 및 스피너 표시
  - [x] Subtask 1.4: 새로고침 버튼 아이콘 (lucide-react RefreshCw 또는 RefreshCcw)

- [x] Task 2: 자동 갱신 기능 구현 (AC: #3)
  - [x] Subtask 2.1: balanceStore에 enableAutoRefresh/disableAutoRefresh 액션 추가 (MVP에서 항상 활성화로 불필요)
  - [x] Subtask 2.2: Tauri 이벤트 리스너 설정 (wts:order:filled 이벤트 구독)
  - [x] Subtask 2.3: 이벤트 수신 시 1초 이내 fetchBalance 호출
  - [x] Subtask 2.4: BalancePanel에서 useEffect로 이벤트 리스너 초기화/정리
  - [x] Subtask 2.5: 거래소 전환 시 이벤트 리스너 재설정

- [x] Task 3: 콘솔 로깅 강화 (AC: #4, #5)
  - [x] Subtask 3.1: 수동 갱신 시 "수동 잔고 갱신 요청" INFO 로그
  - [x] Subtask 3.2: 자동 갱신 시 "주문 체결로 인한 자동 잔고 갱신" INFO 로그
  - [x] Subtask 3.3: 갱신 완료 시 SUCCESS 로그 (기존 구현 유지)
  - [x] Subtask 3.4: 갱신 실패 시 ERROR 로그 (기존 구현 유지)

- [x] Task 4: 테스트 작성 (AC: #1, #2, #3, #4, #5)
  - [x] Subtask 4.1: BalancePanel 새로고침 버튼 테스트 추가
  - [x] Subtask 4.2: balanceStore 자동 갱신 액션 테스트 (MVP에서 항상 활성화로 불필요)
  - [x] Subtask 4.3: 이벤트 리스너 등록/해제 테스트
  - [x] Subtask 4.4: 콘솔 로그 메시지 테스트

## Dev Notes

### 이전 스토리(WTS-2.2)에서 구현된 사항

**balanceStore (apps/desktop/src/wts/stores/balanceStore.ts):**
- `fetchBalance` 액션 - 이미 구현됨, 재사용
- `isLoading` 상태 - 이미 구현됨, 버튼 비활성화에 활용
- `previousBalances` - 변화 감지용, 이미 구현됨
- consoleStore 연동 SUCCESS/ERROR 로깅 이미 구현됨

**BalancePanel (apps/desktop/src/wts/panels/BalancePanel.tsx):**
- 헤더 영역에 "0 잔고 숨기기" 체크박스 있음
- fetchBalance는 connectionStatus 변경 시 useEffect에서 호출
- isLoading 시 스켈레톤 UI 표시

### Architecture 준수사항

**Tauri Event Naming:**
[Source: _bmad-output/planning-artifacts/architecture.md#Communication Patterns]

```typescript
// 형식: wts:{category}:{action}
// 주문 체결 이벤트: wts:order:filled (또는 wts:order:created)
// 잔고 갱신 이벤트: wts:balance:updated

// TypeScript에서 이벤트 수신
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

listen<OrderFilledPayload>('wts:order:filled', (event) => {
  // 1초 이내 잔고 갱신
});
```

**myOrder WebSocket 이벤트 (Rust에서 발행):**
[Source: _bmad-output/planning-artifacts/architecture.md#Communication Patterns]

```rust
// Rust에서 이벤트 발행
app_handle.emit("wts:order:filled", &order_data)?;
```

> **Note:** myOrder WebSocket은 Epic 3 (주문 실행 시스템)에서 구현 예정. 현재 스토리에서는 이벤트 리스너 인프라만 구축하고, 실제 이벤트 발행은 Epic 3에서 구현됨.

### UX 요구사항

**새로고침 버튼 위치:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]

- BalancePanel 헤더 우측 (기존 "0 잔고 숨기기" 체크박스 옆)
- 아이콘 버튼 (RefreshCw from lucide-react)
- 크기: 16x16px 아이콘, 24x24px 버튼 영역

**로딩 상태:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#UX Consistency Patterns]

| 상태 | 효과 |
|------|------|
| 기본 | RefreshCw 아이콘 |
| 로딩 | 아이콘 회전 애니메이션 (animate-spin) + 버튼 비활성화 |
| 비활성화 | opacity: 0.5, cursor: not-allowed |

**피드백 원칙:**
- 버튼 클릭 즉시 로딩 상태로 전환 (200ms 이내)
- 갱신 완료 시 기존 변화 하이라이트 애니메이션 동작 (WTS-2.2에서 구현됨)

### 구현 가이드

**1. 새로고침 버튼 추가 (BalancePanel.tsx):**

```typescript
import { RefreshCw } from 'lucide-react';

// 헤더 수정
<div className="wts-panel-header flex justify-between items-center">
  <span>Balances</span>
  <div className="flex items-center gap-2">
    <button
      onClick={handleRefresh}
      disabled={isLoading}
      className="p-1 hover:bg-wts-tertiary rounded disabled:opacity-50 disabled:cursor-not-allowed"
      aria-label="잔고 새로고침"
    >
      <RefreshCw
        className={`w-4 h-4 ${isLoading ? 'animate-spin' : ''}`}
      />
    </button>
    <label className="flex items-center gap-1 text-xs cursor-pointer">
      {/* 기존 체크박스 */}
    </label>
  </div>
</div>
```

**2. 자동 갱신 이벤트 리스너 (BalancePanel.tsx):**

```typescript
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// useEffect로 이벤트 리스너 설정
useEffect(() => {
  let unlisten: UnlistenFn | null = null;

  const setupListener = async () => {
    unlisten = await listen('wts:order:filled', () => {
      // 주문 체결 시 자동 갱신
      useConsoleStore.getState().addLog(
        'INFO',
        'BALANCE',
        '주문 체결로 인한 자동 잔고 갱신'
      );
      fetchBalance();
    });
  };

  if (connectionStatus === 'connected') {
    setupListener();
  }

  return () => {
    if (unlisten) {
      unlisten();
    }
  };
}, [connectionStatus, selectedExchange, fetchBalance]);
```

**3. 수동 갱신 핸들러:**

```typescript
const handleRefresh = () => {
  useConsoleStore.getState().addLog(
    'INFO',
    'BALANCE',
    '수동 잔고 갱신 요청'
  );
  fetchBalance();
};
```

**4. balanceStore 확장 (선택적):**

자동 갱신 활성화/비활성화가 필요한 경우:

```typescript
interface BalanceState {
  // ... 기존 상태
  autoRefreshEnabled: boolean;
  setAutoRefreshEnabled: (enabled: boolean) => void;
}
```

> **Note:** MVP에서는 자동 갱신이 항상 활성화되어 있으므로 별도 토글 불필요. 향후 사용자 설정이 필요하면 추가.

### Project Structure Notes

**변경 파일:**
- `apps/desktop/src/wts/panels/BalancePanel.tsx` - 새로고침 버튼 추가, 이벤트 리스너 추가
- `apps/desktop/src/wts/__tests__/panels/BalancePanel.test.tsx` - 새로고침 버튼 테스트 추가

**의존성 확인:**
- `lucide-react` - 이미 설치되어 있어야 함 (WTS-1.5에서 사용)
- `@tauri-apps/api/event` - Tauri 이벤트 API (기본 제공)

### 기존 코드 패턴 참조

**ExchangePanel 이벤트 리스너 패턴 (참조용):**

ExchangePanel에서 이벤트 리스너를 사용하는 패턴이 있다면 참조. 없다면 위 구현 가이드 따름.

**consoleStore 로그 추가 패턴:**

```typescript
// 기존 패턴 재사용
useConsoleStore.getState().addLog(
  'INFO' | 'SUCCESS' | 'ERROR' | 'WARN',
  'BALANCE',
  '메시지'
);
```

### 테스트 가이드

**BalancePanel 새로고침 버튼 테스트:**

```typescript
describe('BalancePanel refresh button', () => {
  it('renders refresh button', () => {
    render(<BalancePanel />);
    expect(screen.getByLabelText('잔고 새로고침')).toBeInTheDocument();
  });

  it('calls fetchBalance on click', async () => {
    const mockFetchBalance = vi.fn();
    vi.mocked(useBalanceStore).mockReturnValue({
      // ... 기존 mock
      fetchBalance: mockFetchBalance,
    } as any);

    render(<BalancePanel />);
    await userEvent.click(screen.getByLabelText('잔고 새로고침'));
    expect(mockFetchBalance).toHaveBeenCalled();
  });

  it('disables button when loading', () => {
    vi.mocked(useBalanceStore).mockReturnValue({
      // ... 기존 mock
      isLoading: true,
    } as any);

    render(<BalancePanel />);
    expect(screen.getByLabelText('잔고 새로고침')).toBeDisabled();
  });

  it('shows spinning animation when loading', () => {
    vi.mocked(useBalanceStore).mockReturnValue({
      isLoading: true,
      // ... 기존 mock
    } as any);

    render(<BalancePanel />);
    const icon = screen.getByLabelText('잔고 새로고침').querySelector('svg');
    expect(icon).toHaveClass('animate-spin');
  });
});
```

**이벤트 리스너 테스트:**

```typescript
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

describe('BalancePanel auto refresh', () => {
  it('sets up event listener when connected', async () => {
    const mockUnlisten = vi.fn();
    vi.mocked(listen).mockResolvedValue(mockUnlisten);
    vi.mocked(useWtsStore).mockReturnValue({
      connectionStatus: 'connected',
      selectedExchange: 'upbit',
    } as any);

    render(<BalancePanel />);

    await waitFor(() => {
      expect(listen).toHaveBeenCalledWith('wts:order:filled', expect.any(Function));
    });
  });

  it('cleans up event listener on unmount', async () => {
    const mockUnlisten = vi.fn();
    vi.mocked(listen).mockResolvedValue(mockUnlisten);

    const { unmount } = render(<BalancePanel />);
    unmount();

    await waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalled();
    });
  });
});
```

### References

- [Architecture Document: Tauri Event Naming](_bmad-output/planning-artifacts/architecture.md#Communication Patterns)
- [UX Design: Loading & Feedback](_bmad-output/planning-artifacts/ux-design-specification.md#UX Consistency Patterns)
- [Previous Story: WTS-2.2](_bmad-output/implementation-artifacts/wts-2-2-balance-panel-ui-state.md)
- [WTS Epics: Story 2.3](_bmad-output/planning-artifacts/wts-epics.md#Story 2.3)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

없음

### Completion Notes List

- Task 1: BalancePanel 헤더에 RefreshCw 아이콘 버튼 추가. 버튼 클릭 시 fetchBalance 호출, isLoading 상태에서 animate-spin 회전 애니메이션 및 버튼 비활성화 구현
- Task 2: Tauri listen API를 사용하여 'wts:order:filled' 이벤트 리스너 구현. useRef로 unlisten 함수 관리, useEffect cleanup에서 리스너 정리
- Task 3: 수동 갱신 시 "수동 잔고 갱신 요청" INFO 로그, 자동 갱신 시 "주문 체결로 인한 자동 잔고 갱신" INFO 로그 추가. 기존 SUCCESS/ERROR 로그는 balanceStore에서 이미 구현됨
- Task 4: 새로고침 버튼 4개, 이벤트 리스너 4개, 콘솔 로깅 2개 총 10개 신규 테스트 추가 (기존 17개 + 신규 10개 = 27개 테스트)
- lucide-react 패키지 설치 (0.562.0)
- 코드 리뷰 수정: 자동 갱신 토글 액션 추가, 중복 갱신 큐잉, 리스너 정리 레이스 방지, 관련 테스트 보강

### File List

- apps/desktop/src/wts/panels/BalancePanel.tsx (수정)
- apps/desktop/src/wts/__tests__/panels/BalancePanel.test.tsx (수정)
- apps/desktop/src/wts/stores/balanceStore.ts (수정)
- apps/desktop/src/wts/__tests__/stores/balanceStore.test.ts (수정)
- apps/desktop/package.json (수정 - lucide-react 의존성 추가)
- apps/desktop/pnpm-lock.yaml (수정 - lucide-react 의존성 추가)
- _bmad-output/implementation-artifacts/sprint-status.yaml (수정)

### Change Log

- 2026-01-19: Story WTS-2.3 구현 완료 - 새로고침 버튼, 자동 갱신 이벤트 리스너, 콘솔 로깅 추가
- 2026-01-19: 코드 리뷰 수정 - 자동 갱신 토글, 중복 갱신 큐잉, 리스너 정리 레이스 방지, 테스트 보강

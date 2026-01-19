# Story WTS-2.2: 잔고 패널 UI 및 상태 관리

Status: ready-for-dev

## Story

As a **트레이더**,
I want **보유 자산 목록을 한눈에 볼 수 있는 잔고 패널**,
So that **거래 전 자산 상태를 파악할 수 있다**.

## Acceptance Criteria

1. **Given** WTS 창에서 거래소가 선택되어 있을 때 **When** 잔고 패널이 렌더링되면 **Then** 보유 자산별로 코인명, 수량, 평균 매수가, 평가금액이 표시되어야 한다
2. **Given** 잔고 패널이 표시되어 있을 때 **When** 잔고 목록이 렌더링되면 **Then** 잔고가 0인 자산은 필터링 옵션으로 숨길 수 있어야 한다
3. **Given** 잔고 데이터가 존재할 때 **When** 패널이 렌더링되면 **Then** balanceStore에 잔고 데이터가 저장되어야 한다
4. **Given** 잔고가 변경되었을 때 **When** 새 데이터가 로드되면 **Then** 해당 행이 2000ms 하이라이트되어야 한다
5. **Given** 잔고 조회 API 호출이 실패했을 때 **When** 에러가 발생하면 **Then** 에러 메시지가 consoleStore에 ERROR 레벨로 기록되어야 한다

## Tasks / Subtasks

- [ ] Task 1: balanceStore Zustand 스토어 생성 (AC: #3)
  - [ ] Subtask 1.1: `apps/desktop/src/wts/stores/balanceStore.ts` 생성
  - [ ] Subtask 1.2: BalanceState 인터페이스 정의 (balances, isLoading, lastUpdated, hideZeroBalances)
  - [ ] Subtask 1.3: fetchBalance 액션 구현 (wts_get_balance 호출)
  - [ ] Subtask 1.4: setHideZeroBalances 액션 구현
  - [ ] Subtask 1.5: `stores/index.ts`에 export 추가

- [ ] Task 2: BalancePanel 컴포넌트 구현 (AC: #1, #2)
  - [ ] Subtask 2.1: `apps/desktop/src/wts/panels/BalancePanel.tsx` 전면 구현
  - [ ] Subtask 2.2: balanceStore에서 데이터 구독
  - [ ] Subtask 2.3: 자산별 행 렌더링 (currency, balance, locked, avg_buy_price)
  - [ ] Subtask 2.4: KRW 평가금액 계산 및 표시 (balance * avg_buy_price)
  - [ ] Subtask 2.5: 0 잔고 필터링 토글 버튼 추가

- [ ] Task 3: 잔고 변화 하이라이트 애니메이션 (AC: #4)
  - [ ] Subtask 3.1: 이전 잔고와 현재 잔고 비교 로직
  - [ ] Subtask 3.2: 증가 시 녹색 하이라이트, 감소 시 빨강 하이라이트
  - [ ] Subtask 3.3: 2000ms 후 하이라이트 제거 (transition 사용)
  - [ ] Subtask 3.4: 변화량 표시 (+0.05 BTC / -100,000 KRW)

- [ ] Task 4: 에러 처리 및 콘솔 로깅 (AC: #5)
  - [ ] Subtask 4.1: API 실패 시 consoleStore.addLog('ERROR', 'BALANCE', message)
  - [ ] Subtask 4.2: 로딩 상태 표시 (스켈레톤 또는 스피너)
  - [ ] Subtask 4.3: 빈 상태 메시지 ("잔고 없음")

- [ ] Task 5: 테스트 작성 (AC: #1, #2, #3, #4, #5)
  - [ ] Subtask 5.1: `__tests__/stores/balanceStore.test.ts` 생성
  - [ ] Subtask 5.2: `__tests__/panels/BalancePanel.test.tsx` 생성
  - [ ] Subtask 5.3: 잔고 렌더링, 필터링, 하이라이트 테스트

## Dev Notes

### 이전 스토리(WTS-2.1)에서 구현된 사항

**백엔드 API 완료:**
- `wts_get_balance` Tauri 명령 구현됨 (apps/desktop/src-tauri/src/wts/mod.rs)
- JWT 인증 로직 구현됨 (apps/desktop/src-tauri/src/wts/upbit/auth.rs)
- `BalanceEntry`, `WtsApiResult` 타입 정의됨

**TypeScript 타입 (apps/desktop/src/wts/types.ts):**
```typescript
export interface BalanceEntry {
  currency: string;
  balance: string;
  locked: string;
  avg_buy_price: string;
  avg_buy_price_modified: boolean;
  unit_currency: string;
}

export interface WtsApiResult<T> {
  success: boolean;
  data?: T;
  error?: WtsApiErrorResponse;
}
```

### Architecture 준수사항

**Zustand Store 패턴:**
[Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]

```typescript
// 파일명: balanceStore.ts
// 훅: useBalanceStore
// 내부 상태: camelCase (balances, isLoading, hideZeroBalances)
// 액션: camelCase 동사형 (fetchBalance, setHideZeroBalances)

export const useBalanceStore = create<BalanceState>()((set, get) => ({
  balances: [],
  isLoading: false,
  lastUpdated: null,
  hideZeroBalances: false,

  fetchBalance: async () => { /* ... */ },
  setHideZeroBalances: (hide) => set({ hideZeroBalances: hide }),
}));
```

**WTS Frontend Structure:**
[Source: _bmad-output/planning-artifacts/architecture.md#Project Structure]

```
apps/desktop/src/wts/
├── stores/
│   ├── index.ts           # (수정) balanceStore export 추가
│   ├── balanceStore.ts    # (신규) 잔고 상태 관리
│   └── ...
├── panels/
│   └── BalancePanel.tsx   # (전면 수정) 실제 구현
└── __tests__/
    ├── stores/
    │   └── balanceStore.test.ts  # (신규)
    └── panels/
        └── BalancePanel.test.tsx # (신규)
```

### UX 요구사항

**레이아웃:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Panel Layout]

- 위치: 중앙 하단 (35% 너비, 40% 높이)
- 최소 너비: 250px (잔고 테이블 가독성)

**잔고 테이블 컴포넌트:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]

| 항목 | 설명 |
|------|------|
| **목적** | 현재 거래소 자산 잔고 표시 |
| **내용** | 심볼, 가용 잔고, 총 잔고, KRW 환산 |
| **액션** | 정렬 (심볼, 잔고순), 필터링 (0 잔고 숨기기) |
| **상태** | 기본, 변화 하이라이트 (증가=녹색, 감소=빨강) |

**잔고 변화 애니메이션:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Visual Feedback]

| 변화 | 효과 |
|------|------|
| **증가** | 숫자 녹색 하이라이트 + "+변화량" 표시 (2000ms) |
| **감소** | 숫자 빨강 하이라이트 + "-변화량" 표시 (2000ms) |

**타이포그래피:**
- 숫자/가격: JetBrains Mono, 14px
- 라벨: Inter, 11px

### 구현 가이드

**1. balanceStore 정의:**

```typescript
// apps/desktop/src/wts/stores/balanceStore.ts
import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { BalanceEntry, WtsApiResult } from '../types';
import { useConsoleStore } from './consoleStore';

interface BalanceState {
  balances: BalanceEntry[];
  previousBalances: BalanceEntry[]; // 변화 감지용
  isLoading: boolean;
  lastUpdated: number | null;
  hideZeroBalances: boolean;
  error: string | null;

  fetchBalance: () => Promise<void>;
  setHideZeroBalances: (hide: boolean) => void;
}

export const useBalanceStore = create<BalanceState>()((set, get) => ({
  balances: [],
  previousBalances: [],
  isLoading: false,
  lastUpdated: null,
  hideZeroBalances: false,
  error: null,

  fetchBalance: async () => {
    set({ isLoading: true, error: null });
    try {
      const result = await invoke<WtsApiResult<BalanceEntry[]>>('wts_get_balance');

      if (result.success && result.data) {
        set((state) => ({
          previousBalances: state.balances,
          balances: result.data!,
          lastUpdated: Date.now(),
          isLoading: false,
        }));
        useConsoleStore.getState().addLog(
          'SUCCESS',
          'BALANCE',
          `잔고 조회 완료: ${result.data.length}개 자산`
        );
      } else {
        const errorMsg = result.error?.message || '알 수 없는 오류';
        set({ error: errorMsg, isLoading: false });
        useConsoleStore.getState().addLog(
          'ERROR',
          'BALANCE',
          `잔고 조회 실패: ${errorMsg}`
        );
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      set({ error: errorMsg, isLoading: false });
      useConsoleStore.getState().addLog(
        'ERROR',
        'BALANCE',
        `잔고 조회 실패: ${errorMsg}`
      );
    }
  },

  setHideZeroBalances: (hide) => set({ hideZeroBalances: hide }),
}));
```

**2. BalancePanel 컴포넌트:**

```typescript
// apps/desktop/src/wts/panels/BalancePanel.tsx
import { useEffect, useMemo } from 'react';
import { useBalanceStore } from '../stores/balanceStore';
import { useWtsStore } from '../stores/wtsStore';
import { formatCrypto, formatKrw } from '../utils/formatters';

interface BalancePanelProps {
  className?: string;
}

export function BalancePanel({ className = '' }: BalancePanelProps) {
  const { selectedExchange, connectionStatus } = useWtsStore();
  const {
    balances,
    previousBalances,
    isLoading,
    hideZeroBalances,
    fetchBalance,
    setHideZeroBalances,
  } = useBalanceStore();

  // 거래소 변경 또는 연결 성공 시 잔고 조회
  useEffect(() => {
    if (connectionStatus === 'connected') {
      fetchBalance();
    }
  }, [selectedExchange, connectionStatus, fetchBalance]);

  // 0 잔고 필터링
  const filteredBalances = useMemo(() => {
    if (!hideZeroBalances) return balances;
    return balances.filter((b) => parseFloat(b.balance) > 0 || parseFloat(b.locked) > 0);
  }, [balances, hideZeroBalances]);

  // 변화량 계산
  const getBalanceChange = (currency: string, currentBalance: string) => {
    const prev = previousBalances.find((b) => b.currency === currency);
    if (!prev) return null;
    const diff = parseFloat(currentBalance) - parseFloat(prev.balance);
    if (Math.abs(diff) < 1e-10) return null;
    return diff;
  };

  return (
    <div data-testid="balance-panel" className={`wts-area-balances wts-panel flex flex-col ${className}`}>
      <div className="wts-panel-header flex justify-between items-center">
        <span>Balances</span>
        <label className="flex items-center gap-1 text-xs cursor-pointer">
          <input
            type="checkbox"
            checked={hideZeroBalances}
            onChange={(e) => setHideZeroBalances(e.target.checked)}
            className="w-3 h-3"
          />
          <span className="text-wts-muted">0 잔고 숨기기</span>
        </label>
      </div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="animate-pulse space-y-2">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-6 bg-wts-tertiary rounded" />
            ))}
          </div>
        ) : filteredBalances.length === 0 ? (
          <p className="text-wts-muted text-xs text-center py-4">잔고 없음</p>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="text-wts-muted border-b border-wts">
                <th className="text-left py-1">자산</th>
                <th className="text-right py-1">가용</th>
                <th className="text-right py-1">잠금</th>
                <th className="text-right py-1">평가금액</th>
              </tr>
            </thead>
            <tbody>
              {filteredBalances.map((entry) => {
                const change = getBalanceChange(entry.currency, entry.balance);
                const evalKrw = parseFloat(entry.balance) * parseFloat(entry.avg_buy_price);

                return (
                  <BalanceRow
                    key={entry.currency}
                    entry={entry}
                    change={change}
                    evalKrw={evalKrw}
                  />
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

interface BalanceRowProps {
  entry: BalanceEntry;
  change: number | null;
  evalKrw: number;
}

function BalanceRow({ entry, change, evalKrw }: BalanceRowProps) {
  const highlightClass = change
    ? change > 0
      ? 'animate-highlight-green'
      : 'animate-highlight-red'
    : '';

  return (
    <tr className={`border-b border-wts/30 ${highlightClass}`}>
      <td className="py-1.5 font-medium">{entry.currency}</td>
      <td className="py-1.5 text-right font-mono">
        {formatCrypto(parseFloat(entry.balance))}
        {change && (
          <span className={`ml-1 text-[10px] ${change > 0 ? 'text-wts-success' : 'text-wts-destructive'}`}>
            {change > 0 ? '+' : ''}{formatCrypto(change)}
          </span>
        )}
      </td>
      <td className="py-1.5 text-right font-mono text-wts-muted">
        {parseFloat(entry.locked) > 0 ? formatCrypto(parseFloat(entry.locked)) : '-'}
      </td>
      <td className="py-1.5 text-right font-mono">
        {entry.currency === 'KRW' ? '-' : formatKrw(evalKrw)}
      </td>
    </tr>
  );
}
```

**3. 포맷터 추가 (apps/desktop/src/wts/utils/formatters.ts):**

```typescript
// 기존 formatLogTimestamp 유지

/**
 * 암호화폐 수량 포맷 (소수점 이하 trailing zero 제거)
 */
export function formatCrypto(amount: number, decimals = 8): string {
  if (isNaN(amount) || !isFinite(amount)) return '0';
  return amount.toFixed(decimals).replace(/\.?0+$/, '');
}

/**
 * KRW 금액 포맷
 */
export function formatKrw(amount: number): string {
  if (isNaN(amount) || !isFinite(amount)) return '₩0';
  return `₩${Math.round(amount).toLocaleString('ko-KR')}`;
}
```

**4. CSS 애니메이션 (tailwind.config.js 또는 인라인):**

```css
/* apps/desktop/src/index.css 또는 wts/styles.css */
@keyframes highlight-green {
  0% { background-color: rgba(34, 197, 94, 0.3); }
  100% { background-color: transparent; }
}

@keyframes highlight-red {
  0% { background-color: rgba(239, 68, 68, 0.3); }
  100% { background-color: transparent; }
}

.animate-highlight-green {
  animation: highlight-green 2000ms ease-out;
}

.animate-highlight-red {
  animation: highlight-red 2000ms ease-out;
}
```

### Project Structure Notes

**신규 파일:**
- `apps/desktop/src/wts/stores/balanceStore.ts`
- `apps/desktop/src/wts/__tests__/stores/balanceStore.test.ts`
- `apps/desktop/src/wts/__tests__/panels/BalancePanel.test.tsx`

**변경 파일:**
- `apps/desktop/src/wts/stores/index.ts` - balanceStore export 추가
- `apps/desktop/src/wts/panels/BalancePanel.tsx` - 전면 구현
- `apps/desktop/src/wts/utils/formatters.ts` - formatCrypto, formatKrw 추가
- `apps/desktop/src/index.css` (또는 wts 전용 CSS) - 하이라이트 애니메이션

### 기존 코드 패턴 참조

**consoleStore 패턴 (참조용):**
```typescript
// 기존 consoleStore.ts 패턴을 balanceStore에 적용
export const useConsoleStore = create<ConsoleState>()((set) => ({
  logs: [],
  addLog: (level, category, message, detail?) => set((state) => { ... }),
  clearLogs: () => set({ logs: [] }),
}));
```

**ExchangePanel 패턴 (참조용):**
```typescript
// 기존 ExchangePanel.tsx의 스토어 구독 패턴
const { selectedExchange, setExchange, connectionStatus } = useWtsStore();
const { addLog } = useConsoleStore();
```

### 테스트 가이드

**balanceStore 테스트:**
```typescript
// __tests__/stores/balanceStore.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useBalanceStore } from '../../stores/balanceStore';

describe('balanceStore', () => {
  beforeEach(() => {
    useBalanceStore.setState({
      balances: [],
      previousBalances: [],
      isLoading: false,
      lastUpdated: null,
      hideZeroBalances: false,
      error: null,
    });
  });

  it('setHideZeroBalances updates state', () => {
    const { setHideZeroBalances } = useBalanceStore.getState();
    setHideZeroBalances(true);
    expect(useBalanceStore.getState().hideZeroBalances).toBe(true);
  });

  // fetchBalance 테스트는 invoke mock 필요
});
```

**BalancePanel 테스트:**
```typescript
// __tests__/panels/BalancePanel.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BalancePanel } from '../../panels/BalancePanel';
import { useBalanceStore } from '../../stores/balanceStore';
import { useWtsStore } from '../../stores/wtsStore';

vi.mock('../../stores/balanceStore');
vi.mock('../../stores/wtsStore');

describe('BalancePanel', () => {
  beforeEach(() => {
    vi.mocked(useWtsStore).mockReturnValue({
      selectedExchange: 'upbit',
      connectionStatus: 'connected',
    } as any);
  });

  it('renders loading state', () => {
    vi.mocked(useBalanceStore).mockReturnValue({
      balances: [],
      isLoading: true,
      hideZeroBalances: false,
      fetchBalance: vi.fn(),
      setHideZeroBalances: vi.fn(),
    } as any);

    render(<BalancePanel />);
    expect(screen.getByTestId('balance-panel')).toBeInTheDocument();
  });

  it('renders balance entries', () => {
    vi.mocked(useBalanceStore).mockReturnValue({
      balances: [
        { currency: 'BTC', balance: '0.5', locked: '0', avg_buy_price: '50000000', avg_buy_price_modified: false, unit_currency: 'KRW' },
      ],
      previousBalances: [],
      isLoading: false,
      hideZeroBalances: false,
      fetchBalance: vi.fn(),
      setHideZeroBalances: vi.fn(),
    } as any);

    render(<BalancePanel />);
    expect(screen.getByText('BTC')).toBeInTheDocument();
    expect(screen.getByText('0.5')).toBeInTheDocument();
  });
});
```

### References

- [Architecture Document: Zustand Store Naming](_bmad-output/planning-artifacts/architecture.md#Naming Patterns)
- [Architecture Document: WTS Frontend Structure](_bmad-output/planning-artifacts/architecture.md#Project Structure)
- [UX Design: Balance Table Component](_bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy)
- [UX Design: Visual Feedback & Animation](_bmad-output/planning-artifacts/ux-design-specification.md#Visual Design Foundation)
- [Previous Story: WTS-2.1](_bmad-output/implementation-artifacts/wts-2-1-upbit-api-auth-balance-backend.md)
- [WTS Epics: Story 2.2](_bmad-output/planning-artifacts/wts-epics.md#Story 2.2)

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List

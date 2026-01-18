# Story WTS-1.4: 거래소 탭 및 선택 기능

Status: ready-for-dev

## Story

As a **트레이더**,
I want **상단 탭에서 거래소를 선택할 수 있는 기능**,
So that **원하는 거래소로 빠르게 전환할 수 있다**.

## Acceptance Criteria

1. **Given** WTS 창이 열렸을 때 **When** 거래소 탭을 클릭하면 **Then** 선택한 거래소가 활성화 상태로 표시되어야 한다
2. **Given** WTS 창이 열렸을 때 **When** 거래소 탭을 클릭하면 **Then** wtsStore의 selectedExchange 상태가 업데이트되어야 한다
3. **Given** WTS 창이 열렸을 때 **When** 탭을 확인하면 **Then** MVP에서는 Upbit 탭만 활성화되고 나머지는 비활성화(Coming Soon)되어야 한다
4. **Given** WTS 창이 열렸을 때 **When** 키보드 단축키 1-6을 입력하면 **Then** 해당 거래소로 전환이 가능해야 한다 (단, 비활성화된 거래소는 전환 불가)

## Tasks / Subtasks

- [ ] Task 1: ExchangePanel 거래소 탭 UI 구현 (AC: #1, #3)
  - [ ] Subtask 1.1: 6개 거래소 탭 버튼 렌더링 (Upbit, Bithumb, Binance, Coinbase, Bybit, GateIO)
  - [ ] Subtask 1.2: 활성화된 탭 스타일 (흰색 텍스트, 하단 보더 또는 배경 하이라이트)
  - [ ] Subtask 1.3: 비활성화된 탭 스타일 (opacity: 0.5, "Coming Soon" 툴팁)
  - [ ] Subtask 1.4: 탭 간 간격 및 레이아웃 조정 (UX 디자인 준수)

- [ ] Task 2: Zustand 스토어 연동 (AC: #2)
  - [ ] Subtask 2.1: Exchange 타입 확장 (6개 거래소 타입 정의)
  - [ ] Subtask 2.2: 탭 클릭 시 `setExchange()` 호출
  - [ ] Subtask 2.3: 활성화된 거래소 목록 상태 추가 (MVP: upbit만 활성)

- [ ] Task 3: 키보드 단축키 구현 (AC: #4)
  - [ ] Subtask 3.1: 전역 키보드 이벤트 리스너 (키 1-6)
  - [ ] Subtask 3.2: 활성화된 거래소만 전환 허용
  - [ ] Subtask 3.3: 비활성화된 거래소 전환 시도 시 무시 또는 콘솔 로그

- [ ] Task 4: 콘솔 로그 연동 (AC: #2)
  - [ ] Subtask 4.1: 거래소 전환 시 콘솔에 INFO 로그 기록
  - [ ] Subtask 4.2: 콘솔 로그 형식: `[INFO] 거래소 전환: {exchange}`

- [ ] Task 5: 테스트 작성 (AC: #1, #2, #3, #4)
  - [ ] Subtask 5.1: 탭 렌더링 테스트 (6개 탭 존재 확인)
  - [ ] Subtask 5.2: 탭 클릭 시 스토어 업데이트 테스트
  - [ ] Subtask 5.3: 활성/비활성 탭 스타일 테스트
  - [ ] Subtask 5.4: 키보드 단축키 동작 테스트

## Dev Notes

### Architecture 준수사항

**거래소 탭 패턴:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Navigation Patterns]

```
탭 바 (거래소 전환):
- Default: 회색 텍스트, 투명 배경
- Hover: 밝은 회색 배경
- Active: 흰색 텍스트, 하단 보더 또는 배경 하이라이트
- Disabled: opacity: 0.5 (연결 실패 시)
```

**키보드 단축키:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Navigation Patterns]

| 키 | 액션 |
|----|------|
| **1** | Upbit 선택 |
| **2** | Bithumb 선택 (Coming Soon) |
| **3** | Binance 선택 (Coming Soon) |
| **4** | Coinbase 선택 (Coming Soon) |
| **5** | Bybit 선택 (Coming Soon) |
| **6** | GateIO 선택 (Coming Soon) |

### 네이밍 규칙

**타입 정의:**
[Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]

```typescript
// apps/desktop/src/wts/types.ts - Exchange 타입 확장
export type Exchange = 'upbit' | 'bithumb' | 'binance' | 'coinbase' | 'bybit' | 'gateio';

// 활성화된 거래소 (MVP)
export const ENABLED_EXCHANGES: Exchange[] = ['upbit'];

// 거래소 표시 정보
export const EXCHANGE_INFO: Record<Exchange, { name: string; shortKey: string }> = {
  upbit: { name: 'Upbit', shortKey: 'UP' },
  bithumb: { name: 'Bithumb', shortKey: 'BT' },
  binance: { name: 'Binance', shortKey: 'BN' },
  coinbase: { name: 'Coinbase', shortKey: 'CB' },
  bybit: { name: 'Bybit', shortKey: 'BY' },
  gateio: { name: 'GateIO', shortKey: 'GT' },
};
```

**스토어 패턴:**
[Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]

```typescript
// apps/desktop/src/wts/stores/wtsStore.ts
export const useWtsStore = create<WtsState>()((set) => ({
  selectedExchange: 'upbit',
  setExchange: (exchange: Exchange) => set({ selectedExchange: exchange }),
  // ... existing code
}));
```

### UX 디자인 요구사항

**레이아웃:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design Direction Decision]

```
┌─────────────────────────────────────────────────────────────┐
│  WTS                    [UP][BN][HB][BT][...]     ● 연결됨  │
├───────────────┬─────────────────────┬───────────────────────┤
```

- 탭은 헤더 영역 중앙에 배치
- 각 탭은 짧은 약어 표시 (UP, BN, BT 등)
- 활성 탭: 밝은 배경 또는 하단 보더
- 연결 상태 인디케이터: 우측에 위치

**색상:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Color System]

| 상태 | 스타일 |
|------|--------|
| 기본 | `text-wts-muted` (회색) |
| 호버 | `bg-wts-tertiary` (밝은 배경) |
| 활성 | `text-wts-foreground` (흰색), `border-b-2 border-wts-accent` |
| 비활성 | `opacity-50`, `cursor-not-allowed` |

### 이전 스토리 (WTS-1.3)에서 학습한 사항

**완료된 작업:**
- ExchangePanel.tsx 기본 구조 생성 (플레이스홀더)
- wtsStore에 selectedExchange, setExchange 구현됨
- 연결 상태 표시기 구현됨
- 6패널 그리드 레이아웃 완성

**현재 ExchangePanel 상태:**
- 단순히 현재 거래소명과 연결 상태만 표시
- 탭 버튼 UI 없음 - 이번 스토리에서 추가 필요

**재사용 가능한 패턴:**
- `useWtsStore` 훅으로 거래소 선택 상태 접근
- `useConsoleStore` 훅으로 콘솔 로그 추가
- WTS CSS 변수 시스템 (`--wts-*`)

### Git 최근 커밋 패턴

**최근 작업:**
- `feat(wts): scaffold stores and tests` - WTS 스토어 및 테스트 스캐폴딩
- 커밋 메시지 형식: `feat(wts): 설명`

### 구현 가이드

**1. Exchange 타입 확장 (types.ts):**

```typescript
/** 지원 거래소 (전체) */
export type Exchange = 'upbit' | 'bithumb' | 'binance' | 'coinbase' | 'bybit' | 'gateio';

/** 활성화된 거래소 (MVP: Upbit만) */
export const ENABLED_EXCHANGES: readonly Exchange[] = ['upbit'] as const;

/** 거래소 메타데이터 */
export interface ExchangeMeta {
  name: string;
  shortKey: string;
  keyboardShortcut: number; // 1-6
}

export const EXCHANGE_META: Record<Exchange, ExchangeMeta> = {
  upbit: { name: 'Upbit', shortKey: 'UP', keyboardShortcut: 1 },
  bithumb: { name: 'Bithumb', shortKey: 'BT', keyboardShortcut: 2 },
  binance: { name: 'Binance', shortKey: 'BN', keyboardShortcut: 3 },
  coinbase: { name: 'Coinbase', shortKey: 'CB', keyboardShortcut: 4 },
  bybit: { name: 'Bybit', shortKey: 'BY', keyboardShortcut: 5 },
  gateio: { name: 'GateIO', shortKey: 'GT', keyboardShortcut: 6 },
};

/** 거래소 순서 (탭 표시 순서) */
export const EXCHANGE_ORDER: readonly Exchange[] = [
  'upbit', 'bithumb', 'binance', 'coinbase', 'bybit', 'gateio'
] as const;
```

**2. ExchangePanel 탭 UI:**

```typescript
// apps/desktop/src/wts/panels/ExchangePanel.tsx
import { useWtsStore, useConsoleStore } from '../stores';
import { Exchange, EXCHANGE_META, EXCHANGE_ORDER, ENABLED_EXCHANGES } from '../types';
import { useEffect, useCallback } from 'react';

export function ExchangePanel() {
  const { selectedExchange, setExchange, connectionStatus } = useWtsStore();
  const { addLog } = useConsoleStore();

  const isEnabled = (exchange: Exchange) => ENABLED_EXCHANGES.includes(exchange);

  const handleExchangeSelect = useCallback((exchange: Exchange) => {
    if (!isEnabled(exchange)) return;
    setExchange(exchange);
    addLog('INFO', 'SYSTEM', `거래소 전환: ${EXCHANGE_META[exchange].name}`);
  }, [setExchange, addLog]);

  // 키보드 단축키
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const key = parseInt(e.key);
      if (key >= 1 && key <= 6) {
        const exchange = EXCHANGE_ORDER[key - 1];
        if (isEnabled(exchange)) {
          handleExchangeSelect(exchange);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleExchangeSelect]);

  return (
    <div className="wts-area-header flex items-center justify-between px-4 bg-wts-secondary border-b border-wts">
      <h1 className="text-wts-foreground font-semibold text-base">WTS</h1>

      {/* 거래소 탭 */}
      <div className="flex items-center gap-1">
        {EXCHANGE_ORDER.map((exchange) => {
          const meta = EXCHANGE_META[exchange];
          const isActive = selectedExchange === exchange;
          const enabled = isEnabled(exchange);

          return (
            <button
              key={exchange}
              onClick={() => handleExchangeSelect(exchange)}
              disabled={!enabled}
              className={`
                px-3 py-1 text-sm font-medium transition-colors
                ${isActive
                  ? 'text-wts-foreground border-b-2 border-wts-accent'
                  : 'text-wts-muted hover:text-wts-foreground hover:bg-wts-tertiary'}
                ${!enabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
              `}
              title={enabled ? meta.name : `${meta.name} (Coming Soon)`}
            >
              {meta.shortKey}
            </button>
          );
        })}
      </div>

      {/* 연결 상태 */}
      <div className="flex items-center gap-2">
        <span className={`w-2 h-2 rounded-full ${
          connectionStatus === 'connected' ? 'bg-green-500' : 'bg-red-500'
        }`} />
        <span className="text-wts-muted text-sm">
          {connectionStatus === 'connected' ? '연결됨' : '연결 안됨'}
        </span>
      </div>
    </div>
  );
}
```

**3. 테스트 예시:**

```typescript
// apps/desktop/src/wts/__tests__/ExchangePanel.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { ExchangePanel } from '../panels/ExchangePanel';

describe('ExchangePanel', () => {
  it('should render all 6 exchange tabs', () => {
    render(<ExchangePanel />);
    expect(screen.getByText('UP')).toBeInTheDocument();
    expect(screen.getByText('BT')).toBeInTheDocument();
    expect(screen.getByText('BN')).toBeInTheDocument();
    expect(screen.getByText('CB')).toBeInTheDocument();
    expect(screen.getByText('BY')).toBeInTheDocument();
    expect(screen.getByText('GT')).toBeInTheDocument();
  });

  it('should have Upbit tab enabled and others disabled', () => {
    render(<ExchangePanel />);
    const upbitTab = screen.getByText('UP');
    const bithumbTab = screen.getByText('BT');

    expect(upbitTab).not.toBeDisabled();
    expect(bithumbTab).toBeDisabled();
  });

  it('should update selectedExchange when clicking enabled tab', () => {
    render(<ExchangePanel />);
    const upbitTab = screen.getByText('UP');
    fireEvent.click(upbitTab);
    // Assert store update via mock or integration test
  });

  it('should support keyboard shortcut 1 for Upbit', () => {
    render(<ExchangePanel />);
    fireEvent.keyDown(window, { key: '1' });
    // Assert store update
  });
});
```

### Project Structure Notes

**기존 파일:**
- `apps/desktop/src/wts/types.ts` - Exchange 타입 확장 (변경)
- `apps/desktop/src/wts/panels/ExchangePanel.tsx` - 탭 UI 추가 (변경)
- `apps/desktop/src/wts/stores/wtsStore.ts` - 필요 시 확장 (변경 가능)

**신규 파일:**
- `apps/desktop/src/wts/__tests__/ExchangePanel.test.tsx` - ExchangePanel 전용 테스트

**디렉토리 구조:**
```
apps/desktop/src/wts/
├── types.ts            # Exchange 타입 확장
├── panels/
│   └── ExchangePanel.tsx  # 거래소 탭 UI (변경)
├── stores/
│   └── wtsStore.ts     # 상태 관리 (필요 시)
└── __tests__/
    └── ExchangePanel.test.tsx (신규)
```

### 성능 고려사항

**키보드 이벤트 최적화:**
- `useCallback`으로 이벤트 핸들러 메모이제이션
- 컴포넌트 언마운트 시 이벤트 리스너 정리 필수

**렌더링 최적화:**
- 탭 상태 변경 시 전체 패널 리렌더링 방지
- Zustand selector로 필요한 상태만 구독

### References

- [Architecture Document: Naming Patterns](_bmad-output/planning-artifacts/architecture.md#Naming Patterns)
- [Architecture Document: Project Structure](_bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure)
- [UX Design: Navigation Patterns](_bmad-output/planning-artifacts/ux-design-specification.md#Navigation Patterns)
- [UX Design: Design Direction Decision](_bmad-output/planning-artifacts/ux-design-specification.md#Design Direction Decision)
- [UX Design: Color System](_bmad-output/planning-artifacts/ux-design-specification.md#Color System)
- [WTS Epics: Story 1.4](_bmad-output/planning-artifacts/wts-epics.md#Story 1.4)
- [Previous Story: WTS-1.3](_bmad-output/implementation-artifacts/wts-1-3-six-panel-grid-layout.md)

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List


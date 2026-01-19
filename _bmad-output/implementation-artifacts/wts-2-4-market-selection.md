# Story WTS-2.4: 마켓 선택 기능

Status: done

## Story

As a **트레이더**,
I want **거래할 마켓(BTC/KRW, ETH/KRW 등)을 선택하는 기능**,
So that **원하는 마켓의 호가와 주문을 관리할 수 있다**.

## Acceptance Criteria

1. **Given** WTS 창에서 거래소가 선택되어 있을 때 **When** 마켓 선택 드롭다운을 클릭하면 **Then** 사용 가능한 마켓 목록이 표시되어야 한다
2. **Given** 마켓 목록이 표시되어 있을 때 **When** 마켓을 선택하면 **Then** wtsStore.selectedMarket이 업데이트되어야 한다
3. **Given** 마켓이 선택되었을 때 **When** 선택이 완료되면 **Then** 선택된 마켓에 따라 오더북과 주문 패널이 업데이트 준비가 되어야 한다
4. **Given** 마켓이 변경되었을 때 **When** 변경이 완료되면 **Then** 콘솔에 마켓 변경 로그가 기록되어야 한다
5. **Given** 거래소가 변경되었을 때 **When** 새 거래소가 선택되면 **Then** 마켓 목록이 해당 거래소의 마켓으로 갱신되어야 한다
6. **Given** 마켓 검색 기능이 필요할 때 **When** 검색어를 입력하면 **Then** 마켓 목록이 필터링되어야 한다

## Tasks / Subtasks

- [x] Task 1: 마켓 타입 및 상수 정의 (AC: #1, #2)
  - [x] Subtask 1.1: types.ts에 Market 타입 정의 (code, base, quote)
  - [x] Subtask 1.2: Upbit 마켓 목록 상수 정의 (MVP: 주요 KRW 마켓)
  - [x] Subtask 1.3: Market 메타데이터 인터페이스 정의

- [x] Task 2: wtsStore 마켓 관련 확장 (AC: #2, #5)
  - [x] Subtask 2.1: availableMarkets 상태 추가
  - [x] Subtask 2.2: setMarket 액션 강화 (null 허용, 유효성 검사)
  - [x] Subtask 2.3: 거래소 변경 시 마켓 초기화 로직

- [x] Task 3: MarketSelector 컴포넌트 구현 (AC: #1, #6)
  - [x] Subtask 3.1: 컴포넌트 파일 생성 (apps/desktop/src/wts/components/MarketSelector.tsx)
  - [x] Subtask 3.2: 드롭다운 UI 구현 (shadcn/ui Select 또는 커스텀)
  - [x] Subtask 3.3: 검색 필터링 기능 구현
  - [x] Subtask 3.4: 선택된 마켓 하이라이트 표시
  - [x] Subtask 3.5: 키보드 탐색 지원 (↑↓, Enter)

- [x] Task 4: 오더북 패널에 MarketSelector 통합 (AC: #1, #3)
  - [x] Subtask 4.1: OrderbookPanel 헤더에 MarketSelector 배치
  - [x] Subtask 4.2: 선택된 마켓 표시 (현재 마켓 코드)
  - [x] Subtask 4.3: 마켓 미선택 시 안내 메시지 표시

- [x] Task 5: 콘솔 로깅 구현 (AC: #4)
  - [x] Subtask 5.1: 마켓 변경 시 INFO 로그 추가
  - [x] Subtask 5.2: 로그 포맷: "마켓 변경: {market_code}"

- [x] Task 6: 테스트 작성 (AC: #1-#6)
  - [x] Subtask 6.1: MarketSelector 컴포넌트 테스트
  - [x] Subtask 6.2: wtsStore 마켓 관련 액션 테스트
  - [x] Subtask 6.3: OrderbookPanel 통합 테스트

## Dev Notes

### 이전 스토리(WTS-2.3)에서 구현된 사항

**wtsStore (apps/desktop/src/wts/stores/wtsStore.ts):**
- `selectedMarket: string | null` - 이미 정의됨
- `setMarket: (market: string | null) => void` - 이미 정의됨
- 현재 selectedMarket은 null 기본값

**OrderbookPanel (apps/desktop/src/wts/panels/OrderbookPanel.tsx):**
- 현재 플레이스홀더 상태
- "오더북이 여기에 표시됩니다 (Epic 2에서 구현)" 메시지만 표시
- wts-panel 스타일 적용됨

**types.ts:**
- Exchange, ConnectionStatus 등 기본 타입 정의됨
- Market 타입은 아직 미정의

### Architecture 준수사항

**마켓 코드 형식:**
[Source: _bmad-output/planning-artifacts/architecture.md#Upbit API]

```typescript
// Upbit 마켓 코드 형식: {quote}-{base}
// 예: "KRW-BTC", "KRW-ETH", "BTC-ETH"
type MarketCode = `${string}-${string}`;
```

**컴포넌트 구조:**
[Source: _bmad-output/planning-artifacts/architecture.md#WTS Frontend Structure]

```
apps/desktop/src/wts/
├── components/
│   ├── MarketSelector.tsx  # 이 스토리에서 신규 생성
│   └── ...
```

**Zustand Store 패턴:**
[Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]

```typescript
// wtsStore.ts 확장 패턴
export const useWtsStore = create<WtsState>()((set, get) => ({
  // ... 기존 상태
  availableMarkets: UPBIT_DEFAULT_MARKETS,
  setMarket: (market) => {
    set({ selectedMarket: market });
    // 콘솔 로그
  },
}));
```

### UX 요구사항

**마켓 선택 UI:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]

| 항목 | 설명 |
|------|------|
| 위치 | OrderbookPanel 헤더 영역 |
| 형태 | 드롭다운 Select (검색 가능) |
| 표시 | 마켓 코드 (예: KRW-BTC) |
| 검색 | 마켓 코드 또는 자산명으로 필터링 |

**인터랙션 패턴:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#UX Consistency Patterns]

- 드롭다운 클릭 → 마켓 목록 표시
- 검색어 입력 → 실시간 필터링
- 마켓 선택 → 드롭다운 닫힘 + 마켓 적용
- ESC → 드롭다운 닫힘 (선택 취소)

**키보드 접근성:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Accessibility Strategy]

| 키 | 동작 |
|----|------|
| Enter/Space | 드롭다운 열기/선택 |
| ↑↓ | 목록 탐색 |
| ESC | 드롭다운 닫기 |
| 문자 입력 | 검색 필터 |

### 구현 가이드

**1. Market 타입 정의 (types.ts):**

```typescript
/** 마켓 코드 (예: "KRW-BTC") */
export type MarketCode = string;

/** 마켓 정보 */
export interface Market {
  /** 마켓 코드 (예: "KRW-BTC") */
  code: MarketCode;
  /** 기준 화폐 (예: "BTC") */
  base: string;
  /** 결제 화폐 (예: "KRW") */
  quote: string;
  /** 표시명 (예: "비트코인") */
  displayName?: string;
}

/** Upbit MVP 마켓 목록 */
export const UPBIT_DEFAULT_MARKETS: readonly Market[] = [
  { code: 'KRW-BTC', base: 'BTC', quote: 'KRW', displayName: '비트코인' },
  { code: 'KRW-ETH', base: 'ETH', quote: 'KRW', displayName: '이더리움' },
  { code: 'KRW-XRP', base: 'XRP', quote: 'KRW', displayName: '리플' },
  { code: 'KRW-SOL', base: 'SOL', quote: 'KRW', displayName: '솔라나' },
  { code: 'KRW-DOGE', base: 'DOGE', quote: 'KRW', displayName: '도지코인' },
  { code: 'KRW-ADA', base: 'ADA', quote: 'KRW', displayName: '에이다' },
  { code: 'KRW-AVAX', base: 'AVAX', quote: 'KRW', displayName: '아발란체' },
  { code: 'KRW-DOT', base: 'DOT', quote: 'KRW', displayName: '폴카닷' },
] as const;
```

**2. wtsStore 확장:**

```typescript
// 상태 추가
interface WtsState {
  // ... 기존
  availableMarkets: readonly Market[];
  setAvailableMarkets: (markets: readonly Market[]) => void;
}

// setMarket 강화 - 콘솔 로깅 추가
setMarket: (market: string | null) => {
  const prevMarket = get().selectedMarket;
  set({ selectedMarket: market });

  if (market && market !== prevMarket) {
    useConsoleStore.getState().addLog(
      'INFO',
      'SYSTEM',
      `마켓 변경: ${market}`
    );
  }
},
```

**3. MarketSelector 컴포넌트:**

```typescript
// apps/desktop/src/wts/components/MarketSelector.tsx
interface MarketSelectorProps {
  markets: readonly Market[];
  selectedMarket: string | null;
  onSelect: (marketCode: string) => void;
  disabled?: boolean;
}

export function MarketSelector({
  markets,
  selectedMarket,
  onSelect,
  disabled = false,
}: MarketSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // 검색 필터링
  const filteredMarkets = useMemo(() => {
    if (!searchQuery) return markets;
    const query = searchQuery.toLowerCase();
    return markets.filter(
      (m) =>
        m.code.toLowerCase().includes(query) ||
        m.base.toLowerCase().includes(query) ||
        m.displayName?.toLowerCase().includes(query)
    );
  }, [markets, searchQuery]);

  // 키보드 탐색
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') setIsOpen(false);
    // ↑↓ 탐색 로직
  };

  return (
    <div className="relative">
      <button
        onClick={() => !disabled && setIsOpen(!isOpen)}
        disabled={disabled}
        className="..."
      >
        {selectedMarket || '마켓 선택'}
      </button>

      {isOpen && (
        <div className="absolute z-10 ...">
          <input
            type="text"
            placeholder="마켓 검색..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="..."
          />
          <ul>
            {filteredMarkets.map((market) => (
              <li
                key={market.code}
                onClick={() => {
                  onSelect(market.code);
                  setIsOpen(false);
                }}
                className={`... ${selectedMarket === market.code ? 'bg-wts-tertiary' : ''}`}
              >
                <span className="font-mono">{market.code}</span>
                {market.displayName && (
                  <span className="text-wts-muted ml-2">{market.displayName}</span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
```

**4. OrderbookPanel 통합:**

```typescript
// apps/desktop/src/wts/panels/OrderbookPanel.tsx
import { MarketSelector } from '../components/MarketSelector';
import { useWtsStore } from '../stores/wtsStore';
import { UPBIT_DEFAULT_MARKETS } from '../types';

export function OrderbookPanel({ className = '' }: OrderbookPanelProps) {
  const { selectedMarket, setMarket, connectionStatus } = useWtsStore();

  return (
    <div className={`wts-area-orderbook wts-panel flex flex-col ${className}`}>
      <div className="wts-panel-header flex justify-between items-center">
        <span>Orderbook</span>
        <MarketSelector
          markets={UPBIT_DEFAULT_MARKETS}
          selectedMarket={selectedMarket}
          onSelect={setMarket}
          disabled={connectionStatus !== 'connected'}
        />
      </div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        {!selectedMarket ? (
          <p className="text-wts-muted text-xs text-center py-4">
            마켓을 선택하세요
          </p>
        ) : (
          <p className="text-wts-muted text-xs">
            오더북 데이터 (Story 2.5에서 구현)
          </p>
        )}
      </div>
    </div>
  );
}
```

### Project Structure Notes

**신규 파일:**
- `apps/desktop/src/wts/components/MarketSelector.tsx`
- `apps/desktop/src/wts/__tests__/components/MarketSelector.test.tsx`

**수정 파일:**
- `apps/desktop/src/wts/types.ts` - Market 타입 추가
- `apps/desktop/src/wts/stores/wtsStore.ts` - availableMarkets, 로깅 추가
- `apps/desktop/src/wts/panels/OrderbookPanel.tsx` - MarketSelector 통합
- `apps/desktop/src/wts/__tests__/stores/wtsStore.test.ts` - 마켓 관련 테스트
- `apps/desktop/src/wts/__tests__/panels/OrderbookPanel.test.tsx` - 통합 테스트

### 기존 코드 패턴 참조

**컴포넌트 패턴 (BalancePanel 참조):**

```typescript
// 헤더 + 컨텐츠 구조
<div className="wts-panel-header flex justify-between items-center">
  <span>Title</span>
  <div className="flex items-center gap-2">
    {/* 액션 버튼/선택기 */}
  </div>
</div>
```

**스토어 패턴 (balanceStore 참조):**

```typescript
// consoleStore 연동 패턴
import { useConsoleStore } from './consoleStore';

// 액션 내에서 로깅
useConsoleStore.getState().addLog('INFO', 'SYSTEM', '메시지');
```

**테스트 패턴 (BalancePanel.test.tsx 참조):**

```typescript
// 컴포넌트 테스트 패턴
vi.mock('../stores/wtsStore', () => ({
  useWtsStore: vi.fn(),
}));

describe('MarketSelector', () => {
  it('renders market dropdown', () => {
    render(<MarketSelector ... />);
    expect(screen.getByRole('button')).toBeInTheDocument();
  });
});
```

### 테스트 가이드

**MarketSelector 컴포넌트 테스트:**

```typescript
describe('MarketSelector', () => {
  const mockMarkets = [
    { code: 'KRW-BTC', base: 'BTC', quote: 'KRW', displayName: '비트코인' },
    { code: 'KRW-ETH', base: 'ETH', quote: 'KRW', displayName: '이더리움' },
  ];

  it('renders trigger button with selected market', () => {
    render(
      <MarketSelector
        markets={mockMarkets}
        selectedMarket="KRW-BTC"
        onSelect={vi.fn()}
      />
    );
    expect(screen.getByText('KRW-BTC')).toBeInTheDocument();
  });

  it('opens dropdown on click', async () => {
    render(
      <MarketSelector
        markets={mockMarkets}
        selectedMarket={null}
        onSelect={vi.fn()}
      />
    );
    await userEvent.click(screen.getByRole('button'));
    expect(screen.getByPlaceholderText('마켓 검색...')).toBeInTheDocument();
  });

  it('filters markets by search query', async () => {
    render(
      <MarketSelector
        markets={mockMarkets}
        selectedMarket={null}
        onSelect={vi.fn()}
      />
    );
    await userEvent.click(screen.getByRole('button'));
    await userEvent.type(screen.getByPlaceholderText('마켓 검색...'), 'BTC');
    expect(screen.getByText('KRW-BTC')).toBeInTheDocument();
    expect(screen.queryByText('KRW-ETH')).not.toBeInTheDocument();
  });

  it('calls onSelect when market is clicked', async () => {
    const onSelect = vi.fn();
    render(
      <MarketSelector
        markets={mockMarkets}
        selectedMarket={null}
        onSelect={onSelect}
      />
    );
    await userEvent.click(screen.getByRole('button'));
    await userEvent.click(screen.getByText('KRW-BTC'));
    expect(onSelect).toHaveBeenCalledWith('KRW-BTC');
  });

  it('closes dropdown on ESC key', async () => {
    render(
      <MarketSelector
        markets={mockMarkets}
        selectedMarket={null}
        onSelect={vi.fn()}
      />
    );
    await userEvent.click(screen.getByRole('button'));
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByPlaceholderText('마켓 검색...')).not.toBeInTheDocument();
  });

  it('disables button when disabled prop is true', () => {
    render(
      <MarketSelector
        markets={mockMarkets}
        selectedMarket={null}
        onSelect={vi.fn()}
        disabled
      />
    );
    expect(screen.getByRole('button')).toBeDisabled();
  });
});
```

**wtsStore 마켓 테스트:**

```typescript
describe('wtsStore market actions', () => {
  beforeEach(() => {
    useWtsStore.setState({
      selectedMarket: null,
      availableMarkets: UPBIT_DEFAULT_MARKETS,
    });
  });

  it('setMarket updates selectedMarket', () => {
    useWtsStore.getState().setMarket('KRW-BTC');
    expect(useWtsStore.getState().selectedMarket).toBe('KRW-BTC');
  });

  it('setMarket logs to console on change', () => {
    const addLog = vi.spyOn(useConsoleStore.getState(), 'addLog');
    useWtsStore.getState().setMarket('KRW-BTC');
    expect(addLog).toHaveBeenCalledWith('INFO', 'SYSTEM', '마켓 변경: KRW-BTC');
  });

  it('setMarket allows null value', () => {
    useWtsStore.getState().setMarket('KRW-BTC');
    useWtsStore.getState().setMarket(null);
    expect(useWtsStore.getState().selectedMarket).toBeNull();
  });
});
```

### References

- [Architecture Document: WTS Frontend Structure](_bmad-output/planning-artifacts/architecture.md#Project Structure)
- [UX Design: Component Strategy](_bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy)
- [Previous Story: WTS-2.3](_bmad-output/implementation-artifacts/wts-2-3-balance-manual-auto-refresh.md)
- [WTS Epics: Story 2.4](_bmad-output/planning-artifacts/wts-epics.md#Story 2.4)
- [Upbit API: Market Codes](_bmad-output/planning-artifacts/architecture.md#Upbit API)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- 모든 테스트 통과 (222/223, 기존 useConnectionCheck 타임아웃 1개 제외)

### Completion Notes List

- Task 1: Market, MarketCode 타입 및 UPBIT_DEFAULT_MARKETS 상수 정의 완료
- Task 2: wtsStore에 availableMarkets 상태, setAvailableMarkets 액션 추가, setMarket에 콘솔 로깅 통합, setExchange에서 마켓 초기화 로직 추가
- Task 3: MarketSelector 컴포넌트 구현 (드롭다운 UI, 검색 필터링, 키보드 탐색, 외부 클릭 감지)
- Task 4: OrderbookPanel에 MarketSelector 통합, 마켓 미선택 시 안내 메시지 표시
- Task 5: wtsStore.setMarket에서 마켓 변경 시 INFO 로그 기록 (Task 2에서 구현)
- Task 6: 각 Task에서 RED-GREEN-REFACTOR 사이클로 테스트 작성 완료

### Review Fixes (2026-01-19)

- setMarket 유효성 검사 추가 및 잘못된 마켓 입력 무시
- 거래소 변경 시 availableMarkets 갱신 로직 추가
- MarketCode 타입 형식 제약 적용
- wtsStore 테스트에 유효성/마켓 목록 갱신 케이스 추가

### File List

**신규 파일:**
- apps/desktop/src/wts/components/MarketSelector.tsx
- apps/desktop/src/wts/__tests__/components/MarketSelector.test.tsx
- apps/desktop/src/wts/__tests__/panels/OrderbookPanel.test.tsx
- apps/desktop/src/wts/__tests__/types.test.ts

**수정 파일:**
- _bmad-output/implementation-artifacts/wts-2-4-market-selection.md - 상태/리뷰 수정 기록 업데이트
- _bmad-output/implementation-artifacts/sprint-status.yaml - 스토리 상태 동기화
- apps/desktop/src/wts/types.ts - MarketCode 형식 제약, UPBIT_DEFAULT_MARKETS 상수 추가, WtsState에 availableMarkets 추가
- apps/desktop/src/wts/stores/wtsStore.ts - 마켓 유효성 검사, availableMarkets 갱신, setExchange 마켓 초기화
- apps/desktop/src/wts/panels/OrderbookPanel.tsx - MarketSelector 통합, 마켓 미선택 안내 메시지
- apps/desktop/src/wts/__tests__/stores/wtsStore.test.ts - 마켓 유효성/목록 갱신 테스트 추가

## Change Log

- 2026-01-19: Story WTS-2.4 마켓 선택 기능 구현 완료
- 2026-01-19: 코드 리뷰 수정 (마켓 유효성/목록 갱신)

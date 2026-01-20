# Story WTS-2.5: 실시간 오더북 WebSocket 연동

Status: done

## Story

As a **트레이더**,
I want **선택한 마켓의 실시간 호가창을 볼 수 있는 기능**,
So that **시장 상황을 실시간으로 파악할 수 있다**.

## Acceptance Criteria

1. **Given** 마켓이 선택되어 있을 때 **When** 오더북 WebSocket 연결이 설정되면 **Then** 매수/매도 호가 각 15단계가 실시간 표시되어야 한다
2. **Given** WebSocket 연결이 활성화되어 있을 때 **When** 오더북 데이터가 수신되면 **Then** 오더북 갱신은 WebSocket 수신 후 100ms 이내에 UI에 반영되어야 한다
3. **Given** WebSocket 연결 중일 때 **When** 연결이 끊기면 **Then** 자동 재연결이 시도되어야 한다
4. **Given** WebSocket 연결 상태가 변경될 때 **When** 연결/끊김/재연결 이벤트가 발생하면 **Then** 연결 상태가 콘솔에 로그로 기록되어야 한다
5. **Given** 마켓이 변경되었을 때 **When** 새 마켓이 선택되면 **Then** 기존 구독을 해제하고 새 마켓을 구독해야 한다
6. **Given** 거래소가 변경되었을 때 **When** 새 거래소가 선택되면 **Then** WebSocket 연결을 닫고 새 거래소에 재연결해야 한다

## Tasks / Subtasks

- [x] Task 1: 오더북 데이터 타입 정의 (AC: #1)
  - [x] Subtask 1.1: types.ts에 OrderbookEntry 타입 정의 (price, size)
  - [x] Subtask 1.2: types.ts에 Orderbook 타입 정의 (asks, bids, timestamp)
  - [x] Subtask 1.3: Upbit WebSocket 응답 타입 정의 (OrderbookUnit)

- [x] Task 2: orderbookStore 구현 (AC: #1, #2)
  - [x] Subtask 2.1: stores/orderbookStore.ts 파일 생성
  - [x] Subtask 2.2: orderbook 상태 (asks, bids, timestamp) 정의
  - [x] Subtask 2.3: setOrderbook 액션 구현
  - [x] Subtask 2.4: clearOrderbook 액션 구현
  - [x] Subtask 2.5: 연결 상태 (wsStatus) 관리

- [x] Task 3: useUpbitOrderbookWs 커스텀 훅 구현 (AC: #1, #2, #3, #4, #5, #6)
  - [x] Subtask 3.1: hooks/useUpbitOrderbookWs.ts 파일 생성
  - [x] Subtask 3.2: WebSocket 연결 로직 구현 (wss://api.upbit.com/websocket/v1)
  - [x] Subtask 3.3: 구독 메시지 형식 구현 ([{"ticket":"..."}, {"type":"orderbook", "codes":["..."]}])
  - [x] Subtask 3.4: 메시지 파싱 및 orderbookStore 업데이트
  - [x] Subtask 3.5: 자동 재연결 로직 (exponential backoff: 1초→2초→4초→8초→최대16초)
  - [x] Subtask 3.6: 마켓 변경 시 재구독 로직
  - [x] Subtask 3.7: 연결 상태 콘솔 로깅 (연결됨/끊김/재연결 시도)
  - [x] Subtask 3.8: cleanup (언마운트 시 연결 종료)

- [x] Task 4: OrderbookPanel UI 구현 (AC: #1, #2)
  - [x] Subtask 4.1: 오더북 데이터 렌더링 (매도호가 위, 매수호가 아래)
  - [x] Subtask 4.2: 가격, 수량 컬럼 표시 (고정 소수점 포맷)
  - [x] Subtask 4.3: depth bar 표시 (수량 비율에 따른 막대)
  - [x] Subtask 4.4: 매수(녹색)/매도(빨강) 색상 구분
  - [x] Subtask 4.5: 로딩/에러 상태 표시
  - [x] Subtask 4.6: 연결 상태 인디케이터 표시

- [x] Task 5: 성능 최적화 (AC: #2)
  - [x] Subtask 5.1: React.memo로 불필요한 리렌더링 방지
  - [x] Subtask 5.2: useMemo로 depth bar 계산 최적화
  - [x] Subtask 5.3: 스로틀링 적용 (16ms, 60fps 타겟)

- [x] Task 6: 테스트 작성 (AC: #1-#6)
  - [x] Subtask 6.1: orderbookStore 단위 테스트
  - [x] Subtask 6.2: useUpbitOrderbookWs 훅 테스트 (WebSocket 모킹)
  - [x] Subtask 6.3: OrderbookPanel 통합 테스트

## Dev Notes

### 이전 스토리(WTS-2.4)에서 구현된 사항

**wtsStore:**
- `selectedMarket: string | null` - 마켓 선택 상태
- `setMarket(market: string | null)` - 마켓 변경 (유효성 검사 + 콘솔 로깅)
- `availableMarkets: readonly Market[]` - 사용 가능한 마켓 목록
- `connectionStatus: ConnectionStatus` - 연결 상태

**OrderbookPanel:**
- MarketSelector 통합 완료
- 현재 "오더북 데이터 (Story 2.5에서 구현)" 플레이스홀더 상태

**types.ts:**
- `Market`, `MarketCode` 타입 정의됨
- `UPBIT_DEFAULT_MARKETS` 상수 정의됨

### Upbit WebSocket API 명세

[Source: architecture.md#Upbit WebSocket API]

**엔드포인트:** `wss://api.upbit.com/websocket/v1`

**Rate Limit:**
- WebSocket Connect: 5회/초 (IP/계정)
- WebSocket Message: 5회/초, 100회/분 (IP/계정)

**오더북 구독 메시지 형식:**
```json
[
  {"ticket": "unique-ticket-id"},
  {"type": "orderbook", "codes": ["KRW-BTC"], "isOnlyRealtime": true}
]
```

**오더북 응답 형식:**
```json
{
  "type": "orderbook",
  "code": "KRW-BTC",
  "timestamp": 1704067200000,
  "total_ask_size": 10.12345678,
  "total_bid_size": 8.87654321,
  "orderbook_units": [
    {
      "ask_price": 50100000,
      "bid_price": 50000000,
      "ask_size": 0.5,
      "bid_size": 0.8
    }
    // ... 최대 15개 단계
  ]
}
```

**주의사항:**
- 인증 불필요 (공개 데이터)
- 바이너리(MessagePack) 또는 JSON 형식 (isOnlyRealtime: true면 JSON)
- orderbook_units는 가격 순서로 정렬됨 (ask: 오름차순, bid: 내림차순)

### Architecture 준수사항

**파일 구조:**
[Source: architecture.md#WTS Frontend Structure]

```
apps/desktop/src/wts/
├── stores/
│   └── orderbookStore.ts  # 신규 생성
├── hooks/
│   └── useUpbitOrderbookWs.ts  # 신규 생성
├── panels/
│   └── OrderbookPanel.tsx  # 수정
└── types.ts  # 오더북 타입 추가
```

**Zustand Store 패턴:**
```typescript
// orderbookStore.ts
interface OrderbookState {
  asks: OrderbookEntry[];  // 매도 호가 (가격 오름차순)
  bids: OrderbookEntry[];  // 매수 호가 (가격 내림차순)
  timestamp: number | null;
  wsStatus: 'connecting' | 'connected' | 'disconnected';
  setOrderbook: (asks: OrderbookEntry[], bids: OrderbookEntry[], timestamp: number) => void;
  clearOrderbook: () => void;
  setWsStatus: (status: 'connecting' | 'connected' | 'disconnected') => void;
}
```

**콘솔 로깅 패턴:**
[Source: architecture.md#Console Log Format]

```typescript
// 연결 성공
useConsoleStore.getState().addLog('INFO', 'SYSTEM', 'Upbit 오더북 WebSocket 연결됨');

// 연결 끊김
useConsoleStore.getState().addLog('WARN', 'SYSTEM', 'Upbit 오더북 WebSocket 연결 끊김');

// 재연결 시도
useConsoleStore.getState().addLog('INFO', 'SYSTEM', 'Upbit 오더북 WebSocket 재연결 시도 중...');
```

### UX 요구사항

**오더북 레이아웃:**
[Source: ux-design-specification.md#Orderbook Component]

```
┌─────────────────────────────┐
│ ASK (매도 호가) - 빨강       │
│ [depth bar] 가격       수량  │
│ ████████   50,100,000  0.500 │
│ ██████     50,050,000  1.200 │
│ ...                          │
│ ──────────────────────────── │
│ BID (매수 호가) - 녹색       │
│ ██████     50,000,000  0.800 │
│ ████████   49,950,000  2.100 │
│ ...                          │
└─────────────────────────────┘
```

**색상 시스템:**
- 매도(ASK): `text-destructive` (#ef4444)
- 매수(BID): `text-success` (#22c55e)
- 배경 depth bar:
  - 매도: `bg-destructive/20`
  - 매수: `bg-success/20`

**숫자 포맷:**
- 가격: 천 단위 콤마, 정수 (KRW)
- 수량: 소수점 8자리, 후행 0 제거

**성능 목표:**
- WebSocket 메시지 수신 → UI 갱신: < 100ms
- 스로틀링: 16ms (60fps)

### 구현 가이드

**1. OrderbookEntry 타입 정의 (types.ts):**

```typescript
/** 오더북 호가 엔트리 */
export interface OrderbookEntry {
  /** 가격 */
  price: number;
  /** 수량 */
  size: number;
}

/** 오더북 상태 */
export interface OrderbookData {
  /** 매도 호가 (가격 오름차순) */
  asks: OrderbookEntry[];
  /** 매수 호가 (가격 내림차순) */
  bids: OrderbookEntry[];
  /** 타임스탬프 (ms) */
  timestamp: number | null;
}

/** Upbit 오더북 WebSocket 응답 */
export interface UpbitOrderbookResponse {
  type: 'orderbook';
  code: string;
  timestamp: number;
  total_ask_size: number;
  total_bid_size: number;
  orderbook_units: UpbitOrderbookUnit[];
}

export interface UpbitOrderbookUnit {
  ask_price: number;
  bid_price: number;
  ask_size: number;
  bid_size: number;
}
```

**2. orderbookStore 구현:**

```typescript
// stores/orderbookStore.ts
import { create } from 'zustand';
import type { OrderbookEntry } from '../types';

type WsStatus = 'connecting' | 'connected' | 'disconnected';

interface OrderbookState {
  asks: OrderbookEntry[];
  bids: OrderbookEntry[];
  timestamp: number | null;
  wsStatus: WsStatus;
  setOrderbook: (asks: OrderbookEntry[], bids: OrderbookEntry[], timestamp: number) => void;
  clearOrderbook: () => void;
  setWsStatus: (status: WsStatus) => void;
}

export const useOrderbookStore = create<OrderbookState>()((set) => ({
  asks: [],
  bids: [],
  timestamp: null,
  wsStatus: 'disconnected',

  setOrderbook: (asks, bids, timestamp) => set({ asks, bids, timestamp }),
  clearOrderbook: () => set({ asks: [], bids: [], timestamp: null }),
  setWsStatus: (wsStatus) => set({ wsStatus }),
}));
```

**3. useUpbitOrderbookWs 훅 구현:**

```typescript
// hooks/useUpbitOrderbookWs.ts
import { useEffect, useRef, useCallback } from 'react';
import { useOrderbookStore } from '../stores/orderbookStore';
import { useConsoleStore } from '../stores/consoleStore';
import type { UpbitOrderbookResponse, OrderbookEntry } from '../types';

const UPBIT_WS_URL = 'wss://api.upbit.com/websocket/v1';
const RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000]; // exponential backoff

export function useUpbitOrderbookWs(marketCode: string | null) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptRef = useRef(0);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const { setOrderbook, clearOrderbook, setWsStatus } = useOrderbookStore();
  const addLog = useConsoleStore((state) => state.addLog);

  const connect = useCallback(() => {
    if (!marketCode) return;

    // 기존 연결 정리
    if (wsRef.current) {
      wsRef.current.close();
    }

    setWsStatus('connecting');
    const ws = new WebSocket(UPBIT_WS_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      setWsStatus('connected');
      reconnectAttemptRef.current = 0;
      addLog('INFO', 'SYSTEM', `Upbit 오더북 WebSocket 연결됨: ${marketCode}`);

      // 구독 메시지 전송
      const subscribeMsg = [
        { ticket: `orderbook-${Date.now()}` },
        { type: 'orderbook', codes: [marketCode], isOnlyRealtime: true }
      ];
      ws.send(JSON.stringify(subscribeMsg));
    };

    ws.onmessage = (event) => {
      try {
        const data: UpbitOrderbookResponse = JSON.parse(event.data);
        if (data.type !== 'orderbook') return;

        const asks: OrderbookEntry[] = data.orderbook_units.map((u) => ({
          price: u.ask_price,
          size: u.ask_size,
        }));
        const bids: OrderbookEntry[] = data.orderbook_units.map((u) => ({
          price: u.bid_price,
          size: u.bid_size,
        }));

        setOrderbook(asks, bids, data.timestamp);
      } catch (err) {
        console.error('Orderbook parse error:', err);
      }
    };

    ws.onclose = () => {
      setWsStatus('disconnected');
      clearOrderbook();
      addLog('WARN', 'SYSTEM', 'Upbit 오더북 WebSocket 연결 끊김');

      // 자동 재연결
      const delay = RECONNECT_DELAYS[Math.min(reconnectAttemptRef.current, RECONNECT_DELAYS.length - 1)];
      reconnectAttemptRef.current += 1;
      addLog('INFO', 'SYSTEM', `Upbit 오더북 WebSocket 재연결 시도 (${delay/1000}초 후)`);

      reconnectTimeoutRef.current = setTimeout(connect, delay);
    };

    ws.onerror = () => {
      addLog('ERROR', 'SYSTEM', 'Upbit 오더북 WebSocket 에러 발생');
    };
  }, [marketCode, setOrderbook, clearOrderbook, setWsStatus, addLog]);

  useEffect(() => {
    connect();

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
      clearOrderbook();
    };
  }, [connect, clearOrderbook]);
}
```

**4. OrderbookPanel UI 구현:**

```typescript
// panels/OrderbookPanel.tsx
import { useMemo, memo } from 'react';
import { MarketSelector } from '../components/MarketSelector';
import { useWtsStore } from '../stores/wtsStore';
import { useOrderbookStore } from '../stores/orderbookStore';
import { useUpbitOrderbookWs } from '../hooks/useUpbitOrderbookWs';
import type { OrderbookEntry } from '../types';

// 숫자 포맷 유틸리티
function formatPrice(price: number): string {
  return price.toLocaleString('ko-KR');
}

function formatSize(size: number): string {
  return size.toFixed(8).replace(/\.?0+$/, '');
}

// 개별 호가 행 (메모이제이션)
const OrderbookRow = memo(function OrderbookRow({
  entry,
  side,
  maxSize,
}: {
  entry: OrderbookEntry;
  side: 'ask' | 'bid';
  maxSize: number;
}) {
  const depthWidth = (entry.size / maxSize) * 100;
  const colorClass = side === 'ask' ? 'text-destructive' : 'text-success';
  const bgClass = side === 'ask' ? 'bg-destructive/20' : 'bg-success/20';

  return (
    <div className="relative flex items-center h-6 px-2 text-xs font-mono">
      <div
        className={`absolute inset-y-0 ${side === 'ask' ? 'right-0' : 'left-0'} ${bgClass}`}
        style={{ width: `${depthWidth}%` }}
      />
      <span className={`relative flex-1 text-left ${colorClass}`}>
        {formatPrice(entry.price)}
      </span>
      <span className="relative text-right text-wts-foreground">
        {formatSize(entry.size)}
      </span>
    </div>
  );
});

export function OrderbookPanel({ className = '' }: { className?: string }) {
  const { selectedMarket, setMarket, connectionStatus, availableMarkets } = useWtsStore();
  const { asks, bids, wsStatus } = useOrderbookStore();

  // WebSocket 연결 훅
  useUpbitOrderbookWs(selectedMarket);

  // depth bar 계산용 최대 수량
  const maxSize = useMemo(() => {
    const allSizes = [...asks.map((a) => a.size), ...bids.map((b) => b.size)];
    return Math.max(...allSizes, 0.00000001);
  }, [asks, bids]);

  const isDisabled = connectionStatus !== 'connected';
  const isLoading = wsStatus === 'connecting';
  const hasData = asks.length > 0 && bids.length > 0;

  return (
    <div data-testid="orderbook-panel" className={`wts-area-orderbook wts-panel flex flex-col ${className}`}>
      <div className="wts-panel-header flex justify-between items-center">
        <div className="flex items-center gap-2">
          <span>Orderbook</span>
          {/* WebSocket 상태 인디케이터 */}
          <span className={`w-2 h-2 rounded-full ${
            wsStatus === 'connected' ? 'bg-success' :
            wsStatus === 'connecting' ? 'bg-warning animate-pulse' :
            'bg-destructive'
          }`} />
        </div>
        <MarketSelector
          markets={availableMarkets}
          selectedMarket={selectedMarket}
          onSelect={setMarket}
          disabled={isDisabled}
        />
      </div>

      <div className="wts-panel-content flex-1 overflow-y-auto">
        {!selectedMarket ? (
          <p className="text-wts-muted text-xs text-center py-4">마켓을 선택하세요</p>
        ) : isLoading ? (
          <p className="text-wts-muted text-xs text-center py-4">연결 중...</p>
        ) : !hasData ? (
          <p className="text-wts-muted text-xs text-center py-4">데이터 대기 중...</p>
        ) : (
          <div className="flex flex-col">
            {/* 매도 호가 (역순: 높은 가격이 아래) */}
            <div className="flex flex-col-reverse">
              {asks.slice(0, 15).map((entry) => (
                <OrderbookRow key={`ask-${entry.price}`} entry={entry} side="ask" maxSize={maxSize} />
              ))}
            </div>

            {/* 중앙 구분선 */}
            <div className="h-px bg-wts-border my-1" />

            {/* 매수 호가 */}
            <div className="flex flex-col">
              {bids.slice(0, 15).map((entry) => (
                <OrderbookRow key={`bid-${entry.price}`} entry={entry} side="bid" maxSize={maxSize} />
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
```

### 기존 코드 패턴 참조

**balanceStore 패턴 (스토어 구조):**
[Source: apps/desktop/src/wts/stores/balanceStore.ts]

```typescript
// 상태 + 액션 패턴
interface Store {
  data: T;
  setData: (data: T) => void;
  clear: () => void;
}
```

**consoleStore 로깅 패턴:**
[Source: apps/desktop/src/wts/stores/consoleStore.ts]

```typescript
useConsoleStore.getState().addLog('INFO', 'SYSTEM', 'message');
```

**컴포넌트 memo 패턴:**
```typescript
const Component = memo(function Component(props) {
  // ...
});
```

### 테스트 가이드

**orderbookStore 테스트:**

```typescript
// __tests__/stores/orderbookStore.test.ts
describe('orderbookStore', () => {
  beforeEach(() => {
    useOrderbookStore.setState({
      asks: [],
      bids: [],
      timestamp: null,
      wsStatus: 'disconnected',
    });
  });

  it('setOrderbook updates asks, bids, timestamp', () => {
    const asks = [{ price: 50100000, size: 0.5 }];
    const bids = [{ price: 50000000, size: 0.8 }];

    useOrderbookStore.getState().setOrderbook(asks, bids, 1704067200000);

    expect(useOrderbookStore.getState().asks).toEqual(asks);
    expect(useOrderbookStore.getState().bids).toEqual(bids);
    expect(useOrderbookStore.getState().timestamp).toBe(1704067200000);
  });

  it('clearOrderbook resets state', () => {
    useOrderbookStore.getState().setOrderbook(
      [{ price: 50100000, size: 0.5 }],
      [{ price: 50000000, size: 0.8 }],
      1704067200000
    );

    useOrderbookStore.getState().clearOrderbook();

    expect(useOrderbookStore.getState().asks).toEqual([]);
    expect(useOrderbookStore.getState().bids).toEqual([]);
    expect(useOrderbookStore.getState().timestamp).toBeNull();
  });

  it('setWsStatus updates connection status', () => {
    useOrderbookStore.getState().setWsStatus('connected');
    expect(useOrderbookStore.getState().wsStatus).toBe('connected');
  });
});
```

**useUpbitOrderbookWs 훅 테스트:**

```typescript
// __tests__/hooks/useUpbitOrderbookWs.test.ts
import { renderHook, act, waitFor } from '@testing-library/react';
import WS from 'jest-websocket-mock';

describe('useUpbitOrderbookWs', () => {
  let server: WS;

  beforeEach(() => {
    server = new WS('wss://api.upbit.com/websocket/v1');
  });

  afterEach(() => {
    WS.clean();
  });

  it('connects when marketCode is provided', async () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC'));

    await server.connected;
    expect(server).toHaveBeenConnected();
  });

  it('sends subscription message on connect', async () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC'));

    await server.connected;

    const msg = await server.nextMessage;
    const parsed = JSON.parse(msg as string);
    expect(parsed[1].type).toBe('orderbook');
    expect(parsed[1].codes).toContain('KRW-BTC');
  });

  it('updates store on message', async () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC'));

    await server.connected;

    act(() => {
      server.send(JSON.stringify({
        type: 'orderbook',
        code: 'KRW-BTC',
        timestamp: 1704067200000,
        orderbook_units: [
          { ask_price: 50100000, bid_price: 50000000, ask_size: 0.5, bid_size: 0.8 }
        ]
      }));
    });

    await waitFor(() => {
      expect(useOrderbookStore.getState().asks).toHaveLength(1);
    });
  });

  it('reconnects on close', async () => {
    vi.useFakeTimers();

    renderHook(() => useUpbitOrderbookWs('KRW-BTC'));

    await server.connected;
    server.close();

    // 첫 번째 재연결 (1초 후)
    vi.advanceTimersByTime(1000);

    server = new WS('wss://api.upbit.com/websocket/v1');
    await server.connected;

    expect(server).toHaveBeenConnected();

    vi.useRealTimers();
  });
});
```

### 성능 최적화 체크리스트

1. **React.memo** - OrderbookRow 컴포넌트 메모이제이션
2. **useMemo** - maxSize 계산 캐싱
3. **스로틀링** - 필요시 requestAnimationFrame 또는 lodash.throttle 적용
4. **key 최적화** - price를 key로 사용 (안정적인 식별자)

### 주의사항

1. **WebSocket 연결 제한**: Upbit WebSocket 연결은 5회/초 제한. 빠른 마켓 전환 시 주의
2. **메모리 누수 방지**: useEffect cleanup에서 WebSocket 종료 필수
3. **재연결 로직**: exponential backoff로 서버 부하 방지
4. **타입 안전성**: Upbit 응답 타입과 내부 타입 구분

### References

- [Architecture: WTS Frontend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Frontend Structure)
- [Architecture: Upbit WebSocket API](/_bmad-output/planning-artifacts/architecture.md#Upbit WebSocket API)
- [UX Design: Orderbook Component](/_bmad-output/planning-artifacts/ux-design-specification.md#Orderbook)
- [Previous Story: WTS-2.4](/_bmad-output/implementation-artifacts/wts-2-4-market-selection.md)
- [WTS Epics: Story 2.5](/_bmad-output/planning-artifacts/wts-epics.md#Story 2.5)

### Project Structure Notes

**신규 파일:**
- `apps/desktop/src/wts/stores/orderbookStore.ts`
- `apps/desktop/src/wts/hooks/useUpbitOrderbookWs.ts`
- `apps/desktop/src/wts/__tests__/stores/orderbookStore.test.ts`
- `apps/desktop/src/wts/__tests__/hooks/useUpbitOrderbookWs.test.ts`

**수정 파일:**
- `apps/desktop/src/wts/types.ts` - OrderbookEntry, Orderbook, Upbit 응답 타입 추가
- `apps/desktop/src/wts/panels/OrderbookPanel.tsx` - 실시간 오더북 UI 구현
- `apps/desktop/src/wts/__tests__/panels/OrderbookPanel.test.tsx` - 통합 테스트 추가

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)
GPT-5 Codex (code review fixes)

### Debug Log References

### Completion Notes List

- Task 1 완료: types.ts에 OrderbookEntry, OrderbookData, UpbitOrderbookUnit, UpbitOrderbookResponse 타입 추가
- Task 2 완료: orderbookStore.ts 생성 - asks, bids, timestamp, wsStatus 상태와 setOrderbook, clearOrderbook, setWsStatus 액션 구현
- Task 3 완료: useUpbitOrderbookWs.ts 훅 생성 - WebSocket 연결, 구독, 메시지 파싱, exponential backoff 재연결, 콘솔 로깅 구현
- Task 4 완료: OrderbookPanel.tsx 업데이트 - 실시간 오더북 UI, 매도/매수 호가 렌더링, depth bar, 연결 상태 인디케이터 구현
- Task 5 완료: 성능 최적화 - React.memo (OrderbookRow), useMemo (maxSize), requestAnimationFrame 기반 스로틀링 (16ms) 적용
- Task 6 완료: orderbookStore.test.ts, useUpbitOrderbookWs.test.ts, OrderbookPanel.test.tsx, types.test.ts 테스트 작성/확장
- 코드 리뷰 수정: wsError 상태 추가, 거래소 변경 시 연결 정리, 의도적 close 재연결 방지, 관련 테스트 업데이트 (미실행)

### File List

**신규 파일:**
- apps/desktop/src/wts/stores/orderbookStore.ts
- apps/desktop/src/wts/hooks/useUpbitOrderbookWs.ts
- apps/desktop/src/wts/__tests__/stores/orderbookStore.test.ts
- apps/desktop/src/wts/__tests__/hooks/useUpbitOrderbookWs.test.ts

**수정 파일:**
- apps/desktop/src/wts/types.ts
- apps/desktop/src/wts/panels/OrderbookPanel.tsx
- apps/desktop/src/wts/__tests__/panels/OrderbookPanel.test.tsx
- apps/desktop/src/wts/__tests__/types.test.ts
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Change Log

- 2026-01-19: Story WTS-2.5 구현 완료 - 실시간 오더북 WebSocket 연동, 오더북 UI, 성능 최적화
- 2026-01-19: 코드 리뷰 수정 - wsError 상태/표시, 거래소 변경 처리, 의도적 종료 재연결 방지, 테스트 업데이트

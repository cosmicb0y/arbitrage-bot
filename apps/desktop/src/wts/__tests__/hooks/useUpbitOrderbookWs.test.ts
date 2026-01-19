import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useUpbitOrderbookWs } from '../../hooks/useUpbitOrderbookWs';
import { useOrderbookStore } from '../../stores/orderbookStore';
import { useConsoleStore } from '../../stores/consoleStore';

// Mock WebSocket
class MockWebSocket {
  static instances: MockWebSocket[] = [];
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  url: string;
  readyState: number = MockWebSocket.CONNECTING;
  onopen: ((ev: Event) => void) | null = null;
  onclose: ((ev: CloseEvent) => void) | null = null;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  sentMessages: string[] = [];

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sentMessages.push(data);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close'));
    }
  }

  // Test helpers
  simulateOpen() {
    this.readyState = MockWebSocket.OPEN;
    if (this.onopen) {
      this.onopen(new Event('open'));
    }
  }

  simulateMessage(data: unknown) {
    if (this.onmessage) {
      this.onmessage(new MessageEvent('message', { data: JSON.stringify(data) }));
    }
  }

  simulateError() {
    if (this.onerror) {
      this.onerror(new Event('error'));
    }
  }

  simulateClose() {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close'));
    }
  }
}

describe('useUpbitOrderbookWs', () => {
  const originalWebSocket = globalThis.WebSocket;

  beforeEach(() => {
    vi.useFakeTimers();
    MockWebSocket.instances = [];
    // @ts-expect-error - replace global WebSocket with mock
    globalThis.WebSocket = MockWebSocket;

    // Reset stores
    useOrderbookStore.setState({
      asks: [],
      bids: [],
      timestamp: null,
      wsStatus: 'disconnected',
      wsError: null,
    });
    useConsoleStore.setState({ logs: [] });
  });

  afterEach(() => {
    vi.useRealTimers();
    globalThis.WebSocket = originalWebSocket;
  });

  it('should not connect when marketCode is null', () => {
    renderHook(() => useUpbitOrderbookWs(null, 'upbit'));

    expect(MockWebSocket.instances).toHaveLength(0);
    expect(useOrderbookStore.getState().wsStatus).toBe('disconnected');
  });

  it('should not connect when exchange is not upbit', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'binance'));

    expect(MockWebSocket.instances).toHaveLength(0);
    expect(useOrderbookStore.getState().wsStatus).toBe('disconnected');
  });

  it('should connect when marketCode is provided', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].url).toBe('wss://api.upbit.com/websocket/v1');
  });

  it('should set wsStatus to connecting on connect', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    expect(useOrderbookStore.getState().wsStatus).toBe('connecting');
  });

  it('should set wsStatus to connected on open', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    expect(useOrderbookStore.getState().wsStatus).toBe('connected');
  });

  it('should send subscription message on connect', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    expect(MockWebSocket.instances[0].sentMessages).toHaveLength(1);
    const msg = JSON.parse(MockWebSocket.instances[0].sentMessages[0]);
    expect(msg[0]).toHaveProperty('ticket');
    expect(msg[1]).toEqual({
      type: 'orderbook',
      codes: ['KRW-BTC'],
      isOnlyRealtime: true,
    });
  });

  it('should update orderbook store on message', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    act(() => {
      MockWebSocket.instances[0].simulateMessage({
        type: 'orderbook',
        code: 'KRW-BTC',
        timestamp: 1704067200000,
        total_ask_size: 10.5,
        total_bid_size: 8.5,
        orderbook_units: [
          { ask_price: 50100000, bid_price: 50000000, ask_size: 0.5, bid_size: 0.8 },
          { ask_price: 50200000, bid_price: 49900000, ask_size: 1.0, bid_size: 1.2 },
        ],
      });
    });

    const state = useOrderbookStore.getState();
    expect(state.asks).toHaveLength(2);
    expect(state.bids).toHaveLength(2);
    expect(state.asks[0]).toEqual({ price: 50100000, size: 0.5 });
    expect(state.bids[0]).toEqual({ price: 50000000, size: 0.8 });
    expect(state.timestamp).toBe(1704067200000);
  });

  it('should log connection on open', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    const logs = useConsoleStore.getState().logs;
    expect(logs).toHaveLength(1);
    expect(logs[0].level).toBe('INFO');
    expect(logs[0].category).toBe('SYSTEM');
    expect(logs[0].message).toContain('Upbit 오더북 WebSocket 연결됨');
  });

  it('should set wsStatus to disconnected on close', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    act(() => {
      MockWebSocket.instances[0].simulateClose();
    });

    expect(useOrderbookStore.getState().wsStatus).toBe('disconnected');
  });

  it('should clear orderbook on close', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
      MockWebSocket.instances[0].simulateMessage({
        type: 'orderbook',
        code: 'KRW-BTC',
        timestamp: 1704067200000,
        orderbook_units: [
          { ask_price: 50100000, bid_price: 50000000, ask_size: 0.5, bid_size: 0.8 },
        ],
      });
    });

    act(() => {
      MockWebSocket.instances[0].simulateClose();
    });

    const state = useOrderbookStore.getState();
    expect(state.asks).toEqual([]);
    expect(state.bids).toEqual([]);
  });

  it('should log warning on unexpected close (not manual)', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    useConsoleStore.setState({ logs: [] }); // Clear connection log

    // 예상치 못한 종료 (manualCloseRef가 false인 상태) 시뮬레이션
    act(() => {
      MockWebSocket.instances[0].simulateClose();
    });

    const logs = useConsoleStore.getState().logs;
    expect(logs.some((l) => l.level === 'WARN' && l.message.includes('연결 끊김'))).toBe(
      true
    );
  });

  it('should attempt reconnect after close with exponential backoff', async () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    act(() => {
      MockWebSocket.instances[0].simulateClose();
    });

    // Should schedule reconnect
    const logs = useConsoleStore.getState().logs;
    expect(logs.some((l) => l.message.includes('재연결 시도'))).toBe(true);

    // First reconnect after 1 second
    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it('should use exponential backoff for reconnects', async () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    // First connection fails immediately (no open, just close)
    act(() => {
      MockWebSocket.instances[0].simulateClose();
    });

    // First reconnect at 1s
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(MockWebSocket.instances).toHaveLength(2);

    // Second close (still no successful connect)
    act(() => {
      MockWebSocket.instances[1].simulateClose();
    });

    // Second reconnect at 2s (backoff increases)
    act(() => {
      vi.advanceTimersByTime(2000);
    });
    expect(MockWebSocket.instances).toHaveLength(3);
  });

  it('should reconnect when market changes', () => {
    const { rerender } = renderHook(
      ({ market, exchange }) => useUpbitOrderbookWs(market, exchange),
      {
        initialProps: { market: 'KRW-BTC', exchange: 'upbit' },
      }
    );

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    expect(MockWebSocket.instances).toHaveLength(1);

    // Change market
    rerender({ market: 'KRW-ETH', exchange: 'upbit' });

    expect(MockWebSocket.instances).toHaveLength(2);
    expect(MockWebSocket.instances[0].readyState).toBe(MockWebSocket.CLOSED);
  });

  it('should not schedule reconnect on intentional close', () => {
    const { rerender } = renderHook(
      ({ market, exchange }) => useUpbitOrderbookWs(market, exchange),
      {
        initialProps: { market: 'KRW-BTC', exchange: 'upbit' },
      }
    );

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    rerender({ market: 'KRW-ETH', exchange: 'upbit' });

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it('should close WebSocket on unmount', () => {
    const { unmount } = renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    unmount();

    expect(MockWebSocket.instances[0].readyState).toBe(MockWebSocket.CLOSED);
  });

  it('should clear orderbook on unmount', () => {
    const { unmount } = renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
      MockWebSocket.instances[0].simulateMessage({
        type: 'orderbook',
        code: 'KRW-BTC',
        timestamp: 1704067200000,
        orderbook_units: [
          { ask_price: 50100000, bid_price: 50000000, ask_size: 0.5, bid_size: 0.8 },
        ],
      });
    });

    unmount();

    const state = useOrderbookStore.getState();
    expect(state.asks).toEqual([]);
    expect(state.bids).toEqual([]);
  });

  it('should log error on WebSocket error', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateError();
    });

    const logs = useConsoleStore.getState().logs;
    expect(logs.some((l) => l.level === 'ERROR' && l.message.includes('에러'))).toBe(true);
    expect(useOrderbookStore.getState().wsError).toBe(
      '오더북 WebSocket 에러가 발생했습니다'
    );
  });

  it('should ignore non-orderbook messages', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    act(() => {
      MockWebSocket.instances[0].simulateOpen();
    });

    act(() => {
      MockWebSocket.instances[0].simulateMessage({
        type: 'ticker',
        code: 'KRW-BTC',
      });
    });

    const state = useOrderbookStore.getState();
    expect(state.asks).toEqual([]);
    expect(state.bids).toEqual([]);
  });

  it('should reset reconnect attempt counter on successful connect', () => {
    renderHook(() => useUpbitOrderbookWs('KRW-BTC', 'upbit'));

    // First close (connection attempt, then close)
    act(() => {
      MockWebSocket.instances[0].simulateClose();
    });

    // Check first reconnect message shows 1초
    let logs = useConsoleStore.getState().logs;
    let reconnectLog = logs.find((l) => l.message.includes('재연결 시도'));
    expect(reconnectLog?.message).toContain('1초');

    // First reconnect at 1s
    act(() => {
      vi.advanceTimersByTime(1000);
    });

    // Now successfully connect, then close
    act(() => {
      MockWebSocket.instances[1].simulateOpen(); // This resets the counter
    });

    // Clear logs to check next reconnect
    useConsoleStore.setState({ logs: [] });

    act(() => {
      MockWebSocket.instances[1].simulateClose();
    });

    // Should be back to 1초 since counter was reset on successful connect
    logs = useConsoleStore.getState().logs;
    reconnectLog = logs.find((l) => l.message.includes('재연결 시도'));
    expect(reconnectLog?.message).toContain('1초');
  });
});

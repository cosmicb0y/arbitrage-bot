import { useEffect, useRef, useCallback } from 'react';
import { useOrderbookStore } from '../stores/orderbookStore';
import { useConsoleStore } from '../stores/consoleStore';
import type { Exchange, UpbitOrderbookResponse, OrderbookEntry } from '../types';

const UPBIT_WS_URL = 'wss://api.upbit.com/websocket/v1';
const RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000]; // exponential backoff
const THROTTLE_INTERVAL = 16; // 60fps target

export function useUpbitOrderbookWs(
  marketCode: string | null,
  exchange: Exchange | null
) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptRef = useRef(0);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastUpdateTimeRef = useRef(0);
  const manualCloseRef = useRef(false);
  const pendingUpdateRef = useRef<{
    asks: OrderbookEntry[];
    bids: OrderbookEntry[];
    timestamp: number;
  } | null>(null);
  const rafIdRef = useRef<number | null>(null);

  const { setOrderbook, clearOrderbook, setWsStatus, setWsError } =
    useOrderbookStore();
  const addLog = useConsoleStore((state) => state.addLog);

  const disconnect = useCallback(
    (options?: { clearError?: boolean }) => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
        reconnectTimeoutRef.current = null;
      }
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current);
        rafIdRef.current = null;
      }
      if (wsRef.current) {
        manualCloseRef.current = true;
        wsRef.current.close();
        wsRef.current = null;
      }
      clearOrderbook();
      setWsStatus('disconnected');
      if (options?.clearError) {
        setWsError(null);
      }
    },
    [clearOrderbook, setWsError, setWsStatus]
  );

  const connect = useCallback(() => {
    if (exchange !== 'upbit' || !marketCode) {
      disconnect({ clearError: true });
      return;
    }

    // Clear any pending reconnect
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    // Close existing connection
    if (wsRef.current) {
      manualCloseRef.current = true;
      wsRef.current.close();
    }

    setWsError(null);
    setWsStatus('connecting');
    const ws = new WebSocket(UPBIT_WS_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      setWsStatus('connected');
      reconnectAttemptRef.current = 0;
      setWsError(null);
      addLog('INFO', 'SYSTEM', `Upbit 오더북 WebSocket 연결됨: ${marketCode}`);

      // Send subscription message
      const subscribeMsg = [
        { ticket: `orderbook-${Date.now()}` },
        { type: 'orderbook', codes: [marketCode], isOnlyRealtime: true },
      ];
      ws.send(JSON.stringify(subscribeMsg));
    };

    ws.onmessage = async (event) => {
      try {
        let jsonData: string;

        // Upbit WebSocket은 Blob으로 데이터를 보낼 수 있음
        if (event.data instanceof Blob) {
          jsonData = await event.data.text();
        } else {
          jsonData = event.data;
        }

        const data: UpbitOrderbookResponse = JSON.parse(jsonData);

        if (data.type !== 'orderbook') return;

        const asks: OrderbookEntry[] = data.orderbook_units.map((u) => ({
          price: u.ask_price,
          size: u.ask_size,
        }));
        const bids: OrderbookEntry[] = data.orderbook_units.map((u) => ({
          price: u.bid_price,
          size: u.bid_size,
        }));

        setWsError(null);
        // Throttle updates to 60fps
        const now = Date.now();
        if (now - lastUpdateTimeRef.current >= THROTTLE_INTERVAL) {
          lastUpdateTimeRef.current = now;
          setOrderbook(asks, bids, data.timestamp);
        } else {
          // Store pending update and schedule RAF
          pendingUpdateRef.current = { asks, bids, timestamp: data.timestamp };
          if (rafIdRef.current === null) {
            rafIdRef.current = requestAnimationFrame(() => {
              rafIdRef.current = null;
              if (pendingUpdateRef.current) {
                const { asks: a, bids: b, timestamp: t } = pendingUpdateRef.current;
                pendingUpdateRef.current = null;
                lastUpdateTimeRef.current = Date.now();
                setOrderbook(a, b, t);
              }
            });
          }
        }
      } catch {
        setWsError('오더북 데이터 파싱 실패');
      }
    };

    ws.onclose = () => {
      setWsStatus('disconnected');
      clearOrderbook();

      const wasManualClose = manualCloseRef.current;
      manualCloseRef.current = false;
      if (wasManualClose) {
        // 의도적 종료 (마켓 변경, 언마운트 등) - 경고 로그 생략
        return;
      }

      addLog('WARN', 'SYSTEM', 'Upbit 오더북 WebSocket 연결 끊김');
      setWsError('오더북 WebSocket 연결이 끊겼습니다');

      // Auto reconnect with exponential backoff
      const delay =
        RECONNECT_DELAYS[
          Math.min(reconnectAttemptRef.current, RECONNECT_DELAYS.length - 1)
        ];
      reconnectAttemptRef.current += 1;
      addLog(
        'INFO',
        'SYSTEM',
        `Upbit 오더북 WebSocket 재연결 시도 (${delay / 1000}초 후)`
      );

      reconnectTimeoutRef.current = setTimeout(connect, delay);
    };

    ws.onerror = () => {
      addLog('ERROR', 'SYSTEM', 'Upbit 오더북 WebSocket 에러 발생');
      setWsError('오더북 WebSocket 에러가 발생했습니다');
    };
  }, [
    marketCode,
    exchange,
    disconnect,
    setOrderbook,
    clearOrderbook,
    setWsStatus,
    setWsError,
    addLog,
  ]);

  useEffect(() => {
    connect();

    return () => {
      disconnect({ clearError: true });
    };
  }, [connect, disconnect]);
}

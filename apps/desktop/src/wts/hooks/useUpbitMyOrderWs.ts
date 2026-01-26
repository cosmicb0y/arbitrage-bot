import { useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useOpenOrdersStore } from '../stores/openOrdersStore';
import { useConsoleStore } from '../stores/consoleStore';
import type { UpbitMyOrderResponse, WtsApiResult } from '../types';

const RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000]; // exponential backoff

/**
 * Upbit myOrder WebSocket 훅 (Tauri 이벤트 기반)
 *
 * 브라우저 WebSocket API는 커스텀 HTTP 헤더를 지원하지 않아
 * Upbit private WebSocket 인증이 불가능합니다.
 * Rust 백엔드에서 WebSocket 연결 후 Tauri 이벤트로 메시지를 전달받습니다.
 */
export function useUpbitMyOrderWs() {
  const reconnectAttemptRef = useRef(0);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const unlistenersRef = useRef<UnlistenFn[]>([]);
  const isConnectedRef = useRef(false);
  const isConnectingRef = useRef(false); // 중복 connect 방지 (React StrictMode 대응)

  const { upsertOrder, removeOrder, clearOrders, setWsStatus, setWsError } =
    useOpenOrdersStore();
  const addLog = useConsoleStore((state) => state.addLog);

  const formatSide = (side: 'ask' | 'bid') => (side === 'bid' ? '매수' : '매도');

  const formatPrice = (price: number) =>
    price.toLocaleString('ko-KR', { maximumFractionDigits: 0 });

  const formatVolume = (volume: number, market: string) => {
    const base = market.split('-')[1] ?? market;
    return `${volume} ${base}`;
  };

  const handleMyOrderMessage = useCallback(
    (data: UpbitMyOrderResponse) => {
      if (data.type !== 'myOrder') return;

      const sideText = formatSide(data.side);
      const priceText = formatPrice(data.price);

      switch (data.state) {
        case 'wait':
          // 미체결 주문: 목록에 추가/업데이트
          upsertOrder(data);
          break;

        case 'trade':
          // 부분 체결: 로그 출력 + 목록 업데이트
          upsertOrder(data);
          addLog(
            'INFO',
            'ORDER',
            `[체결] ${data.market} ${sideText} ${formatVolume(data.executed_volume, data.market)} 체결 (${priceText}원)`
          );
          break;

        case 'done':
          // 전체 체결 완료: 로그 출력 + 목록에서 제거
          removeOrder(data.uuid);
          addLog(
            'SUCCESS',
            'ORDER',
            `[완료] ${data.market} ${sideText} 주문 전체 체결 완료`
          );
          break;

        case 'cancel':
          // 주문 취소: 로그 출력 + 목록에서 제거
          removeOrder(data.uuid);
          addLog(
            'INFO',
            'ORDER',
            `[취소] ${data.market} ${sideText} 주문 취소됨`
          );
          break;
      }
    },
    [upsertOrder, removeOrder, addLog]
  );

  const cleanup = useCallback(async () => {
    // Clear pending reconnect
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    // Unlisten all event listeners
    for (const unlisten of unlistenersRef.current) {
      unlisten();
    }
    unlistenersRef.current = [];
  }, []);

  const disconnect = useCallback(
    async (options?: { clearError?: boolean }) => {
      // 연결 플래그 리셋
      isConnectingRef.current = false;

      await cleanup();

      // Stop Rust WebSocket
      if (isConnectedRef.current) {
        try {
          await invoke('wts_stop_myorder_ws');
        } catch {
          // Ignore errors when stopping
        }
        isConnectedRef.current = false;
      }

      clearOrders();
      setWsStatus('disconnected');
      if (options?.clearError) {
        setWsError(null);
      }
    },
    [cleanup, clearOrders, setWsError, setWsStatus]
  );

  const connect = useCallback(async () => {
    // 이미 연결 중이면 skip (React StrictMode 대응)
    if (isConnectingRef.current) {
      return;
    }
    isConnectingRef.current = true;

    // Clear any pending reconnect
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    // Cleanup previous listeners
    await cleanup();

    setWsError(null);
    setWsStatus('connecting');

    try {
      // Setup Tauri event listeners BEFORE starting WebSocket
      const unlistenMessage = await listen<string>('wts:myorder:message', (event) => {
        try {
          const rawData = JSON.parse(event.payload);

          // Upbit API 필드명을 내부 타입으로 매핑
          // - code → market (Upbit은 code 사용)
          // - ask_bid (ASK/BID) → side (ask/bid)
          const data: UpbitMyOrderResponse = {
            ...rawData,
            market: rawData.code ?? rawData.market,
            side: rawData.ask_bid
              ? (rawData.ask_bid.toLowerCase() as 'ask' | 'bid')
              : rawData.side,
          };

          handleMyOrderMessage(data);
        } catch {
          setWsError('myOrder 데이터 파싱 실패');
        }
      });

      const unlistenStatus = await listen<string>('wts:myorder:status', (event) => {
        const status = event.payload;
        if (status === 'connected') {
          setWsStatus('connected');
          setWsError(null);
          reconnectAttemptRef.current = 0;
          isConnectedRef.current = true;
          // 연결 완료, 플래그는 유지 (다음 connect 방지)
          addLog('INFO', 'SYSTEM', 'myOrder WebSocket 연결됨');
        } else if (status === 'disconnected') {
          setWsStatus('disconnected');
          isConnectedRef.current = false;
          isConnectingRef.current = false; // 연결 플래그 리셋
          clearOrders();

          // Auto reconnect with exponential backoff
          const delay =
            RECONNECT_DELAYS[
              Math.min(reconnectAttemptRef.current, RECONNECT_DELAYS.length - 1)
            ];
          reconnectAttemptRef.current += 1;
          addLog(
            'INFO',
            'SYSTEM',
            `myOrder WebSocket 재연결 시도 (${delay / 1000}초 후)`
          );
          reconnectTimeoutRef.current = setTimeout(connect, delay);
        } else if (status === 'error') {
          setWsStatus('disconnected');
          isConnectedRef.current = false;
          isConnectingRef.current = false; // 연결 플래그 리셋
        }
      });

      const unlistenError = await listen<string>('wts:myorder:error', (event) => {
        const errorMsg = event.payload;
        setWsError(errorMsg);
        addLog('ERROR', 'SYSTEM', `myOrder WebSocket 에러: ${errorMsg}`);
      });

      unlistenersRef.current = [unlistenMessage, unlistenStatus, unlistenError];

      // Start Rust WebSocket connection
      const result = await invoke<WtsApiResult<null>>('wts_start_myorder_ws');

      if (!result.success) {
        const errorMsg = result.error?.message ?? 'myOrder WebSocket 연결 실패';
        setWsError(errorMsg);
        setWsStatus('disconnected');
        isConnectingRef.current = false; // 연결 플래그 리셋
        addLog('ERROR', 'SYSTEM', `myOrder WebSocket 연결 실패: ${errorMsg}`);

        // Retry after delay
        const delay =
          RECONNECT_DELAYS[
            Math.min(reconnectAttemptRef.current, RECONNECT_DELAYS.length - 1)
          ];
        reconnectAttemptRef.current += 1;
        reconnectTimeoutRef.current = setTimeout(connect, delay);
      }
    } catch (err) {
      // Tauri invoke 에러는 string 또는 object일 수 있음
      let errorMsg: string;
      if (err instanceof Error) {
        errorMsg = err.message;
      } else if (typeof err === 'string') {
        errorMsg = err;
      } else if (typeof err === 'object' && err !== null) {
        // Tauri error object: { code, message } 또는 직렬화된 객체
        const errObj = err as Record<string, unknown>;
        errorMsg = (errObj.message as string) ?? JSON.stringify(err);
      } else {
        errorMsg = 'myOrder WebSocket 연결 실패 (알 수 없는 에러)';
      }

      console.error('[myOrder WS Error]', err); // 디버깅용 원본 에러 로깅
      setWsError(errorMsg);
      setWsStatus('disconnected');
      isConnectingRef.current = false; // 연결 플래그 리셋
      addLog('ERROR', 'SYSTEM', `myOrder WebSocket 에러: ${errorMsg}`);

      // Retry after delay
      const delay =
        RECONNECT_DELAYS[
          Math.min(reconnectAttemptRef.current, RECONNECT_DELAYS.length - 1)
        ];
      reconnectAttemptRef.current += 1;
      reconnectTimeoutRef.current = setTimeout(connect, delay);
    }
  }, [
    cleanup,
    handleMyOrderMessage,
    clearOrders,
    setWsStatus,
    setWsError,
    addLog,
  ]);

  // 함수 참조를 안정화하여 불필요한 재실행 방지
  const connectRef = useRef(connect);
  const disconnectRef = useRef(disconnect);
  connectRef.current = connect;
  disconnectRef.current = disconnect;

  useEffect(() => {
    connectRef.current();

    return () => {
      disconnectRef.current({ clearError: true });
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    disconnect,
    reconnect: connect,
  };
}

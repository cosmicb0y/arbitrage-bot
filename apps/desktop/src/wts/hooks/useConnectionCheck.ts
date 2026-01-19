import { useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWtsStore } from '../stores';
import { useConsoleStore } from '../stores/consoleStore';
import type { ConnectionCheckResult, Exchange } from '../types';
import { EXCHANGE_META } from '../types';

/** 최대 재시도 횟수 */
const MAX_RETRIES = 5;

/** 초기 재시도 지연 시간 (ms) */
const INITIAL_DELAY = 1000;

/**
 * 거래소 API 연결 상태를 확인하고 자동 재연결을 관리하는 훅
 *
 * - WTS 창 마운트 시 자동 연결 체크 실행
 * - 연결 실패 시 exponential backoff 재시도 (최대 5회)
 * - 거래소 변경 시 자동 재연결
 */
export function useConnectionCheck() {
  const { selectedExchange, setConnectionStatus, setConnectionError } =
    useWtsStore();
  const { addLog } = useConsoleStore();
  const retryCountRef = useRef(0);
  const timeoutRef = useRef<number | null>(null);
  const isMountedRef = useRef(true);
  const requestIdRef = useRef(0);

  const getExchangeName = useCallback((exchange: Exchange) => {
    return EXCHANGE_META[exchange]?.name ?? exchange;
  }, []);

  const checkConnection = useCallback(async () => {
    if (!isMountedRef.current) return;

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    const exchangeName = getExchangeName(selectedExchange);

    setConnectionStatus('connecting');
    setConnectionError(null);
    addLog('INFO', 'SYSTEM', `[INFO] ${exchangeName} API 연결 확인 중...`);

    try {
      const result = await invoke<ConnectionCheckResult>('wts_check_connection', {
        exchange: selectedExchange,
      });

      if (!isMountedRef.current || requestId !== requestIdRef.current) return;

      if (result.success) {
        setConnectionStatus('connected');
        setConnectionError(null);
        retryCountRef.current = 0;
        addLog(
          'SUCCESS',
          'SYSTEM',
          `[SUCCESS] ${exchangeName} API 연결됨${
            result.latency ? ` (${result.latency}ms)` : ''
          }`
        );
      } else {
        throw new Error(result.error || 'Unknown error');
      }
    } catch (error) {
      if (!isMountedRef.current || requestId !== requestIdRef.current) return;

      const errorMessage = error instanceof Error ? error.message : String(error);
      setConnectionStatus('disconnected');
      setConnectionError(errorMessage);
      addLog(
        'ERROR',
        'SYSTEM',
        `[ERROR] ${exchangeName} API 연결 실패: ${errorMessage}`
      );

      // 재시도 로직 (exponential backoff)
      if (retryCountRef.current < MAX_RETRIES) {
        const delay = INITIAL_DELAY * Math.pow(2, retryCountRef.current);
        retryCountRef.current++;
        addLog(
          'INFO',
          'SYSTEM',
          `[INFO] ${exchangeName} API 재연결 시도 중... (${retryCountRef.current}/${MAX_RETRIES}, ${delay / 1000}초 후)`
        );
        timeoutRef.current = window.setTimeout(() => {
          if (isMountedRef.current) {
            checkConnection();
          }
        }, delay);
      } else {
        addLog(
          'ERROR',
          'SYSTEM',
          `${exchangeName} API 재연결 실패 - 최대 재시도 횟수 초과`
        );
      }
    }
  }, [
    selectedExchange,
    setConnectionStatus,
    setConnectionError,
    addLog,
    getExchangeName,
  ]);

  // 마운트 시 자동 연결 체크, 거래소 변경 시 재체크
  useEffect(() => {
    isMountedRef.current = true;
    retryCountRef.current = 0;

    // 기존 타이머 취소
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    checkConnection();

    return () => {
      isMountedRef.current = false;
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    };
  }, [selectedExchange, checkConnection]);

  return { checkConnection };
}

import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWtsStore } from '../stores/wtsStore';
import { useConsoleStore } from '../stores/consoleStore';
import type { Market, WtsApiResult } from '../types';

interface UpbitMarketResponse {
  market: string;
  korean_name: string;
  english_name: string;
  market_warning?: string;
}

/**
 * Upbit 마켓 목록을 동적으로 로드하는 훅
 * connectionStatus가 'connected'가 되면 마켓 목록을 가져옴
 */
export function useUpbitMarkets() {
  const { connectionStatus, selectedExchange, setAvailableMarkets } =
    useWtsStore();
  const addLog = useConsoleStore((state) => state.addLog);
  const hasFetchedRef = useRef(false);

  useEffect(() => {
    // Upbit 거래소가 아니거나 연결되지 않은 경우 스킵
    if (selectedExchange !== 'upbit' || connectionStatus !== 'connected') {
      return;
    }

    // 이미 가져온 경우 스킵 (연결 상태가 변경될 때마다 다시 가져오지 않음)
    if (hasFetchedRef.current) {
      return;
    }

    const fetchMarkets = async () => {
      try {
        const result = await invoke<WtsApiResult<UpbitMarketResponse[]>>(
          'wts_get_markets'
        );

        if (result.success && result.data) {
          const markets: Market[] = result.data.map((m) => ({
            code: m.market as `${string}-${string}`,
            base: m.market.split('-')[1],
            quote: m.market.split('-')[0],
            displayName: m.korean_name,
          }));

          setAvailableMarkets(markets);
          hasFetchedRef.current = true;
          addLog(
            'INFO',
            'SYSTEM',
            `Upbit 마켓 목록 로드 완료: ${markets.length}개`
          );
        } else {
          addLog(
            'ERROR',
            'SYSTEM',
            `마켓 목록 로드 실패: ${result.error?.message || '알 수 없는 오류'}`
          );
        }
      } catch (error) {
        addLog(
          'ERROR',
          'SYSTEM',
          `마켓 목록 로드 실패: ${error instanceof Error ? error.message : '알 수 없는 오류'}`
        );
      }
    };

    fetchMarkets();
  }, [connectionStatus, selectedExchange, setAvailableMarkets, addLog]);

  // 거래소가 변경되면 다시 가져올 수 있도록 리셋
  useEffect(() => {
    hasFetchedRef.current = false;
  }, [selectedExchange]);
}

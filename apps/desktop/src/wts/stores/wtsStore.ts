import { create } from 'zustand';
import { ENABLED_EXCHANGES } from '../types';
import type { Exchange, ConnectionStatus, WtsState } from '../types';

/**
 * WTS 메인 스토어
 * 거래소/마켓 선택 및 연결 상태 관리
 */
export const useWtsStore = create<WtsState>()((set) => ({
  enabledExchanges: ENABLED_EXCHANGES,
  setEnabledExchanges: (exchanges: readonly Exchange[]) =>
    set({ enabledExchanges: exchanges }),

  // 거래소 선택 (MVP: upbit 기본값)
  selectedExchange: 'upbit',
  setExchange: (exchange: Exchange) => set({ selectedExchange: exchange }),

  // 마켓 선택
  selectedMarket: null,
  setMarket: (market: string | null) => set({ selectedMarket: market }),

  // 연결 상태
  connectionStatus: 'disconnected',
  setConnectionStatus: (status: ConnectionStatus) =>
    set({ connectionStatus: status }),

  // 연결 에러
  lastConnectionError: null,
  setConnectionError: (error: string | null) => set({ lastConnectionError: error }),
}));

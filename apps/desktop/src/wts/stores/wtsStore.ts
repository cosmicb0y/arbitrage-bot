import { create } from 'zustand';
import { ENABLED_EXCHANGES, UPBIT_DEFAULT_MARKETS } from '../types';
import type { Exchange, ConnectionStatus, WtsState, Market } from '../types';
import { useConsoleStore } from './consoleStore';

const EMPTY_MARKETS: readonly Market[] = [];

const MARKETS_BY_EXCHANGE: Record<Exchange, readonly Market[]> = {
  upbit: UPBIT_DEFAULT_MARKETS,
  bithumb: EMPTY_MARKETS,
  binance: EMPTY_MARKETS,
  coinbase: EMPTY_MARKETS,
  bybit: EMPTY_MARKETS,
  gateio: EMPTY_MARKETS,
};

/**
 * WTS 메인 스토어
 * 거래소/마켓 선택 및 연결 상태 관리
 */
export const useWtsStore = create<WtsState>()((set, get) => ({
  enabledExchanges: ENABLED_EXCHANGES,
  setEnabledExchanges: (exchanges: readonly Exchange[]) =>
    set({ enabledExchanges: exchanges }),

  // 거래소 선택 (MVP: upbit 기본값)
  selectedExchange: 'upbit',
  setExchange: (exchange: Exchange) => {
    set({
      selectedExchange: exchange,
      selectedMarket: null,
      availableMarkets: MARKETS_BY_EXCHANGE[exchange],
    });
  },

  // 마켓 선택
  selectedMarket: null,
  setMarket: (market: string | null) => {
    const prevMarket = get().selectedMarket;
    if (market === null) {
      set({ selectedMarket: null });
      return;
    }

    const isValidMarket = get().availableMarkets.some(
      (item) => item.code === market
    );
    if (!isValidMarket || market === prevMarket) {
      return;
    }

    set({ selectedMarket: market });

    // 마켓 변경 시 콘솔 로깅
    useConsoleStore.getState().addLog('INFO', 'SYSTEM', `마켓 변경: ${market}`);
  },

  // 사용 가능한 마켓 목록
  availableMarkets: UPBIT_DEFAULT_MARKETS,
  setAvailableMarkets: (markets: readonly Market[]) =>
    set({ availableMarkets: markets }),

  // 연결 상태
  connectionStatus: 'disconnected',
  setConnectionStatus: (status: ConnectionStatus) =>
    set({ connectionStatus: status }),

  // 연결 에러
  lastConnectionError: null,
  setConnectionError: (error: string | null) => set({ lastConnectionError: error }),
}));

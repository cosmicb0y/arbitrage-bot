import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { BalanceEntry, WtsApiResult } from '../types';
import { useConsoleStore } from './consoleStore';

/**
 * Balance 상태 인터페이스
 */
export interface BalanceState {
  /** 현재 잔고 목록 */
  balances: BalanceEntry[];
  /** 이전 잔고 (변화 감지용) */
  previousBalances: BalanceEntry[];
  /** 로딩 상태 */
  isLoading: boolean;
  /** 마지막 업데이트 시간 (Unix timestamp ms) */
  lastUpdated: number | null;
  /** 0 잔고 숨기기 옵션 */
  hideZeroBalances: boolean;
  /** 자동 갱신 활성화 여부 */
  autoRefreshEnabled: boolean;
  /** 대기 중인 잔고 갱신 요청 */
  pendingRefresh: boolean;
  /** 에러 메시지 */
  error: string | null;

  /** 자동 갱신 활성화 */
  enableAutoRefresh: () => void;
  /** 자동 갱신 비활성화 */
  disableAutoRefresh: () => void;
  /** 잔고 조회 */
  fetchBalance: () => Promise<void>;
  /** 0 잔고 숨기기 옵션 설정 */
  setHideZeroBalances: (hide: boolean) => void;
}

/**
 * Balance 스토어
 * 잔고 데이터 관리 및 API 호출
 */
export const useBalanceStore = create<BalanceState>()((set, get) => ({
  balances: [],
  previousBalances: [],
  isLoading: false,
  lastUpdated: null,
  hideZeroBalances: false,
  autoRefreshEnabled: true,
  pendingRefresh: false,
  error: null,

  enableAutoRefresh: () => set({ autoRefreshEnabled: true }),
  disableAutoRefresh: () => set({ autoRefreshEnabled: false }),
  fetchBalance: async () => {
    if (get().isLoading) {
      set({ pendingRefresh: true });
      return;
    }

    set({ isLoading: true, error: null });

    try {
      const result = await invoke<WtsApiResult<BalanceEntry[]>>('wts_get_balance');

      if (result.success && result.data) {
        set((state) => ({
          previousBalances: state.balances,
          balances: result.data!,
          lastUpdated: Date.now(),
        }));

        useConsoleStore.getState().addLog(
          'SUCCESS',
          'BALANCE',
          `잔고 조회 완료: ${result.data.length}개 자산`
        );
      } else {
        const errorMsg = result.error?.message || '알 수 없는 오류';
        set({ error: errorMsg });

        useConsoleStore.getState().addLog(
          'ERROR',
          'BALANCE',
          `잔고 조회 실패: ${errorMsg}`
        );
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      set({ error: errorMsg });

      useConsoleStore.getState().addLog(
        'ERROR',
        'BALANCE',
        `잔고 조회 실패: ${errorMsg}`
      );
    } finally {
      set({ isLoading: false });
      if (get().pendingRefresh) {
        set({ pendingRefresh: false });
        await get().fetchBalance();
      }
    }
  },

  setHideZeroBalances: (hide: boolean) => set({ hideZeroBalances: hide }),
}));

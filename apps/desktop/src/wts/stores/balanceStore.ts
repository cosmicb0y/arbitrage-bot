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
  /** 에러 메시지 */
  error: string | null;

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
  error: null,

  fetchBalance: async () => {
    set({ isLoading: true, error: null });

    try {
      const result = await invoke<WtsApiResult<BalanceEntry[]>>('wts_get_balance');

      if (result.success && result.data) {
        set((state) => ({
          previousBalances: state.balances,
          balances: result.data!,
          lastUpdated: Date.now(),
          isLoading: false,
        }));

        useConsoleStore.getState().addLog(
          'SUCCESS',
          'BALANCE',
          `잔고 조회 완료: ${result.data.length}개 자산`
        );
      } else {
        const errorMsg = result.error?.message || '알 수 없는 오류';
        set({ error: errorMsg, isLoading: false });

        useConsoleStore.getState().addLog(
          'ERROR',
          'BALANCE',
          `잔고 조회 실패: ${errorMsg}`
        );
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      set({ error: errorMsg, isLoading: false });

      useConsoleStore.getState().addLog(
        'ERROR',
        'BALANCE',
        `잔고 조회 실패: ${errorMsg}`
      );
    }
  },

  setHideZeroBalances: (hide: boolean) => set({ hideZeroBalances: hide }),
}));

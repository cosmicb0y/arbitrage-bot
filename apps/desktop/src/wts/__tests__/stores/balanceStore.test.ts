import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useBalanceStore } from '../../stores/balanceStore';
import { useConsoleStore } from '../../stores/consoleStore';
import type { BalanceEntry, WtsApiResult } from '../../types';

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

describe('balanceStore', () => {
  beforeEach(() => {
    // Reset store state before each test
    useBalanceStore.setState({
      balances: [],
      previousBalances: [],
      isLoading: false,
      lastUpdated: null,
      hideZeroBalances: false,
      autoRefreshEnabled: true,
      pendingRefresh: false,
      error: null,
    });
    useConsoleStore.setState({ logs: [] });
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('initial state', () => {
    it('should have correct initial state', () => {
      const state = useBalanceStore.getState();
      expect(state.balances).toEqual([]);
      expect(state.previousBalances).toEqual([]);
      expect(state.isLoading).toBe(false);
      expect(state.lastUpdated).toBeNull();
      expect(state.hideZeroBalances).toBe(false);
      expect(state.error).toBeNull();
    });
  });

  describe('setHideZeroBalances', () => {
    it('should update hideZeroBalances to true', () => {
      const { setHideZeroBalances } = useBalanceStore.getState();
      setHideZeroBalances(true);
      expect(useBalanceStore.getState().hideZeroBalances).toBe(true);
    });

    it('should update hideZeroBalances to false', () => {
      useBalanceStore.setState({ hideZeroBalances: true });
      const { setHideZeroBalances } = useBalanceStore.getState();
      setHideZeroBalances(false);
      expect(useBalanceStore.getState().hideZeroBalances).toBe(false);
    });
  });

  describe('auto refresh toggle', () => {
    it('should disable auto refresh', () => {
      const { disableAutoRefresh } = useBalanceStore.getState();
      disableAutoRefresh();
      expect(useBalanceStore.getState().autoRefreshEnabled).toBe(false);
    });

    it('should enable auto refresh', () => {
      useBalanceStore.setState({ autoRefreshEnabled: false });
      const { enableAutoRefresh } = useBalanceStore.getState();
      enableAutoRefresh();
      expect(useBalanceStore.getState().autoRefreshEnabled).toBe(true);
    });
  });

  describe('fetchBalance', () => {
    const mockBalances: BalanceEntry[] = [
      {
        currency: 'KRW',
        balance: '1000000',
        locked: '0',
        avg_buy_price: '1',
        avg_buy_price_modified: false,
        unit_currency: 'KRW',
      },
      {
        currency: 'BTC',
        balance: '0.5',
        locked: '0.1',
        avg_buy_price: '50000000',
        avg_buy_price_modified: false,
        unit_currency: 'KRW',
      },
    ];

    it('should set isLoading to true while fetching', async () => {
      // Setup a delayed response
      vi.mocked(invoke).mockImplementation(
        () =>
          new Promise((resolve) =>
            setTimeout(
              () => resolve({ success: true, data: mockBalances }),
              100
            )
          )
      );

      const fetchPromise = useBalanceStore.getState().fetchBalance();

      // Check loading state immediately
      expect(useBalanceStore.getState().isLoading).toBe(true);

      await fetchPromise;
    });

    it('should store balances on successful fetch', async () => {
      const mockResult: WtsApiResult<BalanceEntry[]> = {
        success: true,
        data: mockBalances,
      };
      vi.mocked(invoke).mockResolvedValue(mockResult);

      await useBalanceStore.getState().fetchBalance();

      const state = useBalanceStore.getState();
      expect(state.balances).toEqual(mockBalances);
      expect(state.isLoading).toBe(false);
      expect(state.lastUpdated).not.toBeNull();
      expect(state.error).toBeNull();
    });

    it('should preserve previous balances for change detection', async () => {
      // First fetch
      const firstBalances: BalanceEntry[] = [
        {
          currency: 'BTC',
          balance: '0.5',
          locked: '0',
          avg_buy_price: '50000000',
          avg_buy_price_modified: false,
          unit_currency: 'KRW',
        },
      ];
      vi.mocked(invoke).mockResolvedValue({ success: true, data: firstBalances });
      await useBalanceStore.getState().fetchBalance();

      // Second fetch with different balances
      const secondBalances: BalanceEntry[] = [
        {
          currency: 'BTC',
          balance: '0.6',
          locked: '0',
          avg_buy_price: '50000000',
          avg_buy_price_modified: false,
          unit_currency: 'KRW',
        },
      ];
      vi.mocked(invoke).mockResolvedValue({ success: true, data: secondBalances });
      await useBalanceStore.getState().fetchBalance();

      const state = useBalanceStore.getState();
      expect(state.balances).toEqual(secondBalances);
      expect(state.previousBalances).toEqual(firstBalances);
    });

    it('should call wts_get_balance via invoke', async () => {
      vi.mocked(invoke).mockResolvedValue({ success: true, data: [] });

      await useBalanceStore.getState().fetchBalance();

      expect(invoke).toHaveBeenCalledWith('wts_get_balance');
    });

    it('should log success message to consoleStore', async () => {
      vi.mocked(invoke).mockResolvedValue({ success: true, data: mockBalances });

      await useBalanceStore.getState().fetchBalance();

      const logs = useConsoleStore.getState().logs;
      expect(logs.length).toBe(1);
      expect(logs[0].level).toBe('SUCCESS');
      expect(logs[0].category).toBe('BALANCE');
      expect(logs[0].message).toContain('잔고 조회 완료');
      expect(logs[0].message).toContain('2');
    });

    it('should handle API error response', async () => {
      const errorResult: WtsApiResult<BalanceEntry[]> = {
        success: false,
        error: { code: 'AUTH_ERROR', message: 'Invalid API key' },
      };
      vi.mocked(invoke).mockResolvedValue(errorResult);

      await useBalanceStore.getState().fetchBalance();

      const state = useBalanceStore.getState();
      expect(state.error).toBe('Invalid API key');
      expect(state.isLoading).toBe(false);
      expect(state.balances).toEqual([]);
    });

    it('should log error message to consoleStore on API error', async () => {
      const errorResult: WtsApiResult<BalanceEntry[]> = {
        success: false,
        error: { code: 'AUTH_ERROR', message: 'Invalid API key' },
      };
      vi.mocked(invoke).mockResolvedValue(errorResult);

      await useBalanceStore.getState().fetchBalance();

      const logs = useConsoleStore.getState().logs;
      expect(logs.length).toBe(1);
      expect(logs[0].level).toBe('ERROR');
      expect(logs[0].category).toBe('BALANCE');
      expect(logs[0].message).toContain('잔고 조회 실패');
    });

    it('should handle network/invoke exception', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Network error'));

      await useBalanceStore.getState().fetchBalance();

      const state = useBalanceStore.getState();
      expect(state.error).toBe('Network error');
      expect(state.isLoading).toBe(false);
    });

    it('should log error message to consoleStore on exception', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Network error'));

      await useBalanceStore.getState().fetchBalance();

      const logs = useConsoleStore.getState().logs;
      expect(logs.length).toBe(1);
      expect(logs[0].level).toBe('ERROR');
      expect(logs[0].category).toBe('BALANCE');
      expect(logs[0].message).toContain('Network error');
    });
  });
});

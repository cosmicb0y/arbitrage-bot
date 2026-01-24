/**
 * WTS-3.4: 지정가 주문 통합 테스트
 *
 * 전체 플로우 테스트:
 * - 지정가 매수 주문 플로우
 * - 지정가 매도 주문 플로우
 * - 호가 클릭 → 지정가 주문 플로우
 */
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { OrderPanel } from '../../panels/OrderPanel';
import { OrderbookPanel } from '../../panels/OrderbookPanel';
import { useOrderStore } from '../../stores/orderStore';
import { useBalanceStore } from '../../stores/balanceStore';
import { useWtsStore } from '../../stores/wtsStore';
import { useOrderbookStore } from '../../stores/orderbookStore';
import { useConsoleStore } from '../../stores/consoleStore';
import { useToastStore } from '../../stores/toastStore';
import type { BalanceEntry } from '../../types';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock stores
vi.mock('../../stores/balanceStore');
vi.mock('../../stores/wtsStore');
vi.mock('../../stores/orderbookStore');
vi.mock('../../stores/consoleStore');
vi.mock('../../stores/toastStore');

// Mock useUpbitOrderbookWs hook
vi.mock('../../hooks/useUpbitOrderbookWs', () => ({
  useUpbitOrderbookWs: vi.fn(),
}));

describe('WTS-3.4 지정가 주문 통합 테스트', () => {
  const mockAddLog = vi.fn();
  const mockShowToast = vi.fn();
  const mockFetchBalance = vi.fn();

  const mockBalances: BalanceEntry[] = [
    {
      currency: 'KRW',
      balance: '10000000',
      locked: '0',
      avg_buy_price: '0',
      avg_buy_price_modified: false,
      unit_currency: 'KRW',
    },
    {
      currency: 'BTC',
      balance: '0.5',
      locked: '0',
      avg_buy_price: '50000000',
      avg_buy_price_modified: false,
      unit_currency: 'KRW',
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    useOrderStore.setState({
      orderType: 'limit',
      side: 'buy',
      price: '',
      quantity: '',
    });

    vi.mocked(useBalanceStore).mockReturnValue({
      balances: mockBalances,
      previousBalances: [],
      isLoading: false,
      lastUpdated: null,
      hideZeroBalances: false,
      autoRefreshEnabled: true,
      pendingRefresh: false,
      error: null,
      enableAutoRefresh: vi.fn(),
      disableAutoRefresh: vi.fn(),
      fetchBalance: mockFetchBalance,
      setHideZeroBalances: vi.fn(),
    });

    vi.mocked(useWtsStore).mockReturnValue({
      enabledExchanges: ['upbit'],
      setEnabledExchanges: vi.fn(),
      selectedExchange: 'upbit',
      setExchange: vi.fn(),
      selectedMarket: 'KRW-BTC',
      setMarket: vi.fn(),
      availableMarkets: [],
      setAvailableMarkets: vi.fn(),
      connectionStatus: 'connected',
      setConnectionStatus: vi.fn(),
      lastConnectionError: null,
      setConnectionError: vi.fn(),
    });

    vi.mocked(useOrderbookStore).mockReturnValue({
      asks: [],
      bids: [],
      timestamp: null,
      wsStatus: 'connected',
      wsError: null,
      setOrderbook: vi.fn(),
      clearOrderbook: vi.fn(),
      setWsStatus: vi.fn(),
      setWsError: vi.fn(),
    });

    vi.mocked(useConsoleStore).mockImplementation((selector) => {
      if (typeof selector === 'function') {
        return selector({ logs: [], addLog: mockAddLog, clearLogs: vi.fn() });
      }
      return { logs: [], addLog: mockAddLog, clearLogs: vi.fn() };
    });

    vi.mocked(useToastStore).mockImplementation((selector) => {
      if (typeof selector === 'function') {
        return selector({ toasts: [], showToast: mockShowToast, removeToast: vi.fn(), clearToasts: vi.fn() });
      }
      return { toasts: [], showToast: mockShowToast, removeToast: vi.fn(), clearToasts: vi.fn() };
    });
  });

  describe('Subtask 6.1: 지정가 매수 주문 전체 플로우 (AC #1-#5)', () => {
    it('지정가 매수 주문: 입력 → 확인 다이얼로그 → API 호출 → 성공 토스트', async () => {
      useOrderStore.setState({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.01',
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'limit-buy-order-1',
          side: 'bid',
          ord_type: 'limit',
          price: '50000000',
          state: 'wait',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.01',
          remaining_volume: '0.01',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '500500',
          executed_volume: '0',
          trades_count: 0,
        },
      });

      render(<OrderPanel />);

      // Step 1: 주문 버튼 클릭 → 확인 다이얼로그 표시 (AC #1)
      const submitBtn = screen.getByTestId('order-submit-btn');
      expect(submitBtn.textContent).toContain('지정가 매수');
      fireEvent.click(submitBtn);

      // Step 2: 다이얼로그에서 주문 정보 확인 (AC #2)
      const dialog = screen.getByRole('dialog');
      expect(within(dialog).getByText('매수 주문 확인')).toBeDefined();
      expect(within(dialog).getByText('지정가 매수')).toBeDefined();
      expect(within(dialog).getByText('KRW-BTC')).toBeDefined();
      expect(within(dialog).getByText(/50,000,000/)).toBeDefined();
      expect(within(dialog).getByText(/0.01/)).toBeDefined();
      expect(within(dialog).getByText(/지정가 주문은 해당 가격에 도달하면 체결됩니다/)).toBeDefined();

      // Step 3: 확인 버튼 클릭 → API 호출 (AC #3)
      const confirmBtn = within(dialog).getByRole('button', { name: '매수' });
      fireEvent.click(confirmBtn);

      await waitFor(() => {
        expect(vi.mocked(invoke)).toHaveBeenCalledWith('wts_place_order', {
          params: {
            market: 'KRW-BTC',
            side: 'bid',
            ord_type: 'limit',
            volume: '0.01',
            price: '50000000',
          },
        });
      });

      // Step 4: 콘솔 로그 확인 (AC #4)
      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'INFO',
          'ORDER',
          expect.stringContaining('지정가 매수 주문 요청')
        );
      });

      // Step 5: 성공 토스트 확인 (AC #5 - state: 'wait')
      await waitFor(() => {
        expect(mockShowToast).toHaveBeenCalledWith('success', '주문이 등록되었습니다');
      });
    });
  });

  describe('Subtask 6.2: 지정가 매도 주문 전체 플로우 (AC #1-#5)', () => {
    it('지정가 매도 주문: 입력 → 확인 다이얼로그 → API 호출 → 성공 토스트', async () => {
      useOrderStore.setState({
        orderType: 'limit',
        side: 'sell',
        price: '55000000',
        quantity: '0.02',
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'limit-sell-order-1',
          side: 'ask',
          ord_type: 'limit',
          price: '55000000',
          state: 'wait',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.02',
          remaining_volume: '0.02',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '0.02',
          executed_volume: '0',
          trades_count: 0,
        },
      });

      render(<OrderPanel />);

      // Step 1: 주문 버튼 클릭 → 확인 다이얼로그 표시
      const submitBtn = screen.getByTestId('order-submit-btn');
      expect(submitBtn.textContent).toContain('지정가 매도');
      fireEvent.click(submitBtn);

      // Step 2: 다이얼로그에서 주문 정보 확인
      const dialog = screen.getByRole('dialog');
      expect(within(dialog).getByText('지정가 매도')).toBeDefined();
      expect(within(dialog).getByText(/55,000,000/)).toBeDefined();
      expect(within(dialog).getByText(/0.02/)).toBeDefined();

      // Step 3: 확인 버튼 클릭 → API 호출
      const confirmBtn = within(dialog).getByRole('button', { name: '매도' });
      fireEvent.click(confirmBtn);

      await waitFor(() => {
        expect(vi.mocked(invoke)).toHaveBeenCalledWith('wts_place_order', {
          params: {
            market: 'KRW-BTC',
            side: 'ask',
            ord_type: 'limit',
            volume: '0.02',
            price: '55000000',
          },
        });
      });

      // Step 4: 콘솔 로그 확인
      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'INFO',
          'ORDER',
          expect.stringContaining('지정가 매도 주문 요청')
        );
      });

      // Step 5: 성공 토스트 확인
      await waitFor(() => {
        expect(mockShowToast).toHaveBeenCalledWith('success', '주문이 등록되었습니다');
      });
    });
  });

  describe('Subtask 6.3: 호가 클릭 → 지정가 주문 플로우 (AC #6)', () => {
    it('호가 클릭으로 가격 자동 입력 후 지정가 주문이 전송된다', async () => {
      useOrderStore.setState({
        orderType: 'market',
        side: 'sell',
        price: '',
        quantity: '',
      });

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [{ price: 50100000, size: 0.5 }],
        bids: [{ price: 50000000, size: 0.8 }],
        timestamp: 1704067200000,
        wsStatus: 'connected',
        wsError: null,
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'limit-buy-order-from-orderbook',
          side: 'bid',
          ord_type: 'limit',
          price: '50100000',
          state: 'wait',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.01',
          remaining_volume: '0.01',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '501000',
          executed_volume: '0',
          trades_count: 0,
        },
      });

      render(
        <div>
          <OrderbookPanel />
          <OrderPanel />
        </div>
      );

      // 호가 클릭 → 지정가/매수 전환 + 가격 자동 입력
      fireEvent.click(screen.getByText('50,100,000'));

      await waitFor(() => {
        const limitTab = screen.getByRole('tab', { name: '지정가' });
        expect(limitTab.getAttribute('aria-selected')).toBe('true');
        const priceInput = screen.getByLabelText('가격') as HTMLInputElement;
        expect(priceInput.value).toBe('50,100,000');
      });

      // 수량 입력 후 주문 제출
      fireEvent.change(screen.getByLabelText(/수량/), {
        target: { value: '0.01' },
      });
      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

      await waitFor(() => {
        expect(vi.mocked(invoke)).toHaveBeenCalledWith('wts_place_order', {
          params: {
            market: 'KRW-BTC',
            side: 'bid',
            ord_type: 'limit',
            volume: '0.01',
            price: '50100000',
          },
        });
      });
    });
  });

  describe('추가 시나리오: 잔고 검증 및 에러 처리 플로우 (AC #7, #8, #10)', () => {
    it('잔고 초과 시 경고 표시되지만 주문은 가능 (AC #7, #8)', () => {
      useOrderStore.setState({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '1', // 총액 50,000,000 > KRW 잔고 10,000,000
      });

      render(<OrderPanel />);

      // 잔고 초과 경고 표시
      expect(screen.getByText('잔고 초과')).toBeDefined();

      // 버튼은 여전히 활성화 (거래소 API가 최종 검증)
      const submitBtn = screen.getByTestId('order-submit-btn');
      expect(submitBtn).toHaveProperty('disabled', false);
    });

    it('주문 실패 시 에러 로그 및 토스트 표시 (AC #10)', async () => {
      useOrderStore.setState({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.01',
      });

      vi.mocked(invoke).mockResolvedValue({
        success: false,
        error: {
          code: 'insufficient_balance_bid',
          message: '매수 가능 금액이 부족합니다',
        },
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'ERROR',
          'ORDER',
          expect.stringContaining('주문 실패')
        );
        expect(mockShowToast).toHaveBeenCalledWith('error', expect.any(String));
      });
    });
  });

  describe('지정가 주문 즉시 체결 케이스', () => {
    it('즉시 체결(state: done) 시 "주문이 체결되었습니다" 토스트 표시', async () => {
      useOrderStore.setState({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.01',
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'limit-buy-order-done',
          side: 'bid',
          ord_type: 'limit',
          price: '50000000',
          state: 'done', // 즉시 체결
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.01',
          remaining_volume: '0',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '50',
          locked: '0',
          executed_volume: '0.01',
          trades_count: 1,
        },
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

      await waitFor(() => {
        expect(mockShowToast).toHaveBeenCalledWith('success', '주문이 체결되었습니다');
      });
    });
  });
});

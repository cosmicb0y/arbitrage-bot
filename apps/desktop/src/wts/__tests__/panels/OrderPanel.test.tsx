import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { OrderPanel } from '../../panels/OrderPanel';
import { useOrderStore } from '../../stores/orderStore';
import { useBalanceStore } from '../../stores/balanceStore';
import { useWtsStore } from '../../stores/wtsStore';
import { useConsoleStore } from '../../stores/consoleStore';
import { useToastStore } from '../../stores/toastStore';
import type { BalanceEntry } from '../../types';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock stores
vi.mock('../../stores/orderStore');
vi.mock('../../stores/balanceStore');
vi.mock('../../stores/wtsStore');
vi.mock('../../stores/consoleStore');
vi.mock('../../stores/toastStore');

describe('OrderPanel', () => {
  const mockSetOrderType = vi.fn();
  const mockSetSide = vi.fn();
  const mockSetPrice = vi.fn();
  const mockSetQuantity = vi.fn();

  const mockBalances: BalanceEntry[] = [
    {
      currency: 'KRW',
      balance: '1000000',
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

  const mockAddLog = vi.fn();
  const mockShowToast = vi.fn();
  const mockFetchBalance = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(useOrderStore).mockReturnValue({
      orderType: 'limit',
      side: 'buy',
      price: '',
      quantity: '',
      setOrderType: mockSetOrderType,
      setSide: mockSetSide,
      setPrice: mockSetPrice,
      setQuantity: mockSetQuantity,
      setPriceFromOrderbook: vi.fn(),
      resetForm: vi.fn(),
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

    // Mock consoleStore
    vi.mocked(useConsoleStore).mockImplementation((selector) => {
      if (typeof selector === 'function') {
        return selector({ logs: [], addLog: mockAddLog, clearLogs: vi.fn() });
      }
      return { logs: [], addLog: mockAddLog, clearLogs: vi.fn() };
    });
    vi.mocked(useConsoleStore.getState).mockReturnValue({
      logs: [],
      addLog: mockAddLog,
      clearLogs: vi.fn(),
    });

    // Mock toastStore
    vi.mocked(useToastStore).mockImplementation((selector) => {
      if (typeof selector === 'function') {
        return selector({ toasts: [], showToast: mockShowToast, removeToast: vi.fn(), clearToasts: vi.fn() });
      }
      return { toasts: [], showToast: mockShowToast, removeToast: vi.fn(), clearToasts: vi.fn() };
    });
    vi.mocked(useToastStore.getState).mockReturnValue({
      toasts: [],
      showToast: mockShowToast,
      removeToast: vi.fn(),
      clearToasts: vi.fn(),
    });
  });

  describe('Task 1: 주문 유형 탭 UI (AC: #1, #3, #4)', () => {
    it('지정가/시장가 탭이 표시되어야 한다 (AC #1)', () => {
      render(<OrderPanel />);

      expect(screen.getByRole('tab', { name: '지정가' })).toBeDefined();
      expect(screen.getByRole('tab', { name: '시장가' })).toBeDefined();
    });

    it('지정가 탭 클릭 시 setOrderType("limit")이 호출되어야 한다', () => {
      render(<OrderPanel />);

      fireEvent.click(screen.getByRole('tab', { name: '지정가' }));
      expect(mockSetOrderType).toHaveBeenCalledWith('limit');
    });

    it('시장가 탭 클릭 시 setOrderType("market")이 호출되어야 한다', () => {
      render(<OrderPanel />);

      fireEvent.click(screen.getByRole('tab', { name: '시장가' }));
      expect(mockSetOrderType).toHaveBeenCalledWith('market');
    });

    it('지정가 모드에서 가격 입력 필드가 활성화되어야 한다 (AC #3)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      const priceInput = screen.getByLabelText('가격');
      expect(priceInput).not.toHaveProperty('disabled', true);
    });

    it('시장가 매도 모드에서 가격 입력 필드가 비활성화되어야 한다 (AC #4)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'sell',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      const priceInput = screen.getByLabelText('가격');
      expect(priceInput).toHaveProperty('disabled', true);
    });

    it('시장가 매수 모드에서 가격 입력 필드가 활성화되어야 한다 (AC #4)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      const priceInput = screen.getByLabelText('주문 금액');
      expect(priceInput).toHaveProperty('disabled', false);
    });

    it('현재 선택된 탭이 강조 표시되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      const limitTab = screen.getByRole('tab', { name: '지정가' });
      expect(limitTab.getAttribute('aria-selected')).toBe('true');
    });
  });

  describe('Task 2: 가격 입력 UI (AC: #3, #4)', () => {
    it('가격 입력 시 setPrice가 호출되어야 한다', () => {
      render(<OrderPanel />);

      const priceInput = screen.getByLabelText('가격');
      fireEvent.change(priceInput, { target: { value: '50000000' } });
      expect(mockSetPrice).toHaveBeenCalledWith('50000000');
    });

    it('비숫자 문자가 입력되면 필터링되어야 한다 (AC #8)', () => {
      render(<OrderPanel />);

      const priceInput = screen.getByLabelText('가격');
      fireEvent.change(priceInput, { target: { value: 'abc123def' } });
      expect(mockSetPrice).toHaveBeenCalledWith('123');
    });

    it('음수 입력 시 가격이 설정되지 않아야 한다 (양수만 허용)', () => {
      render(<OrderPanel />);

      const priceInput = screen.getByLabelText('가격');
      fireEvent.change(priceInput, { target: { value: '-1' } });
      expect(mockSetPrice).toHaveBeenCalledWith('');
    });

    it('시장가 매도 모드에서 placeholder가 "시장가"여야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'sell',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      const priceInput = screen.getByLabelText('가격');
      expect(priceInput.getAttribute('placeholder')).toBe('시장가');
    });
  });

  describe('Task 3: 수량 입력 UI (AC: #2, #8)', () => {
    it('수량 입력 필드가 표시되어야 한다 (AC #2)', () => {
      render(<OrderPanel />);

      expect(screen.getByLabelText(/수량/)).toBeDefined();
    });

    it('수량 라벨에 코인 심볼이 표시되어야 한다', () => {
      render(<OrderPanel />);

      expect(screen.getByLabelText('수량 BTC')).toBeDefined();
    });

    it('수량 입력 시 setQuantity가 호출되어야 한다', () => {
      render(<OrderPanel />);

      const qtyInput = screen.getByLabelText('수량 BTC');
      fireEvent.change(qtyInput, { target: { value: '0.5' } });
      expect(mockSetQuantity).toHaveBeenCalledWith('0.5');
    });

    it('비숫자 문자가 입력되면 필터링되어야 한다 (AC #8)', () => {
      render(<OrderPanel />);

      const qtyInput = screen.getByLabelText('수량 BTC');
      fireEvent.change(qtyInput, { target: { value: '1.5abc' } });
      expect(mockSetQuantity).toHaveBeenCalledWith('1.5');
    });

    it('시장가 매수 모드에서도 수량 필드가 표시되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByLabelText('수량 BTC')).toBeDefined();
    });
  });

  describe('Task 4: % 버튼 UI (AC: #2, #6, #7)', () => {
    it('25%, 50%, 75%, MAX 버튼이 표시되어야 한다 (AC #2)', () => {
      render(<OrderPanel />);

      expect(screen.getByText('25%')).toBeDefined();
      expect(screen.getByText('50%')).toBeDefined();
      expect(screen.getByText('75%')).toBeDefined();
      expect(screen.getByText('MAX')).toBeDefined();
    });

    it('매수 모드에서 % 버튼 클릭 시 KRW 잔고 기준으로 수량 계산 (AC #6)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByText('50%'));
      // 1000000 * 0.5 / 50000000 = 0.01
      expect(mockSetQuantity).toHaveBeenCalledWith('0.01');
    });

    it('매도 모드에서 % 버튼 클릭 시 코인 잔고 기준으로 수량 설정 (AC #7)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'sell',
        price: '50000000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByText('50%'));
      // 0.5 * 0.5 = 0.25
      expect(mockSetQuantity).toHaveBeenCalledWith('0.25');
    });

    it('시장가 매수 모드에서 % 버튼 클릭 시 KRW 잔고 기준 수량 계산', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '50000000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByText('50%'));
      // 1000000 * 0.5 / 50000000 = 0.01
      expect(mockSetQuantity).toHaveBeenCalledWith('0.01');
    });
  });

  describe('Task 5: 예상 총액 표시 (AC: #5)', () => {
    it('예상 총액이 표시되어야 한다', () => {
      render(<OrderPanel />);

      expect(screen.getByText('예상 총액')).toBeDefined();
    });

    it('지정가 주문에서 예상 총액 = 수량 × 가격', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.01',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      // 50000000 * 0.01 = 500000 → ₩500,000
      expect(screen.getByText('₩500,000')).toBeDefined();
    });

    it('잔고 초과 시 경고가 표시되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '1', // 50000000 > 1000000 (잔고)
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByText('잔고 초과')).toBeDefined();
    });

    it('지정가 매수: 예상 총액 > KRW 잔고 시 "잔고 초과" 표시 (AC #7)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '60000000',
        quantity: '0.02', // 60000000 * 0.02 = 1,200,000 > 1,000,000 (잔고)
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByText('잔고 초과')).toBeDefined();
    });

    it('지정가 매도: 수량 > 코인 잔고 시 "잔고 초과" 표시 (AC #8)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'sell',
        price: '50000000',
        quantity: '0.6', // 0.6 > 0.5 (BTC 잔고)
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByText('잔고 초과')).toBeDefined();
    });

    it('지정가 매도: 수량 <= 코인 잔고 시 "잔고 초과" 미표시', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'sell',
        price: '50000000',
        quantity: '0.3', // 0.3 <= 0.5 (BTC 잔고)
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.queryByText('잔고 초과')).toBeNull();
    });

    it('잔고 초과 상태에서도 주문 버튼은 클릭 가능해야 한다 (거래소 API가 최종 검증)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '1', // 잔고 초과
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByText('잔고 초과')).toBeDefined();
      // 버튼은 비활성화되지 않음 (가격과 수량이 입력되어 있으므로)
      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', false);
    });

    it('시장가 매수에서 예상 총액이 주문 금액으로 표시되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '100000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByText('₩100,000')).toBeDefined();
    });
  });

  describe('Task 6: 매수/매도 버튼 UI (AC: #1)', () => {
    it('매수/매도 버튼이 표시되어야 한다', () => {
      render(<OrderPanel />);

      expect(screen.getByRole('button', { name: '매수' })).toBeDefined();
      expect(screen.getByRole('button', { name: '매도' })).toBeDefined();
    });

    it('매수 버튼 클릭 시 setSide("buy")가 호출되어야 한다', () => {
      render(<OrderPanel />);

      fireEvent.click(screen.getByRole('button', { name: '매수' }));
      expect(mockSetSide).toHaveBeenCalledWith('buy');
    });

    it('매도 버튼 클릭 시 setSide("sell")가 호출되어야 한다', () => {
      render(<OrderPanel />);

      fireEvent.click(screen.getByRole('button', { name: '매도' }));
      expect(mockSetSide).toHaveBeenCalledWith('sell');
    });

    it('매수 모드에서 매수 버튼이 녹색 강조되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      const buyButton = screen.getByRole('button', { name: '매수' });
      expect(buyButton.className).toContain('bg-green-600');
    });

    it('매도 모드에서 매도 버튼이 빨간색 강조되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'sell',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      const sellButton = screen.getByRole('button', { name: '매도' });
      expect(sellButton.className).toContain('bg-red-600');
    });
  });

  describe('WTS-3.3: 주문 제출 버튼 (AC: #1, #7)', () => {
    it('주문 제출 버튼이 표시되어야 한다', () => {
      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toBeDefined();
    });

    it('지정가 매수 모드에서 버튼 텍스트가 "지정가 매수"여야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.01',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn').textContent).toContain('지정가 매수');
    });

    it('시장가 매도 모드에서 버튼 텍스트가 "시장가 매도"여야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'sell',
        price: '',
        quantity: '0.01',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn').textContent).toContain('시장가 매도');
    });

    it('시장가 매수 모드에서 price가 없으면 버튼이 비활성화되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', true);
    });

    it('시장가 매도 모드에서 quantity가 없으면 버튼이 비활성화되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'sell',
        price: '',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', true);
    });

    it('시장가 매수 모드에서 price가 있으면 버튼이 활성화되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '100000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', false);
    });

    it('시장가 매도 모드에서 quantity가 있으면 버튼이 활성화되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'sell',
        price: '',
        quantity: '0.01',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', false);
    });

    it('지정가 모드에서 가격이 0이면 버튼이 비활성화되어야 한다 (AC #9)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '0',
        quantity: '0.01',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', true);
    });

    it('지정가 모드에서 수량이 0이면 버튼이 비활성화되어야 한다 (AC #9)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', true);
    });

    it('지정가 모드에서 가격이 비어있으면 버튼이 비활성화되어야 한다 (AC #9)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '',
        quantity: '0.01',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', true);
    });

    it('지정가 모드에서 수량이 비어있으면 버튼이 비활성화되어야 한다 (AC #9)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn')).toHaveProperty('disabled', true);
    });

    it('버튼 클릭 시 확인 다이얼로그가 표시되어야 한다 (AC #1)', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '100000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      expect(screen.getByRole('dialog')).toBeDefined();
      expect(screen.getByText('매수 주문 확인')).toBeDefined();
    });

    it('매수 버튼은 녹색이어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '100000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn').className).toContain('bg-green');
    });

    it('매도 버튼은 빨간색이어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'sell',
        price: '',
        quantity: '0.01',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      expect(screen.getByTestId('order-submit-btn').className).toContain('bg-red');
    });
  });

  describe('WTS-3.4: 지정가 주문 (AC: #3, #4, #5)', () => {
    it('지정가 주문 시 ord_type=limit, side=bid/ask, volume, price가 전달되어야 한다 (AC #3)', async () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'order-limit-1',
          side: 'bid',
          ord_type: 'limit',
          price: '50000000',
          state: 'wait',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.001',
          remaining_volume: '0.001',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '50050',
          executed_volume: '0',
          trades_count: 0,
        },
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

      await waitFor(() => {
        expect(vi.mocked(invoke)).toHaveBeenCalledWith('wts_place_order', {
          params: {
            market: 'KRW-BTC',
            side: 'bid',
            ord_type: 'limit',
            volume: '0.001',
            price: '50000000',
          },
        });
      });
    });

    it('지정가 주문 성공 시 콘솔에 INFO 로그가 기록되어야 한다 (AC #4)', async () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'order-limit-1',
          side: 'bid',
          ord_type: 'limit',
          price: '50000000',
          state: 'wait',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.001',
          remaining_volume: '0.001',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '50050',
          executed_volume: '0',
          trades_count: 0,
        },
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

      await waitFor(() => {
        // 주문 요청 로그
        expect(mockAddLog).toHaveBeenCalledWith(
          'INFO',
          'ORDER',
          expect.stringContaining('지정가 매수 주문 요청')
        );
      });
    });

    it('지정가 주문 응답 state가 wait이면 "주문이 등록되었습니다" 토스트가 표시되어야 한다 (AC #5)', async () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'order-limit-1',
          side: 'bid',
          ord_type: 'limit',
          price: '50000000',
          state: 'wait',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.001',
          remaining_volume: '0.001',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '50050',
          executed_volume: '0',
          trades_count: 0,
        },
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

      await waitFor(() => {
        expect(mockShowToast).toHaveBeenCalledWith('success', '주문이 등록되었습니다');
      });
    });

    it('지정가 주문 응답 state가 done이면 "주문이 체결되었습니다" 토스트가 표시되어야 한다', async () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'order-limit-1',
          side: 'bid',
          ord_type: 'limit',
          price: '50000000',
          state: 'done',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.001',
          remaining_volume: '0',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '50',
          locked: '0',
          executed_volume: '0.001',
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

    it('지정가 매도 주문 시 콘솔 로그 포맷이 올바르게 표시되어야 한다', async () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'sell',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'order-limit-2',
          side: 'ask',
          ord_type: 'limit',
          price: '50000000',
          state: 'wait',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.001',
          remaining_volume: '0.001',
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '0.001',
          executed_volume: '0',
          trades_count: 0,
        },
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매도' }));

      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'INFO',
          'ORDER',
          expect.stringContaining('지정가 매도 주문 요청')
        );
      });
    });

    it('지정가 주문 실패 시 콘솔에 ERROR 로그가 기록되고 토스트에 에러 메시지가 표시되어야 한다 (AC #10)', async () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: false,
        error: {
          code: 'insufficient_balance',
          message: '잔액이 부족합니다',
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
          expect.stringContaining('주문 실패'),
          expect.anything()
        );
        expect(mockShowToast).toHaveBeenCalledWith('error', expect.any(String));
      });
    });
  });

  describe('WTS-3.5: 확인 다이얼로그 취소 시 폼 상태 유지 (AC: #10)', () => {
    it('취소 버튼 클릭 후 폼 입력값이 유지되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      // 주문 버튼 클릭하여 다이얼로그 표시
      fireEvent.click(screen.getByTestId('order-submit-btn'));

      expect(screen.getByRole('dialog')).toBeDefined();

      // 취소 버튼 클릭
      fireEvent.click(screen.getByRole('button', { name: '취소' }));

      // 다이얼로그 닫힘
      expect(screen.queryByRole('dialog')).toBeNull();

      // 폼 리셋 함수가 호출되지 않아야 함 (폼 상태 유지)
      expect(mockSetPrice).not.toHaveBeenCalled();
      expect(mockSetQuantity).not.toHaveBeenCalled();
    });

    it('ESC 키로 취소 후 폼 입력값이 유지되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      // 주문 버튼 클릭하여 다이얼로그 표시
      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      expect(dialog).toBeDefined();

      // ESC 키로 취소
      fireEvent.keyDown(dialog, { key: 'Escape' });

      // 다이얼로그 닫힘
      expect(screen.queryByRole('dialog')).toBeNull();

      // 폼 리셋 함수가 호출되지 않아야 함 (폼 상태 유지)
      expect(mockSetPrice).not.toHaveBeenCalled();
      expect(mockSetQuantity).not.toHaveBeenCalled();
    });

    it('오버레이 클릭으로 취소 후 폼 입력값이 유지되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'limit',
        side: 'buy',
        price: '50000000',
        quantity: '0.001',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      render(<OrderPanel />);

      // 주문 버튼 클릭하여 다이얼로그 표시
      fireEvent.click(screen.getByTestId('order-submit-btn'));

      expect(screen.getByRole('dialog')).toBeDefined();

      // 오버레이 클릭으로 취소
      const overlay = screen.getByTestId('dialog-overlay');
      fireEvent.click(overlay);

      // 다이얼로그 닫힘
      expect(screen.queryByRole('dialog')).toBeNull();

      // 폼 리셋 함수가 호출되지 않아야 함 (폼 상태 유지)
      expect(mockSetPrice).not.toHaveBeenCalled();
      expect(mockSetQuantity).not.toHaveBeenCalled();
    });
  });

  describe('WTS-3.3: 주문 파라미터 빌드 (AC: #5, #6)', () => {
    it('시장가 매수 주문 시 ord_type=price, side=bid, price가 전달되어야 한다', async () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'buy',
        price: '100000',
        quantity: '',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'order-1',
          side: 'bid',
          ord_type: 'price',
          price: '100000',
          state: 'done',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: null,
          remaining_volume: null,
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '0',
          executed_volume: '0.001',
          trades_count: 1,
        },
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

      await waitFor(() => {
        expect(vi.mocked(invoke)).toHaveBeenCalledWith('wts_place_order', {
          params: {
            market: 'KRW-BTC',
            side: 'bid',
            ord_type: 'price',
            price: '100000',
          },
        });
      });
    });

    it('시장가 매도 주문 시 ord_type=market, side=ask, volume이 전달되어야 한다', async () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'sell',
        price: '',
        quantity: '0.01',
        setOrderType: mockSetOrderType,
        setSide: mockSetSide,
        setPrice: mockSetPrice,
        setQuantity: mockSetQuantity,
        setPriceFromOrderbook: vi.fn(),
        resetForm: vi.fn(),
      });

      vi.mocked(invoke).mockResolvedValue({
        success: true,
        data: {
          uuid: 'order-2',
          side: 'ask',
          ord_type: 'market',
          price: null,
          state: 'done',
          market: 'KRW-BTC',
          created_at: '2026-01-19T00:00:00Z',
          volume: '0.01',
          remaining_volume: null,
          reserved_fee: '0',
          remaining_fee: '0',
          paid_fee: '0',
          locked: '0',
          executed_volume: '0.01',
          trades_count: 1,
        },
      });

      render(<OrderPanel />);

      fireEvent.click(screen.getByTestId('order-submit-btn'));

      const dialog = screen.getByRole('dialog');
      fireEvent.click(within(dialog).getByRole('button', { name: '매도' }));

      await waitFor(() => {
        expect(vi.mocked(invoke)).toHaveBeenCalledWith('wts_place_order', {
          params: {
            market: 'KRW-BTC',
            side: 'ask',
            ord_type: 'market',
            volume: '0.01',
          },
        });
      });
    });
  });

  describe('WTS-3.6: 콘솔 로그 완성 (AC: #1, #2, #3, #6, #8)', () => {
    describe('주문 요청 로깅 (AC: #8)', () => {
      it('시장가 매수 주문 요청 시 INFO 로그에 마켓과 금액이 포함되어야 한다', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'market',
          side: 'buy',
          price: '100000',
          quantity: '',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: true,
          data: { uuid: 'test', state: 'done', executed_volume: '0.001' },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

        await waitFor(() => {
          expect(mockAddLog).toHaveBeenCalledWith(
            'INFO',
            'ORDER',
            expect.stringMatching(/시장가 매수 주문 요청.*KRW-BTC.*₩100,000/)
          );
        });
      });

      it('시장가 매도 주문 요청 시 INFO 로그에 마켓과 수량이 포함되어야 한다', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'market',
          side: 'sell',
          price: '',
          quantity: '0.01',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: true,
          data: { uuid: 'test', state: 'done', executed_volume: '0.01' },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매도' }));

        await waitFor(() => {
          expect(mockAddLog).toHaveBeenCalledWith(
            'INFO',
            'ORDER',
            expect.stringMatching(/시장가 매도 주문 요청.*KRW-BTC.*0\.01.*BTC/)
          );
        });
      });
    });

    describe('주문 성공 로깅 (AC: #2, #6)', () => {
      it('시장가 주문 성공 시 SUCCESS 로그에 주문 유형이 포함되어야 한다', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'market',
          side: 'buy',
          price: '100000',
          quantity: '',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: true,
          data: { uuid: 'test', state: 'done', executed_volume: '0.001' },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

        await waitFor(() => {
          expect(mockAddLog).toHaveBeenCalledWith(
            'SUCCESS',
            'ORDER',
            expect.stringContaining('[시장가]')
          );
        });
      });

      it('지정가 주문 성공 시 SUCCESS 로그에 주문 유형이 포함되어야 한다', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'limit',
          side: 'buy',
          price: '50000000',
          quantity: '0.001',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: true,
          data: { uuid: 'test', state: 'wait', volume: '0.001' },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

        await waitFor(() => {
          expect(mockAddLog).toHaveBeenCalledWith(
            'SUCCESS',
            'ORDER',
            expect.stringContaining('[지정가]')
          );
        });
      });

      it('주문 성공 로그에 수량, 코인, 가격 정보가 포함되어야 한다 (AC #6)', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'limit',
          side: 'sell',
          price: '60000000',
          quantity: '0.05',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: true,
          data: { uuid: 'test', state: 'done', executed_volume: '0.05' },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매도' }));

        await waitFor(() => {
          expect(mockAddLog).toHaveBeenCalledWith(
            'SUCCESS',
            'ORDER',
            expect.stringMatching(/KRW-BTC.*0\.05.*BTC.*₩60,000,000/)
          );
        });
      });
    });

    describe('주문 실패 로깅 (AC: #3)', () => {
      it('주문 실패 시 ERROR 로그가 기록되어야 한다', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'market',
          side: 'buy',
          price: '100000',
          quantity: '',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: false,
          error: { code: 'insufficient_funds_bid', message: '잔액 부족' },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

        await waitFor(() => {
          expect(mockAddLog).toHaveBeenCalledWith(
            'ERROR',
            'ORDER',
            expect.stringContaining('주문 실패'),
            expect.anything()
          );
        });
      });

      it('네트워크 오류 시 ERROR 로그가 기록되어야 한다', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'market',
          side: 'buy',
          price: '100000',
          quantity: '',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockRejectedValue(new Error('Network error'));

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

        await waitFor(() => {
          expect(mockAddLog).toHaveBeenCalledWith(
            'ERROR',
            'ORDER',
            expect.stringContaining('네트워크'),
            expect.anything()
          );
        });
      });
    });

    describe('주문 취소 로깅', () => {
it('Rate Limit 에러 시 INFO 로그에 재시도 안내가 포함되어야 한다 (AC: WTS-3.7 #4, #5)', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'market',
          side: 'buy',
          price: '100000',
          quantity: '',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: false,
          error: {
            code: 'rate_limit',
            message: '요청이 너무 많습니다',
            detail: { remaining_req: 'group=order; sec=2' },
          },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

        await waitFor(() => {
          // Rate Limit 에러 메시지가 토스트에 표시되어야 함
          expect(mockShowToast).toHaveBeenCalledWith(
            'error',
            expect.stringContaining('주문 요청')
          );
          // Remaining-Req 헤더 정보가 로그에 포함되어야 함
          expect(mockAddLog).toHaveBeenCalledWith(
            'INFO',
            'ORDER',
            expect.stringContaining('Remaining-Req: group=order; sec=2')
          );
          // 재시도 안내 INFO 로그가 추가되어야 함
          expect(mockAddLog).toHaveBeenCalledWith(
            'INFO',
            'ORDER',
            expect.stringContaining('초당 8회')
          );
        });
      });

      it('네트워크 에러 시 네트워크 확인 안내가 표시되어야 한다 (AC: WTS-3.7 #6)', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'market',
          side: 'buy',
          price: '100000',
          quantity: '',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: false,
          error: { code: 'network_error', message: 'Connection failed' },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

        await waitFor(() => {
          // 네트워크 에러 메시지가 토스트에 표시되어야 함
          expect(mockShowToast).toHaveBeenCalledWith('error', expect.stringContaining('네트워크'));
        });
      });

      it('주문 취소 시 WARN 로그가 기록되어야 한다', async () => {
        vi.mocked(useOrderStore).mockReturnValue({
          orderType: 'limit',
          side: 'buy',
          price: '50000000',
          quantity: '0.001',
          setOrderType: mockSetOrderType,
          setSide: mockSetSide,
          setPrice: mockSetPrice,
          setQuantity: mockSetQuantity,
          setPriceFromOrderbook: vi.fn(),
          resetForm: vi.fn(),
        });

        vi.mocked(invoke).mockResolvedValue({
          success: true,
          data: { uuid: 'test', state: 'cancel', volume: '0.001' },
        });

        render(<OrderPanel />);

        fireEvent.click(screen.getByTestId('order-submit-btn'));
        const dialog = screen.getByRole('dialog');
        fireEvent.click(within(dialog).getByRole('button', { name: '매수' }));

        await waitFor(() => {
          expect(mockAddLog).toHaveBeenCalledWith(
            'WARN',
            'ORDER',
            expect.stringContaining('취소')
          );
        });
      });
    });
  });
});

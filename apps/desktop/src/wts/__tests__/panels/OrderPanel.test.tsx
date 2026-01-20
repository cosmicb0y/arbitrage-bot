import { render, screen, fireEvent } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { OrderPanel } from '../../panels/OrderPanel';
import { useOrderStore } from '../../stores/orderStore';
import { useBalanceStore } from '../../stores/balanceStore';
import { useWtsStore } from '../../stores/wtsStore';
import type { BalanceEntry } from '../../types';

// Mock stores
vi.mock('../../stores/orderStore');
vi.mock('../../stores/balanceStore');
vi.mock('../../stores/wtsStore');

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
      fetchBalance: vi.fn(),
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

    it('시장가 매수 모드에서 가격 입력 필드가 비활성화되어야 한다 (AC #4)', () => {
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

      const priceInput = screen.getByLabelText('가격');
      expect(priceInput).toHaveProperty('disabled', true);
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

    it('시장가 매도에서도 예상 총액이 계산되어 표시되어야 한다', () => {
      vi.mocked(useOrderStore).mockReturnValue({
        orderType: 'market',
        side: 'sell',
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

      expect(screen.getByText('₩500,000')).toBeDefined();
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
});

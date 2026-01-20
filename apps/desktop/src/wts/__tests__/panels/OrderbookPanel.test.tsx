import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { OrderbookPanel } from '../../panels/OrderbookPanel';
import { useWtsStore } from '../../stores/wtsStore';
import { useOrderbookStore } from '../../stores/orderbookStore';
import { useOrderStore } from '../../stores/orderStore';
import { useConsoleStore } from '../../stores/consoleStore';
import { UPBIT_DEFAULT_MARKETS } from '../../types';

// Mock stores
vi.mock('../../stores/wtsStore');
vi.mock('../../stores/orderbookStore');
vi.mock('../../stores/orderStore');
vi.mock('../../stores/consoleStore');

// Mock useUpbitOrderbookWs hook
vi.mock('../../hooks/useUpbitOrderbookWs', () => ({
  useUpbitOrderbookWs: vi.fn(),
}));

describe('OrderbookPanel', () => {
  const mockSetMarket = vi.fn();
  const mockSetPriceFromOrderbook = vi.fn();
  const mockAddLog = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(useWtsStore).mockReturnValue({
      selectedExchange: 'upbit',
      selectedMarket: null,
      setMarket: mockSetMarket,
      connectionStatus: 'connected',
      availableMarkets: UPBIT_DEFAULT_MARKETS,
    } as ReturnType<typeof useWtsStore>);

    vi.mocked(useOrderbookStore).mockReturnValue({
      asks: [],
      bids: [],
      timestamp: null,
      wsStatus: 'disconnected',
      wsError: null,
      setOrderbook: vi.fn(),
      clearOrderbook: vi.fn(),
      setWsStatus: vi.fn(),
      setWsError: vi.fn(),
    });

    vi.mocked(useOrderStore).mockImplementation((selector) => {
      const state = {
        orderType: 'limit' as const,
        side: 'buy' as const,
        price: '',
        quantity: '',
        setOrderType: vi.fn(),
        setSide: vi.fn(),
        setPrice: vi.fn(),
        setQuantity: vi.fn(),
        setPriceFromOrderbook: mockSetPriceFromOrderbook,
        resetForm: vi.fn(),
      };
      return selector ? selector(state) : state;
    });

    vi.mocked(useConsoleStore).mockImplementation((selector) => {
      const state = {
        logs: [],
        addLog: mockAddLog,
        clearLogs: vi.fn(),
      };
      return selector ? selector(state) : state;
    });
  });

  describe('렌더링', () => {
    it('data-testid orderbook-panel이 렌더링된다', () => {
      render(<OrderbookPanel />);
      expect(screen.getByTestId('orderbook-panel')).toBeTruthy();
    });

    it('헤더에 Orderbook 텍스트가 표시된다', () => {
      render(<OrderbookPanel />);
      expect(screen.getByText('Orderbook')).toBeTruthy();
    });

    it('MarketSelector가 헤더에 렌더링된다', () => {
      render(<OrderbookPanel />);
      // MarketSelector의 버튼이 렌더링됨
      expect(screen.getByRole('button')).toBeTruthy();
    });
  });

  describe('마켓 미선택 상태', () => {
    it('마켓이 선택되지 않았을 때 안내 메시지가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: null,
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      render(<OrderbookPanel />);
      expect(screen.getByText('마켓을 선택하세요')).toBeTruthy();
    });

    it('MarketSelector 버튼에 "마켓 선택" 텍스트가 표시된다', () => {
      render(<OrderbookPanel />);
      expect(screen.getByText('마켓 선택')).toBeTruthy();
    });
  });

  describe('마켓 선택 상태', () => {
    it('선택된 마켓 코드가 MarketSelector에 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [],
        bids: [],
        timestamp: null,
        wsStatus: 'connecting',
        wsError: null,
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      });

      render(<OrderbookPanel />);
      expect(screen.getByText('KRW-BTC')).toBeTruthy();
    });

    it('wsStatus가 connecting일 때 "연결 중..." 메시지가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [],
        bids: [],
        timestamp: null,
        wsStatus: 'connecting',
        wsError: null,
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      });

      render(<OrderbookPanel />);
      expect(screen.getByText('연결 중...')).toBeTruthy();
    });

    it('오더북 데이터가 없을 때 "데이터 대기 중..." 메시지가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

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

      render(<OrderbookPanel />);
      expect(screen.getByText('데이터 대기 중...')).toBeTruthy();
    });

    it('wsError가 있을 때 에러 메시지가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [],
        bids: [],
        timestamp: null,
        wsStatus: 'disconnected',
        wsError: '연결 오류',
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      });

      render(<OrderbookPanel />);
      expect(screen.getByText('연결 오류')).toBeTruthy();
    });
  });

  describe('오더북 데이터 표시', () => {
    it('오더북 데이터가 있을 때 매도/매수 호가가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [
          { price: 50100000, size: 0.5 },
          { price: 50200000, size: 1.0 },
        ],
        bids: [
          { price: 50000000, size: 0.8 },
          { price: 49900000, size: 1.2 },
        ],
        timestamp: 1704067200000,
        wsStatus: 'connected',
        wsError: null,
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      });

      render(<OrderbookPanel />);

      // 가격이 표시되는지 확인 (천 단위 콤마)
      expect(screen.getByText('50,100,000')).toBeTruthy();
      expect(screen.getByText('50,200,000')).toBeTruthy();
      expect(screen.getByText('50,000,000')).toBeTruthy();
      expect(screen.getByText('49,900,000')).toBeTruthy();
    });

    it('수량이 올바른 포맷으로 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

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

      render(<OrderbookPanel />);

      expect(screen.getByText('0.5')).toBeTruthy();
      expect(screen.getByText('0.8')).toBeTruthy();
    });

    it('15개 이상의 호가가 있을 때 15개만 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      const asks = Array.from({ length: 20 }, (_, i) => ({
        price: 50100000 + i * 100000,
        size: 0.5,
      }));
      const bids = Array.from({ length: 20 }, (_, i) => ({
        price: 50000000 - i * 100000,
        size: 0.8,
      }));

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks,
        bids,
        timestamp: 1704067200000,
        wsStatus: 'connected',
        wsError: null,
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      });

      render(<OrderbookPanel />);

      // 총 30개의 행이 아닌 30개(15 ask + 15 bid)만 표시되어야 함
      const rows = screen.getAllByText(/,000$/);
      expect(rows.length).toBeLessThanOrEqual(30);
    });
  });

  describe('가격 변동 플래시 애니메이션', () => {
    it('가격 상승 시 플래시 클래스가 적용된다', async () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      const baseState = {
        bids: [{ price: 50000000, size: 0.8 }],
        timestamp: 1704067200000,
        wsStatus: 'connected',
        wsError: null,
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      };

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [{ price: 50100000, size: 0.5 }],
        ...baseState,
      });

      const { rerender } = render(<OrderbookPanel />);

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [{ price: 50200000, size: 0.5 }],
        ...baseState,
      });

      rerender(<OrderbookPanel />);

      await waitFor(() => {
        const row = screen
          .getByText('50,200,000')
          .closest('[role="button"]');
        expect(row?.classList.contains('animate-flash-up')).toBe(true);
      });
    });
  });

  describe('WebSocket 연결 상태 인디케이터', () => {
    it('connected 상태에서 녹색 인디케이터가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

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

      render(<OrderbookPanel />);
      const indicator = document.querySelector('.bg-success');
      expect(indicator).toBeTruthy();
    });

    it('connecting 상태에서 warning 인디케이터가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [],
        bids: [],
        timestamp: null,
        wsStatus: 'connecting',
        wsError: null,
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      });

      render(<OrderbookPanel />);
      const indicator = document.querySelector('.bg-warning');
      expect(indicator).toBeTruthy();
    });

    it('disconnected 상태에서 빨간색 인디케이터가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      vi.mocked(useOrderbookStore).mockReturnValue({
        asks: [],
        bids: [],
        timestamp: null,
        wsStatus: 'disconnected',
        wsError: null,
        setOrderbook: vi.fn(),
        clearOrderbook: vi.fn(),
        setWsStatus: vi.fn(),
        setWsError: vi.fn(),
      });

      render(<OrderbookPanel />);
      const indicator = document.querySelector('.bg-destructive');
      expect(indicator).toBeTruthy();
    });
  });

  describe('MarketSelector 통합', () => {
    it('마켓 선택 시 setMarket이 호출된다', () => {
      render(<OrderbookPanel />);

      // MarketSelector 버튼 클릭하여 드롭다운 열기
      fireEvent.click(screen.getByRole('button'));

      // 마켓 옵션 클릭
      const btcOption = screen.getByText('KRW-BTC').closest('li');
      fireEvent.click(btcOption!);

      expect(mockSetMarket).toHaveBeenCalledWith('KRW-BTC');
    });
  });

  describe('연결 상태에 따른 동작', () => {
    it('disconnected 상태에서 MarketSelector가 비활성화된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: null,
        setMarket: mockSetMarket,
        connectionStatus: 'disconnected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      render(<OrderbookPanel />);

      const button = screen.getByRole('button');
      expect(button).toHaveProperty('disabled', true);
    });

    it('connecting 상태에서 MarketSelector가 비활성화된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: null,
        setMarket: mockSetMarket,
        connectionStatus: 'connecting',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      render(<OrderbookPanel />);

      const button = screen.getByRole('button');
      expect(button).toHaveProperty('disabled', true);
    });

    it('connected 상태에서 MarketSelector가 활성화된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: null,
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      render(<OrderbookPanel />);

      const button = screen.getByRole('button');
      expect(button).toHaveProperty('disabled', false);
    });
  });

  describe('스타일링', () => {
    it('wts-panel 클래스가 적용된다', () => {
      render(<OrderbookPanel />);
      const panel = screen.getByTestId('orderbook-panel');
      expect(panel.classList.contains('wts-panel')).toBe(true);
    });

    it('wts-area-orderbook 클래스가 적용된다', () => {
      render(<OrderbookPanel />);
      const panel = screen.getByTestId('orderbook-panel');
      expect(panel.classList.contains('wts-area-orderbook')).toBe(true);
    });

    it('className prop이 적용된다', () => {
      render(<OrderbookPanel className="custom-class" />);
      const panel = screen.getByTestId('orderbook-panel');
      expect(panel.classList.contains('custom-class')).toBe(true);
    });
  });

  describe('호가 클릭 상호작용', () => {
    it('매도 호가(ask) 클릭 시 setPriceFromOrderbook이 올바른 인자로 호출된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

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

      render(<OrderbookPanel />);

      // 매도 호가(ask) 행 클릭
      const askRow = screen.getByText('50,100,000').closest('[role="button"]');
      fireEvent.click(askRow!);

      expect(mockSetPriceFromOrderbook).toHaveBeenCalledWith(50100000, 'ask');
      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'ORDER',
        expect.stringContaining('호가 선택')
      );
    });

    it('매수 호가(bid) 클릭 시 setPriceFromOrderbook이 올바른 인자로 호출된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

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

      render(<OrderbookPanel />);

      // 매수 호가(bid) 행 클릭
      const bidRow = screen.getByText('50,000,000').closest('[role="button"]');
      fireEvent.click(bidRow!);

      expect(mockSetPriceFromOrderbook).toHaveBeenCalledWith(50000000, 'bid');
      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'ORDER',
        expect.stringContaining('호가 선택')
      );
    });

    it('호가 행이 클릭 가능한 role="button"을 가진다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

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

      render(<OrderbookPanel />);

      // role="button"을 가진 요소 확인
      const rows = screen.getAllByRole('button');
      // MarketSelector 버튼(1) + 호가 행 2개 = 최소 3개
      expect(rows.length).toBeGreaterThanOrEqual(3);
    });

    it('호가 행에 호버 스타일(cursor-pointer)이 적용된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

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

      render(<OrderbookPanel />);

      const askRow = screen.getByText('50,100,000').closest('[role="button"]');
      expect(askRow?.classList.contains('cursor-pointer')).toBe(true);
    });

    it('키보드 Enter 키로 호가 행을 선택할 수 있다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

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

      render(<OrderbookPanel />);

      const askRow = screen.getByText('50,100,000').closest('[role="button"]');
      fireEvent.keyDown(askRow!, { key: 'Enter' });

      expect(mockSetPriceFromOrderbook).toHaveBeenCalledWith(50100000, 'ask');
    });
  });
});

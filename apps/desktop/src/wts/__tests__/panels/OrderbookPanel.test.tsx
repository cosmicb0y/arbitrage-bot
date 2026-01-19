import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { OrderbookPanel } from '../../panels/OrderbookPanel';
import { useWtsStore } from '../../stores/wtsStore';
import { useOrderbookStore } from '../../stores/orderbookStore';
import { UPBIT_DEFAULT_MARKETS } from '../../types';

// Mock stores
vi.mock('../../stores/wtsStore');
vi.mock('../../stores/orderbookStore');

// Mock useUpbitOrderbookWs hook
vi.mock('../../hooks/useUpbitOrderbookWs', () => ({
  useUpbitOrderbookWs: vi.fn(),
}));

describe('OrderbookPanel', () => {
  const mockSetMarket = vi.fn();

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
});

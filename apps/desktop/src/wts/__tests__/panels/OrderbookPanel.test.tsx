import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { OrderbookPanel } from '../../panels/OrderbookPanel';
import { useWtsStore } from '../../stores/wtsStore';
import { UPBIT_DEFAULT_MARKETS } from '../../types';

// Mock wtsStore
vi.mock('../../stores/wtsStore');

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
    it('마켓이 선택되었을 때 오더북 플레이스홀더가 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      render(<OrderbookPanel />);
      expect(screen.getByText(/오더북 데이터/)).toBeTruthy();
    });

    it('선택된 마켓 코드가 MarketSelector에 표시된다', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        selectedMarket: 'KRW-BTC',
        setMarket: mockSetMarket,
        connectionStatus: 'connected',
        availableMarkets: UPBIT_DEFAULT_MARKETS,
      } as ReturnType<typeof useWtsStore>);

      render(<OrderbookPanel />);
      expect(screen.getByText('KRW-BTC')).toBeTruthy();
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

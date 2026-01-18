import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { WtsWindow } from '../WtsWindow';
import { useWtsStore } from '../stores';

// Mock the store
vi.mock('../stores', () => ({
  useWtsStore: vi.fn(),
}));

describe('WtsWindow', () => {
  beforeEach(() => {
    vi.mocked(useWtsStore).mockReturnValue({
      selectedExchange: 'upbit',
      connectionStatus: 'disconnected',
      selectedMarket: null,
      setExchange: vi.fn(),
      setMarket: vi.fn(),
      setConnectionStatus: vi.fn(),
    });
  });

  describe('6-Panel Grid Layout', () => {
    it('renders the wts-grid container', () => {
      render(<WtsWindow />);
      const grid = document.querySelector('.wts-grid');
      expect(grid).toBeTruthy();
    });

    it('renders all 6 panels', () => {
      render(<WtsWindow />);
      expect(screen.getByTestId('exchange-panel')).toBeTruthy();
      expect(screen.getByTestId('console-panel')).toBeTruthy();
      expect(screen.getByTestId('orderbook-panel')).toBeTruthy();
      expect(screen.getByTestId('balance-panel')).toBeTruthy();
      expect(screen.getByTestId('order-panel')).toBeTruthy();
      expect(screen.getByTestId('open-orders-panel')).toBeTruthy();
    });

    it('applies dark theme background', () => {
      render(<WtsWindow />);
      const grid = document.querySelector('.wts-grid');
      expect(grid?.classList.contains('bg-wts-background')).toBeTruthy();
    });

    it('renders header in header grid area', () => {
      render(<WtsWindow />);
      const header = screen.getByTestId('exchange-panel');
      expect(header.classList.contains('wts-area-header')).toBeTruthy();
    });

    it('renders console panel in console grid area', () => {
      render(<WtsWindow />);
      const consolePanel = screen.getByTestId('console-panel');
      expect(consolePanel.classList.contains('wts-area-console')).toBeTruthy();
    });

    it('renders orderbook panel in orderbook grid area', () => {
      render(<WtsWindow />);
      const orderbook = screen.getByTestId('orderbook-panel');
      expect(orderbook.classList.contains('wts-area-orderbook')).toBeTruthy();
    });

    it('renders balance panel in balances grid area', () => {
      render(<WtsWindow />);
      const balance = screen.getByTestId('balance-panel');
      expect(balance.classList.contains('wts-area-balances')).toBeTruthy();
    });

    it('renders order panel in order grid area', () => {
      render(<WtsWindow />);
      const order = screen.getByTestId('order-panel');
      expect(order.classList.contains('wts-area-order')).toBeTruthy();
    });

    it('renders open orders panel in openOrders grid area', () => {
      render(<WtsWindow />);
      const openOrders = screen.getByTestId('open-orders-panel');
      expect(openOrders.classList.contains('wts-area-openOrders')).toBeTruthy();
    });
  });

  describe('Exchange Panel (Header)', () => {
    it('displays WTS title', () => {
      render(<WtsWindow />);
      expect(screen.getByText('WTS')).toBeTruthy();
    });

    it('displays selected exchange', () => {
      render(<WtsWindow />);
      expect(screen.getByText(/upbit/i)).toBeTruthy();
    });

    it('displays connection status indicator', () => {
      render(<WtsWindow />);
      expect(screen.getByTestId('connection-status')).toBeTruthy();
    });

    it('shows connected status when connected', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connected',
        selectedMarket: null,
        setExchange: vi.fn(),
        setMarket: vi.fn(),
        setConnectionStatus: vi.fn(),
      });
      render(<WtsWindow />);
      const status = screen.getByTestId('connection-status');
      expect(status.textContent).toContain('연결됨');
    });

    it('shows disconnected status when disconnected', () => {
      render(<WtsWindow />);
      const status = screen.getByTestId('connection-status');
      expect(status.textContent).toContain('연결 안됨');
    });
  });

  describe('Panel Placeholders', () => {
    it('console panel shows placeholder text', () => {
      render(<WtsWindow />);
      expect(screen.getByText('Console')).toBeTruthy();
    });

    it('orderbook panel shows placeholder text', () => {
      render(<WtsWindow />);
      expect(screen.getByText('Orderbook')).toBeTruthy();
    });

    it('balance panel shows placeholder text', () => {
      render(<WtsWindow />);
      expect(screen.getByText('Balances')).toBeTruthy();
    });

    it('order panel shows placeholder text', () => {
      render(<WtsWindow />);
      expect(screen.getByText('Order')).toBeTruthy();
    });

    it('open orders panel shows placeholder text', () => {
      render(<WtsWindow />);
      expect(screen.getByText('Open Orders')).toBeTruthy();
    });
  });
});

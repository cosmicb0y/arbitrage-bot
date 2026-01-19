import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/react';
import { ExchangePanel } from '../../panels/ExchangePanel';
import { useWtsStore } from '../../stores';
import { useConsoleStore } from '../../stores/consoleStore';

// Mock the stores
vi.mock('../../stores', () => ({
  useWtsStore: vi.fn(),
}));

vi.mock('../../stores/consoleStore', () => ({
  useConsoleStore: vi.fn(),
}));

describe('ExchangePanel', () => {
  const mockSetExchange = vi.fn();
  const mockAddLog = vi.fn();

  beforeEach(() => {
    vi.mocked(useWtsStore).mockReturnValue({
      enabledExchanges: ['upbit'],
      setEnabledExchanges: vi.fn(),
      selectedExchange: 'upbit',
      connectionStatus: 'connected',
      selectedMarket: null,
      setExchange: mockSetExchange,
      setMarket: vi.fn(),
      setConnectionStatus: vi.fn(),
      lastConnectionError: null,
      setConnectionError: vi.fn(),
    });

    vi.mocked(useConsoleStore).mockReturnValue({
      logs: [],
      addLog: mockAddLog,
      clearLogs: vi.fn(),
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Tab Rendering (Subtask 1.1)', () => {
    it('should render all 6 exchange tabs', () => {
      render(<ExchangePanel />);
      expect(screen.getByText('UP')).toBeTruthy();
      expect(screen.getByText('BT')).toBeTruthy();
      expect(screen.getByText('BN')).toBeTruthy();
      expect(screen.getByText('CB')).toBeTruthy();
      expect(screen.getByText('BY')).toBeTruthy();
      expect(screen.getByText('GT')).toBeTruthy();
    });

    it('should render tabs in correct order', () => {
      render(<ExchangePanel />);
      const tabs = screen.getAllByRole('button');
      const exchangeTabs = tabs.filter((tab) =>
        ['UP', 'BT', 'BN', 'CB', 'BY', 'GT'].some((key) =>
          (tab.textContent || '').includes(key)
        )
      );
      const labels = exchangeTabs.map((tab) =>
        (tab.textContent || '').replace('Coming Soon', '').trim()
      );
      expect(labels).toEqual(['UP', 'BT', 'BN', 'CB', 'BY', 'GT']);
    });
  });

  describe('Active Tab Style (Subtask 1.2)', () => {
    it('should highlight the active tab (Upbit)', () => {
      render(<ExchangePanel />);
      const upbitTab = screen.getByText('UP').closest('button');
      expect(upbitTab?.className).toContain('text-wts-foreground');
      expect(upbitTab?.className).toContain('border-wts-accent');
    });

    it('should not highlight inactive tabs with active style', () => {
      render(<ExchangePanel />);
      const bithumbTab = screen.getByText('BT').closest('button');
      // Inactive tabs have muted text (not active foreground with border)
      expect(bithumbTab?.className).toContain('text-wts-muted');
      expect(bithumbTab?.className).not.toContain('border-wts-accent');
    });
  });

  describe('Disabled Tab Style (Subtask 1.3)', () => {
    it('should disable non-MVP exchanges (MVP: Upbit only)', () => {
      render(<ExchangePanel />);
      const bithumbTab = screen.getByText('BT').closest('button');
      const binanceTab = screen.getByText('BN').closest('button');
      const coinbaseTab = screen.getByText('CB').closest('button');
      const bybitTab = screen.getByText('BY').closest('button');
      const gateioTab = screen.getByText('GT').closest('button');

      expect(bithumbTab?.hasAttribute('disabled')).toBe(true);
      expect(binanceTab?.hasAttribute('disabled')).toBe(true);
      expect(coinbaseTab?.hasAttribute('disabled')).toBe(true);
      expect(bybitTab?.hasAttribute('disabled')).toBe(true);
      expect(gateioTab?.hasAttribute('disabled')).toBe(true);
    });

    it('should apply opacity-50 to disabled tabs', () => {
      render(<ExchangePanel />);
      const bithumbTab = screen.getByText('BT').closest('button');
      expect(bithumbTab?.className).toContain('opacity-50');
    });

    it('should apply cursor-not-allowed to disabled tabs', () => {
      render(<ExchangePanel />);
      const bithumbTab = screen.getByText('BT').closest('button');
      expect(bithumbTab?.className).toContain('cursor-not-allowed');
    });

    it('should show Coming Soon text for disabled tabs', () => {
      render(<ExchangePanel />);
      const bithumbTab = screen.getByText('BT').closest('button');
      expect(within(bithumbTab as HTMLElement).getByText('Coming Soon')).toBeTruthy();
      expect(bithumbTab?.getAttribute('title')).toContain('Coming Soon');
    });

    it('should enable Upbit tab', () => {
      render(<ExchangePanel />);
      const upbitTab = screen.getByText('UP').closest('button');
      expect(upbitTab?.hasAttribute('disabled')).toBe(false);
    });

    it('should NOT call setExchange when clicking disabled tab', () => {
      render(<ExchangePanel />);
      const bithumbTab = screen.getByText('BT').closest('button') as HTMLElement;
      fireEvent.click(bithumbTab);
      expect(mockSetExchange).not.toHaveBeenCalled();
    });

    it('should add INFO log when switching exchange', () => {
      render(<ExchangePanel />);
      const upbitTab = screen.getByText('UP').closest('button') as HTMLElement;
      fireEvent.click(upbitTab);
      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'SYSTEM',
        '[INFO] 거래소 전환: Upbit'
      );
    });
  });

  describe('Keyboard Shortcuts (AC #4)', () => {
    it('should switch to Upbit when pressing key 1', () => {
      render(<ExchangePanel />);
      fireEvent.keyDown(window, { key: '1' });
      expect(mockSetExchange).toHaveBeenCalledWith('upbit');
    });

    it('should NOT switch to disabled exchange when pressing key 2-6', () => {
      render(<ExchangePanel />);
      fireEvent.keyDown(window, { key: '2' });
      expect(mockSetExchange).not.toHaveBeenCalled();
    });

    it('should ignore non-numeric keys', () => {
      render(<ExchangePanel />);
      fireEvent.keyDown(window, { key: 'a' });
      expect(mockSetExchange).not.toHaveBeenCalled();
    });

    it('should ignore keys 7-9', () => {
      render(<ExchangePanel />);
      fireEvent.keyDown(window, { key: '7' });
      expect(mockSetExchange).not.toHaveBeenCalled();
    });
  });

  describe('Connection Status Display', () => {
    it('should display connection status indicator', () => {
      render(<ExchangePanel />);
      expect(screen.getByTestId('connection-status')).toBeTruthy();
    });

    it('should show 연결됨 when connected', () => {
      render(<ExchangePanel />);
      expect(screen.getByText('연결됨')).toBeTruthy();
    });

    it('should show 연결중... when connecting', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        enabledExchanges: ['upbit'],
        setEnabledExchanges: vi.fn(),
        selectedExchange: 'upbit',
        connectionStatus: 'connecting',
        selectedMarket: null,
        setExchange: mockSetExchange,
        setMarket: vi.fn(),
        setConnectionStatus: vi.fn(),
        lastConnectionError: null,
        setConnectionError: vi.fn(),
      });
      render(<ExchangePanel />);
      expect(screen.getByText('연결중...')).toBeTruthy();
    });

    it('should show 연결 안됨 when disconnected', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        enabledExchanges: ['upbit'],
        setEnabledExchanges: vi.fn(),
        selectedExchange: 'upbit',
        connectionStatus: 'disconnected',
        selectedMarket: null,
        setExchange: mockSetExchange,
        setMarket: vi.fn(),
        setConnectionStatus: vi.fn(),
        lastConnectionError: null,
        setConnectionError: vi.fn(),
      });
      render(<ExchangePanel />);
      expect(screen.getByText('연결 안됨')).toBeTruthy();
    });

    it('should apply pulse animation when connecting', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        enabledExchanges: ['upbit'],
        setEnabledExchanges: vi.fn(),
        selectedExchange: 'upbit',
        connectionStatus: 'connecting',
        selectedMarket: null,
        setExchange: mockSetExchange,
        setMarket: vi.fn(),
        setConnectionStatus: vi.fn(),
        lastConnectionError: null,
        setConnectionError: vi.fn(),
      });
      render(<ExchangePanel />);
      const badge = screen.getByText('연결중...').closest('div');
      expect(badge?.className).toContain('animate-pulse');
    });

    it('should display error message in title when disconnected with error', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        enabledExchanges: ['upbit'],
        setEnabledExchanges: vi.fn(),
        selectedExchange: 'upbit',
        connectionStatus: 'disconnected',
        selectedMarket: null,
        setExchange: mockSetExchange,
        setMarket: vi.fn(),
        setConnectionStatus: vi.fn(),
        lastConnectionError: 'Network timeout',
        setConnectionError: vi.fn(),
      });
      render(<ExchangePanel />);
      const badge = screen.getByText('연결 안됨').closest('div');
      expect(badge?.getAttribute('title')).toBe('Network timeout');
    });
  });
});

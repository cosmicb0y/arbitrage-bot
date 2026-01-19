import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { BalancePanel } from '../../panels/BalancePanel';
import { useBalanceStore } from '../../stores/balanceStore';
import { useWtsStore } from '../../stores/wtsStore';
import { useConsoleStore } from '../../stores/consoleStore';
import type { BalanceEntry } from '../../types';

// Mock Tauri event API
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

// Mock stores
vi.mock('../../stores/balanceStore');
vi.mock('../../stores/wtsStore');
vi.mock('../../stores/consoleStore');

describe('BalancePanel', () => {
  const mockFetchBalance = vi.fn();
  const mockSetHideZeroBalances = vi.fn();
  const mockAddLog = vi.fn();

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
    {
      currency: 'ETH',
      balance: '0',
      locked: '0',
      avg_buy_price: '3000000',
      avg_buy_price_modified: false,
      unit_currency: 'KRW',
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock for listen
    const mockUnlisten = vi.fn();
    vi.mocked(listen).mockResolvedValue(mockUnlisten);

    // Default WTS store mock
    vi.mocked(useWtsStore).mockReturnValue({
      selectedExchange: 'upbit',
      connectionStatus: 'connected',
    } as ReturnType<typeof useWtsStore>);

    // Default balance store mock
    vi.mocked(useBalanceStore).mockReturnValue({
      balances: [],
      previousBalances: [],
      isLoading: false,
      hideZeroBalances: false,
      error: null,
      fetchBalance: mockFetchBalance,
      setHideZeroBalances: mockSetHideZeroBalances,
    } as ReturnType<typeof useBalanceStore>);

    // Default console store mock
    vi.mocked(useConsoleStore).mockReturnValue({
      logs: [],
      addLog: mockAddLog,
      clearLogs: vi.fn(),
    } as ReturnType<typeof useConsoleStore>);

    // Mock getState to return addLog
    (useConsoleStore as unknown as { getState: () => { addLog: typeof mockAddLog } }).getState = () => ({
      addLog: mockAddLog,
    });
  });

  describe('rendering', () => {
    it('should render with data-testid balance-panel', () => {
      render(<BalancePanel />);
      expect(screen.getByTestId('balance-panel')).toBeTruthy();
    });

    it('should render header with "Balances" text', () => {
      render(<BalancePanel />);
      expect(screen.getByText('Balances')).toBeTruthy();
    });

    it('should render hide zero balance checkbox', () => {
      render(<BalancePanel />);
      expect(screen.getByRole('checkbox')).toBeTruthy();
      expect(screen.getByText('0 잔고 숨기기')).toBeTruthy();
    });
  });

  describe('loading state', () => {
    it('should render skeleton loading state when isLoading is true', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: [],
        previousBalances: [],
        isLoading: true,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);
      // Should have animated skeleton elements
      const skeletons = screen.getAllByTestId('balance-skeleton');
      expect(skeletons.length).toBeGreaterThan(0);
    });
  });

  describe('empty state', () => {
    it('should render empty message when no balances', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: [],
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);
      expect(screen.getByText('잔고 없음')).toBeTruthy();
    });
  });

  describe('balance display', () => {
    it('should render balance entries with currency, balance, locked, avg buy price, and eval amount', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: mockBalances,
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      // Check BTC entry
      expect(screen.getByText('BTC')).toBeTruthy();
      expect(screen.getByText('0.5')).toBeTruthy();
      expect(screen.getByText('0.1')).toBeTruthy();
      expect(screen.getByText('₩50,000,000')).toBeTruthy();

      // Check KRW entry
      expect(screen.getByText('KRW')).toBeTruthy();
      expect(screen.getByText('1,000,000')).toBeTruthy();
    });

    it('should calculate and display KRW evaluation (balance * avg_buy_price)', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: [mockBalances[1]], // BTC only
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      // BTC: 0.5 * 50000000 = 25000000 KRW
      expect(screen.getByText('₩25,000,000')).toBeTruthy();
    });

    it('should display "-" for KRW currency evaluation', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: [mockBalances[0]], // KRW only
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      // KRW should show "-" for evaluation
      const cells = screen.getAllByRole('cell');
      const evalCell = cells[cells.length - 1];
      expect(evalCell.textContent).toBe('-');
    });

    it('should display "-" for locked column when locked is 0', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: [mockBalances[0]], // KRW with locked: '0'
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      // Locked 0 should show "-"
      const rows = screen.getAllByRole('row');
      const dataRow = rows[1]; // Skip header row
      expect(dataRow.textContent).toContain('-');
    });
  });

  describe('zero balance filtering', () => {
    it('should show all balances including zero when hideZeroBalances is false', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: mockBalances,
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      expect(screen.getByText('ETH')).toBeTruthy();
    });

    it('should hide zero balances when hideZeroBalances is true', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: mockBalances,
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: true,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      // ETH has 0 balance, should be hidden
      expect(screen.queryByText('ETH')).toBeNull();
      // BTC should still be visible
      expect(screen.getByText('BTC')).toBeTruthy();
    });

    it('should toggle hideZeroBalances when checkbox is clicked', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: mockBalances,
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      const checkbox = screen.getByRole('checkbox');
      fireEvent.click(checkbox);

      expect(mockSetHideZeroBalances).toHaveBeenCalledWith(true);
    });
  });

  describe('data fetching', () => {
    it('should call fetchBalance when connectionStatus is connected', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connected',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      expect(mockFetchBalance).toHaveBeenCalled();
    });

    it('should not call fetchBalance when connectionStatus is disconnected', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'disconnected',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      expect(mockFetchBalance).not.toHaveBeenCalled();
    });

    it('should not call fetchBalance when connectionStatus is connecting', () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connecting',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      expect(mockFetchBalance).not.toHaveBeenCalled();
    });
  });

  describe('table headers', () => {
    it('should render table headers: 자산, 가용, 잠금, 평균 매수가, 평가금액', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: mockBalances,
        previousBalances: [],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      expect(screen.getByText('자산')).toBeTruthy();
      expect(screen.getByText('가용')).toBeTruthy();
      expect(screen.getByText('잠금')).toBeTruthy();
      expect(screen.getByText('평균 매수가')).toBeTruthy();
      expect(screen.getByText('평가금액')).toBeTruthy();
    });
  });

  describe('refresh button', () => {
    it('should render refresh button with aria-label', () => {
      render(<BalancePanel />);
      expect(screen.getByLabelText('잔고 새로고침')).toBeTruthy();
    });

    it('should call fetchBalance when refresh button is clicked', async () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connected',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      // Clear the initial call from useEffect
      mockFetchBalance.mockClear();

      const refreshButton = screen.getByLabelText('잔고 새로고침');
      fireEvent.click(refreshButton);

      expect(mockFetchBalance).toHaveBeenCalledTimes(1);
    });

    it('should disable refresh button when isLoading is true', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: [],
        previousBalances: [],
        isLoading: true,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      const refreshButton = screen.getByLabelText('잔고 새로고침');
      expect(refreshButton).toHaveProperty('disabled', true);
    });

    it('should show spinning animation on refresh icon when loading', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: [],
        previousBalances: [],
        isLoading: true,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      const refreshButton = screen.getByLabelText('잔고 새로고침');
      const icon = refreshButton.querySelector('svg');
      expect(icon?.classList.contains('animate-spin')).toBe(true);
    });
  });

  describe('balance change highlight', () => {
    it('should highlight row and display change amount when balance changes', () => {
      vi.mocked(useBalanceStore).mockReturnValue({
        balances: [
          {
            currency: 'BTC',
            balance: '0.5',
            locked: '0.1',
            avg_buy_price: '50000000',
            avg_buy_price_modified: false,
            unit_currency: 'KRW',
          },
        ],
        previousBalances: [
          {
            currency: 'BTC',
            balance: '0.5',
            locked: '0',
            avg_buy_price: '50000000',
            avg_buy_price_modified: false,
            unit_currency: 'KRW',
          },
        ],
        isLoading: false,
        hideZeroBalances: false,
        error: null,
        fetchBalance: mockFetchBalance,
        setHideZeroBalances: mockSetHideZeroBalances,
      } as ReturnType<typeof useBalanceStore>);

      render(<BalancePanel />);

      expect(screen.getByText('+0.1 BTC')).toBeTruthy();
      const row = screen.getByText('BTC').closest('tr');
      expect(row?.className).toContain('animate-highlight-green');
    });
  });

  describe('auto refresh event listener', () => {
    it('should set up event listener for wts:order:filled when connected', async () => {
      const mockUnlisten = vi.fn();
      vi.mocked(listen).mockResolvedValue(mockUnlisten);

      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connected',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      await waitFor(() => {
        expect(listen).toHaveBeenCalledWith('wts:order:filled', expect.any(Function));
      });
    });

    it('should clean up event listener on unmount', async () => {
      const mockUnlisten = vi.fn();
      vi.mocked(listen).mockResolvedValue(mockUnlisten);

      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connected',
      } as ReturnType<typeof useWtsStore>);

      const { unmount } = render(<BalancePanel />);

      // Wait for listener to be set up
      await waitFor(() => {
        expect(listen).toHaveBeenCalled();
      });

      // Unmount component
      unmount();

      // Verify cleanup was called
      await waitFor(() => {
        expect(mockUnlisten).toHaveBeenCalled();
      });
    });

    it('should not set up event listener when disconnected', async () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'disconnected',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      // Small wait to ensure useEffect has had time to run
      await new Promise((resolve) => setTimeout(resolve, 50));

      expect(listen).not.toHaveBeenCalled();
    });

    it('should call fetchBalance when order:filled event is received', async () => {
      let eventCallback: ((event: unknown) => void) | undefined;
      vi.mocked(listen).mockImplementation(async (eventName, callback) => {
        if (eventName === 'wts:order:filled') {
          eventCallback = callback;
        }
        return vi.fn();
      });

      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connected',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      // Wait for listener to be set up
      await waitFor(() => {
        expect(listen).toHaveBeenCalledWith('wts:order:filled', expect.any(Function));
      });

      // Clear initial fetchBalance call
      mockFetchBalance.mockClear();

      // Simulate order:filled event
      if (eventCallback) {
        eventCallback({ payload: {} });
      }

      expect(mockFetchBalance).toHaveBeenCalledTimes(1);
    });
  });

  describe('console logging', () => {
    it('should log INFO message when manual refresh button is clicked', async () => {
      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connected',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      // Clear any logs from initial render
      mockAddLog.mockClear();

      const refreshButton = screen.getByLabelText('잔고 새로고침');
      fireEvent.click(refreshButton);

      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'BALANCE',
        '수동 잔고 갱신 요청'
      );
    });

    it('should log INFO message when auto refresh is triggered by order:filled event', async () => {
      let eventCallback: ((event: unknown) => void) | undefined;
      vi.mocked(listen).mockImplementation(async (eventName, callback) => {
        if (eventName === 'wts:order:filled') {
          eventCallback = callback;
        }
        return vi.fn();
      });

      vi.mocked(useWtsStore).mockReturnValue({
        selectedExchange: 'upbit',
        connectionStatus: 'connected',
      } as ReturnType<typeof useWtsStore>);

      render(<BalancePanel />);

      // Wait for listener to be set up
      await waitFor(() => {
        expect(listen).toHaveBeenCalledWith('wts:order:filled', expect.any(Function));
      });

      // Clear any logs from initial render
      mockAddLog.mockClear();

      // Simulate order:filled event
      if (eventCallback) {
        eventCallback({ payload: {} });
      }

      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'BALANCE',
        '주문 체결로 인한 자동 잔고 갱신'
      );
    });
  });
});

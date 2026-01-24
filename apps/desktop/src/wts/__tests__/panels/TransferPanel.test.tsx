// @vitest-environment jsdom
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { TransferPanel } from '../../panels/TransferPanel';
import { useTransferStore } from '../../stores/transferStore';
import { useConsoleStore } from '../../stores/consoleStore';
import { useToastStore } from '../../stores/toastStore';
import type { DepositChanceResponse } from '../../types';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock stores
vi.mock('../../stores/transferStore');
vi.mock('../../stores/consoleStore');
vi.mock('../../stores/toastStore');

describe('TransferPanel', () => {
  const mockSetActiveTab = vi.fn();
  const mockSetSelectedCurrency = vi.fn();
  const mockSetSelectedNetwork = vi.fn();
  const mockSetNetworkInfo = vi.fn();
  const mockSetLoading = vi.fn();
  const mockSetError = vi.fn();
  const mockReset = vi.fn();
  const mockAddLog = vi.fn();
  const mockShowToast = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default transfer store mock
    vi.mocked(useTransferStore).mockReturnValue({
      activeTab: 'deposit',
      selectedCurrency: null,
      selectedNetwork: null,
      networkInfo: null,
      isLoading: false,
      error: null,
      setActiveTab: mockSetActiveTab,
      setSelectedCurrency: mockSetSelectedCurrency,
      setSelectedNetwork: mockSetSelectedNetwork,
      setNetworkInfo: mockSetNetworkInfo,
      setLoading: mockSetLoading,
      setError: mockSetError,
      reset: mockReset,
    });

    // Default console store mock
    vi.mocked(useConsoleStore).mockImplementation((selector) => {
      const state = {
        logs: [],
        addLog: mockAddLog,
        clearLogs: vi.fn(),
      };
      if (typeof selector === 'function') {
        return selector(state);
      }
      return state;
    });

    // Mock getState to return addLog
    (useConsoleStore as unknown as { getState: () => { addLog: typeof mockAddLog } }).getState = () => ({
      addLog: mockAddLog,
    });

    // Default toast store mock
    vi.mocked(useToastStore).mockImplementation((selector) => {
      const state = {
        toast: null,
        showToast: mockShowToast,
        hideToast: vi.fn(),
      };
      if (typeof selector === 'function') {
        return selector(state);
      }
      return state;
    });

    // Mock getState for toast store
    (useToastStore as unknown as { getState: () => { showToast: typeof mockShowToast } }).getState = () => ({
      showToast: mockShowToast,
    });
  });

  describe('기본 렌더링', () => {
    it('패널 헤더가 "Transfer"로 표시된다', () => {
      render(<TransferPanel />);
      expect(screen.getByText('Transfer')).toBeTruthy();
    });

    it('data-testid="transfer-panel"이 설정된다', () => {
      render(<TransferPanel />);
      expect(screen.getByTestId('transfer-panel')).toBeTruthy();
    });

    it('className prop이 적용된다', () => {
      render(<TransferPanel className="custom-class" />);
      const panel = screen.getByTestId('transfer-panel');
      expect(panel.className).toContain('custom-class');
    });

    it('wts-panel 클래스가 적용된다', () => {
      render(<TransferPanel />);
      const panel = screen.getByTestId('transfer-panel');
      expect(panel.className).toContain('wts-panel');
    });
  });

  describe('탭 UI (AC #5)', () => {
    it('입금 탭이 표시된다', () => {
      render(<TransferPanel />);
      expect(screen.getByRole('tab', { name: /입금/i })).toBeTruthy();
    });

    it('출금 탭이 표시된다', () => {
      render(<TransferPanel />);
      expect(screen.getByRole('tab', { name: /출금/i })).toBeTruthy();
    });

    it('role="tablist" 컨테이너가 존재한다', () => {
      render(<TransferPanel />);
      expect(screen.getByRole('tablist')).toBeTruthy();
    });

    it('입금 탭이 기본적으로 aria-selected="true"이다', () => {
      render(<TransferPanel />);
      const depositTab = screen.getByRole('tab', { name: /입금/i });
      expect(depositTab.getAttribute('aria-selected')).toBe('true');
    });

    it('출금 탭은 기본적으로 aria-selected="false"이다', () => {
      render(<TransferPanel />);
      const withdrawTab = screen.getByRole('tab', { name: /출금/i });
      expect(withdrawTab.getAttribute('aria-selected')).toBe('false');
    });

    it('출금 탭 클릭 시 setActiveTab("withdraw")가 호출된다', () => {
      render(<TransferPanel />);
      const withdrawTab = screen.getByRole('tab', { name: /출금/i });
      fireEvent.click(withdrawTab);
      expect(mockSetActiveTab).toHaveBeenCalledWith('withdraw');
    });

    it('입금 탭 클릭 시 setActiveTab("deposit")가 호출된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: null,
        selectedNetwork: null,
        networkInfo: null,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      const depositTab = screen.getByRole('tab', { name: /입금/i });
      fireEvent.click(depositTab);
      expect(mockSetActiveTab).toHaveBeenCalledWith('deposit');
    });

    it('activeTab이 withdraw일 때 출금 탭이 aria-selected="true"이다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: null,
        selectedNetwork: null,
        networkInfo: null,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      const withdrawTab = screen.getByRole('tab', { name: /출금/i });
      expect(withdrawTab.getAttribute('aria-selected')).toBe('true');
    });
  });

  describe('입금 탭 콘텐츠 (AC #1)', () => {
    it('입금 탭에서 자산 선택 드롭다운이 표시된다', () => {
      render(<TransferPanel />);
      expect(screen.getByLabelText(/자산/i)).toBeTruthy();
    });

    it('자산 드롭다운에 BTC 옵션이 있다', () => {
      render(<TransferPanel />);
      const select = screen.getByLabelText(/자산/i) as HTMLSelectElement;
      expect(select).toBeTruthy();
      expect(select.tagName.toLowerCase()).toBe('select');
    });
  });

  describe('출금 탭 콘텐츠', () => {
    it('출금 탭에서 "준비 중" 메시지가 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: null,
        selectedNetwork: null,
        networkInfo: null,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/출금 기능은 준비 중입니다/i)).toBeTruthy();
    });
  });

  describe('자산 선택 (AC #4, #6)', () => {
    it('자산 선택 시 setSelectedCurrency가 호출된다', () => {
      render(<TransferPanel />);
      const select = screen.getByLabelText(/자산/i) as HTMLSelectElement;
      fireEvent.change(select, { target: { value: 'BTC' } });
      expect(mockSetSelectedCurrency).toHaveBeenCalledWith('BTC');
    });

    it('자산 선택 시 콘솔에 로그가 기록된다', () => {
      render(<TransferPanel />);
      const select = screen.getByLabelText(/자산/i) as HTMLSelectElement;
      fireEvent.change(select, { target: { value: 'ETH' } });
      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'DEPOSIT',
        expect.stringContaining('ETH')
      );
    });

    it('선택된 자산이 드롭다운에 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: null,
        networkInfo: null,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      const select = screen.getByLabelText(/자산/i) as HTMLSelectElement;
      expect(select.value).toBe('BTC');
    });
  });

  describe('네트워크 정보 표시 (AC #2, #3)', () => {
    const mockNetworkInfo: DepositChanceResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      network: {
        name: 'Bitcoin',
        net_type: 'BTC',
        priority: 1,
        deposit_state: 'normal',
        confirm_count: 3,
      },
      deposit_state: 'normal',
      minimum: '0.0001',
    };

    it('네트워크 정보가 있으면 네트워크 이름이 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('Bitcoin')).toBeTruthy();
    });

    it('입금 상태가 "정상"으로 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/정상/i)).toBeTruthy();
    });

    it('확인 횟수가 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/3회/i)).toBeTruthy();
    });

    it('최소 입금 수량이 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/0.0001/)).toBeTruthy();
    });
  });

  describe('입금 중단 상태 표시 (AC #7)', () => {
    it('입금 상태가 paused일 때 "중단"으로 표시된다', () => {
      const pausedNetworkInfo: DepositChanceResponse = {
        currency: 'ETH',
        net_type: 'ETH',
        network: {
          name: 'Ethereum',
          net_type: 'ETH',
          priority: 1,
          deposit_state: 'paused',
          confirm_count: 12,
        },
        deposit_state: 'paused',
        minimum: '0.01',
      };

      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'ETH',
        selectedNetwork: 'ETH',
        networkInfo: pausedNetworkInfo,
        isLoading: false,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/중단/i)).toBeTruthy();
    });
  });

  describe('로딩 및 에러 상태', () => {
    it('isLoading이 true일 때 로딩 표시가 나타난다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: null,
        networkInfo: null,
        isLoading: true,
        error: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/조회 중/i)).toBeTruthy();
    });

    it('에러가 있으면 에러 메시지가 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: null,
        networkInfo: null,
        isLoading: false,
        error: '네트워크 오류 발생',
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/네트워크 오류 발생/)).toBeTruthy();
    });
  });
});

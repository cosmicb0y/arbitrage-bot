// @vitest-environment jsdom
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { TransferPanel } from '../../panels/TransferPanel';
import { useTransferStore, MAX_GENERATE_RETRIES } from '../../stores/transferStore';
import { useConsoleStore } from '../../stores/consoleStore';
import { useToastStore } from '../../stores/toastStore';
import type {
  DepositChanceResponse,
  DepositAddressResponse,
  WithdrawChanceResponse,
  WithdrawAddressResponse,
} from '../../types';

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
  const mockSetDepositAddress = vi.fn();
  const mockSetAddressLoading = vi.fn();
  const mockSetAddressError = vi.fn();
  const mockSetGenerating = vi.fn();
  const mockSetGenerateRetryCount = vi.fn();
  const mockResetGenerateState = vi.fn();
  const mockReset = vi.fn();
  const mockAddLog = vi.fn();
  const mockShowToast = vi.fn();
  // 출금 관련 mock (WTS-5.2)
  const mockSetWithdrawChanceInfo = vi.fn();
  const mockSetWithdrawAddresses = vi.fn();
  const mockSetSelectedWithdrawAddress = vi.fn();
  const mockSetWithdrawAmount = vi.fn();
  const mockSetWithdrawLoading = vi.fn();
  const mockSetWithdrawError = vi.fn();
  const mockResetWithdrawState = vi.fn();

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
      depositAddress: null,
      isAddressLoading: false,
      addressError: null,
      isGenerating: false,
      generateRetryCount: 0,
      // 출금 상태 (WTS-5.2)
      withdrawChanceInfo: null,
      withdrawAddresses: [],
      selectedWithdrawAddress: null,
      withdrawAmount: '',
      isWithdrawLoading: false,
      withdrawError: null,
      setActiveTab: mockSetActiveTab,
      setSelectedCurrency: mockSetSelectedCurrency,
      setSelectedNetwork: mockSetSelectedNetwork,
      setNetworkInfo: mockSetNetworkInfo,
      setLoading: mockSetLoading,
      setError: mockSetError,
      setDepositAddress: mockSetDepositAddress,
      setAddressLoading: mockSetAddressLoading,
      setAddressError: mockSetAddressError,
      setGenerating: mockSetGenerating,
      setGenerateRetryCount: mockSetGenerateRetryCount,
      resetGenerateState: mockResetGenerateState,
      // 출금 액션 (WTS-5.2)
      setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
      setWithdrawAddresses: mockSetWithdrawAddresses,
      setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
      setWithdrawAmount: mockSetWithdrawAmount,
      setWithdrawLoading: mockSetWithdrawLoading,
      setWithdrawError: mockSetWithdrawError,
      resetWithdrawState: mockResetWithdrawState,
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
    (
      useConsoleStore as unknown as { getState: () => { addLog: typeof mockAddLog } }
    ).getState = () => ({
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
    (
      useToastStore as unknown as {
        getState: () => { showToast: typeof mockShowToast };
      }
    ).getState = () => ({
      showToast: mockShowToast,
    });

    // Mock clipboard
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn(),
      },
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

  describe('입금 주소 표시 (AC #1)', () => {
    const mockNetworkInfo: DepositChanceResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      is_deposit_possible: true,
      deposit_impossible_reason: null,
      minimum_deposit_amount: 0.0001,
      minimum_deposit_confirmations: 3,
      decimal_precision: 8,
    };

    const mockDepositAddress: DepositAddressResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      deposit_address: '1A2b3C4d5E6f7g8H9i0J',
      secondary_address: null,
    };

    it('입금 주소가 있으면 화면에 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: mockDepositAddress,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('1A2b3C4d5E6f7g8H9i0J')).toBeTruthy();
    });

    it('보조 주소(Tag/Memo)가 있으면 화면에 표시된다 (AC #6)', () => {
      const mockXrpAddress: DepositAddressResponse = {
        currency: 'XRP',
        net_type: 'XRP',
        deposit_address: 'rExampleAddress',
        secondary_address: '123456789',
      };

      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'XRP',
        selectedNetwork: 'XRP',
        networkInfo: { ...mockNetworkInfo, currency: 'XRP', net_type: 'XRP' },
        isLoading: false,
        error: null,
        depositAddress: mockXrpAddress,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('rExampleAddress')).toBeTruthy();
      expect(screen.getByText('123456789')).toBeTruthy();
      expect(screen.getByText('Memo/Tag')).toBeTruthy();
    });

    it('보조 주소가 있을 때 경고 메시지가 표시된다', () => {
      const mockXrpAddress: DepositAddressResponse = {
        currency: 'XRP',
        net_type: 'XRP',
        deposit_address: 'rExampleAddress',
        secondary_address: '123456789',
      };

      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'XRP',
        selectedNetwork: 'XRP',
        networkInfo: { ...mockNetworkInfo, currency: 'XRP', net_type: 'XRP' },
        isLoading: false,
        error: null,
        depositAddress: mockXrpAddress,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/Memo\/Tag 필수/)).toBeTruthy();
    });
  });

  describe('주소 복사 기능 (AC #2, #3)', () => {
    const mockNetworkInfo: DepositChanceResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      is_deposit_possible: true,
      deposit_impossible_reason: null,
      minimum_deposit_amount: 0.0001,
      minimum_deposit_confirmations: 3,
      decimal_precision: 8,
    };

    const mockDepositAddress: DepositAddressResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      deposit_address: '1A2b3C4d5E6f7g8H9i0J',
      secondary_address: null,
    };

    it('주소 옆에 복사 버튼이 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: mockDepositAddress,
        isAddressLoading: false,
        addressError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByTitle('주소 복사')).toBeTruthy();
    });

    it('복사 버튼 클릭 시 클립보드에 복사된다', async () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: mockDepositAddress,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      const copyBtn = screen.getByTitle('주소 복사');
      fireEvent.click(copyBtn);

      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
          '1A2b3C4d5E6f7g8H9i0J'
        );
      });
    });

    it('복사 성공 시 로그가 기록된다', async () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: mockDepositAddress,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      const copyBtn = screen.getByTitle('주소 복사');
      fireEvent.click(copyBtn);

      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'SUCCESS',
          'DEPOSIT',
          expect.stringContaining('클립보드에 복사')
        );
      });
    });
  });

  describe('주소 생성 및 에러 처리 (AC #7)', () => {
    const mockNetworkInfo: DepositChanceResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      is_deposit_possible: true,
      deposit_impossible_reason: null,
      minimum_deposit_amount: 0.0001,
      minimum_deposit_confirmations: 3,
      decimal_precision: 8,
    };

    it('주소가 null일 때 주소 생성 버튼이 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        // 주소 없음 상태
        depositAddress: {
          currency: 'BTC',
          net_type: 'BTC',
          deposit_address: null,
          secondary_address: null,
        },
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('주소 생성')).toBeTruthy();
      expect(screen.getByText('입금 주소가 없습니다')).toBeTruthy();
    });

    it('주소 로딩 중일 때 로딩 표시가 나타난다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: true, // 로딩 중
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('주소 로딩 중...')).toBeTruthy();
    });

    it('주소 조회 에러 시 에러 메시지가 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: '주소 조회 실패', // 에러 발생
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: vi.fn(),
        setGenerateRetryCount: vi.fn(),
        resetGenerateState: vi.fn(),
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('주소 조회 실패')).toBeTruthy();
    });
  });

  describe('비동기 주소 생성 (WTS-4.4)', () => {
    const mockNetworkInfo: DepositChanceResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      is_deposit_possible: true,
      deposit_impossible_reason: null,
      minimum_deposit_amount: 0.0001,
      minimum_deposit_confirmations: 3,
      decimal_precision: 8,
    };

    const mockSetGenerating = vi.fn();
    const mockSetGenerateRetryCount = vi.fn();
    const mockResetGenerateState = vi.fn();

    beforeEach(() => {
      mockSetGenerating.mockClear();
      mockSetGenerateRetryCount.mockClear();
      mockResetGenerateState.mockClear();
    });

    it('주소 생성 요청 중일 때 "주소 생성 요청 중..." 메시지가 표시된다 (AC #1)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: {
          currency: 'BTC',
          net_type: 'BTC',
          deposit_address: null,
          secondary_address: null,
        },
        isAddressLoading: false,
        addressError: null,
        isGenerating: true,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('주소 생성 요청 중...')).toBeTruthy();
    });

    it('폴링 중일 때 재시도 진행 상태가 표시된다 (AC #5)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: {
          currency: 'BTC',
          net_type: 'BTC',
          deposit_address: null,
          secondary_address: null,
        },
        isAddressLoading: false,
        addressError: null,
        isGenerating: true,
        generateRetryCount: 2,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(`주소 확인 중 (2/${MAX_GENERATE_RETRIES})`)).toBeTruthy();
    });

    it('생성 중일 때 취소 버튼이 표시된다 (AC #5)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: {
          currency: 'BTC',
          net_type: 'BTC',
          deposit_address: null,
          secondary_address: null,
        },
        isAddressLoading: false,
        addressError: null,
        isGenerating: true,
        generateRetryCount: 1,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('취소')).toBeTruthy();
    });

    it('최대 재시도 초과 에러 메시지가 표시되고 다시 시도 버튼이 있다 (AC #4)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: {
          currency: 'BTC',
          net_type: 'BTC',
          deposit_address: null,
          secondary_address: null,
        },
        isAddressLoading: false,
        addressError: `주소 생성 실패: 최대 재시도 횟수(${MAX_GENERATE_RETRIES}회) 초과`,
        isGenerating: false,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/최대 재시도 횟수/)).toBeTruthy();
      expect(screen.getByText('다시 시도')).toBeTruthy();
    });

    it('생성 중 스피너 애니메이션이 표시된다 (AC #5)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'deposit',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: mockNetworkInfo,
        isLoading: false,
        error: null,
        depositAddress: {
          currency: 'BTC',
          net_type: 'BTC',
          deposit_address: null,
          secondary_address: null,
        },
        isAddressLoading: false,
        addressError: null,
        isGenerating: true,
        generateRetryCount: 0,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      // 스피너 클래스 확인
      const spinner = document.querySelector('.animate-spin');
      expect(spinner).toBeTruthy();
    });
  });

  // ============================================================================
  // 출금 탭 테스트 (WTS-5.2)
  // ============================================================================

  describe('출금 탭 (WTS-5.2)', () => {
    const mockWithdrawChanceInfo: WithdrawChanceResponse = {
      member_level: {
        security_level: 3,
        fee_level: 0,
        email_verified: true,
        identity_auth_verified: true,
        bank_account_verified: true,
        two_factor_auth_verified: true,
        locked: false,
        wallet_locked: false,
      },
      currency: {
        code: 'BTC',
        withdraw_fee: '0.0005',
        is_coin: true,
        wallet_state: 'working',
        wallet_support: ['default'],
      },
      account: {
        currency: 'BTC',
        balance: '1.5',
        locked: '0.1',
        avg_buy_price: '50000000',
        avg_buy_price_modified: false,
        unit_currency: 'KRW',
      },
      withdraw_limit: {
        currency: 'BTC',
        minimum: '0.001',
        onetime: '10',
        daily: '100',
        remaining_daily: '99.5',
        remaining_daily_krw: '4975000000',
        fixed: 8,
        can_withdraw: true,
      },
    };

    const mockWithdrawAddress: WithdrawAddressResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      network_name: 'Bitcoin',
      withdraw_address: '1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa',
      secondary_address: null,
    };

    const mockSetGenerating = vi.fn();
    const mockSetGenerateRetryCount = vi.fn();
    const mockResetGenerateState = vi.fn();

    beforeEach(() => {
      mockSetGenerating.mockClear();
      mockSetGenerateRetryCount.mockClear();
      mockResetGenerateState.mockClear();
    });

    it('출금 탭 선택 시 자산 선택 드롭다운이 표시된다 (AC #1)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: null,
        selectedNetwork: null,
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: null,
        withdrawAddresses: [],
        selectedWithdrawAddress: null,
        withdrawAmount: '',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByLabelText('출금 자산')).toBeTruthy();
    });

    it('자산 선택 시 네트워크 버튼이 표시된다 (AC #2)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: null,
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: null,
        withdrawAddresses: [],
        selectedWithdrawAddress: null,
        withdrawAmount: '',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('네트워크 선택')).toBeTruthy();
      expect(screen.getByText('BTC')).toBeTruthy();
    });

    it('출금 가능 정보가 조회되면 잔고, 수수료, 한도가 표시된다 (AC #5, #6, #7)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: mockWithdrawChanceInfo,
        withdrawAddresses: [mockWithdrawAddress],
        selectedWithdrawAddress: null,
        withdrawAmount: '',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('출금 가능 잔고')).toBeTruthy();
      expect(screen.getByText('출금 수수료')).toBeTruthy();
      expect(screen.getByText('최소 출금')).toBeTruthy();
    });

    it('등록된 출금 주소가 드롭다운으로 표시된다 (AC #3)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: mockWithdrawChanceInfo,
        withdrawAddresses: [mockWithdrawAddress],
        selectedWithdrawAddress: null,
        withdrawAmount: '',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByLabelText('출금 주소')).toBeTruthy();
    });

    it('등록된 출금 주소가 없을 때 안내 메시지가 표시된다 (AC #9)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: mockWithdrawChanceInfo,
        withdrawAddresses: [], // 빈 배열
        selectedWithdrawAddress: null,
        withdrawAmount: '',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText(/Upbit에서 출금 주소를 먼저 등록해주세요/)).toBeTruthy();
      expect(screen.getByText(/Upbit 출금 주소 등록하기/)).toBeTruthy();
    });

    it('출금 주소 선택 후 수량 입력 필드가 표시된다 (AC #4)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: mockWithdrawChanceInfo,
        withdrawAddresses: [mockWithdrawAddress],
        selectedWithdrawAddress: mockWithdrawAddress,
        withdrawAmount: '',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByLabelText('출금 수량')).toBeTruthy();
    });

    it('% 버튼이 표시된다 (AC #8)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: mockWithdrawChanceInfo,
        withdrawAddresses: [mockWithdrawAddress],
        selectedWithdrawAddress: mockWithdrawAddress,
        withdrawAmount: '',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('25%')).toBeTruthy();
      expect(screen.getByText('50%')).toBeTruthy();
      expect(screen.getByText('75%')).toBeTruthy();
      expect(screen.getByText('MAX')).toBeTruthy();
    });

    it('출금 불가 상태일 때 안내가 표시된다 (AC #10)', () => {
      const withdrawNotAllowed: WithdrawChanceResponse = {
        ...mockWithdrawChanceInfo,
        withdraw_limit: {
          ...mockWithdrawChanceInfo.withdraw_limit,
          can_withdraw: false,
        },
      };

      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: withdrawNotAllowed,
        withdrawAddresses: [mockWithdrawAddress],
        selectedWithdrawAddress: null,
        withdrawAmount: '',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('출금 불가')).toBeTruthy();
    });

    it('수량 입력 후 실수령액이 표시된다 (AC #6)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: mockWithdrawChanceInfo,
        withdrawAddresses: [mockWithdrawAddress],
        selectedWithdrawAddress: mockWithdrawAddress,
        withdrawAmount: '0.5',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('실수령액')).toBeTruthy();
      expect(screen.getByText('0.49950000 BTC')).toBeTruthy();
    });

    it('수수료보다 적은 금액 입력 시 실수령액이 0으로 표시되고 경고가 나타난다 (AC #6)', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: mockWithdrawChanceInfo,
        withdrawAddresses: [mockWithdrawAddress],
        selectedWithdrawAddress: mockWithdrawAddress,
        withdrawAmount: '0.0001', // 수수료(0.0005)보다 적음
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByText('0 BTC')).toBeTruthy();
      expect(screen.getByText(/수수료 차감 후 실수령액이 0 이하입니다/)).toBeTruthy();
    });

    it('출금 버튼이 표시된다', () => {
      vi.mocked(useTransferStore).mockReturnValue({
        activeTab: 'withdraw',
        selectedCurrency: 'BTC',
        selectedNetwork: 'BTC',
        networkInfo: null,
        isLoading: false,
        error: null,
        depositAddress: null,
        isAddressLoading: false,
        addressError: null,
        isGenerating: false,
        generateRetryCount: 0,
        withdrawChanceInfo: mockWithdrawChanceInfo,
        withdrawAddresses: [mockWithdrawAddress],
        selectedWithdrawAddress: mockWithdrawAddress,
        withdrawAmount: '0.5',
        isWithdrawLoading: false,
        withdrawError: null,
        setActiveTab: mockSetActiveTab,
        setSelectedCurrency: mockSetSelectedCurrency,
        setSelectedNetwork: mockSetSelectedNetwork,
        setNetworkInfo: mockSetNetworkInfo,
        setLoading: mockSetLoading,
        setError: mockSetError,
        setDepositAddress: mockSetDepositAddress,
        setAddressLoading: mockSetAddressLoading,
        setAddressError: mockSetAddressError,
        setGenerating: mockSetGenerating,
        setGenerateRetryCount: mockSetGenerateRetryCount,
        resetGenerateState: mockResetGenerateState,
        setWithdrawChanceInfo: mockSetWithdrawChanceInfo,
        setWithdrawAddresses: mockSetWithdrawAddresses,
        setSelectedWithdrawAddress: mockSetSelectedWithdrawAddress,
        setWithdrawAmount: mockSetWithdrawAmount,
        setWithdrawLoading: mockSetWithdrawLoading,
        setWithdrawError: mockSetWithdrawError,
        resetWithdrawState: mockResetWithdrawState,
        reset: mockReset,
      });

      render(<TransferPanel />);
      expect(screen.getByRole('button', { name: '출금' })).toBeTruthy();
    });
  });
});

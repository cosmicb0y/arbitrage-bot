import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { WtsWindow } from '../WtsWindow';

const {
  mockInvoke,
  mockAddLog,
  mockShowToast,
  mockFetchBalance,
  withdrawParams,
} = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockAddLog: vi.fn(),
  mockShowToast: vi.fn(),
  mockFetchBalance: vi.fn(),
  withdrawParams: {
    currency: 'BTC',
    net_type: 'BTC',
    amount: '0.1',
    address: 'btc-address-123',
    secondary_address: null,
  },
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

vi.mock('../hooks', () => ({
  useConnectionCheck: vi.fn(),
}));

vi.mock('../hooks/useUpbitMarkets', () => ({
  useUpbitMarkets: vi.fn(),
}));

vi.mock('../stores/consoleStore', () => {
  const mockStore = { addLog: mockAddLog };
  const useConsoleStore = Object.assign(
    vi.fn((selector) => selector(mockStore)),
    { getState: vi.fn(() => mockStore) }
  );
  return { useConsoleStore };
});

vi.mock('../stores/toastStore', () => {
  const mockStore = { showToast: mockShowToast, toasts: [], removeToast: vi.fn(), clearToasts: vi.fn() };
  const useToastStore = Object.assign(
    vi.fn((selector) => selector(mockStore)),
    { getState: vi.fn(() => mockStore) }
  );
  return { useToastStore };
});

vi.mock('../stores/balanceStore', () => ({
  useBalanceStore: vi.fn((selector) => selector({ fetchBalance: mockFetchBalance })),
}));

vi.mock('../stores/transferStore', () => ({
  useTransferStore: vi.fn((selector) =>
    selector({
      withdrawChanceInfo: {
        currency_info: { withdraw_fee: '0.0005' },
        withdraw_limit: { fixed: 8 },
      },
    })
  ),
}));

vi.mock('../panels', () => ({
  ExchangePanel: () => <div />,
  ConsolePanel: () => <div />,
  OrderbookPanel: () => <div />,
  BalancePanel: () => <div />,
  OrderPanel: () => <div />,
  OpenOrdersPanel: () => <div />,
  TransferPanel: ({ onWithdrawClick }: { onWithdrawClick?: (params: typeof withdrawParams) => void }) => (
    <button type="button" onClick={() => onWithdrawClick?.(withdrawParams)}>
      trigger-withdraw
    </button>
  ),
}));

vi.mock('../components/WithdrawConfirmDialog', () => ({
  WithdrawConfirmDialog: ({
    isOpen,
    onConfirm,
    onCancel,
    retryable,
    onRetry,
  }: {
    isOpen: boolean;
    onConfirm: () => void;
    onCancel: () => void;
    retryable?: boolean;
    onRetry?: () => void;
  }) =>
    isOpen ? (
      <div>
        <button type="button" onClick={onConfirm}>
          confirm-withdraw
        </button>
        <button type="button" onClick={onCancel}>
          cancel-withdraw
        </button>
        {retryable && onRetry && (
          <button type="button" onClick={onRetry}>
            retry-withdraw
          </button>
        )}
      </div>
    ) : null,
}));

vi.mock('../components/WithdrawResultDialog', () => ({
  WithdrawResultDialog: ({ isOpen, onCheckStatus }: { isOpen: boolean; onCheckStatus: () => void }) =>
    isOpen ? (
      <button type="button" onClick={onCheckStatus}>
        check-status
      </button>
    ) : null,
}));

vi.mock('../components/ToastContainer', () => ({
  ToastContainer: () => null,
}));

const advanceRetryDelay = async () => {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(3000);
  });
};

afterEach(() => {
  vi.useRealTimers();
});

// WTS-5.6: withdrawWithRetry 헬퍼 테스트
describe('withdrawWithRetry (WTS-5.6)', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockAddLog.mockReset();
    mockShowToast.mockReset();
    mockFetchBalance.mockReset();
  });

  it('성공 시 재시도 없이 결과를 반환한다', async () => {
    const mockResult = {
      success: true,
      data: {
        type: 'withdraw',
        uuid: 'withdraw-uuid-123',
        currency: 'BTC',
        net_type: 'BTC',
        txid: null,
        state: 'processing',
        created_at: '2026-01-25T10:00:00Z',
        done_at: null,
        amount: '0.1',
        fee: '0.0005',
        transaction_type: 'default',
      },
    };
    mockInvoke.mockResolvedValueOnce(mockResult);

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    // 성공 로그 확인
    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith(
        'SUCCESS',
        'WITHDRAW',
        expect.stringContaining('출금 요청 완료')
      );
    });

    // 재시도 로그가 없어야 함
    expect(mockAddLog).not.toHaveBeenCalledWith(
      'INFO',
      'WITHDRAW',
      expect.stringContaining('재시도 중')
    );
  });

  it('네트워크 에러 시 1회 자동 재시도를 수행하고 재시도 로그를 기록한다', async () => {
    vi.useFakeTimers();
    mockInvoke
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      })
      .mockResolvedValueOnce({
        success: true,
        data: {
          type: 'withdraw',
          uuid: 'withdraw-uuid-123',
          currency: 'BTC',
          net_type: 'BTC',
          txid: null,
          state: 'processing',
          created_at: '2026-01-25T10:00:00Z',
          done_at: null,
          amount: '0.1',
          fee: '0.0005',
          transaction_type: 'default',
        },
      });

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await advanceRetryDelay();

    // 재시도 INFO 로그 확인 (AC #7)
    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'WITHDRAW',
        expect.stringContaining('재시도 중')
      );
    });

    // 재시도 성공 로그 확인 (AC #8)
    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith('SUCCESS', 'WITHDRAW', '재시도 성공');
    });

    // 총 2회 호출 확인
    expect(mockInvoke).toHaveBeenCalledTimes(2);
  });

  it('비네트워크 에러 시 재시도하지 않는다', async () => {
    mockInvoke.mockResolvedValueOnce({
      success: false,
      error: { code: 'insufficient_funds_withdraw', message: '잔고 부족' },
    });

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    // 에러 처리 확인
    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'WITHDRAW',
        expect.anything(),
        expect.anything()
      );
    });

    // 재시도 로그가 없어야 함
    expect(mockAddLog).not.toHaveBeenCalledWith(
      'INFO',
      'WITHDRAW',
      expect.stringContaining('재시도 중')
    );
  });

  it('재시도 실패 시 에러 처리 흐름으로 전달된다', async () => {
    vi.useFakeTimers();
    mockInvoke
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      })
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      });

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await advanceRetryDelay();

    // 재시도 로그 확인
    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'WITHDRAW',
        expect.stringContaining('재시도 중')
      );
    });

    // 총 2회 호출 확인 (원래 + 재시도 1회)
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });

    // 재시도 성공 로그는 없어야 함
    expect(mockAddLog).not.toHaveBeenCalledWith('SUCCESS', 'WITHDRAW', '재시도 성공');
  });

  it('자동 재시도 실패 후 수동 재시도 버튼이 표시된다 (AC #6)', async () => {
    vi.useFakeTimers();
    mockInvoke
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      })
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      });

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await advanceRetryDelay();

    // 자동 재시도 완료 대기
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });

    // 수동 재시도 버튼 표시 확인
    expect(screen.getByText('retry-withdraw')).toBeTruthy();
  });
});

// WTS-5.6: 에러 메시지 및 통합 테스트
describe('Withdraw error messages (WTS-5.6 AC #1-4)', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockAddLog.mockReset();
    mockShowToast.mockReset();
    mockFetchBalance.mockReset();
  });

  it('네트워크 에러 시 한국어 에러 메시지가 토스트로 표시된다 (AC #1, #3)', async () => {
    vi.useFakeTimers();
    mockInvoke
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      })
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      });

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await advanceRetryDelay();

    // 에러 토스트 표시 확인
    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith('error', expect.any(String));
    });
  });

  it('네트워크 에러 시 콘솔에 ERROR 레벨로 기록된다 (AC #2)', async () => {
    vi.useFakeTimers();
    mockInvoke
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      })
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'network_error', message: 'test error' },
      });

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await advanceRetryDelay();

    // ERROR 레벨 로그 확인
    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'WITHDRAW',
        expect.any(String),
        expect.anything()
      );
    });
  });
});

describe('WtsWindow withdraw flow', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockAddLog.mockReset();
    mockShowToast.mockReset();
    mockFetchBalance.mockReset();
  });

  it('출금 확인 성공 시 로그/토스트/잔고 갱신이 수행된다', async () => {
    mockInvoke.mockResolvedValueOnce({
      success: true,
      data: {
        type: 'withdraw',
        uuid: 'withdraw-uuid-123',
        currency: 'BTC',
        net_type: 'BTC',
        txid: null,
        state: 'processing',
        created_at: '2026-01-25T10:00:00Z',
        done_at: null,
        amount: '0.1',
        fee: '0.0005',
        transaction_type: 'default',
      },
    });

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('wts_withdraw', {
        params: {
          currency: withdrawParams.currency,
          net_type: withdrawParams.net_type,
          amount: withdrawParams.amount,
          address: withdrawParams.address,
          secondary_address: withdrawParams.secondary_address,
        },
      });
    });

    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith(
        'SUCCESS',
        'WITHDRAW',
        expect.stringContaining('출금 요청 완료: withdraw-uuid-123')
      );
    });
    expect(mockShowToast).toHaveBeenCalledWith('success', '출금 요청이 완료되었습니다');
    expect(mockFetchBalance).toHaveBeenCalledTimes(1);
    await waitFor(() => {
      expect(screen.getByText('check-status')).toBeTruthy();
    });
  });

  it('출금 상태 조회 시 TXID 생성 로그가 기록된다', async () => {
    mockInvoke
      .mockResolvedValueOnce({
        success: true,
        data: {
          type: 'withdraw',
          uuid: 'withdraw-uuid-123',
          currency: 'BTC',
          net_type: 'BTC',
          txid: null,
          state: 'processing',
          created_at: '2026-01-25T10:00:00Z',
          done_at: null,
          amount: '0.1',
          fee: '0.0005',
          transaction_type: 'default',
        },
      })
      .mockResolvedValueOnce({
        success: true,
        data: {
          type: 'withdraw',
          uuid: 'withdraw-uuid-123',
          currency: 'BTC',
          net_type: 'BTC',
          txid: 'txid-123',
          state: 'processing',
          created_at: '2026-01-25T10:00:00Z',
          done_at: null,
          amount: '0.1',
          fee: '0.0005',
          transaction_type: 'default',
        },
      });

    render(<WtsWindow />);

    fireEvent.click(screen.getByText('trigger-withdraw'));
    fireEvent.click(screen.getByText('confirm-withdraw'));

    await waitFor(() => {
      expect(screen.getByText('check-status')).toBeTruthy();
    });

    fireEvent.click(screen.getByText('check-status'));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('wts_get_withdraw', {
        params: { uuid: 'withdraw-uuid-123' },
      });
    });

    expect(mockAddLog).toHaveBeenCalledWith('INFO', 'WITHDRAW', 'TXID 생성됨: txid-123');
    expect(mockAddLog).toHaveBeenCalledWith(
      'INFO',
      'WITHDRAW',
      expect.stringContaining('출금 상태')
    );
  });

  // WTS-5.5: 2FA 및 출금 에러 처리
  describe('WTS-5.5: 출금 에러 처리', () => {
    it('2FA 에러 시 WARN 레벨로 기록하고 다이얼로그를 유지한다', async () => {
      mockInvoke.mockResolvedValueOnce({
        success: false,
        error: {
          code: 'two_factor_auth_required',
          message: '2FA 필요',
        },
      });

      render(<WtsWindow />);

      fireEvent.click(screen.getByText('trigger-withdraw'));
      fireEvent.click(screen.getByText('confirm-withdraw'));

      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'WARN',
          'WITHDRAW',
          expect.stringContaining('2FA'),
          expect.anything()
        );
      });

      // 다이얼로그가 여전히 열려있어야 함 (재시도 가능)
      expect(screen.getByText('confirm-withdraw')).toBeTruthy();
    });

    it('2FA 에러 시 추가 안내 메시지를 INFO 레벨로 기록한다', async () => {
      mockInvoke.mockResolvedValueOnce({
        success: false,
        error: {
          code: 'two_factor_auth_required',
          message: '2FA 필요',
        },
      });

      render(<WtsWindow />);

      fireEvent.click(screen.getByText('trigger-withdraw'));
      fireEvent.click(screen.getByText('confirm-withdraw'));

      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'INFO',
          'WITHDRAW',
          expect.stringContaining('Upbit 모바일 앱')
        );
      });
    });

    it('미등록 주소 에러 시 WARN 레벨로 기록하고 다이얼로그를 유지한다', async () => {
      mockInvoke.mockResolvedValueOnce({
        success: false,
        error: {
          code: 'unregistered_withdraw_address',
          message: '미등록 주소',
        },
      });

      render(<WtsWindow />);

      fireEvent.click(screen.getByText('trigger-withdraw'));
      fireEvent.click(screen.getByText('confirm-withdraw'));

      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'WARN',
          'WITHDRAW',
          expect.stringContaining('출금 주소'),
          expect.anything()
        );
      });

      // 다이얼로그가 여전히 열려있어야 함
      expect(screen.getByText('confirm-withdraw')).toBeTruthy();
    });

    it('미등록 주소 에러 시 등록 안내 URL을 INFO 레벨로 기록한다', async () => {
      mockInvoke.mockResolvedValueOnce({
        success: false,
        error: {
          code: 'unregistered_withdraw_address',
          message: '미등록 주소',
        },
      });

      render(<WtsWindow />);

      fireEvent.click(screen.getByText('trigger-withdraw'));
      fireEvent.click(screen.getByText('confirm-withdraw'));

      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'INFO',
          'WITHDRAW',
          expect.stringContaining('upbit.com')
        );
      });
    });

    it('under_min_amount 에러 시 ERROR 레벨로 기록한다', async () => {
      mockInvoke.mockResolvedValueOnce({
        success: false,
        error: {
          code: 'under_min_amount',
          message: '최소 수량 미만',
        },
      });

      render(<WtsWindow />);

      fireEvent.click(screen.getByText('trigger-withdraw'));
      fireEvent.click(screen.getByText('confirm-withdraw'));

      await waitFor(() => {
        expect(mockAddLog).toHaveBeenCalledWith(
          'ERROR',
          'WITHDRAW',
          expect.anything(),
          expect.anything()
        );
      });
    });
  });
});

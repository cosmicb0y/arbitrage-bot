import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
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

vi.mock('../stores/consoleStore', () => ({
  useConsoleStore: vi.fn((selector) => selector({ addLog: mockAddLog })),
}));

vi.mock('../stores/toastStore', () => ({
  useToastStore: vi.fn((selector) => selector({ showToast: mockShowToast })),
}));

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
  WithdrawConfirmDialog: ({ isOpen, onConfirm, onCancel }: { isOpen: boolean; onConfirm: () => void; onCancel: () => void }) =>
    isOpen ? (
      <div>
        <button type="button" onClick={onConfirm}>
          confirm-withdraw
        </button>
        <button type="button" onClick={onCancel}>
          cancel-withdraw
        </button>
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
});

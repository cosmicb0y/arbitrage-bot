import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { WithdrawConfirmDialog } from '../components/WithdrawConfirmDialog';
import type { WithdrawConfirmInfo } from '../types';

const mockWithdrawInfo: WithdrawConfirmInfo = {
  currency: 'BTC',
  net_type: 'BTC',
  address: 'bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh',
  secondary_address: null,
  amount: '0.1',
  fee: '0.0005',
  receivable: '0.0995',
};

describe('WithdrawConfirmDialog (WTS-5.6)', () => {
  describe('재시도 버튼 렌더링 (Subtask 5.6)', () => {
    it('retryable=true일 때 "다시 시도" 버튼이 표시된다', () => {
      render(
        <WithdrawConfirmDialog
          isOpen={true}
          withdrawInfo={mockWithdrawInfo}
          onConfirm={vi.fn()}
          onCancel={vi.fn()}
          retryable={true}
          onRetry={vi.fn()}
        />
      );

      expect(screen.getByText('다시 시도')).toBeTruthy();
    });

    it('retryable=false일 때 "다시 시도" 버튼이 표시되지 않는다', () => {
      render(
        <WithdrawConfirmDialog
          isOpen={true}
          withdrawInfo={mockWithdrawInfo}
          onConfirm={vi.fn()}
          onCancel={vi.fn()}
          retryable={false}
        />
      );

      expect(screen.queryByText('다시 시도')).toBeNull();
    });

    it('retryable=true이지만 onRetry가 없으면 버튼이 표시되지 않는다', () => {
      render(
        <WithdrawConfirmDialog
          isOpen={true}
          withdrawInfo={mockWithdrawInfo}
          onConfirm={vi.fn()}
          onCancel={vi.fn()}
          retryable={true}
        />
      );

      expect(screen.queryByText('다시 시도')).toBeNull();
    });

    it('retryLoading=true일 때 "재시도 중..." 텍스트가 표시된다', () => {
      render(
        <WithdrawConfirmDialog
          isOpen={true}
          withdrawInfo={mockWithdrawInfo}
          onConfirm={vi.fn()}
          onCancel={vi.fn()}
          retryable={true}
          onRetry={vi.fn()}
          retryLoading={true}
        />
      );

      expect(screen.getByText('재시도 중...')).toBeTruthy();
    });
  });

  describe('수동 재시도 클릭 (Subtask 5.7)', () => {
    it('재시도 버튼 클릭 시 onRetry가 호출된다', () => {
      const mockOnRetry = vi.fn();

      render(
        <WithdrawConfirmDialog
          isOpen={true}
          withdrawInfo={mockWithdrawInfo}
          onConfirm={vi.fn()}
          onCancel={vi.fn()}
          retryable={true}
          onRetry={mockOnRetry}
        />
      );

      fireEvent.click(screen.getByText('다시 시도'));

      expect(mockOnRetry).toHaveBeenCalledTimes(1);
    });

    it('retryLoading=true일 때 재시도 버튼이 비활성화된다', () => {
      const mockOnRetry = vi.fn();

      render(
        <WithdrawConfirmDialog
          isOpen={true}
          withdrawInfo={mockWithdrawInfo}
          onConfirm={vi.fn()}
          onCancel={vi.fn()}
          retryable={true}
          onRetry={mockOnRetry}
          retryLoading={true}
        />
      );

      const button = screen.getByText('재시도 중...').closest('button');
      expect(button).toHaveProperty('disabled', true);
    });
  });

  describe('기본 다이얼로그 동작', () => {
    it('isOpen=false일 때 다이얼로그가 렌더링되지 않는다', () => {
      render(
        <WithdrawConfirmDialog
          isOpen={false}
          withdrawInfo={mockWithdrawInfo}
          onConfirm={vi.fn()}
          onCancel={vi.fn()}
        />
      );

      expect(screen.queryByText('출금 확인')).toBeNull();
    });

    it('isOpen=true일 때 출금 정보가 표시된다', () => {
      render(
        <WithdrawConfirmDialog
          isOpen={true}
          withdrawInfo={mockWithdrawInfo}
          onConfirm={vi.fn()}
          onCancel={vi.fn()}
        />
      );

      expect(screen.getByText('출금 확인')).toBeTruthy();
      // BTC는 currency와 net_type 두 곳에 표시됨
      expect(screen.getAllByText('BTC').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('0.1')).toBeTruthy();
      expect(screen.getByText('0.0005')).toBeTruthy();
      expect(screen.getByText('0.0995')).toBeTruthy();
    });
  });
});

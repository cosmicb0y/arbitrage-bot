import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { WithdrawResultDialog } from '../../components/WithdrawResultDialog';
import type { WithdrawResultInfo } from '../../types';

describe('WithdrawResultDialog', () => {
  const baseResult: WithdrawResultInfo = {
    uuid: 'withdraw-uuid-12345',
    currency: 'BTC',
    net_type: 'BTC',
    state: 'processing',
    amount: '0.1',
    fee: '0.0005',
    txid: null,
    created_at: '2026-01-25T10:00:00Z',
  };

  const defaultProps = {
    isOpen: true,
    result: baseResult,
    onClose: vi.fn(),
    onCheckStatus: vi.fn().mockResolvedValue(undefined),
    isCheckingStatus: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('렌더링 (AC: #5)', () => {
    it('isOpen=true일 때 다이얼로그가 렌더링된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByRole('dialog')).toBeTruthy();
    });

    it('isOpen=false일 때 다이얼로그가 렌더링되지 않는다', () => {
      render(<WithdrawResultDialog {...defaultProps} isOpen={false} />);
      expect(screen.queryByRole('dialog')).toBeNull();
    });

    it('성공 제목이 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByText(/출금 요청 완료/)).toBeTruthy();
    });
  });

  describe('출금 정보 표시 (AC: #5)', () => {
    it('자산 코드가 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByText('자산')).toBeTruthy();
      expect(screen.getAllByText('BTC').length).toBeGreaterThanOrEqual(1);
    });

    it('네트워크가 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByText('네트워크')).toBeTruthy();
    });

    it('출금 수량이 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByText('수량')).toBeTruthy();
      expect(screen.getByText('0.1')).toBeTruthy();
    });

    it('출금 수수료가 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByText('수수료')).toBeTruthy();
      expect(screen.getByText('0.0005')).toBeTruthy();
    });
  });

  describe('출금 상태 표시 (AC: #5)', () => {
    it('processing 상태일 때 한국어 메시지가 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByText(/처리 중/)).toBeTruthy();
    });

    it('진행 중 상태일 때 예상 완료 안내가 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByTestId('withdraw-eta-text').textContent).toContain('예상 완료');
    });

    it('submitting 상태일 때 한국어 메시지가 표시된다', () => {
      render(
        <WithdrawResultDialog
          {...defaultProps}
          result={{ ...baseResult, state: 'submitting' }}
        />
      );
      expect(screen.getByText(/제출 중/)).toBeTruthy();
    });

    it('done 상태일 때 한국어 메시지가 표시된다', () => {
      render(
        <WithdrawResultDialog
          {...defaultProps}
          result={{ ...baseResult, state: 'done', txid: 'txid123' }}
        />
      );
      // 상태 텍스트에 "완료" 메시지가 표시되는지 확인
      const statusText = screen.getByTestId('withdraw-status-text');
      expect(statusText.textContent).toContain('완료');
    });

    it('진행 중 상태는 노란색으로 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      const statusText = screen.getByTestId('withdraw-status-text');
      expect(statusText.className).toContain('text-yellow-400');
    });

    it('완료 상태는 초록색으로 표시된다', () => {
      render(
        <WithdrawResultDialog
          {...defaultProps}
          result={{ ...baseResult, state: 'done', txid: 'txid123' }}
        />
      );
      const statusText = screen.getByTestId('withdraw-status-text');
      expect(statusText.className).toContain('text-green-400');
    });

    it('rejected 상태는 빨간색으로 표시된다', () => {
      render(
        <WithdrawResultDialog
          {...defaultProps}
          result={{ ...baseResult, state: 'rejected' }}
        />
      );
      const statusText = screen.getByTestId('withdraw-status-text');
      expect(statusText.className).toContain('text-red-400');
    });
  });

  describe('TXID 표시 (AC: #6, #8)', () => {
    it('TXID가 null이면 "블록체인 전송 대기 중" 메시지가 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.getByText(/블록체인 전송 대기 중/)).toBeTruthy();
    });

    it('TXID가 있으면 TXID가 표시된다', () => {
      const txid = 'abc123def456ghi789';
      render(
        <WithdrawResultDialog
          {...defaultProps}
          result={{ ...baseResult, txid }}
        />
      );
      expect(screen.getByText(txid)).toBeTruthy();
    });

    it('TXID가 있으면 복사 버튼이 표시된다', () => {
      render(
        <WithdrawResultDialog
          {...defaultProps}
          result={{ ...baseResult, txid: 'txid123' }}
        />
      );
      expect(screen.getByRole('button', { name: /복사/ })).toBeTruthy();
    });

    it('TXID가 null이면 복사 버튼이 표시되지 않는다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      expect(screen.queryByRole('button', { name: /복사/ })).toBeNull();
    });
  });

  describe('버튼 동작', () => {
    it('닫기 버튼 클릭 시 onClose가 호출된다', () => {
      const onClose = vi.fn();
      render(<WithdrawResultDialog {...defaultProps} onClose={onClose} />);

      fireEvent.click(screen.getByRole('button', { name: /닫기/ }));
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it('상태 확인 버튼 클릭 시 onCheckStatus가 호출된다', () => {
      const onCheckStatus = vi.fn().mockResolvedValue(undefined);
      render(
        <WithdrawResultDialog {...defaultProps} onCheckStatus={onCheckStatus} />
      );

      fireEvent.click(screen.getByRole('button', { name: /상태 확인/ }));
      expect(onCheckStatus).toHaveBeenCalledTimes(1);
    });

    it('isCheckingStatus=true일 때 상태 확인 버튼이 비활성화된다', () => {
      render(<WithdrawResultDialog {...defaultProps} isCheckingStatus={true} />);

      const checkBtn = screen.getByRole('button', { name: /확인 중/ });
      expect(checkBtn).toHaveProperty('disabled', true);
    });

    it('isCheckingStatus=true일 때 로딩 표시가 나타난다', () => {
      render(<WithdrawResultDialog {...defaultProps} isCheckingStatus={true} />);
      expect(screen.getByTestId('status-check-spinner')).toBeTruthy();
    });
  });

  describe('ESC 키 핸들러', () => {
    it('ESC 키로 onClose가 호출된다', () => {
      const onClose = vi.fn();
      render(<WithdrawResultDialog {...defaultProps} onClose={onClose} />);

      fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  describe('오버레이 클릭', () => {
    it('오버레이 클릭 시 onClose가 호출된다', () => {
      const onClose = vi.fn();
      render(<WithdrawResultDialog {...defaultProps} onClose={onClose} />);

      const overlay = screen.getByTestId('withdraw-result-overlay');
      fireEvent.click(overlay);
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  describe('TXID 복사 기능 (AC: #8)', () => {
    it('복사 버튼 클릭 시 클립보드에 TXID가 복사된다', async () => {
      const mockWriteText = vi.fn().mockResolvedValue(undefined);
      Object.assign(navigator, {
        clipboard: { writeText: mockWriteText },
      });

      render(
        <WithdrawResultDialog
          {...defaultProps}
          result={{ ...baseResult, txid: 'txid-to-copy' }}
        />
      );

      fireEvent.click(screen.getByRole('button', { name: /복사/ }));
      expect(mockWriteText).toHaveBeenCalledWith('txid-to-copy');
    });
  });

  describe('스타일링', () => {
    it('다이얼로그 헤더는 성공 아이콘과 함께 표시된다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      const header = screen.getByTestId('withdraw-result-header');
      expect(header.textContent).toContain('✅');
    });

    it('상태 확인 버튼은 wts-accent 스타일을 가진다', () => {
      render(<WithdrawResultDialog {...defaultProps} />);
      const checkBtn = screen.getByRole('button', { name: /상태 확인/ });
      expect(checkBtn.className).toContain('bg-wts-accent');
    });
  });
});

import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { WithdrawConfirmDialog } from '../../components/WithdrawConfirmDialog';
import type { WithdrawConfirmInfo } from '../../types';

describe('WithdrawConfirmDialog', () => {
  const baseWithdrawInfo: WithdrawConfirmInfo = {
    currency: 'BTC',
    net_type: 'BTC',
    address: 'bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh',
    secondary_address: null,
    amount: '0.1',
    fee: '0.0005',
    receivable: '0.0995',
  };

  const defaultProps = {
    isOpen: true,
    withdrawInfo: baseWithdrawInfo,
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
    isLoading: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('렌더링', () => {
    it('isOpen=true일 때 다이얼로그가 렌더링된다 (AC #1)', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByRole('dialog')).toBeTruthy();
    });

    it('isOpen=false일 때 다이얼로그가 렌더링되지 않는다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} isOpen={false} />);
      expect(screen.queryByRole('dialog')).toBeNull();
    });

    it('출금 확인 제목이 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText('출금 확인')).toBeTruthy();
    });
  });

  describe('출금 정보 표시 (AC #1)', () => {
    it('자산 코드가 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText('자산')).toBeTruthy();
      // BTC는 자산과 네트워크에 모두 표시되므로 getAllByText 사용
      expect(screen.getAllByText('BTC').length).toBeGreaterThanOrEqual(1);
    });

    it('네트워크가 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText('네트워크')).toBeTruthy();
    });

    it('출금 수량이 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText('출금 수량')).toBeTruthy();
      expect(screen.getByText('0.1')).toBeTruthy();
    });

    it('출금 수수료가 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText('출금 수수료')).toBeTruthy();
      expect(screen.getByText('0.0005')).toBeTruthy();
    });

    it('실수령액이 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText('실수령액')).toBeTruthy();
      expect(screen.getByText('0.0995')).toBeTruthy();
    });
  });

  describe('주소 표시 (AC #2)', () => {
    it('출금 주소가 전체 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText('출금 주소')).toBeTruthy();
      // 주소는 여러 span으로 분할되어 강조 표시됨
      const addressSection = screen.getByTestId('withdraw-address-highlighted');
      expect(addressSection.textContent).toBe('bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh');
    });

    it('주소 앞 8자와 뒤 8자가 강조 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      const addressSection = screen.getByTestId('withdraw-address-highlighted');
      expect(addressSection).toBeTruthy();
      // 앞 8자: bc1qxy2k
      // 뒤 8자: jhx0wlh
      const highlights = addressSection.querySelectorAll('.text-yellow-400');
      expect(highlights.length).toBe(2);
    });

    it('보조 주소(XRP tag)가 있으면 표시된다', () => {
      const infoWithTag: WithdrawConfirmInfo = {
        ...baseWithdrawInfo,
        currency: 'XRP',
        net_type: 'XRP',
        address: 'rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh',
        secondary_address: '123456789',
      };
      render(<WithdrawConfirmDialog {...defaultProps} withdrawInfo={infoWithTag} />);
      expect(screen.getByText(/Memo\/Tag/)).toBeTruthy();
      expect(screen.getByText('123456789')).toBeTruthy();
    });

    it('보조 주소가 없으면 Memo/Tag 섹션이 표시되지 않는다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.queryByText(/Memo\/Tag/)).toBeNull();
    });
  });

  describe('경고 문구 (AC #3)', () => {
    it('"주소를 다시 확인하세요" 경고 문구가 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText(/출금 주소를 다시 확인하세요/)).toBeTruthy();
    });

    it('경고 문구에 위험 안내가 포함된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText(/잘못된 주소로 출금하면 자산을 되찾을 수 없습니다/)).toBeTruthy();
    });
  });

  describe('버튼 (AC #4)', () => {
    it('확인 버튼이 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByRole('button', { name: /출금/ })).toBeTruthy();
    });

    it('취소 버튼이 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByRole('button', { name: /취소/ })).toBeTruthy();
    });
  });

  describe('3초 카운트다운 타이머 (AC #5)', () => {
    it('다이얼로그 열린 직후 확인 버튼이 비활성화된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      const confirmBtn = screen.getByRole('button', { name: /출금.*3초/ });
      expect(confirmBtn).toHaveProperty('disabled', true);
    });

    it('카운트다운이 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      expect(screen.getByText(/3초/)).toBeTruthy();
    });

    it('1초 후 카운트다운이 2초로 업데이트된다', async () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);

      act(() => {
        vi.advanceTimersByTime(1000);
      });

      expect(screen.getByText(/2초/)).toBeTruthy();
    });

    it('2초 후 카운트다운이 1초로 업데이트된다', async () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);

      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      expect(screen.getByText(/1초/)).toBeTruthy();
    });

    it('3초 후 확인 버튼이 활성화된다', async () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);

      // 각 1초씩 3번 진행 (React state update를 위해)
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      const confirmBtn = screen.getByRole('button', { name: '출금' });
      expect(confirmBtn).toHaveProperty('disabled', false);
    });

    it('다이얼로그가 닫혔다가 다시 열리면 카운트다운이 리셋된다', async () => {
      const { rerender } = render(<WithdrawConfirmDialog {...defaultProps} />);

      act(() => {
        vi.advanceTimersByTime(2000);
      });

      rerender(<WithdrawConfirmDialog {...defaultProps} isOpen={false} />);
      rerender(<WithdrawConfirmDialog {...defaultProps} isOpen={true} />);

      expect(screen.getByText(/3초/)).toBeTruthy();
    });
  });

  describe('버튼 동작', () => {
    it('확인 버튼 클릭 시 onConfirm이 호출된다 (카운트다운 후)', () => {
      const onConfirm = vi.fn();
      render(<WithdrawConfirmDialog {...defaultProps} onConfirm={onConfirm} />);

      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      fireEvent.click(screen.getByRole('button', { name: '출금' }));
      expect(onConfirm).toHaveBeenCalledTimes(1);
    });

    it('카운트다운 중에는 확인 버튼 클릭이 무시된다', () => {
      const onConfirm = vi.fn();
      render(<WithdrawConfirmDialog {...defaultProps} onConfirm={onConfirm} />);

      const confirmBtn = screen.getByRole('button', { name: /출금.*3초/ });
      fireEvent.click(confirmBtn);

      expect(onConfirm).not.toHaveBeenCalled();
    });

    it('취소 버튼 클릭 시 onCancel이 호출된다', () => {
      const onCancel = vi.fn();
      render(<WithdrawConfirmDialog {...defaultProps} onCancel={onCancel} />);

      fireEvent.click(screen.getByRole('button', { name: /취소/ }));
      expect(onCancel).toHaveBeenCalledTimes(1);
    });
  });

  describe('로딩 상태 (AC #6)', () => {
    it('isLoading=true일 때 확인 버튼이 비활성화된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} isLoading={true} />);

      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      const confirmBtn = screen.getByRole('button', { name: /처리중/ });
      expect(confirmBtn).toHaveProperty('disabled', true);
    });

    it('isLoading=true일 때 로딩 스피너가 표시된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} isLoading={true} />);

      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      expect(screen.getByTestId('withdraw-loading-spinner')).toBeTruthy();
    });

    it('isLoading=true일 때 취소 버튼이 비활성화된다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} isLoading={true} />);
      const cancelBtn = screen.getByRole('button', { name: /취소/ });
      expect(cancelBtn).toHaveProperty('disabled', true);
    });
  });

  describe('ESC 키 핸들러 (AC #9)', () => {
    it('ESC 키로 onCancel이 호출된다', () => {
      const onCancel = vi.fn();
      render(<WithdrawConfirmDialog {...defaultProps} onCancel={onCancel} />);

      fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it('isLoading=true일 때도 ESC 키로 onCancel이 호출된다', () => {
      const onCancel = vi.fn();
      render(<WithdrawConfirmDialog {...defaultProps} onCancel={onCancel} isLoading={true} />);

      fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
      expect(onCancel).toHaveBeenCalledTimes(1);
    });
  });

  describe('오버레이 클릭 (AC #10)', () => {
    it('오버레이 클릭 시 onCancel이 호출된다', () => {
      const onCancel = vi.fn();
      render(<WithdrawConfirmDialog {...defaultProps} onCancel={onCancel} />);

      const overlay = screen.getByTestId('withdraw-dialog-overlay');
      fireEvent.click(overlay);
      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it('isLoading=true일 때도 오버레이 클릭 시 onCancel이 호출된다', () => {
      const onCancel = vi.fn();
      render(<WithdrawConfirmDialog {...defaultProps} onCancel={onCancel} isLoading={true} />);

      const overlay = screen.getByTestId('withdraw-dialog-overlay');
      fireEvent.click(overlay);
      expect(onCancel).toHaveBeenCalledTimes(1);
    });
  });

  describe('포커스 트랩', () => {
    it('다이얼로그가 열리면 첫 번째 버튼에 포커스가 이동한다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);

      const cancelBtn = screen.getByRole('button', { name: /취소/ });
      // useEffect runs synchronously after render with fake timers
      expect(document.activeElement).toBe(cancelBtn);
    });

    it('Shift+Tab으로 마지막 버튼으로 순환된다 (카운트다운 완료 후)', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);

      // 카운트다운 완료 후 확인 버튼이 활성화되어야 포커스 가능
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      const dialog = screen.getByRole('dialog');
      const cancelBtn = screen.getByRole('button', { name: /취소/ });
      const confirmBtn = screen.getByRole('button', { name: '출금' });

      cancelBtn.focus();
      fireEvent.keyDown(dialog, { key: 'Tab', shiftKey: true });

      expect(document.activeElement).toBe(confirmBtn);
    });

    it('Tab으로 첫 번째 버튼으로 순환된다 (카운트다운 완료 후)', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);

      // 카운트다운 완료 후 확인 버튼이 활성화되어야 포커스 가능
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      const dialog = screen.getByRole('dialog');
      const cancelBtn = screen.getByRole('button', { name: /취소/ });
      const confirmBtn = screen.getByRole('button', { name: '출금' });

      confirmBtn.focus();
      fireEvent.keyDown(dialog, { key: 'Tab' });

      expect(document.activeElement).toBe(cancelBtn);
    });
  });

  describe('스타일링', () => {
    it('확인 버튼은 wts-accent(파란색) 스타일을 가진다', async () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);

      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      const confirmBtn = screen.getByRole('button', { name: '출금' });
      expect(confirmBtn.className).toContain('bg-wts-accent');
    });

    it('다이얼로그 헤더는 wts-accent 스타일을 가진다', () => {
      render(<WithdrawConfirmDialog {...defaultProps} />);
      const header = screen.getByText('출금 확인');
      expect(header.className).toContain('text-wts-accent');
    });
  });
});

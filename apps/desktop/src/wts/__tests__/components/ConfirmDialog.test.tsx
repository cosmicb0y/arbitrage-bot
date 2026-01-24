import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import type { OrderConfirmInfo } from '../../components/ConfirmDialog';

describe('ConfirmDialog', () => {
  const baseOrderInfo: OrderConfirmInfo = {
    market: 'KRW-BTC',
    side: 'buy',
    orderType: 'market',
    price: '100000',
  };

  const defaultProps = {
    isOpen: true,
    orderInfo: baseOrderInfo,
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
    isLoading: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('렌더링', () => {
    it('isOpen=true일 때 다이얼로그가 렌더링된다', () => {
      render(<ConfirmDialog {...defaultProps} />);
      expect(screen.getByRole('dialog')).toBeTruthy();
    });

    it('isOpen=false일 때 다이얼로그가 렌더링되지 않는다', () => {
      render(<ConfirmDialog {...defaultProps} isOpen={false} />);
      expect(screen.queryByRole('dialog')).toBeNull();
    });

    it('주문 확인 제목이 표시된다', () => {
      render(<ConfirmDialog {...defaultProps} />);
      expect(screen.getByText('주문 확인')).toBeTruthy();
    });

    it('마켓 정보가 표시된다', () => {
      render(<ConfirmDialog {...defaultProps} />);
      expect(screen.getByText('KRW-BTC')).toBeTruthy();
    });

    it('시장가 매수 정보가 표시된다', () => {
      render(<ConfirmDialog {...defaultProps} />);
      expect(screen.getByText('시장가 매수')).toBeTruthy();
    });

    it('시장가 매도 정보가 표시된다', () => {
      const sellInfo: OrderConfirmInfo = {
        market: 'KRW-BTC',
        side: 'sell',
        orderType: 'market',
        quantity: '0.001',
      };
      render(<ConfirmDialog {...defaultProps} orderInfo={sellInfo} />);
      expect(screen.getByText('시장가 매도')).toBeTruthy();
    });

    it('시장가 주문 경고 메시지가 표시된다', () => {
      render(<ConfirmDialog {...defaultProps} />);
      expect(screen.getByText(/시장가 주문은 즉시 체결됩니다/)).toBeTruthy();
    });

    it('확인/취소 버튼이 표시된다', () => {
      render(<ConfirmDialog {...defaultProps} />);
      expect(screen.getByRole('button', { name: /취소/ })).toBeTruthy();
      expect(screen.getByRole('button', { name: /매수/ })).toBeTruthy();
    });
  });

  describe('주문 정보 표시', () => {
    it('시장가 매수: 주문 금액(KRW)이 표시된다', () => {
      render(<ConfirmDialog {...defaultProps} />);
      expect(screen.getByText(/주문금액/)).toBeTruthy();
      expect(screen.getByText(/100,000/)).toBeTruthy();
    });

    it('시장가 매도: 수량이 표시된다', () => {
      const sellInfo: OrderConfirmInfo = {
        market: 'KRW-BTC',
        side: 'sell',
        orderType: 'market',
        quantity: '0.001',
      };
      render(<ConfirmDialog {...defaultProps} orderInfo={sellInfo} />);
      expect(screen.getByText(/수량/)).toBeTruthy();
      expect(screen.getByText(/0.001/)).toBeTruthy();
    });

    it('지정가 주문: 가격, 수량, 총액이 표시된다', () => {
      const limitInfo: OrderConfirmInfo = {
        market: 'KRW-BTC',
        side: 'buy',
        orderType: 'limit',
        quantity: '0.1',
        price: '50000000',
        total: 5000000,
      };
      render(<ConfirmDialog {...defaultProps} orderInfo={limitInfo} />);
      // "가격" 라벨이 존재하는지 확인 (getAllBy 사용 - 안내 문구에도 "가격" 포함됨)
      expect(screen.getAllByText(/가격/).length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText(/50,000,000/)).toBeTruthy();
      expect(screen.getByText(/수량/)).toBeTruthy();
      expect(screen.getByText(/0.1/)).toBeTruthy();
      expect(screen.getByText(/총액/)).toBeTruthy();
      expect(screen.getByText(/5,000,000/)).toBeTruthy();
    });

    it('지정가 주문: 안내 문구가 표시된다 (AC #2)', () => {
      const limitInfo: OrderConfirmInfo = {
        market: 'KRW-BTC',
        side: 'buy',
        orderType: 'limit',
        quantity: '0.1',
        price: '50000000',
        total: 5000000,
      };
      render(<ConfirmDialog {...defaultProps} orderInfo={limitInfo} />);
      expect(screen.getByText(/지정가 주문은 해당 가격에 도달하면 체결됩니다/)).toBeTruthy();
    });

    it('지정가 주문: 시장가 경고 문구는 표시되지 않는다', () => {
      const limitInfo: OrderConfirmInfo = {
        market: 'KRW-BTC',
        side: 'buy',
        orderType: 'limit',
        quantity: '0.1',
        price: '50000000',
        total: 5000000,
      };
      render(<ConfirmDialog {...defaultProps} orderInfo={limitInfo} />);
      expect(screen.queryByText(/시장가 주문은 즉시 체결됩니다/)).toBeNull();
    });
  });

  describe('스타일링', () => {
    it('매수 버튼은 녹색 계열 스타일을 가진다', () => {
      render(<ConfirmDialog {...defaultProps} />);
      const confirmBtn = screen.getByRole('button', { name: /매수/ });
      expect(confirmBtn.className).toContain('bg-green');
    });

    it('매도 버튼은 빨간색 계열 스타일을 가진다', () => {
      const sellInfo: OrderConfirmInfo = {
        market: 'KRW-BTC',
        side: 'sell',
        orderType: 'market',
        quantity: '0.001',
      };
      render(<ConfirmDialog {...defaultProps} orderInfo={sellInfo} />);
      const confirmBtn = screen.getByRole('button', { name: /매도/ });
      expect(confirmBtn.className).toContain('bg-red');
    });
  });

  describe('버튼 동작', () => {
    it('확인 버튼 클릭 시 onConfirm이 호출된다', () => {
      const onConfirm = vi.fn();
      render(<ConfirmDialog {...defaultProps} onConfirm={onConfirm} />);

      fireEvent.click(screen.getByRole('button', { name: /매수/ }));

      expect(onConfirm).toHaveBeenCalledTimes(1);
    });

    it('취소 버튼 클릭 시 onCancel이 호출된다', () => {
      const onCancel = vi.fn();
      render(<ConfirmDialog {...defaultProps} onCancel={onCancel} />);

      fireEvent.click(screen.getByRole('button', { name: /취소/ }));

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it('isLoading=true일 때 확인 버튼이 비활성화된다', () => {
      render(<ConfirmDialog {...defaultProps} isLoading={true} />);
      const confirmBtn = screen.getByRole('button', { name: /처리중/ });
      expect(confirmBtn).toHaveProperty('disabled', true);
    });

    it('isLoading=true일 때 취소 버튼이 비활성화된다', () => {
      render(<ConfirmDialog {...defaultProps} isLoading={true} />);
      const cancelBtn = screen.getByRole('button', { name: /취소/ });
      expect(cancelBtn).toHaveProperty('disabled', true);
    });
  });

  describe('키보드 동작', () => {
    it('Enter 키로 확인이 실행된다', () => {
      const onConfirm = vi.fn();
      render(<ConfirmDialog {...defaultProps} onConfirm={onConfirm} />);

      fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Enter' });

      expect(onConfirm).toHaveBeenCalledTimes(1);
    });

    it('Escape 키로 취소가 실행된다', () => {
      const onCancel = vi.fn();
      render(<ConfirmDialog {...defaultProps} onCancel={onCancel} />);

      fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it('isLoading=true일 때 Enter 키가 무시된다', () => {
      const onConfirm = vi.fn();
      render(<ConfirmDialog {...defaultProps} onConfirm={onConfirm} isLoading={true} />);

      fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Enter' });

      expect(onConfirm).not.toHaveBeenCalled();
    });

    it('isLoading=true일 때 Escape 키가 무시된다', () => {
      const onCancel = vi.fn();
      render(<ConfirmDialog {...defaultProps} onCancel={onCancel} isLoading={true} />);

      fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });

      expect(onCancel).not.toHaveBeenCalled();
    });
  });

  describe('오버레이', () => {
    it('오버레이 클릭 시 onCancel이 호출된다', () => {
      const onCancel = vi.fn();
      render(<ConfirmDialog {...defaultProps} onCancel={onCancel} />);

      const overlay = screen.getByTestId('dialog-overlay');
      fireEvent.click(overlay);

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it('isLoading=true일 때 오버레이 클릭이 무시된다', () => {
      const onCancel = vi.fn();
      render(<ConfirmDialog {...defaultProps} onCancel={onCancel} isLoading={true} />);

      const overlay = screen.getByTestId('dialog-overlay');
      fireEvent.click(overlay);

      expect(onCancel).not.toHaveBeenCalled();
    });
  });
});

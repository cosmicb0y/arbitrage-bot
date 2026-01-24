import { useCallback, useEffect, useRef } from 'react';
import { formatKrw } from '../utils/formatters';

/**
 * 주문 확인 정보
 */
export interface OrderConfirmInfo {
  /** 마켓 코드 (예: "KRW-BTC") */
  market: string;
  /** 주문 방향: buy(매수) | sell(매도) */
  side: 'buy' | 'sell';
  /** 주문 유형: market(시장가) | limit(지정가) */
  orderType: 'market' | 'limit';
  /** 수량 (시장가 매도 또는 지정가) */
  quantity?: string;
  /** 가격 (시장가 매수: KRW 총액, 지정가: 단가) */
  price?: string;
  /** 예상 총액 (지정가) */
  total?: number;
}

interface ConfirmDialogProps {
  /** 다이얼로그 표시 여부 */
  isOpen: boolean;
  /** 주문 정보 */
  orderInfo: OrderConfirmInfo;
  /** 확인 콜백 */
  onConfirm: () => void;
  /** 취소 콜백 */
  onCancel: () => void;
  /** 로딩 상태 (API 호출 중) */
  isLoading?: boolean;
}

/**
 * 주문 유형 + 방향 → 한국어 라벨
 */
function getOrderLabel(orderType: 'market' | 'limit', side: 'buy' | 'sell'): string {
  const typeLabel = orderType === 'market' ? '시장가' : '지정가';
  const sideLabel = side === 'buy' ? '매수' : '매도';
  return `${typeLabel} ${sideLabel}`;
}

/**
 * 주문 확인 다이얼로그 컴포넌트
 * - Enter: 확인
 * - Escape: 취소
 * - 매수: 녹색, 매도: 빨간색 스타일링
 */
export function ConfirmDialog({
  isOpen,
  orderInfo,
  onConfirm,
  onCancel,
  isLoading = false,
}: ConfirmDialogProps) {
  const { market, side, orderType, quantity, price } = orderInfo;
  const isBuy = side === 'buy';
  const isMarket = orderType === 'market';
  const dialogRef = useRef<HTMLDivElement>(null);

  // 키보드 이벤트 핸들러 (dialog 요소에서만 사용)
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (isLoading) return;

      if (e.key === 'Enter') {
        e.preventDefault();
        onConfirm();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onCancel();
      }
    },
    [isLoading, onConfirm, onCancel]
  );

  useEffect(() => {
    if (isOpen) {
      dialogRef.current?.focus();
    }
  }, [isOpen]);

  if (!isOpen) return null;

  // 오버레이 클릭 핸들러
  const handleOverlayClick = () => {
    if (!isLoading) {
      onCancel();
    }
  };

  // 다이얼로그 내부 클릭 시 이벤트 전파 방지
  const handleDialogClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  // 확인 버튼 라벨
  const confirmLabel = isBuy ? '매수' : '매도';

  // 확인 버튼 색상
  const confirmBtnClass = isBuy
    ? 'bg-green-600 hover:bg-green-700 disabled:bg-green-800'
    : 'bg-red-600 hover:bg-red-700 disabled:bg-red-800';

  return (
    <div
      data-testid="dialog-overlay"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={handleOverlayClick}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
        className="bg-wts-secondary border border-wts rounded-lg shadow-xl w-80 max-w-[90vw]"
        onClick={handleDialogClick}
        onKeyDown={handleKeyDown}
        ref={dialogRef}
        tabIndex={-1}
      >
        {/* 헤더 */}
        <div className="px-4 py-3 border-b border-wts">
          <h2 id="dialog-title" className="text-base font-semibold text-wts-foreground">
            주문 확인
          </h2>
        </div>

        {/* 본문 */}
        <div className="px-4 py-4 space-y-3">
          {/* 마켓 */}
          <div className="flex justify-between text-sm">
            <span className="text-wts-muted">마켓</span>
            <span className="text-wts-foreground font-mono">{market}</span>
          </div>

          {/* 주문 유형 */}
          <div className="flex justify-between text-sm">
            <span className="text-wts-muted">유형</span>
            <span className={`font-medium ${isBuy ? 'text-green-500' : 'text-red-500'}`}>
              {getOrderLabel(orderType, side)}
            </span>
          </div>

          {/* 시장가 매수: 주문금액 (KRW) */}
          {isMarket && isBuy && price && (
            <div className="flex justify-between text-sm">
              <span className="text-wts-muted">주문금액</span>
              <span className="text-wts-foreground font-mono">
                {formatKrw(parseFloat(price))}
              </span>
            </div>
          )}

          {/* 시장가 매도: 수량 */}
          {isMarket && !isBuy && quantity && (
            <div className="flex justify-between text-sm">
              <span className="text-wts-muted">수량</span>
              <span className="text-wts-foreground font-mono">{quantity}</span>
            </div>
          )}

          {/* 지정가: 가격/수량/총액 */}
          {!isMarket && (
            <>
              {price && (
                <div className="flex justify-between text-sm">
                  <span className="text-wts-muted">가격</span>
                  <span className="text-wts-foreground font-mono">
                    {formatKrw(parseFloat(price))}
                  </span>
                </div>
              )}
              {quantity && (
                <div className="flex justify-between text-sm">
                  <span className="text-wts-muted">수량</span>
                  <span className="text-wts-foreground font-mono">{quantity}</span>
                </div>
              )}
              {orderInfo.total !== undefined && (
                <div className="flex justify-between text-sm">
                  <span className="text-wts-muted">총액</span>
                  <span className="text-wts-foreground font-mono">
                    {formatKrw(orderInfo.total)}
                  </span>
                </div>
              )}
            </>
          )}

          {/* 경고/안내 메시지 */}
          {isMarket && (
            <div className="mt-3 p-2 bg-yellow-900/30 border border-yellow-700/50 rounded text-xs text-yellow-500">
              ⚠️ 시장가 주문은 즉시 체결됩니다
            </div>
          )}
          {!isMarket && (
            <div className="mt-3 p-2 bg-blue-900/30 border border-blue-700/50 rounded text-xs text-blue-400">
              ℹ️ 지정가 주문은 해당 가격에 도달하면 체결됩니다
            </div>
          )}
        </div>

        {/* 버튼 영역 */}
        <div className="px-4 py-3 border-t border-wts flex gap-2">
          <button
            onClick={onCancel}
            disabled={isLoading}
            className="flex-1 py-2 text-sm font-medium rounded
                       bg-wts-tertiary text-wts-muted
                       hover:bg-wts-secondary hover:text-wts-foreground
                       disabled:opacity-50 disabled:cursor-not-allowed
                       transition-colors"
          >
            취소
          </button>
          <button
            onClick={onConfirm}
            disabled={isLoading}
            className={`flex-1 py-2 text-sm font-medium rounded text-white
                        ${confirmBtnClass}
                        disabled:opacity-50 disabled:cursor-not-allowed
                        transition-colors`}
          >
            {isLoading ? '처리중...' : confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

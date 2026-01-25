import { useCallback, useEffect, useRef, useState } from 'react';
import type { WithdrawConfirmInfo } from '../types';

interface WithdrawConfirmDialogProps {
  /** 다이얼로그 표시 여부 */
  isOpen: boolean;
  /** 출금 정보 */
  withdrawInfo: WithdrawConfirmInfo;
  /** 확인 콜백 */
  onConfirm: () => void;
  /** 취소 콜백 */
  onCancel: () => void;
  /** 로딩 상태 (API 호출 중) */
  isLoading?: boolean;
}

/**
 * 주소 강조 표시
 * 앞 8자, 뒤 8자를 노란색으로 강조
 */
function formatHighlightedAddress(address: string): JSX.Element {
  if (address.length <= 16) {
    return <span className="text-yellow-400 font-mono">{address}</span>;
  }

  const prefix = address.slice(0, 8);
  const middle = address.slice(8, -8);
  const suffix = address.slice(-8);

  return (
    <span className="font-mono">
      <span className="text-yellow-400">{prefix}</span>
      <span className="text-wts-muted">{middle}</span>
      <span className="text-yellow-400">{suffix}</span>
    </span>
  );
}

/**
 * 출금 확인 다이얼로그 컴포넌트
 * - 3초 카운트다운 후 확인 버튼 활성화
 * - ESC: 취소
 * - 주소 앞/뒤 8자 강조 표시
 * - wts-accent(파란색) 스타일링
 */
export function WithdrawConfirmDialog({
  isOpen,
  withdrawInfo,
  onConfirm,
  onCancel,
  isLoading = false,
}: WithdrawConfirmDialogProps) {
  const {
    currency,
    net_type,
    address,
    secondary_address,
    amount,
    fee,
    receivable,
  } = withdrawInfo;

  const dialogRef = useRef<HTMLDivElement>(null);
  const [countdown, setCountdown] = useState(3);
  const [isConfirmEnabled, setIsConfirmEnabled] = useState(false);

  // 카운트다운 타이머
  useEffect(() => {
    if (!isOpen) {
      setCountdown(3);
      setIsConfirmEnabled(false);
      return;
    }

    if (countdown > 0) {
      const timer = setTimeout(() => setCountdown(countdown - 1), 1000);
      return () => clearTimeout(timer);
    } else {
      setIsConfirmEnabled(true);
    }
  }, [isOpen, countdown]);

  const getFocusableElements = useCallback(() => {
    const dialog = dialogRef.current;
    if (!dialog) return [];
    const elements = Array.from(
      dialog.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      )
    );
    return elements.filter((element) => {
      const isDisabled = (element as HTMLButtonElement).disabled === true;
      const isHidden = element.getAttribute('aria-hidden') === 'true';
      return !isDisabled && !isHidden;
    });
  }, []);

  // 키보드 이벤트 핸들러
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Tab') {
        const focusable = getFocusableElements();
        if (focusable.length === 0) {
          e.preventDefault();
          return;
        }

        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        const active = document.activeElement as HTMLElement | null;

        if (e.shiftKey) {
          if (active === first || !dialogRef.current?.contains(active)) {
            e.preventDefault();
            last.focus();
          }
        } else {
          if (active === last) {
            e.preventDefault();
            first.focus();
          }
        }
        return;
      }

      if (e.key === 'Escape') {
        e.preventDefault();
        onCancel();
      }
    },
    [getFocusableElements, onCancel]
  );

  // 다이얼로그 열릴 때 첫 번째 요소에 포커스
  useEffect(() => {
    if (isOpen) {
      const focusable = getFocusableElements();
      if (focusable.length > 0) {
        focusable[0].focus();
      } else {
        dialogRef.current?.focus();
      }
    }
  }, [getFocusableElements, isOpen]);

  if (!isOpen) return null;

  // 오버레이 클릭 핸들러
  const handleOverlayClick = () => {
    onCancel();
  };

  // 다이얼로그 내부 클릭 시 이벤트 전파 방지
  const handleDialogClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  // 확인 버튼 클릭
  const handleConfirmClick = () => {
    if (isConfirmEnabled && !isLoading) {
      onConfirm();
    }
  };

  return (
    <div
      data-testid="withdraw-dialog-overlay"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={handleOverlayClick}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="withdraw-dialog-title"
        className="bg-wts-secondary border border-wts rounded-lg shadow-xl w-96 max-w-[90vw]"
        onClick={handleDialogClick}
        onKeyDown={handleKeyDown}
        ref={dialogRef}
        tabIndex={-1}
      >
        {/* 헤더 */}
        <div className="px-4 py-3 border-b border-wts-accent/50">
          <h2
            id="withdraw-dialog-title"
            className="text-base font-semibold text-wts-accent"
          >
            출금 확인
          </h2>
        </div>

        {/* 본문 */}
        <div className="px-4 py-4 space-y-3">
          {/* 자산 */}
          <div className="flex justify-between text-sm">
            <span className="text-wts-muted">자산</span>
            <span className="text-wts-foreground font-mono">{currency}</span>
          </div>

          {/* 네트워크 */}
          <div className="flex justify-between text-sm">
            <span className="text-wts-muted">네트워크</span>
            <span className="text-wts-foreground font-mono">{net_type}</span>
          </div>

          {/* 출금 주소 */}
          <div className="text-sm">
            <span className="text-wts-muted block mb-1">출금 주소</span>
            <div
              data-testid="withdraw-address-highlighted"
              className="bg-black/20 p-2 rounded break-all text-xs"
            >
              {formatHighlightedAddress(address)}
            </div>
          </div>

          {/* 보조 주소 (XRP tag, EOS memo 등) */}
          {secondary_address && (
            <div className="text-sm">
              <span className="text-wts-muted block mb-1">Memo/Tag</span>
              <div className="font-mono text-wts-foreground bg-black/20 p-2 rounded">
                {secondary_address}
              </div>
            </div>
          )}

          {/* 경고 문구 */}
          <div className="mt-3 p-2 bg-red-900/30 border border-red-700/50 rounded text-xs text-red-400">
            <div className="flex items-start gap-1">
              <span>⚠️</span>
              <div>
                <div className="font-medium">출금 주소를 다시 확인하세요</div>
                <div className="text-red-300 mt-0.5">
                  잘못된 주소로 출금하면 자산을 되찾을 수 없습니다.
                </div>
              </div>
            </div>
          </div>

          {/* 출금 정보 */}
          <div className="border-t border-wts pt-3 space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-wts-muted">출금 수량</span>
              <span className="text-wts-foreground font-mono">{amount}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-wts-muted">출금 수수료</span>
              <span className="text-wts-foreground font-mono">{fee}</span>
            </div>
            <div className="flex justify-between text-sm font-medium">
              <span className="text-wts-muted">실수령액</span>
              <span className="text-green-400 font-mono">{receivable}</span>
            </div>
          </div>
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
            onClick={handleConfirmClick}
            disabled={!isConfirmEnabled || isLoading}
            className="flex-1 py-2 text-sm font-medium rounded text-white
                       bg-wts-accent hover:bg-wts-accent/80
                       disabled:opacity-50 disabled:cursor-not-allowed
                       transition-colors"
          >
            {isLoading ? (
              <span className="inline-flex items-center justify-center gap-2">
                <span
                  data-testid="withdraw-loading-spinner"
                  aria-hidden="true"
                  className="animate-spin"
                >
                  ⏳
                </span>
                처리중...
              </span>
            ) : isConfirmEnabled ? (
              '출금'
            ) : (
              `출금 (${countdown}초)`
            )}
          </button>
        </div>
      </div>
    </div>
  );
}

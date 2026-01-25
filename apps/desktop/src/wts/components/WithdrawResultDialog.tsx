import { useCallback, useEffect, useRef } from 'react';
import type { WithdrawResultInfo, WithdrawState } from '../types';
import {
  WITHDRAW_STATE_MESSAGES,
  isWithdrawPending,
  isWithdrawFailed,
  isWithdrawComplete,
} from '../types';

interface WithdrawResultDialogProps {
  /** 다이얼로그 표시 여부 */
  isOpen: boolean;
  /** 출금 결과 정보 */
  result: WithdrawResultInfo;
  /** 닫기 콜백 */
  onClose: () => void;
  /** 상태 확인 콜백 */
  onCheckStatus: () => Promise<void>;
  /** 상태 확인 로딩 중 */
  isCheckingStatus: boolean;
}

/**
 * 출금 상태에 따른 색상 클래스 반환
 */
function getStatusColorClass(state: WithdrawState): string {
  if (isWithdrawComplete(state)) {
    return 'text-green-400';
  }
  if (isWithdrawFailed(state)) {
    return 'text-red-400';
  }
  // 진행 중 상태
  return 'text-yellow-400';
}

/**
 * 출금 결과 다이얼로그 컴포넌트
 * - 출금 성공 후 상세 정보 표시
 * - TXID 표시 및 복사 기능
 * - 상태 조회 기능
 * - ESC: 닫기
 */
export function WithdrawResultDialog({
  isOpen,
  result,
  onClose,
  onCheckStatus,
  isCheckingStatus,
}: WithdrawResultDialogProps) {
  const { currency, net_type, state, amount, fee, txid } = result;
  const dialogRef = useRef<HTMLDivElement>(null);

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
        onClose();
      }
    },
    [getFocusableElements, onClose]
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
    onClose();
  };

  // 다이얼로그 내부 클릭 시 이벤트 전파 방지
  const handleDialogClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  // 상태 확인 버튼 클릭
  const handleCheckStatus = () => {
    if (!isCheckingStatus) {
      onCheckStatus();
    }
  };

  // TXID 복사
  const handleCopyTxid = async () => {
    if (!txid) return;
    try {
      await navigator.clipboard.writeText(txid);
    } catch {
      // 복사 실패 - 에러 무시
    }
  };

  const stateMessage = WITHDRAW_STATE_MESSAGES[state];
  const statusColorClass = getStatusColorClass(state);

  return (
    <div
      data-testid="withdraw-result-overlay"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={handleOverlayClick}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="withdraw-result-title"
        className="bg-wts-secondary border border-wts rounded-lg shadow-xl w-96 max-w-[90vw]"
        onClick={handleDialogClick}
        onKeyDown={handleKeyDown}
        ref={dialogRef}
        tabIndex={-1}
      >
        {/* 헤더 */}
        <div
          data-testid="withdraw-result-header"
          className="px-4 py-3 border-b border-wts-accent/50"
        >
          <h2
            id="withdraw-result-title"
            className="text-base font-semibold text-green-400"
          >
            ✅ 출금 요청 완료
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

          {/* 수량 */}
          <div className="flex justify-between text-sm">
            <span className="text-wts-muted">수량</span>
            <span className="text-wts-foreground font-mono">{amount}</span>
          </div>

          {/* 수수료 */}
          <div className="flex justify-between text-sm">
            <span className="text-wts-muted">수수료</span>
            <span className="text-wts-foreground font-mono">{fee}</span>
          </div>

          {/* 구분선 */}
          <div className="border-t border-wts pt-3">
            {/* 상태 */}
            <div className="flex justify-between text-sm mb-3">
              <span className="text-wts-muted">상태</span>
              <span
                data-testid="withdraw-status-text"
                className={`font-medium ${statusColorClass}`}
              >
                {stateMessage}
                {isWithdrawPending(state) && ' 🔄'}
              </span>
            </div>
            {isWithdrawPending(state) && (
              <div
                data-testid="withdraw-eta-text"
                className="text-xs text-wts-muted mb-3"
              >
                예상 완료: 블록체인 전송까지 수 분 소요될 수 있습니다.
              </div>
            )}

            {/* TXID */}
            <div className="text-sm">
              <span className="text-wts-muted block mb-1">TXID</span>
              {txid ? (
                <div className="flex items-center gap-2">
                  <div className="flex-1 bg-black/20 p-2 rounded break-all text-xs font-mono text-wts-foreground">
                    {txid}
                  </div>
                  <button
                    onClick={handleCopyTxid}
                    className="px-2 py-1 text-xs font-medium rounded
                               bg-wts-tertiary text-wts-muted
                               hover:bg-wts-secondary hover:text-wts-foreground
                               transition-colors"
                  >
                    복사
                  </button>
                </div>
              ) : (
                <div className="bg-black/20 p-2 rounded text-xs text-wts-muted italic">
                  블록체인 전송 대기 중...
                </div>
              )}
            </div>
          </div>
        </div>

        {/* 버튼 영역 */}
        <div className="px-4 py-3 border-t border-wts flex gap-2">
          <button
            onClick={handleCheckStatus}
            disabled={isCheckingStatus}
            className="flex-1 py-2 text-sm font-medium rounded text-white
                       bg-wts-accent hover:bg-wts-accent/80
                       disabled:opacity-50 disabled:cursor-not-allowed
                       transition-colors"
          >
            {isCheckingStatus ? (
              <span className="inline-flex items-center justify-center gap-2">
                <span
                  data-testid="status-check-spinner"
                  aria-hidden="true"
                  className="animate-spin"
                >
                  ⏳
                </span>
                확인 중...
              </span>
            ) : (
              '상태 확인'
            )}
          </button>
          <button
            onClick={onClose}
            className="flex-1 py-2 text-sm font-medium rounded
                       bg-wts-tertiary text-wts-muted
                       hover:bg-wts-secondary hover:text-wts-foreground
                       transition-colors"
          >
            닫기
          </button>
        </div>
      </div>
    </div>
  );
}

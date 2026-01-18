interface BalancePanelProps {
  className?: string;
}

export function BalancePanel({ className = '' }: BalancePanelProps) {
  return (
    <div
      data-testid="balance-panel"
      className={`wts-area-balances wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Balances</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        <p className="text-wts-muted text-xs">
          잔고가 여기에 표시됩니다 (Epic 2에서 구현)
        </p>
      </div>
    </div>
  );
}

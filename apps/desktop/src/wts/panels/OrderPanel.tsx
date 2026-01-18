interface OrderPanelProps {
  className?: string;
}

export function OrderPanel({ className = '' }: OrderPanelProps) {
  return (
    <div
      data-testid="order-panel"
      className={`wts-area-order wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Order</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        <p className="text-wts-muted text-xs">
          주문 폼이 여기에 표시됩니다 (Epic 3에서 구현)
        </p>
      </div>
    </div>
  );
}

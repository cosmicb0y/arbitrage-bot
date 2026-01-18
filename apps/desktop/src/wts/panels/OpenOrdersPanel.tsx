interface OpenOrdersPanelProps {
  className?: string;
}

export function OpenOrdersPanel({ className = '' }: OpenOrdersPanelProps) {
  return (
    <div
      data-testid="open-orders-panel"
      className={`wts-area-openOrders wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Open Orders</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        <p className="text-wts-muted text-xs">
          미체결 주문이 여기에 표시됩니다 (Epic 3에서 구현)
        </p>
      </div>
    </div>
  );
}

interface OrderbookPanelProps {
  className?: string;
}

export function OrderbookPanel({ className = '' }: OrderbookPanelProps) {
  return (
    <div
      data-testid="orderbook-panel"
      className={`wts-area-orderbook wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Orderbook</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        <p className="text-wts-muted text-xs">
          오더북이 여기에 표시됩니다 (Epic 2에서 구현)
        </p>
      </div>
    </div>
  );
}

import { useOrderStore } from '../stores/orderStore';

interface OrderPanelProps {
  className?: string;
}

export function OrderPanel({ className = '' }: OrderPanelProps) {
  const { orderType, side, price, setPrice } = useOrderStore();
  const orderTypeLabel = orderType === 'limit' ? '지정가' : '시장가';
  const sideLabel = side === 'buy' ? '매수' : '매도';

  return (
    <div
      data-testid="order-panel"
      className={`wts-area-order wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Order</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        <div className="space-y-2">
          <div className="flex items-center justify-between text-xs">
            <span className="text-wts-muted">주문 유형</span>
            <span className="text-wts-foreground">{orderTypeLabel}</span>
          </div>
          <div className="flex items-center justify-between text-xs">
            <span className="text-wts-muted">방향</span>
            <span className="text-wts-foreground">{sideLabel}</span>
          </div>
          <label className="flex items-center justify-between gap-2 text-xs">
            <span className="text-wts-muted">가격</span>
            <input
              type="text"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              placeholder="가격 입력"
              disabled={orderType !== 'limit'}
              className="w-28 px-2 py-1 rounded border border-wts bg-wts-secondary text-wts-foreground focus:outline-none disabled:opacity-50"
            />
          </label>
        </div>
      </div>
    </div>
  );
}

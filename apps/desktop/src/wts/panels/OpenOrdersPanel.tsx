import { useMemo } from 'react';
import { Wifi, WifiOff, AlertCircle } from 'lucide-react';
import { useOpenOrdersStore } from '../stores/openOrdersStore';
import { useUpbitMyOrderWs } from '../hooks/useUpbitMyOrderWs';
import type { UpbitMyOrderResponse } from '../types';

interface OpenOrdersPanelProps {
  className?: string;
}

export function OpenOrdersPanel({ className = '' }: OpenOrdersPanelProps) {
  // Initialize WebSocket connection
  useUpbitMyOrderWs();

  const { orders, wsStatus, wsError } = useOpenOrdersStore();

  // Sort orders by order_timestamp descending (newest first)
  const sortedOrders = useMemo(() => {
    return [...orders].sort((a, b) => b.order_timestamp - a.order_timestamp);
  }, [orders]);

  const statusIcon =
    wsStatus === 'connected' ? (
      <Wifi className="w-3 h-3 text-wts-success" />
    ) : wsStatus === 'connecting' ? (
      <Wifi className="w-3 h-3 text-wts-warning animate-pulse" />
    ) : (
      <WifiOff className="w-3 h-3 text-wts-muted" />
    );

  return (
    <div
      data-testid="open-orders-panel"
      className={`wts-area-openOrders wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header flex justify-between items-center">
        <span>Open Orders</span>
        <div className="flex items-center gap-2">
          {wsError && (
            <div className="flex items-center gap-1 text-wts-destructive">
              <AlertCircle className="w-3 h-3" />
              <span className="text-[10px]">{wsError}</span>
            </div>
          )}
          <div className="flex items-center gap-1" title={`WebSocket: ${wsStatus}`}>
            {statusIcon}
            <span className="text-[10px] text-wts-muted">{orders.length}</span>
          </div>
        </div>
      </div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        {sortedOrders.length === 0 ? (
          <p className="text-wts-muted text-xs text-center py-4">
            미체결 주문이 없습니다
          </p>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="text-wts-muted border-b border-wts">
                <th className="text-left py-1">마켓</th>
                <th className="text-center py-1">구분</th>
                <th className="text-right py-1">가격</th>
                <th className="text-right py-1">수량</th>
                <th className="text-right py-1">미체결</th>
                <th className="text-right py-1">시간</th>
              </tr>
            </thead>
            <tbody>
              {sortedOrders.map((order) => (
                <OrderRow key={order.uuid} order={order} />
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

interface OrderRowProps {
  order: UpbitMyOrderResponse;
}

function OrderRow({ order }: OrderRowProps) {
  const isBuy = order.side === 'bid';
  const sideText = isBuy ? '매수' : '매도';
  const sideClass = isBuy ? 'text-wts-success' : 'text-wts-destructive';

  const formatPrice = (price: number) =>
    price.toLocaleString('ko-KR', { maximumFractionDigits: 0 });

  const formatVolume = (volume: number) => {
    if (volume >= 1) {
      return volume.toLocaleString('ko-KR', { maximumFractionDigits: 4 });
    }
    return volume.toLocaleString('ko-KR', { maximumFractionDigits: 8 });
  };

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('ko-KR', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  };

  // Calculate filled percentage
  const filledPercent =
    order.volume > 0
      ? Math.round((order.executed_volume / order.volume) * 100)
      : 0;

  return (
    <tr className="border-b border-wts/30 hover:bg-wts-tertiary/50">
      <td className="py-1.5 font-medium">{order.market}</td>
      <td className={`py-1.5 text-center font-medium ${sideClass}`}>
        {sideText}
      </td>
      <td className="py-1.5 text-right font-mono">{formatPrice(order.price)}</td>
      <td className="py-1.5 text-right font-mono">{formatVolume(order.volume)}</td>
      <td className="py-1.5 text-right font-mono">
        <span>{formatVolume(order.remaining_volume)}</span>
        {filledPercent > 0 && (
          <span className="ml-1 text-[10px] text-wts-muted">
            ({filledPercent}%)
          </span>
        )}
      </td>
      <td className="py-1.5 text-right font-mono text-wts-muted">
        {formatTime(order.order_timestamp)}
      </td>
    </tr>
  );
}

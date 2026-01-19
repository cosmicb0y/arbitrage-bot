import { useMemo, memo } from 'react';
import { MarketSelector } from '../components/MarketSelector';
import { useWtsStore } from '../stores/wtsStore';
import { useOrderbookStore } from '../stores/orderbookStore';
import { useUpbitOrderbookWs } from '../hooks/useUpbitOrderbookWs';
import type { OrderbookEntry } from '../types';

/**
 * 가격 포맷: 천 단위 콤마, 정수 (KRW)
 */
function formatPrice(price: number): string {
  return price.toLocaleString('ko-KR');
}

/**
 * 수량 포맷: 소수점 8자리, 후행 0 제거
 */
function formatSize(size: number): string {
  return size.toFixed(8).replace(/\.?0+$/, '');
}

interface OrderbookRowProps {
  entry: OrderbookEntry;
  side: 'ask' | 'bid';
  maxSize: number;
}

/**
 * 개별 호가 행 (메모이제이션)
 */
const OrderbookRow = memo(function OrderbookRow({
  entry,
  side,
  maxSize,
}: OrderbookRowProps) {
  const depthWidth = maxSize > 0 ? (entry.size / maxSize) * 100 : 0;
  const colorClass = side === 'ask' ? 'text-destructive' : 'text-success';
  const bgClass = side === 'ask' ? 'bg-destructive/20' : 'bg-success/20';

  return (
    <div className="relative flex items-center h-6 px-2 text-xs font-mono">
      <div
        className={`absolute inset-y-0 ${side === 'ask' ? 'right-0' : 'left-0'} ${bgClass}`}
        style={{ width: `${depthWidth}%` }}
      />
      <span className={`relative flex-1 text-left ${colorClass}`}>
        {formatPrice(entry.price)}
      </span>
      <span className="relative text-right text-wts-foreground">
        {formatSize(entry.size)}
      </span>
    </div>
  );
});

interface OrderbookPanelProps {
  className?: string;
}

export function OrderbookPanel({ className = '' }: OrderbookPanelProps) {
  const {
    selectedExchange,
    selectedMarket,
    setMarket,
    connectionStatus,
    availableMarkets,
  } = useWtsStore();
  const { asks, bids, wsStatus, wsError } = useOrderbookStore();

  // WebSocket 연결 훅
  useUpbitOrderbookWs(selectedMarket, selectedExchange);

  // depth bar 계산용 최대 수량
  const maxSize = useMemo(() => {
    const allSizes = [...asks.map((a) => a.size), ...bids.map((b) => b.size)];
    return Math.max(...allSizes, 0.00000001);
  }, [asks, bids]);

  const isDisabled = connectionStatus !== 'connected';
  const isLoading = wsStatus === 'connecting';
  const hasData = asks.length > 0 && bids.length > 0;
  const showError = Boolean(wsError) && !isLoading;

  return (
    <div
      data-testid="orderbook-panel"
      className={`wts-area-orderbook wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header flex justify-between items-center">
        <div className="flex items-center gap-2">
          <span>Orderbook</span>
          {/* WebSocket 상태 인디케이터 */}
          <span
            className={`w-2 h-2 rounded-full ${
              wsStatus === 'connected'
                ? 'bg-success'
                : wsStatus === 'connecting'
                  ? 'bg-warning animate-pulse'
                  : 'bg-destructive'
            }`}
          />
        </div>
        <MarketSelector
          markets={availableMarkets}
          selectedMarket={selectedMarket}
          onSelect={setMarket}
          disabled={isDisabled}
        />
      </div>

      <div className="wts-panel-content flex-1 overflow-y-auto">
        {!selectedMarket ? (
          <p className="text-wts-muted text-xs text-center py-4">
            마켓을 선택하세요
          </p>
        ) : isLoading ? (
          <p className="text-wts-muted text-xs text-center py-4">연결 중...</p>
        ) : showError ? (
          <p className="text-destructive text-xs text-center py-4">
            {wsError}
          </p>
        ) : !hasData ? (
          <p className="text-wts-muted text-xs text-center py-4">
            데이터 대기 중...
          </p>
        ) : (
          <div className="flex flex-col">
            {/* 매도 호가 (역순: 높은 가격이 아래) */}
            <div className="flex flex-col-reverse">
              {asks.slice(0, 15).map((entry) => (
                <OrderbookRow
                  key={`ask-${entry.price}`}
                  entry={entry}
                  side="ask"
                  maxSize={maxSize}
                />
              ))}
            </div>

            {/* 중앙 구분선 */}
            <div className="h-px bg-wts-border my-1" />

            {/* 매수 호가 */}
            <div className="flex flex-col">
              {bids.slice(0, 15).map((entry) => (
                <OrderbookRow
                  key={`bid-${entry.price}`}
                  entry={entry}
                  side="bid"
                  maxSize={maxSize}
                />
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

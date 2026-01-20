import { useMemo, memo, useRef, useEffect, useState, useCallback } from 'react';
import { MarketSelector } from '../components/MarketSelector';
import { useWtsStore } from '../stores/wtsStore';
import { useOrderbookStore } from '../stores/orderbookStore';
import { useOrderStore } from '../stores/orderStore';
import { useConsoleStore } from '../stores/consoleStore';
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

/**
 * 가격 변동 플래시 애니메이션 훅
 * @param currentPrice 현재 가격
 * @returns 플래시 상태 ('up' | 'down' | null)
 */
function usePriceFlash(currentPrice: number): 'up' | 'down' | null {
  const prevPriceRef = useRef<number>(currentPrice);
  const [flash, setFlash] = useState<'up' | 'down' | null>(null);

  useEffect(() => {
    if (prevPriceRef.current !== currentPrice) {
      if (currentPrice > prevPriceRef.current) {
        setFlash('up');
      } else if (currentPrice < prevPriceRef.current) {
        setFlash('down');
      }
      prevPriceRef.current = currentPrice;

      const timer = setTimeout(() => setFlash(null), 300);
      return () => clearTimeout(timer);
    }
  }, [currentPrice]);

  return flash;
}

interface OrderbookRowProps {
  entry: OrderbookEntry;
  side: 'ask' | 'bid';
  maxSize: number;
  onClick?: (price: number, side: 'ask' | 'bid') => void;
}

/**
 * 개별 호가 행 (메모이제이션)
 * - 가격 변동 시 플래시 애니메이션 (300ms)
 * - 호버 시 배경색 하이라이트
 * - 클릭 시 주문 폼에 가격 자동 입력
 */
const OrderbookRow = memo(function OrderbookRow({
  entry,
  side,
  maxSize,
  onClick,
}: OrderbookRowProps) {
  const flash = usePriceFlash(entry.price);
  const depthWidth = maxSize > 0 ? (entry.size / maxSize) * 100 : 0;
  const colorClass = side === 'ask' ? 'text-destructive' : 'text-success';
  const bgClass = side === 'ask' ? 'bg-destructive/20' : 'bg-success/20';

  const flashClass =
    flash === 'up'
      ? 'animate-flash-up'
      : flash === 'down'
        ? 'animate-flash-down'
        : '';

  const handleClick = () => {
    onClick?.(entry.price, side);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleClick();
    }
  };

  return (
    <div
      className={`relative flex items-center h-6 px-2 text-xs font-mono cursor-pointer hover:bg-wts-secondary/50 transition-colors ${flashClass}`}
      onClick={handleClick}
      role="button"
      tabIndex={0}
      onKeyDown={handleKeyDown}
    >
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
  const setPriceFromOrderbook = useOrderStore(
    (state) => state.setPriceFromOrderbook
  );
  const addLog = useConsoleStore((state) => state.addLog);

  // WebSocket 연결 훅
  useUpbitOrderbookWs(selectedMarket, selectedExchange);

  // depth bar 계산용 최대 수량
  const maxSize = useMemo(() => {
    const allSizes = [...asks.map((a) => a.size), ...bids.map((b) => b.size)];
    return Math.max(...allSizes, 0.00000001);
  }, [asks, bids]);

  // 호가 클릭 핸들러
  const handleRowClick = useCallback(
    (price: number, clickedSide: 'ask' | 'bid') => {
      setPriceFromOrderbook(price, clickedSide);

      const side = clickedSide === 'ask' ? '매수' : '매도';
      addLog(
        'INFO',
        'ORDER',
        `호가 선택: ${price.toLocaleString('ko-KR')} KRW (${side})`
      );
    },
    [setPriceFromOrderbook, addLog]
  );

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
              {asks.slice(0, 15).map((entry, index) => (
                <OrderbookRow
                  key={`ask-${index}`}
                  entry={entry}
                  side="ask"
                  maxSize={maxSize}
                  onClick={handleRowClick}
                />
              ))}
            </div>

            {/* 중앙 구분선 */}
            <div className="h-px bg-wts-border my-1" />

            {/* 매수 호가 */}
            <div className="flex flex-col">
              {bids.slice(0, 15).map((entry, index) => (
                <OrderbookRow
                  key={`bid-${index}`}
                  entry={entry}
                  side="bid"
                  maxSize={maxSize}
                  onClick={handleRowClick}
                />
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

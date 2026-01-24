import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useOrderStore } from '../stores/orderStore';
import { useBalanceStore } from '../stores/balanceStore';
import { useWtsStore } from '../stores/wtsStore';
import { useConsoleStore } from '../stores/consoleStore';
import { useToastStore } from '../stores/toastStore';
import { formatKrw, formatNumber } from '../utils/formatters';
import { ConfirmDialog } from '../components/ConfirmDialog';
import type { OrderConfirmInfo } from '../components/ConfirmDialog';
import type {
  OrderType,
  OrderSide,
  OrderParams,
  OrderResponse,
  WtsApiResult,
} from '../types';
import { toUpbitSide, toUpbitOrderType, getOrderErrorMessage } from '../types';

interface OrderPanelProps {
  className?: string;
}

/**
 * 숫자 + 소수점만 허용하는 입력 검증
 */
function sanitizeNumericInput(value: string): string {
  if (value.includes('-')) return '';
  // 소수점 하나만 허용, 숫자만 허용
  const sanitized = value.replace(/[^0-9.]/g, '');
  const parts = sanitized.split('.');
  if (parts.length > 2) {
    return parts[0] + '.' + parts.slice(1).join('');
  }
  return sanitized;
}

/**
 * 가격 입력값 포맷팅 (천 단위 콤마) - 편집 가능하도록 trailing 0 유지
 */
function formatPriceInput(value: string): string {
  if (!value) return '';
  const numStr = value.replace(/,/g, '');
  const num = parseFloat(numStr);
  if (isNaN(num)) return value;
  // 정수 부분만 포맷팅, 소수점 이하는 그대로
  const parts = numStr.split('.');
  const intPart = parseInt(parts[0], 10);
  if (isNaN(intPart)) return value;
  const formatted = intPart.toLocaleString('ko-KR');
  if (parts.length > 1) {
    return formatted + '.' + parts[1];
  }
  return formatted;
}

/**
 * 포맷된 가격에서 원래 숫자 문자열 추출
 */
function unformatPrice(value: string): string {
  return value.replace(/,/g, '');
}

/**
 * 마켓 코드에서 코인 심볼 추출 (예: "KRW-BTC" → "BTC")
 */
function getCoinFromMarket(market: string | null): string {
  if (!market) return 'COIN';
  const parts = market.split('-');
  return parts[1] || 'COIN';
}

/**
 * 특정 화폐의 가용 잔고 조회
 */
function getAvailableBalance(
  balances: { currency: string; balance: string }[],
  currency: string
): number {
  const entry = balances.find((b) => b.currency === currency);
  if (!entry) return 0;
  return parseFloat(entry.balance);
}

export function OrderPanel({ className = '' }: OrderPanelProps) {
  const { orderType, side, price, quantity, setOrderType, setSide, setPrice, setQuantity } =
    useOrderStore();
  const { balances, fetchBalance } = useBalanceStore();
  const { selectedMarket } = useWtsStore();
  const addLog = useConsoleStore((state) => state.addLog);
  const showToast = useToastStore((state) => state.showToast);

  // 주문 관련 상태
  const [isConfirmOpen, setIsConfirmOpen] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const coin = getCoinFromMarket(selectedMarket);
  const krwBalance = getAvailableBalance(balances, 'KRW');
  const coinBalance = getAvailableBalance(balances, coin);

  const isMarket = orderType === 'market';

  // 예상 총액 계산
  const calculateTotal = (): number => {
    if (isMarket && side === 'buy') {
      const priceNum = parseFloat(unformatPrice(price));
      return Number.isFinite(priceNum) ? priceNum : 0;
    }
    if (isMarket && side === 'sell') {
      return 0;
    }
    const priceNum = parseFloat(unformatPrice(price));
    const qtyNum = parseFloat(quantity);
    if (!Number.isFinite(priceNum) || !Number.isFinite(qtyNum)) return 0;
    return priceNum * qtyNum;
  };

  const total = calculateTotal();

  // 잔고 초과 여부 확인
  const isOverBalance = (): boolean => {
    if (side === 'buy') {
      return total > krwBalance;
    } else {
      // 매도: 코인 잔고 확인
      const qtyNum = parseFloat(quantity) || 0;
      return qtyNum > coinBalance;
    }
    return false;
  };

  const overBalance = isOverBalance();

  // 가격 입력 핸들러
  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const raw = unformatPrice(e.target.value);
    const sanitized = sanitizeNumericInput(raw);
    setPrice(sanitized);
  };

  // 수량 입력 핸들러
  const handleQuantityChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const sanitized = sanitizeNumericInput(e.target.value);
    setQuantity(sanitized);
  };

  // % 버튼 클릭 핸들러
  const handlePercentClick = (percent: number) => {
    if (side === 'buy') {
      // 매수: KRW 잔고 / 가격 = 수량
      const priceNum = parseFloat(unformatPrice(price)) || 0;
      if (priceNum > 0) {
        const totalKrw = krwBalance * (percent / 100);
        const qty = totalKrw / priceNum;
        setQuantity(qty.toFixed(8).replace(/\.?0+$/, ''));
      }
    } else {
      // 매도: 코인 잔고의 %
      const qty = coinBalance * (percent / 100);
      setQuantity(qty.toFixed(8).replace(/\.?0+$/, ''));
    }
  };

  // 탭 클릭 핸들러
  const handleTabClick = (type: OrderType) => {
    setOrderType(type);
  };

  // 매수/매도 방향 버튼 클릭 핸들러
  const handleSideClick = (newSide: OrderSide) => {
    setSide(newSide);
  };

  // 주문 제출 버튼 클릭 → 확인 다이얼로그 표시
  const handleOrderClick = () => {
    if (!selectedMarket) return;
    if (isMarket && side === 'buy' && !price) return;
    if (isMarket && side === 'sell' && !quantity) return;
    setIsConfirmOpen(true);
  };

  // 주문 확인 → 실제 API 호출
  const handleConfirmOrder = useCallback(async () => {
    if (!selectedMarket || isSubmitting) return;

    setIsSubmitting(true);

    // 파라미터 빌드
    const upbitSide = toUpbitSide(side);
    const upbitOrdType = toUpbitOrderType(orderType, side);

    const params: OrderParams = {
      market: selectedMarket,
      side: upbitSide,
      ord_type: upbitOrdType,
    };

    // 시장가 매수: price(총액) 설정, volume 없음
    if (isMarket && side === 'buy') {
      params.price = unformatPrice(price);
      addLog(
        'INFO',
        'ORDER',
        `시장가 매수 주문 요청: ${selectedMarket}, ${formatKrw(parseFloat(unformatPrice(price)))}`
      );
    }
    // 시장가 매도: volume 설정, price 없음
    else if (isMarket && side === 'sell') {
      params.volume = quantity;
      addLog(
        'INFO',
        'ORDER',
        `시장가 매도 주문 요청: ${selectedMarket}, ${quantity} ${coin}`
      );
    }
    // 지정가: volume + price 모두 설정
    else {
      params.volume = quantity;
      params.price = unformatPrice(price);
      const sideLabel = side === 'buy' ? '매수' : '매도';
      addLog(
        'INFO',
        'ORDER',
        `지정가 ${sideLabel} 주문 요청: ${selectedMarket}, ${quantity} @ ${formatKrw(parseFloat(unformatPrice(price)))}`
      );
    }

    try {
      const result = await invoke<WtsApiResult<OrderResponse>>('wts_place_order', { params });

      if (result.success && result.data) {
        const sideLabel = side === 'buy' ? '매수' : '매도';
        const executedVolume = result.data.executed_volume || result.data.volume || quantity;
        const orderState = result.data.state;
        const isWait = orderState === 'wait';
        const isDone = orderState === 'done';
        const isCancel = orderState === 'cancel';
        const isTrade = orderState === 'trade';
        const statusLabel = isWait
          ? '등록'
          : isDone
            ? '체결'
            : isCancel
              ? '취소'
              : isTrade
                ? '부분 체결'
                : '처리';
        const logLevel = isCancel ? 'WARN' : 'SUCCESS';

        addLog(
          logLevel,
          'ORDER',
          `주문 ${statusLabel}: ${sideLabel} ${executedVolume} ${coin} @ ${
            isMarket ? '시장가' : formatKrw(parseFloat(unformatPrice(price)))
          }`
        );

        // 응답 상태에 따른 토스트 메시지 분기 (AC #5)
        const toastMessage = isWait
          ? '주문이 등록되었습니다'
          : isDone
            ? '주문이 체결되었습니다'
            : isCancel
              ? '주문이 취소되었습니다'
              : isTrade
                ? '주문이 부분 체결되었습니다'
                : '주문이 처리되었습니다';
        showToast(isCancel ? 'error' : 'success', toastMessage);

        // 잔고 갱신
        setTimeout(() => {
          fetchBalance();
        }, 500);

        // 폼 리셋
        setQuantity('');
        if (!isMarket) setPrice('');

        setIsConfirmOpen(false);
      } else {
        const errorCode = result.error?.code || 'unknown';
        const errorMsg = getOrderErrorMessage(errorCode, result.error?.message);

        addLog('ERROR', 'ORDER', `주문 실패: ${errorMsg}`);
        showToast('error', errorMsg);
        setIsConfirmOpen(false);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      addLog('ERROR', 'ORDER', `주문 실패: ${errorMsg}`);
      showToast('error', '주문 처리 중 오류가 발생했습니다');
      setIsConfirmOpen(false);
    } finally {
      setIsSubmitting(false);
    }
  }, [
    selectedMarket,
    isSubmitting,
    side,
    orderType,
    isMarket,
    price,
    quantity,
    coin,
    addLog,
    showToast,
    fetchBalance,
    setQuantity,
    setPrice,
  ]);

  // 확인 다이얼로그용 주문 정보
  const normalizedPrice = unformatPrice(price);
  const orderConfirmInfo: OrderConfirmInfo = {
    market: selectedMarket || '',
    side,
    orderType,
    quantity: quantity ? quantity : undefined,
    price: normalizedPrice ? normalizedPrice : undefined,
    total: !isMarket ? total : undefined,
  };

  // 주문 버튼 비활성화 조건
  const isOrderDisabled = (() => {
    if (!selectedMarket) return true;
    if (isSubmitting) return true;
    if (isMarket && side === 'buy') {
      const priceNum = parseFloat(unformatPrice(price)) || 0;
      return priceNum <= 0;
    }
    if (isMarket && side === 'sell') {
      const qtyNum = parseFloat(quantity) || 0;
      return qtyNum <= 0;
    }
    // 지정가
    const priceNum = parseFloat(unformatPrice(price)) || 0;
    const qtyNum = parseFloat(quantity) || 0;
    return priceNum <= 0 || qtyNum <= 0;
  })();

  // 가격 필드 비활성화 조건: 시장가
  const isPriceDisabled = isMarket && side === 'sell';
  // 가격 필드 placeholder
  const pricePlaceholder =
    isMarket && side === 'buy' ? '주문 금액 입력' : isMarket ? '시장가' : '가격 입력';
  // 가격 필드 라벨
  const priceLabel = isMarket && side === 'buy' ? '주문 금액' : '가격';
  // 수량 필드 표시 조건: 항상 표시
  const showQuantityField = true;

  return (
    <div
      data-testid="order-panel"
      className={`wts-area-order wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Order</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        <div className="space-y-3">
          {/* 주문 유형 탭 (Task 1) */}
          <div className="flex border-b border-wts" role="tablist">
            <button
              role="tab"
              aria-selected={orderType === 'limit'}
              onClick={() => handleTabClick('limit')}
              className={`flex-1 py-2 text-sm font-medium transition-colors
                ${
                  orderType === 'limit'
                    ? 'text-wts-foreground border-b-2 border-wts-accent'
                    : 'text-wts-muted hover:text-wts-foreground'
                }
              `}
            >
              지정가
            </button>
            <button
              role="tab"
              aria-selected={orderType === 'market'}
              onClick={() => handleTabClick('market')}
              className={`flex-1 py-2 text-sm font-medium transition-colors
                ${
                  orderType === 'market'
                    ? 'text-wts-foreground border-b-2 border-wts-accent'
                    : 'text-wts-muted hover:text-wts-foreground'
                }
              `}
            >
              시장가
            </button>
          </div>

          {/* 매수/매도 버튼 (Task 6) */}
          <div className="flex gap-2">
            <button
              onClick={() => handleSideClick('buy')}
              className={`flex-1 py-2 text-sm font-medium rounded transition-colors
                ${
                  side === 'buy'
                    ? 'bg-green-600 text-white'
                    : 'bg-wts-secondary text-wts-muted hover:bg-wts-tertiary'
                }
              `}
            >
              매수
            </button>
            <button
              onClick={() => handleSideClick('sell')}
              className={`flex-1 py-2 text-sm font-medium rounded transition-colors
                ${
                  side === 'sell'
                    ? 'bg-red-600 text-white'
                    : 'bg-wts-secondary text-wts-muted hover:bg-wts-tertiary'
                }
              `}
            >
              매도
            </button>
          </div>

          {/* 가격 입력 (Task 2) */}
          <label className="block text-xs">
            <span className="text-wts-muted mb-1 block">
              {priceLabel} <span className="text-wts-foreground">KRW</span>
            </span>
            <input
              type="text"
              inputMode="decimal"
              value={formatPriceInput(price)}
              onChange={handlePriceChange}
              placeholder={pricePlaceholder}
              disabled={isPriceDisabled}
              aria-label={priceLabel}
              className="w-full px-3 py-2 rounded border border-wts bg-wts-secondary
                         text-wts-foreground font-mono text-right
                         focus:outline-none focus:border-wts-focus
                         disabled:opacity-50 disabled:cursor-not-allowed"
            />
          </label>

          {/* 수량 입력 (Task 3) */}
          {showQuantityField && (
            <label className="block text-xs">
              <span className="text-wts-muted mb-1 block">
                수량 <span className="text-wts-foreground">{coin}</span>
              </span>
              <input
                type="text"
                inputMode="decimal"
                value={quantity}
                onChange={handleQuantityChange}
                placeholder="수량 입력"
                aria-label={`수량 ${coin}`}
                className="w-full px-3 py-2 rounded border border-wts bg-wts-secondary
                           text-wts-foreground font-mono text-right
                           focus:outline-none focus:border-wts-focus"
              />
            </label>
          )}

          {/* % 버튼 (Task 4) */}
          <div className="grid grid-cols-4 gap-1">
            {[25, 50, 75, 100].map((percent) => (
              <button
                key={percent}
                onClick={() => handlePercentClick(percent)}
                className="py-1.5 text-xs font-medium rounded
                           bg-wts-secondary text-wts-muted
                           hover:bg-wts-tertiary hover:text-wts-foreground
                           transition-colors"
              >
                {percent === 100 ? 'MAX' : `${percent}%`}
              </button>
            ))}
          </div>

          {/* 예상 총액 (Task 5) */}
          <div className="pt-2 border-t border-wts">
            <div className="flex items-center justify-between text-xs">
              <span className="text-wts-muted">예상 총액</span>
              <span className={`font-mono ${overBalance ? 'text-red-500' : 'text-wts-foreground'}`}>
                {formatKrw(total)}
              </span>
            </div>
            {overBalance && (
              <div className="text-xs text-red-500 mt-1">잔고 초과</div>
            )}
          </div>

          {/* 가용 잔고 표시 */}
          <div className="text-xs text-wts-muted">
            <div className="flex justify-between">
              <span>가용 KRW</span>
              <span className="font-mono">{formatNumber(krwBalance)}</span>
            </div>
            <div className="flex justify-between">
              <span>가용 {coin}</span>
              <span className="font-mono">{coinBalance.toFixed(8).replace(/\.?0+$/, '')}</span>
            </div>
          </div>

          {/* 주문 제출 버튼 */}
          <button
            data-testid="order-submit-btn"
            onClick={handleOrderClick}
            disabled={isOrderDisabled}
            className={`w-full py-3 text-sm font-bold rounded transition-colors
              ${
                side === 'buy'
                  ? 'bg-green-600 hover:bg-green-700 disabled:bg-green-800'
                  : 'bg-red-600 hover:bg-red-700 disabled:bg-red-800'
              }
              text-white disabled:opacity-50 disabled:cursor-not-allowed
            `}
          >
            {isSubmitting ? (
              <span className="flex items-center justify-center gap-2">
                <span className="animate-spin">⏳</span>
                처리중...
              </span>
            ) : (
              `${isMarket ? '시장가' : '지정가'} ${side === 'buy' ? '매수' : '매도'}`
            )}
          </button>
        </div>
      </div>

      {/* 주문 확인 다이얼로그 */}
      <ConfirmDialog
        isOpen={isConfirmOpen}
        orderInfo={orderConfirmInfo}
        onConfirm={handleConfirmOrder}
        onCancel={() => setIsConfirmOpen(false)}
        isLoading={isSubmitting}
      />
    </div>
  );
}

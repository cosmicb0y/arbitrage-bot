import { useEffect, useMemo } from 'react';
import { useBalanceStore } from '../stores/balanceStore';
import { useWtsStore } from '../stores/wtsStore';
import { formatCrypto, formatKrw, formatNumber } from '../utils/formatters';
import type { BalanceEntry } from '../types';

interface BalancePanelProps {
  className?: string;
}

export function BalancePanel({ className = '' }: BalancePanelProps) {
  const { connectionStatus, selectedExchange } = useWtsStore();
  const {
    balances,
    previousBalances,
    isLoading,
    hideZeroBalances,
    fetchBalance,
    setHideZeroBalances,
  } = useBalanceStore();

  // 연결 성공 시 잔고 조회
  useEffect(() => {
    if (connectionStatus === 'connected') {
      fetchBalance();
    }
  }, [connectionStatus, selectedExchange, fetchBalance]);

  // 0 잔고 필터링
  const filteredBalances = useMemo(() => {
    if (!hideZeroBalances) return balances;
    return balances.filter(
      (b) => parseFloat(b.balance) > 0 || parseFloat(b.locked) > 0
    );
  }, [balances, hideZeroBalances]);

  // 변화량 계산 함수
  const getBalanceChange = (
    currency: string,
    currentTotal: number
  ): number | null => {
    const prev = previousBalances.find((b) => b.currency === currency);
    if (!prev) return null;
    const prevTotal = parseFloat(prev.balance) + parseFloat(prev.locked);
    const diff = currentTotal - prevTotal;
    if (Math.abs(diff) < 1e-10) return null;
    return diff;
  };

  return (
    <div
      data-testid="balance-panel"
      className={`wts-area-balances wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header flex justify-between items-center">
        <span>Balances</span>
        <label className="flex items-center gap-1 text-xs cursor-pointer">
          <input
            type="checkbox"
            checked={hideZeroBalances}
            onChange={(e) => setHideZeroBalances(e.target.checked)}
            className="w-3 h-3"
          />
          <span className="text-wts-muted">0 잔고 숨기기</span>
        </label>
      </div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="animate-pulse space-y-2">
            {[1, 2, 3].map((i) => (
              <div
                key={i}
                data-testid="balance-skeleton"
                className="h-6 bg-wts-tertiary rounded"
              />
            ))}
          </div>
        ) : filteredBalances.length === 0 ? (
          <p className="text-wts-muted text-xs text-center py-4">잔고 없음</p>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="text-wts-muted border-b border-wts">
                <th className="text-left py-1">자산</th>
                <th className="text-right py-1">가용</th>
                <th className="text-right py-1">잠금</th>
                <th className="text-right py-1">평균 매수가</th>
                <th className="text-right py-1">평가금액</th>
              </tr>
            </thead>
            <tbody>
              {filteredBalances.map((entry) => {
                const totalBalance =
                  parseFloat(entry.balance) + parseFloat(entry.locked);
                const change = getBalanceChange(entry.currency, totalBalance);
                const evalKrw =
                  parseFloat(entry.balance) * parseFloat(entry.avg_buy_price);

                return (
                  <BalanceRow
                    key={entry.currency}
                    entry={entry}
                    change={change}
                    evalKrw={evalKrw}
                  />
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

interface BalanceRowProps {
  entry: BalanceEntry;
  change: number | null;
  evalKrw: number;
}

function BalanceRow({ entry, change, evalKrw }: BalanceRowProps) {
  const balance = parseFloat(entry.balance);
  const locked = parseFloat(entry.locked);
  const avgBuyPrice = parseFloat(entry.avg_buy_price);
  const isKrw = entry.currency === 'KRW';

  // 하이라이트 클래스 (Task 3에서 CSS 애니메이션 추가)
  const highlightClass = change !== null
    ? change > 0
      ? 'animate-highlight-green'
      : 'animate-highlight-red'
    : '';
  const changeSign = change !== null && change > 0 ? '+' : '-';
  const changeValue =
    change !== null
      ? isKrw
        ? `${formatNumber(Math.abs(change))} KRW`
        : `${formatCrypto(Math.abs(change))} ${entry.currency}`
      : null;

  return (
    <tr className={`border-b border-wts/30 ${highlightClass}`}>
      <td className="py-1.5 font-medium">{entry.currency}</td>
      <td className="py-1.5 text-right font-mono">
        {isKrw ? formatNumber(balance) : formatCrypto(balance)}
        {changeValue && (
          <span
            className={`ml-1 text-[10px] ${change > 0 ? 'text-wts-success' : 'text-wts-destructive'}`}
          >
            {changeSign}
            {changeValue}
          </span>
        )}
      </td>
      <td className="py-1.5 text-right font-mono text-wts-muted">
        {locked > 0 ? formatCrypto(locked) : '-'}
      </td>
      <td className="py-1.5 text-right font-mono">
        {isKrw ? '-' : formatKrw(avgBuyPrice)}
      </td>
      <td className="py-1.5 text-right font-mono">
        {isKrw ? '-' : formatKrw(evalKrw)}
      </td>
    </tr>
  );
}

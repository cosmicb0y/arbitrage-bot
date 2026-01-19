import { useEffect, useCallback } from 'react';
import { useWtsStore } from '../stores';
import { useConsoleStore } from '../stores/consoleStore';
import { Badge } from '../../components/ui/badge';
import {
  Exchange,
  EXCHANGE_META,
  EXCHANGE_ORDER,
} from '../types';

interface ExchangePanelProps {
  className?: string;
}

export function ExchangePanel({ className = '' }: ExchangePanelProps) {
  const {
    selectedExchange,
    setExchange,
    connectionStatus,
    enabledExchanges,
    lastConnectionError,
  } = useWtsStore();
  const { addLog } = useConsoleStore();

  const isEnabled = useCallback(
    (exchange: Exchange) => enabledExchanges.includes(exchange),
    [enabledExchanges]
  );

  const handleExchangeSelect = useCallback(
    (exchange: Exchange) => {
      if (!isEnabled(exchange)) return;
      setExchange(exchange);
      addLog(
        'INFO',
        'SYSTEM',
        `[INFO] 거래소 전환: ${EXCHANGE_META[exchange].name}`
      );
    },
    [setExchange, addLog, isEnabled]
  );

  // 키보드 단축키 (1-6)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 입력 필드에서는 단축키 비활성화
      const target = e.target as HTMLElement;
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        return;
      }

      const key = parseInt(e.key);
      if (key >= 1 && key <= 6) {
        const exchange = EXCHANGE_ORDER[key - 1];
        if (isEnabled(exchange)) {
          handleExchangeSelect(exchange);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleExchangeSelect, isEnabled]);

  return (
    <div
      data-testid="exchange-panel"
      className={`wts-area-header flex items-center justify-between px-4 bg-wts-secondary border-b border-wts ${className}`}
    >
      <h1 className="text-wts-foreground font-semibold text-base">WTS</h1>

      {/* 거래소 탭 */}
      <div className="flex items-center gap-1">
        {EXCHANGE_ORDER.map((exchange) => {
          const meta = EXCHANGE_META[exchange];
          const isActive = selectedExchange === exchange;
          const enabled = isEnabled(exchange);

          return (
            <button
              key={exchange}
              onClick={() => handleExchangeSelect(exchange)}
              disabled={!enabled}
              className={`
                px-3 py-1 text-sm font-medium transition-colors flex items-center
                ${
                  isActive
                    ? 'text-wts-foreground border-b-2 border-wts-accent'
                    : 'text-wts-muted hover:text-wts-foreground hover:bg-wts-tertiary'
                }
                ${!enabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
              `}
              title={enabled ? meta.name : `${meta.name} (Coming Soon)`}
            >
              <span>{meta.shortKey}</span>
              {!enabled && (
                <span className="ml-1.5 text-[10px] whitespace-nowrap opacity-70">
                  Coming Soon
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* 연결 상태 */}
      <div data-testid="connection-status" className="flex items-center">
        <Badge
          variant={
            connectionStatus === 'connected'
              ? 'success'
              : connectionStatus === 'connecting'
              ? 'warning'
              : 'destructive'
          }
          className={connectionStatus === 'connecting' ? 'animate-pulse' : ''}
          title={lastConnectionError || undefined}
        >
          {connectionStatus === 'connected'
            ? '연결됨'
            : connectionStatus === 'connecting'
            ? '연결중...'
            : '연결 안됨'}
        </Badge>
      </div>
    </div>
  );
}

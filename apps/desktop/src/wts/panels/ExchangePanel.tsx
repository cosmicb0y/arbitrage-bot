import { useWtsStore } from '../stores';

interface ExchangePanelProps {
  className?: string;
}

export function ExchangePanel({ className = '' }: ExchangePanelProps) {
  const { selectedExchange, connectionStatus } = useWtsStore();

  return (
    <div
      data-testid="exchange-panel"
      className={`wts-area-header flex items-center justify-between px-4 bg-wts-secondary border-b border-wts ${className}`}
    >
      <div className="flex items-center gap-4">
        <h1 className="text-wts-foreground font-semibold text-base">WTS</h1>
        <span className="text-wts-muted text-sm uppercase">{selectedExchange}</span>
      </div>
      <div
        data-testid="connection-status"
        className="flex items-center gap-2"
      >
        <span
          className={`w-2 h-2 rounded-full ${
            connectionStatus === 'connected' ? 'bg-green-500' : 'bg-red-500'
          }`}
        />
        <span className="text-wts-muted text-sm">
          {connectionStatus === 'connected' ? '연결됨' : '연결 안됨'}
        </span>
      </div>
    </div>
  );
}

import { useWtsStore } from '../stores';
import { Badge } from '../../components/ui/badge';

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
        className="flex items-center"
      >
        <Badge variant={connectionStatus === 'connected' ? 'success' : 'destructive'}>
          {connectionStatus === 'connected' ? '연결됨' : '연결 안됨'}
        </Badge>
      </div>
    </div>
  );
}

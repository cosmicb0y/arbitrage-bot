import { MarketSelector } from '../components/MarketSelector';
import { useWtsStore } from '../stores/wtsStore';

interface OrderbookPanelProps {
  className?: string;
}

export function OrderbookPanel({ className = '' }: OrderbookPanelProps) {
  const { selectedMarket, setMarket, connectionStatus, availableMarkets } =
    useWtsStore();

  const isDisabled = connectionStatus !== 'connected';

  return (
    <div
      data-testid="orderbook-panel"
      className={`wts-area-orderbook wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header flex justify-between items-center">
        <span>Orderbook</span>
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
        ) : (
          <p className="text-wts-muted text-xs">
            오더북 데이터 (Story 2.5에서 구현)
          </p>
        )}
      </div>
    </div>
  );
}

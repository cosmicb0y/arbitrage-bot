import {
  ExchangePanel,
  ConsolePanel,
  OrderbookPanel,
  BalancePanel,
  OrderPanel,
  OpenOrdersPanel,
} from './panels';

/**
 * WTS 창 메인 레이아웃
 * Bloomberg 터미널 스타일의 6패널 그리드 레이아웃
 */
export function WtsWindow() {
  return (
    <div className="wts-grid bg-wts-background text-wts-foreground">
      <ExchangePanel />
      <ConsolePanel />
      <OrderbookPanel />
      <BalancePanel />
      <OrderPanel />
      <OpenOrdersPanel />
    </div>
  );
}

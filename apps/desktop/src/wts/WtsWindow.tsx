import {
  ExchangePanel,
  ConsolePanel,
  OrderbookPanel,
  BalancePanel,
  OrderPanel,
  OpenOrdersPanel,
} from './panels';
import { ToastContainer } from './components/ToastContainer';
import { useConnectionCheck } from './hooks';
import { useUpbitMarkets } from './hooks/useUpbitMarkets';

/**
 * WTS 창 메인 레이아웃
 * Bloomberg 터미널 스타일의 6패널 그리드 레이아웃
 */
export function WtsWindow() {
  // 거래소 연결 상태 체크 (마운트 시 자동 실행, 거래소 변경 시 재실행)
  useConnectionCheck();

  // 연결 성공 시 Upbit 마켓 목록 동적 로드
  useUpbitMarkets();

  return (
    <>
      <div className="wts-grid bg-wts-background text-wts-foreground">
        <ExchangePanel />
        <ConsolePanel />
        <OrderbookPanel />
        <BalancePanel />
        <OrderPanel />
        <OpenOrdersPanel />
      </div>
      <ToastContainer />
    </>
  );
}

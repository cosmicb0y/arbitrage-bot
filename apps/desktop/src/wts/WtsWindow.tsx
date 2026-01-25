import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  ExchangePanel,
  ConsolePanel,
  OrderbookPanel,
  BalancePanel,
  OrderPanel,
  OpenOrdersPanel,
  TransferPanel,
} from './panels';
import { ToastContainer } from './components/ToastContainer';
import { WithdrawConfirmDialog } from './components/WithdrawConfirmDialog';
import { WithdrawResultDialog } from './components/WithdrawResultDialog';
import { useConnectionCheck } from './hooks';
import { useUpbitMarkets } from './hooks/useUpbitMarkets';
import { useTransferStore } from './stores/transferStore';
import { useConsoleStore } from './stores/consoleStore';
import { useToastStore } from './stores/toastStore';
import { useBalanceStore } from './stores/balanceStore';
import { handleApiError, handleWithdrawError } from './utils/errorHandler';
import type {
  WithdrawParams,
  WithdrawResponse,
  WithdrawConfirmInfo,
  WithdrawResultInfo,
  WithdrawState,
  WtsApiResult,
} from './types';
import { WITHDRAW_STATE_MESSAGES } from './types';

/**
 * WTS 창 메인 레이아웃
 * Bloomberg 터미널 스타일의 6패널 그리드 레이아웃
 */
export function WtsWindow() {
  // 거래소 연결 상태 체크 (마운트 시 자동 실행, 거래소 변경 시 재실행)
  useConnectionCheck();

  // 연결 성공 시 Upbit 마켓 목록 동적 로드
  useUpbitMarkets();

  // 출금 확인 다이얼로그 상태
  const [isWithdrawDialogOpen, setIsWithdrawDialogOpen] = useState(false);
  const [withdrawConfirmInfo, setWithdrawConfirmInfo] = useState<WithdrawConfirmInfo | null>(null);
  const [isWithdrawLoading, setIsWithdrawLoading] = useState(false);

  // 출금 결과 다이얼로그 상태 (WTS-5.4)
  const [isWithdrawResultOpen, setIsWithdrawResultOpen] = useState(false);
  const [withdrawResult, setWithdrawResult] = useState<WithdrawResultInfo | null>(null);
  const [isCheckingWithdrawStatus, setIsCheckingWithdrawStatus] = useState(false);

  // Stores
  const withdrawChanceInfo = useTransferStore((state) => state.withdrawChanceInfo);
  const addLog = useConsoleStore((state) => state.addLog);
  const showToast = useToastStore((state) => state.showToast);
  const fetchBalance = useBalanceStore((state) => state.fetchBalance);

  // TransferPanel에서 출금 버튼 클릭 시
  const handleWithdrawClick = useCallback(
    (params: WithdrawParams) => {
      if (!withdrawChanceInfo) return;

      const fee = withdrawChanceInfo.currency_info.withdraw_fee;
      const amount = parseFloat(params.amount);
      const feeNum = parseFloat(fee);
      const receivable = (amount - feeNum).toFixed(
        withdrawChanceInfo.withdraw_limit.fixed
      );

      setWithdrawConfirmInfo({
        currency: params.currency,
        net_type: params.net_type,
        address: params.address,
        secondary_address: params.secondary_address ?? null,
        amount: params.amount,
        fee,
        receivable,
      });
      setIsWithdrawDialogOpen(true);
    },
    [withdrawChanceInfo]
  );

  // 출금 확인
  const handleWithdrawConfirm = useCallback(async () => {
    if (!withdrawConfirmInfo) return;

    setIsWithdrawLoading(true);
    try {
      const result = await invoke<WtsApiResult<WithdrawResponse>>('wts_withdraw', {
        params: {
          currency: withdrawConfirmInfo.currency,
          net_type: withdrawConfirmInfo.net_type,
          amount: withdrawConfirmInfo.amount,
          address: withdrawConfirmInfo.address,
          secondary_address: withdrawConfirmInfo.secondary_address,
        },
      });

      if (result.success && result.data) {
        // 상태 메시지와 함께 로그 기록 (AC #2)
        const stateMessage = WITHDRAW_STATE_MESSAGES[result.data.state as WithdrawState] || result.data.state;
        addLog(
          'SUCCESS',
          'WITHDRAW',
          `출금 요청 완료: ${result.data.uuid} (${stateMessage})`
        );
        showToast('success', '출금 요청이 완료되었습니다');
        // 잔고 갱신 (AC #4)
        fetchBalance();

        // 확인 다이얼로그 닫고 결과 다이얼로그 표시 (AC #1, #5)
        setIsWithdrawDialogOpen(false);
        setWithdrawResult({
          uuid: result.data.uuid,
          currency: withdrawConfirmInfo.currency,
          net_type: withdrawConfirmInfo.net_type,
          state: result.data.state as WithdrawState,
          amount: result.data.amount,
          fee: result.data.fee,
          txid: result.data.txid,
          created_at: result.data.created_at,
        });
        setIsWithdrawResultOpen(true);
        setWithdrawConfirmInfo(null);
      } else {
        // WTS-5.5: handleWithdrawError - 액션 필요 에러는 WARN, 다이얼로그 유지
        handleWithdrawError(result.error, '출금 실패');
      }
    } catch (error) {
      addLog('ERROR', 'WITHDRAW', `출금 요청 실패: ${error}`);
      showToast('error', '출금 요청에 실패했습니다');
    } finally {
      setIsWithdrawLoading(false);
    }
  }, [withdrawConfirmInfo, addLog, showToast, fetchBalance]);

  // 출금 취소
  const handleWithdrawCancel = useCallback(() => {
    setIsWithdrawDialogOpen(false);
    setWithdrawConfirmInfo(null);
  }, []);

  // 출금 결과 다이얼로그 닫기
  const handleWithdrawResultClose = useCallback(() => {
    setIsWithdrawResultOpen(false);
    setWithdrawResult(null);
  }, []);

  // 출금 상태 조회 (AC #7, #8)
  const handleCheckWithdrawStatus = useCallback(async () => {
    if (!withdrawResult) return;

    setIsCheckingWithdrawStatus(true);
    try {
      const result = await invoke<WtsApiResult<WithdrawResponse>>('wts_get_withdraw', {
        params: { uuid: withdrawResult.uuid }
      });

      if (result.success && result.data) {
        const prevTxid = withdrawResult.txid;
        const newTxid = result.data.txid;

        // TXID가 새로 생성된 경우 로그 기록 (AC #8)
        if (!prevTxid && newTxid) {
          addLog('INFO', 'WITHDRAW', `TXID 생성됨: ${newTxid}`);
        }

        setWithdrawResult(prev => prev ? {
          ...prev,
          state: result.data!.state as WithdrawState,
          txid: result.data!.txid,
        } : null);

        const stateMessage = WITHDRAW_STATE_MESSAGES[result.data.state as WithdrawState] || result.data.state;
        addLog('INFO', 'WITHDRAW', `출금 상태: ${stateMessage}`);
      } else {
        handleApiError(result.error, 'WITHDRAW', '상태 조회 실패');
      }
    } catch (error) {
      addLog('ERROR', 'WITHDRAW', `상태 조회 실패: ${error}`);
    } finally {
      setIsCheckingWithdrawStatus(false);
    }
  }, [withdrawResult, addLog]);

  return (
    <>
      <div className="wts-grid bg-wts-background text-wts-foreground">
        <ExchangePanel />
        <ConsolePanel />
        <OrderbookPanel />
        <BalancePanel />
        <OrderPanel />
        <TransferPanel onWithdrawClick={handleWithdrawClick} />
        <OpenOrdersPanel />
      </div>
      <ToastContainer />
      {withdrawConfirmInfo && (
        <WithdrawConfirmDialog
          isOpen={isWithdrawDialogOpen}
          withdrawInfo={withdrawConfirmInfo}
          onConfirm={handleWithdrawConfirm}
          onCancel={handleWithdrawCancel}
          isLoading={isWithdrawLoading}
        />
      )}
      {withdrawResult && (
        <WithdrawResultDialog
          isOpen={isWithdrawResultOpen}
          result={withdrawResult}
          onClose={handleWithdrawResultClose}
          onCheckStatus={handleCheckWithdrawStatus}
          isCheckingStatus={isCheckingWithdrawStatus}
        />
      )}
    </>
  );
}

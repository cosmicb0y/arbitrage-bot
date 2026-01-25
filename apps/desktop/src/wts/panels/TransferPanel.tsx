import { useCallback, useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  useTransferStore,
  MAX_GENERATE_RETRIES,
  GENERATE_RETRY_INTERVAL,
} from '../stores/transferStore';
import { useConsoleStore } from '../stores/consoleStore';
import { handleApiError } from '../utils/errorHandler';
import type {
  DepositChanceResponse,
  WtsApiResult,
  DepositChanceParams,
  DepositAddressParams,
  DepositAddressResponse,
  GenerateAddressResponse,
  WithdrawChanceResponse,
  WithdrawChanceParams,
  WithdrawAddressResponse,
  WithdrawParams,
} from '../types';

interface TransferPanelProps {
  className?: string;
  /** 출금 버튼 클릭 시 콜백 (WTS-5.3 확인 다이얼로그 연결용) */
  onWithdrawClick?: (params: WithdrawParams) => void;
}

// 아이콘 컴포넌트
const CopyIcon = ({ className = 'w-4 h-4' }: { className?: string }) => (
  <svg
    className={className}
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
    />
  </svg>
);

const CheckIcon = ({ className = 'w-4 h-4' }: { className?: string }) => (
  <svg
    className={className}
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M5 13l4 4L19 7"
    />
  </svg>
);

const WarningIcon = ({ className = 'w-4 h-4' }: { className?: string }) => (
  <svg
    className={className}
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
    />
  </svg>
);

/** 입금 가능 자산 목록 (MVP) */
export const DEPOSIT_CURRENCIES = [
  { code: 'BTC', name: '비트코인', networks: ['BTC'] },
  { code: 'ETH', name: '이더리움', networks: ['ETH'] },
  { code: 'XRP', name: '리플', networks: ['XRP'] },
  { code: 'SOL', name: '솔라나', networks: ['SOL'] },
  { code: 'DOGE', name: '도지코인', networks: ['DOGE'] },
  { code: 'ADA', name: '에이다', networks: ['ADA'] },
  { code: 'USDT', name: '테더', networks: ['TRX', 'ETH'] },
  { code: 'USDC', name: 'USD 코인', networks: ['ETH', 'SOL', 'ARB'] },
] as const;

/** 출금 가능 자산 목록 (MVP) - 입금과 동일 */
export const WITHDRAW_CURRENCIES = DEPOSIT_CURRENCIES;

/**
 * Transfer 패널 컴포넌트
 * 입금/출금 기능을 제공하는 패널
 */
export function TransferPanel({ className = '', onWithdrawClick }: TransferPanelProps) {
  const {
    activeTab,
    selectedCurrency,
    selectedNetwork,
    networkInfo,
    isLoading,
    error,
    depositAddress,
    isAddressLoading,
    addressError,
    isGenerating,
    generateRetryCount,
    setActiveTab,
    setSelectedCurrency,
    setSelectedNetwork,
    setNetworkInfo,
    setLoading,
    setError,
    setDepositAddress,
    setAddressLoading,
    setAddressError,
    setGenerating,
    setGenerateRetryCount,
    resetGenerateState,
    // 출금 상태 (WTS-5.2)
    withdrawChanceInfo,
    withdrawAddresses,
    selectedWithdrawAddress,
    withdrawAmount,
    isWithdrawLoading,
    withdrawError,
    setWithdrawChanceInfo,
    setWithdrawAddresses,
    setSelectedWithdrawAddress,
    setWithdrawAmount,
    setWithdrawLoading,
    setWithdrawError,
    resetWithdrawState,
  } = useTransferStore();

  const addLog = useConsoleStore((state) => state.addLog);
  const [copiedField, setCopiedField] = useState<string | null>(null);

  // 폴링 타이머 ref
  const pollTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 컴포넌트 언마운트 시 타이머 정리
  useEffect(() => {
    return () => {
      if (pollTimerRef.current) {
        clearTimeout(pollTimerRef.current);
      }
    };
  }, []);

  // 폴링 중단 함수
  const cancelPolling = useCallback(() => {
    if (pollTimerRef.current) {
      clearTimeout(pollTimerRef.current);
      pollTimerRef.current = null;
    }
    resetGenerateState();
    addLog('WARN', 'DEPOSIT', '주소 생성 취소됨');
  }, [resetGenerateState, addLog]);

  // 입금 주소 조회
  const fetchDepositAddress = useCallback(
    async (currency: string, netType: string) => {
      setAddressLoading(true);
      setAddressError(null);

      try {
        const params: DepositAddressParams = { currency, net_type: netType };
        console.log('[DEBUG] fetchDepositAddress: calling wts_get_deposit_address with params:', params);

        const result = await invoke<WtsApiResult<DepositAddressResponse>>(
          'wts_get_deposit_address',
          { params }
        );

        console.log('[DEBUG] fetchDepositAddress: raw result:', JSON.stringify(result, null, 2));

        if (result.success && result.data) {
          console.log('[DEBUG] fetchDepositAddress: success, data:', result.data);
          setDepositAddress(result.data);
          addLog(
            'INFO',
            'DEPOSIT',
            `입금 주소 조회 완료: ${result.data.deposit_address ? '성공' : '미생성'}`
          );
        } else if (result.error?.code === 'coin_address_not_found') {
          console.log('[DEBUG] fetchDepositAddress: address not found, setting null');
          // 주소가 없는 경우 null 설정 (생성 버튼 표시용)
          setDepositAddress({
            currency,
            net_type: netType,
            deposit_address: null,
            secondary_address: null,
          });
        } else {
          console.error('[DEBUG] fetchDepositAddress: failed, error:', result.error);
          handleApiError(result.error, 'DEPOSIT', '입금 주소 조회 실패');
          setAddressError(result.error?.message || '입금 주소 조회 실패');
        }
      } catch (err) {
        console.error('[DEBUG] fetchDepositAddress: exception caught:', err);
        handleApiError(err, 'DEPOSIT', '입금 주소 조회 실패');
        setAddressError('입금 주소 조회 실패');
      } finally {
        setAddressLoading(false);
      }
    },
    [setAddressLoading, setAddressError, setDepositAddress, addLog]
  );

  // 입금 가능 정보 조회
  const fetchDepositChance = useCallback(
    async (currency: string, netType: string) => {
      setLoading(true);
      setError(null);

      try {
        const params: DepositChanceParams = { currency, net_type: netType };
        console.log('[DEBUG] fetchDepositChance: calling wts_get_deposit_chance with params:', params);

        const result = await invoke<WtsApiResult<DepositChanceResponse>>(
          'wts_get_deposit_chance',
          { params }
        );

        console.log('[DEBUG] fetchDepositChance: raw result:', JSON.stringify(result, null, 2));

        if (result.success && result.data) {
          console.log('[DEBUG] fetchDepositChance: success, data:', result.data);
          setNetworkInfo(result.data);
          setSelectedNetwork(netType);
          addLog(
            'INFO',
            'DEPOSIT',
            `입금 정보 조회 완료: ${currency}/${netType}`
          );
          // 입금 정보 조회 성공 시 주소도 조회
          await fetchDepositAddress(currency, netType);
        } else {
          console.error('[DEBUG] fetchDepositChance: failed, error:', result.error);
          handleApiError(result.error, 'DEPOSIT', '입금 정보 조회 실패');
          setError(result.error?.message || '입금 정보 조회 실패');
        }
      } catch (err) {
        console.error('[DEBUG] fetchDepositChance: exception caught:', err);
        handleApiError(err, 'DEPOSIT', '입금 정보 조회 실패');
        setError('입금 정보 조회 실패');
      } finally {
        setLoading(false);
      }
    },
    [
      setLoading,
      setError,
      setNetworkInfo,
      setSelectedNetwork,
      addLog,
      fetchDepositAddress,
    ]
  );

  // 자산 선택 핸들러
  const handleCurrencyChange = useCallback(
    async (currency: string) => {
      if (!currency) {
        setSelectedCurrency(null);
        return;
      }

      setSelectedCurrency(currency);
      addLog('INFO', 'DEPOSIT', `자산 선택: ${currency}`);

      // 자산에 해당하는 기본 네트워크로 입금 정보 조회
      const currencyInfo = DEPOSIT_CURRENCIES.find((c) => c.code === currency);
      if (!currencyInfo) {
        return;
      }

      const defaultNetwork = currencyInfo.networks[0];
      await fetchDepositChance(currency, defaultNetwork);
    },
    [setSelectedCurrency, addLog, fetchDepositChance]
  );

  // 주소 복사 핸들러
  const handleCopyAddress = useCallback(
    async (text: string | null, field: string) => {
      if (!text) return;

      try {
        await navigator.clipboard.writeText(text);
        setCopiedField(field);
        addLog(
          'SUCCESS',
          'DEPOSIT',
          `${field === 'address' ? '입금 주소' : 'Tag'}가 클립보드에 복사되었습니다`
        );

        setTimeout(() => setCopiedField(null), 2000);
      } catch (err) {
        addLog('ERROR', 'DEPOSIT', '클립보드 복사 실패');
      }
    },
    [addLog]
  );

  // 주소 폴링 함수
  const pollForAddress = useCallback(
    async (currency: string, netType: string, attempt: number) => {
      if (attempt > MAX_GENERATE_RETRIES) {
        setAddressError(
          `주소 생성 실패: 최대 재시도 횟수(${MAX_GENERATE_RETRIES}회) 초과`
        );
        resetGenerateState();
        addLog(
          'ERROR',
          'DEPOSIT',
          `주소 생성 실패: ${MAX_GENERATE_RETRIES}회 시도 후 타임아웃`
        );
        return;
      }

      setGenerateRetryCount(attempt);
      addLog(
        'INFO',
        'DEPOSIT',
        `입금 주소 확인 중 (${attempt}/${MAX_GENERATE_RETRIES})`
      );

      try {
        const params: DepositAddressParams = { currency, net_type: netType };
        const result = await invoke<WtsApiResult<DepositAddressResponse>>(
          'wts_get_deposit_address',
          { params }
        );

        if (result.success && result.data?.deposit_address) {
          // 성공! 주소 획득
          setDepositAddress(result.data);
          resetGenerateState();
          addLog(
            'SUCCESS',
            'DEPOSIT',
            `입금 주소 생성 완료: ${result.data.deposit_address.slice(0, 10)}...`
          );
          return;
        }

        // 주소가 아직 없음 - 다음 폴링 예약
        pollTimerRef.current = setTimeout(() => {
          pollForAddress(currency, netType, attempt + 1);
        }, GENERATE_RETRY_INTERVAL);
      } catch (err) {
        // 네트워크 오류 시에도 재시도
        addLog(
          'WARN',
          'DEPOSIT',
          `주소 확인 실패, 재시도 중 (${attempt}/${MAX_GENERATE_RETRIES})`
        );
        pollTimerRef.current = setTimeout(() => {
          pollForAddress(currency, netType, attempt + 1);
        }, GENERATE_RETRY_INTERVAL);
      }
    },
    [
      setGenerateRetryCount,
      setDepositAddress,
      resetGenerateState,
      setAddressError,
      addLog,
    ]
  );

  // 주소 생성 핸들러 (비동기 폴링 지원)
  const handleGenerateAddress = useCallback(async () => {
    if (!selectedCurrency || !selectedNetwork) return;

    // 기존 폴링 취소
    if (pollTimerRef.current) {
      clearTimeout(pollTimerRef.current);
    }

    setGenerating(true);
    setAddressError(null);
    setGenerateRetryCount(0);

    try {
      const result = await invoke<WtsApiResult<GenerateAddressResponse>>(
        'wts_generate_deposit_address',
        {
          params: {
            currency: selectedCurrency,
            net_type: selectedNetwork,
          },
        }
      );

      if (result.success) {
        addLog('INFO', 'DEPOSIT', '입금 주소 생성 요청 완료, 폴링 시작');
        // 첫 폴링 시작 (3초 후)
        pollTimerRef.current = setTimeout(() => {
          pollForAddress(selectedCurrency, selectedNetwork, 1);
        }, GENERATE_RETRY_INTERVAL);
      } else {
        handleApiError(result.error, 'DEPOSIT', '주소 생성 요청 실패');
        setAddressError(result.error?.message || '주소 생성 요청 실패');
        resetGenerateState();
      }
    } catch (err) {
      handleApiError(err, 'DEPOSIT', '주소 생성 요청 실패');
      setAddressError('주소 생성 요청 실패');
      resetGenerateState();
    }
  }, [
    selectedCurrency,
    selectedNetwork,
    setGenerating,
    setAddressError,
    setGenerateRetryCount,
    addLog,
    pollForAddress,
    resetGenerateState,
  ]);

  // ============================================================================
  // 출금 관련 함수 (WTS-5.2)
  // ============================================================================

  // 출금 가능 정보 조회
  const fetchWithdrawChance = useCallback(
    async (currency: string, netType: string) => {
      setWithdrawLoading(true);
      setWithdrawError(null);

      try {
        const params: WithdrawChanceParams = { currency, net_type: netType };
        const result = await invoke<WtsApiResult<WithdrawChanceResponse>>(
          'wts_get_withdraw_chance',
          { params }
        );

        if (result.success && result.data) {
          setWithdrawChanceInfo(result.data);
          addLog(
            'INFO',
            'WITHDRAW',
            `출금 정보 조회 완료: ${currency}/${netType}`
          );
          // 출금 가능 정보 조회 성공 시 등록된 출금 주소도 조회
          await fetchWithdrawAddresses(currency, netType);
        } else {
          handleApiError(result.error, 'WITHDRAW', '출금 정보 조회 실패');
          setWithdrawError(result.error?.message || '출금 정보 조회 실패');
        }
      } catch (err) {
        handleApiError(err, 'WITHDRAW', '출금 정보 조회 실패');
        setWithdrawError('출금 정보 조회 실패');
      } finally {
        setWithdrawLoading(false);
      }
    },
    [setWithdrawLoading, setWithdrawError, setWithdrawChanceInfo, addLog]
  );

  // 등록된 출금 주소 조회
  const fetchWithdrawAddresses = useCallback(
    async (currency: string, netType: string) => {
      try {
        const result = await invoke<WtsApiResult<WithdrawAddressResponse[]>>(
          'wts_get_withdraw_addresses',
          { params: { currency, net_type: netType } }
        );

        if (result.success && result.data) {
          setWithdrawAddresses(result.data);
          addLog(
            'INFO',
            'WITHDRAW',
            `등록된 출금 주소 ${result.data.length}개 조회 완료`
          );
        } else {
          // 주소가 없는 경우 빈 배열로 설정
          setWithdrawAddresses([]);
          if (result.error?.code !== 'withdraw_address_not_found') {
            addLog('WARN', 'WITHDRAW', '출금 주소 조회 실패');
          }
        }
      } catch (err) {
        setWithdrawAddresses([]);
        addLog('WARN', 'WITHDRAW', '출금 주소 조회 실패');
      }
    },
    [setWithdrawAddresses, addLog]
  );

  // 출금 자산 선택 핸들러
  const handleWithdrawCurrencyChange = useCallback(
    async (currency: string) => {
      if (!currency) {
        setSelectedCurrency(null);
        return;
      }

      setSelectedCurrency(currency);
      addLog('INFO', 'WITHDRAW', `출금 자산 선택: ${currency}`);

      // 자산에 해당하는 기본 네트워크로 출금 정보 조회
      const currencyInfo = WITHDRAW_CURRENCIES.find((c) => c.code === currency);
      if (!currencyInfo) {
        return;
      }

      const defaultNetwork = currencyInfo.networks[0];
      setSelectedNetwork(defaultNetwork);
      await fetchWithdrawChance(currency, defaultNetwork);
    },
    [setSelectedCurrency, setSelectedNetwork, addLog, fetchWithdrawChance]
  );

  // 출금 네트워크 선택 핸들러
  const handleWithdrawNetworkChange = useCallback(
    async (netType: string) => {
      if (!selectedCurrency) return;
      setSelectedNetwork(netType);
      await fetchWithdrawChance(selectedCurrency, netType);
    },
    [selectedCurrency, setSelectedNetwork, fetchWithdrawChance]
  );

  // 출금 주소 선택 핸들러
  const handleSelectWithdrawAddress = useCallback(
    (address: WithdrawAddressResponse | null) => {
      setSelectedWithdrawAddress(address);
      if (address) {
        addLog('INFO', 'WITHDRAW', `출금 주소 선택: ${address.withdraw_address.slice(0, 10)}...`);
      }
    },
    [setSelectedWithdrawAddress, addLog]
  );

  // % 버튼 핸들러
  const handlePercentClick = useCallback(
    (percent: number) => {
      if (!withdrawChanceInfo) return;

      const balance = parseFloat(withdrawChanceInfo.account_info.balance);
      const locked = parseFloat(withdrawChanceInfo.account_info.locked);
      const available = balance - locked;

      if (available <= 0) {
        addLog('WARN', 'WITHDRAW', '출금 가능 잔고가 없습니다');
        return;
      }

      const amount = percent === 100 ? available : available * (percent / 100);

      // 소수점 정밀도 적용 및 가용 잔고 초과 방지 (부동소수점 오차 대응)
      const fixed = withdrawChanceInfo.withdraw_limit.fixed;
      const safeAmount = Math.min(amount, available);
      const formattedAmount = safeAmount.toFixed(fixed);

      setWithdrawAmount(formattedAmount);
    },
    [withdrawChanceInfo, setWithdrawAmount, addLog]
  );

  // 출금 버튼 클릭 핸들러
  const handleWithdrawButtonClick = useCallback(() => {
    if (!selectedCurrency || !selectedNetwork || !selectedWithdrawAddress || !withdrawAmount) {
      return;
    }

    const params: WithdrawParams = {
      currency: selectedCurrency,
      net_type: selectedNetwork,
      amount: withdrawAmount,
      address: selectedWithdrawAddress.withdraw_address,
      secondary_address: selectedWithdrawAddress.secondary_address,
    };

    if (onWithdrawClick) {
      onWithdrawClick(params);
    }
  }, [selectedCurrency, selectedNetwork, selectedWithdrawAddress, withdrawAmount, onWithdrawClick]);

  // 출금 버튼 활성화 조건
  const isWithdrawButtonEnabled = useCallback(() => {
    if (!withdrawChanceInfo) return false;
    if (!selectedWithdrawAddress) return false;
    if (!withdrawAmount || parseFloat(withdrawAmount) <= 0) return false;
    if (!withdrawChanceInfo.withdraw_limit.can_withdraw) return false;

    const amount = parseFloat(withdrawAmount);
    const minimum = parseFloat(withdrawChanceInfo.withdraw_limit.minimum);
    const available =
      parseFloat(withdrawChanceInfo.account_info.balance) -
      parseFloat(withdrawChanceInfo.account_info.locked);

    if (amount < minimum) return false;
    if (amount > available) return false;

    return true;
  }, [withdrawChanceInfo, selectedWithdrawAddress, withdrawAmount]);

  // 실수령액 계산
  const receivableAmount = useCallback(() => {
    if (!withdrawChanceInfo || !withdrawAmount) return null;

    const amount = parseFloat(withdrawAmount);
    if (isNaN(amount) || amount <= 0) return null;

    const fee = parseFloat(withdrawChanceInfo.currency_info.withdraw_fee);
    const result = amount - fee;

    return result > 0
      ? result.toFixed(withdrawChanceInfo.withdraw_limit.fixed)
      : '0';
  }, [withdrawChanceInfo, withdrawAmount]);

  // 출금 주소 축약 표시 (앞 8자 ... 뒤 6자)
  const formatAddress = (address: string) => {
    if (address.length <= 16) return address;
    return `${address.slice(0, 8)}...${address.slice(-6)}`;
  };

  return (
    <div
      data-testid="transfer-panel"
      className={`wts-area-transfer wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Transfer</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        {/* 탭 UI */}
        <div className="flex border-b border-wts" role="tablist">
          <button
            role="tab"
            aria-selected={activeTab === 'deposit'}
            onClick={() => setActiveTab('deposit')}
            className={`flex-1 py-2 text-sm font-medium transition-colors
              ${
                activeTab === 'deposit'
                  ? 'text-wts-foreground border-b-2 border-wts-accent'
                  : 'text-wts-muted hover:text-wts-foreground'
              }
            `}
          >
            입금
          </button>
          <button
            role="tab"
            aria-selected={activeTab === 'withdraw'}
            onClick={() => setActiveTab('withdraw')}
            className={`flex-1 py-2 text-sm font-medium transition-colors
              ${
                activeTab === 'withdraw'
                  ? 'text-wts-foreground border-b-2 border-wts-accent'
                  : 'text-wts-muted hover:text-wts-foreground'
              }
            `}
          >
            출금
          </button>
        </div>

        {/* 탭 콘텐츠 */}
        <div className="p-3 space-y-3">
          {activeTab === 'deposit' && (
            <>
              {/* 자산 선택 드롭다운 */}
              <label className="block text-xs">
                <span className="text-wts-muted mb-1 block">자산</span>
                <select
                  value={selectedCurrency || ''}
                  onChange={(e) => handleCurrencyChange(e.target.value)}
                  aria-label="자산"
                  className="w-full px-3 py-2 rounded border border-wts bg-wts-secondary
                             text-wts-foreground text-sm
                             focus:outline-none focus:border-wts-focus"
                >
                  <option value="">자산 선택</option>
                  {DEPOSIT_CURRENCIES.map((c) => (
                    <option key={c.code} value={c.code}>
                      {c.code} - {c.name}
                    </option>
                  ))}
                </select>
              </label>

              {/* 네트워크 선택 */}
              {selectedCurrency && (
                <div className="mt-3">
                  <span className="text-wts-muted text-xs mb-1 block">
                    네트워크 선택
                  </span>
                  <div className="flex flex-wrap gap-2">
                    {DEPOSIT_CURRENCIES.find(
                      (c) => c.code === selectedCurrency
                    )?.networks.map((net) => (
                      <button
                        key={net}
                        onClick={() =>
                          fetchDepositChance(selectedCurrency, net)
                        }
                        className={`px-3 py-1 text-xs rounded border transition-colors
                            ${
                              networkInfo?.net_type === net
                                ? 'bg-wts-accent text-white border-wts-accent'
                                : 'bg-wts-secondary text-wts-muted border-wts hover:border-wts-foreground'
                            }
                          `}
                      >
                        {net}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* 로딩 상태 */}
              {isLoading && (
                <div className="text-center py-4 text-wts-muted text-sm">
                  조회 중...
                </div>
              )}

              {/* 에러 메시지 */}
              {error && (
                <div className="p-3 rounded bg-red-900/20 border border-red-500/30 text-red-400 text-xs">
                  {error}
                </div>
              )}

              {/* 네트워크 정보 표시 */}
              {networkInfo && !isLoading && (
                <div className="mt-3 p-3 rounded bg-wts-tertiary text-xs space-y-2">
                  <div className="flex justify-between">
                    <span className="text-wts-muted">네트워크</span>
                    <span className="text-wts-foreground">
                      {networkInfo.net_type}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-wts-muted">입금 상태</span>
                    <span
                      className={
                        networkInfo.is_deposit_possible
                          ? 'text-green-500'
                          : 'text-red-500'
                      }
                    >
                      {networkInfo.is_deposit_possible ? '정상' : '중단'}
                    </span>
                  </div>
                  {!networkInfo.is_deposit_possible &&
                    networkInfo.deposit_impossible_reason && (
                      <div className="flex justify-between">
                        <span className="text-wts-muted">중단 사유</span>
                        <span className="text-red-400">
                          {networkInfo.deposit_impossible_reason}
                        </span>
                      </div>
                    )}
                  <div className="flex justify-between">
                    <span className="text-wts-muted">확인 횟수</span>
                    <span className="text-wts-foreground">
                      {networkInfo.minimum_deposit_confirmations}회
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-wts-muted">최소 입금</span>
                    <span className="text-wts-foreground font-mono">
                      {networkInfo.minimum_deposit_amount} {networkInfo.currency}
                    </span>
                  </div>
                </div>
              )}

              {/* 입금 주소 표시 섹션 */}
              {networkInfo && !isLoading && !error && (
                <div className="mt-3 p-3 rounded bg-wts-tertiary text-xs space-y-2 border-t border-wts">
                  {isGenerating ? (
                    // 생성 중 상태
                    <div className="text-center py-4">
                      <div className="animate-pulse mb-2">
                        <div className="inline-block w-6 h-6 border-2 border-wts-accent border-t-transparent rounded-full animate-spin" />
                      </div>
                      <div className="text-wts-foreground mb-1">
                        {generateRetryCount === 0
                          ? '주소 생성 요청 중...'
                          : `주소 확인 중 (${generateRetryCount}/${MAX_GENERATE_RETRIES})`}
                      </div>
                      <div className="text-wts-muted text-[10px] mb-2">
                        Upbit에서 주소를 생성하고 있습니다
                      </div>
                      <button
                        onClick={cancelPolling}
                        className="text-xs text-wts-muted hover:text-red-400 underline"
                      >
                        취소
                      </button>
                    </div>
                  ) : isAddressLoading ? (
                    <div className="text-center py-2 text-wts-muted">
                      주소 로딩 중...
                    </div>
                  ) : addressError ? (
                    // 에러 상태 (재시도 버튼 포함)
                    <div className="text-center py-2">
                      <div className="text-red-400 mb-2">{addressError}</div>
                      <button
                        onClick={handleGenerateAddress}
                        className="px-3 py-1.5 bg-wts-accent text-white rounded hover:bg-opacity-90 transition-colors text-xs"
                      >
                        다시 시도
                      </button>
                    </div>
                  ) : depositAddress?.deposit_address ? (
                    <>
                      <div className="flex justify-between items-start">
                        <span className="text-wts-muted">입금 주소</span>
                        <button
                          onClick={() =>
                            handleCopyAddress(
                              depositAddress.deposit_address,
                              'address'
                            )
                          }
                          className="ml-2 p-1 rounded hover:bg-wts-secondary text-wts-accent"
                          title="주소 복사"
                        >
                          {copiedField === 'address' ? (
                            <CheckIcon />
                          ) : (
                            <CopyIcon />
                          )}
                        </button>
                      </div>
                      <div className="font-mono text-wts-foreground break-all text-[11px] bg-black/20 p-2 rounded">
                        {depositAddress.deposit_address}
                      </div>

                      {/* 보조 주소 (XRP tag, EOS memo 등) */}
                      {depositAddress.secondary_address && (
                        <>
                          <div className="mt-2 p-2 rounded bg-yellow-900/20 border border-yellow-500/30">
                            <div className="flex items-center gap-1 text-yellow-400 text-xs mb-1">
                              <WarningIcon className="w-3 h-3" />
                              <span>Memo/Tag 필수</span>
                            </div>
                            <div className="text-yellow-200 text-[10px]">
                              입금 시 반드시 아래 Tag를 포함해야 합니다
                            </div>
                          </div>
                          <div className="flex justify-between items-start mt-2">
                            <span className="text-wts-muted">Memo/Tag</span>
                            <button
                              onClick={() =>
                                handleCopyAddress(
                                  depositAddress.secondary_address,
                                  'tag'
                                )
                              }
                              className="ml-2 p-1 rounded hover:bg-wts-secondary text-wts-accent"
                              title="Tag 복사"
                            >
                              {copiedField === 'tag' ? (
                                <CheckIcon />
                              ) : (
                                <CopyIcon />
                              )}
                            </button>
                          </div>
                          <div className="font-mono text-wts-foreground bg-black/20 p-2 rounded">
                            {depositAddress.secondary_address}
                          </div>
                        </>
                      )}
                    </>
                  ) : (
                    <div className="text-center py-2">
                      <div className="text-wts-muted mb-2">
                        입금 주소가 없습니다
                      </div>
                      <button
                        onClick={handleGenerateAddress}
                        className="px-3 py-1.5 bg-wts-accent text-white rounded hover:bg-opacity-90 transition-colors text-xs"
                      >
                        주소 생성
                      </button>
                    </div>
                  )}
                </div>
              )}
            </>
          )}

          {activeTab === 'withdraw' && (
            <>
              {/* 자산 선택 드롭다운 */}
              <label className="block text-xs">
                <span className="text-wts-muted mb-1 block">자산</span>
                <select
                  value={selectedCurrency || ''}
                  onChange={(e) => handleWithdrawCurrencyChange(e.target.value)}
                  aria-label="출금 자산"
                  className="w-full px-3 py-2 rounded border border-wts bg-wts-secondary
                             text-wts-foreground text-sm
                             focus:outline-none focus:border-wts-focus"
                >
                  <option value="">자산 선택</option>
                  {WITHDRAW_CURRENCIES.map((c) => (
                    <option key={c.code} value={c.code}>
                      {c.code} - {c.name}
                    </option>
                  ))}
                </select>
              </label>

              {/* 네트워크 선택 */}
              {selectedCurrency && (
                <div className="mt-3">
                  <span className="text-wts-muted text-xs mb-1 block">
                    네트워크 선택
                  </span>
                  <div className="flex flex-wrap gap-2">
                    {WITHDRAW_CURRENCIES.find(
                      (c) => c.code === selectedCurrency
                    )?.networks.map((net) => (
                      <button
                        key={net}
                        onClick={() => handleWithdrawNetworkChange(net)}
                        disabled={isWithdrawLoading}
                        className={`px-3 py-1 text-xs rounded border transition-colors
                            ${
                              selectedNetwork === net
                                ? 'bg-wts-accent text-white border-wts-accent'
                                : 'bg-wts-secondary text-wts-muted border-wts hover:border-wts-foreground'
                            }
                            ${isWithdrawLoading ? 'opacity-50 cursor-not-allowed' : ''}
                          `}
                      >
                        {net}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* 로딩 상태 */}
              {isWithdrawLoading && (
                <div className="text-center py-4 text-wts-muted text-sm">
                  조회 중...
                </div>
              )}

              {/* 에러 메시지 */}
              {withdrawError && (
                <div className="p-3 rounded bg-red-900/20 border border-red-500/30 text-red-400 text-xs">
                  {withdrawError}
                </div>
              )}

              {/* 출금 가능 정보 표시 */}
              {withdrawChanceInfo && !isWithdrawLoading && (
                <>
                  {/* 지갑 상태 및 출금 가능 여부 */}
                  {!withdrawChanceInfo.withdraw_limit.can_withdraw && (
                    <div className="mt-3 p-3 rounded bg-red-900/20 border border-red-500/30 text-red-400 text-xs">
                      <div className="flex items-center gap-1 mb-1">
                        <WarningIcon className="w-3 h-3" />
                        <span>출금 불가</span>
                      </div>
                      <div className="text-red-300">
                        {withdrawChanceInfo.currency_info.wallet_state === 'paused'
                          ? '지갑 점검 중입니다'
                          : withdrawChanceInfo.currency_info.wallet_state === 'suspended'
                          ? '출금이 일시 중단되었습니다'
                          : '현재 출금이 불가능합니다'}
                      </div>
                    </div>
                  )}

                  {/* 출금 정보 카드 */}
                  <div className="mt-3 p-3 rounded bg-wts-tertiary text-xs space-y-2">
                    <div className="flex justify-between">
                      <span className="text-wts-muted">출금 가능 잔고</span>
                      <span className="text-wts-foreground font-mono">
                        {(
                          parseFloat(withdrawChanceInfo.account_info.balance) -
                          parseFloat(withdrawChanceInfo.account_info.locked)
                        ).toFixed(withdrawChanceInfo.withdraw_limit.fixed)}{' '}
                        {withdrawChanceInfo.currency}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-wts-muted">출금 수수료</span>
                      <span className="text-wts-foreground font-mono">
                        {withdrawChanceInfo.currency_info.withdraw_fee}{' '}
                        {withdrawChanceInfo.currency}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-wts-muted">최소 출금</span>
                      <span className="text-wts-foreground font-mono">
                        {withdrawChanceInfo.withdraw_limit.minimum}{' '}
                        {withdrawChanceInfo.currency}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-wts-muted">1회 최대</span>
                      <span className="text-wts-foreground font-mono">
                        {withdrawChanceInfo.withdraw_limit.onetime}{' '}
                        {withdrawChanceInfo.currency}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-wts-muted">일일 잔여 한도</span>
                      <span className="text-wts-foreground font-mono">
                        {withdrawChanceInfo.withdraw_limit.remaining_daily}{' '}
                        {withdrawChanceInfo.currency}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-wts-muted">지갑 상태</span>
                      <span
                        className={
                          withdrawChanceInfo.currency_info.wallet_state === 'working'
                            ? 'text-green-500'
                            : 'text-red-500'
                        }
                      >
                        {withdrawChanceInfo.currency_info.wallet_state === 'working'
                          ? '정상'
                          : '중단'}
                      </span>
                    </div>
                  </div>

                  {/* 출금 주소 선택 */}
                  <div className="mt-3">
                    <span className="text-wts-muted text-xs mb-1 block">출금 주소</span>
                    {withdrawAddresses.length > 0 ? (
                      <select
                        value={selectedWithdrawAddress?.withdraw_address || ''}
                        onChange={(e) => {
                          const addr = withdrawAddresses.find(
                            (a) => a.withdraw_address === e.target.value
                          );
                          handleSelectWithdrawAddress(addr || null);
                        }}
                        aria-label="출금 주소"
                        className="w-full px-3 py-2 rounded border border-wts bg-wts-secondary
                                   text-wts-foreground text-sm font-mono
                                   focus:outline-none focus:border-wts-focus"
                      >
                        <option value="">주소 선택</option>
                        {withdrawAddresses.map((addr) => (
                          <option key={addr.withdraw_address} value={addr.withdraw_address}>
                            {formatAddress(addr.withdraw_address)}
                            {addr.secondary_address && ` (Tag: ${addr.secondary_address})`}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <div className="p-3 rounded bg-yellow-900/20 border border-yellow-500/30 text-xs">
                        <div className="flex items-center gap-1 text-yellow-400 mb-1">
                          <WarningIcon className="w-3 h-3" />
                          <span>등록된 출금 주소 없음</span>
                        </div>
                        <div className="text-yellow-200 mb-2">
                          Upbit에서 출금 주소를 먼저 등록해주세요
                        </div>
                        <a
                          href="https://upbit.com/mypage/address"
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-wts-accent hover:underline"
                        >
                          Upbit 출금 주소 등록하기 →
                        </a>
                      </div>
                    )}

                    {/* 선택된 주소의 보조 주소 표시 */}
                    {selectedWithdrawAddress?.secondary_address && (
                      <div className="mt-2 p-2 rounded bg-wts-tertiary text-xs">
                        <span className="text-wts-muted">Memo/Tag: </span>
                        <span className="font-mono text-wts-foreground">
                          {selectedWithdrawAddress.secondary_address}
                        </span>
                      </div>
                    )}
                  </div>

                  {/* 수량 입력 */}
                  {selectedWithdrawAddress && (
                    <div className="mt-3">
                      <span className="text-wts-muted text-xs mb-1 block">출금 수량</span>
                      <input
                        type="text"
                        inputMode="decimal"
                        value={withdrawAmount}
                        onChange={(e) => {
                          // 숫자와 소수점만 허용
                          const value = e.target.value.replace(/[^0-9.]/g, '');
                          setWithdrawAmount(value);
                        }}
                        placeholder="0.00"
                        aria-label="출금 수량"
                        className="w-full px-3 py-2 rounded border border-wts bg-wts-secondary
                                   text-wts-foreground text-sm font-mono
                                   focus:outline-none focus:border-wts-focus"
                      />
                      {/* % 버튼 */}
                      <div className="flex gap-2 mt-2">
                        {[25, 50, 75, 100].map((percent) => (
                          <button
                            key={percent}
                            onClick={() => handlePercentClick(percent)}
                            className="flex-1 px-2 py-1 text-xs rounded bg-wts-secondary
                                       hover:bg-wts-tertiary text-wts-muted
                                       transition-colors"
                          >
                            {percent === 100 ? 'MAX' : `${percent}%`}
                          </button>
                        ))}
                      </div>

                      {/* 입력값 유효성 검사 메시지 */}
                      {withdrawAmount && parseFloat(withdrawAmount) > 0 && (
                        <>
                          {parseFloat(withdrawAmount) <
                            parseFloat(withdrawChanceInfo.withdraw_limit.minimum) && (
                            <div className="mt-1 text-red-400 text-xs">
                              최소 출금 수량은 {withdrawChanceInfo.withdraw_limit.minimum}{' '}
                              {withdrawChanceInfo.currency} 입니다
                            </div>
                          )}
                          {parseFloat(withdrawAmount) >
                            parseFloat(withdrawChanceInfo.account_info.balance) -
                              parseFloat(withdrawChanceInfo.account_info.locked) && (
                            <div className="mt-1 text-red-400 text-xs">
                              출금 가능 잔고를 초과했습니다
                            </div>
                          )}
                        </>
                      )}
                    </div>
                  )}

                  {/* 실수령액 표시 */}
                  {selectedWithdrawAddress && withdrawAmount && (
                    <div className="mt-3 p-3 rounded bg-wts-tertiary text-xs">
                      <div className="flex justify-between items-center">
                        <span className="text-wts-muted">실수령액</span>
                        <span
                          className={`font-mono text-lg ${
                            receivableAmount() && parseFloat(receivableAmount()!) > 0
                              ? 'text-green-400'
                              : 'text-red-400'
                          }`}
                        >
                          {receivableAmount() || '0'} {withdrawChanceInfo.currency}
                        </span>
                      </div>
                      {receivableAmount() &&
                        parseFloat(receivableAmount()!) <= 0 && (
                          <div className="mt-1 text-red-400 text-xs flex items-center gap-1">
                            <WarningIcon className="w-3 h-3" />
                            수수료 차감 후 실수령액이 0 이하입니다
                          </div>
                        )}
                    </div>
                  )}

                  {/* 출금 버튼 */}
                  {selectedWithdrawAddress && (
                    <button
                      onClick={handleWithdrawButtonClick}
                      disabled={!isWithdrawButtonEnabled()}
                      className="mt-3 w-full py-2 rounded font-medium
                                 bg-wts-accent text-white
                                 disabled:opacity-50 disabled:cursor-not-allowed
                                 hover:bg-opacity-90 transition-colors"
                    >
                      출금
                    </button>
                  )}
                </>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

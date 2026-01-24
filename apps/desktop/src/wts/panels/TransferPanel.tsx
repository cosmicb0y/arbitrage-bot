import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTransferStore } from '../stores/transferStore';
import { useConsoleStore } from '../stores/consoleStore';
import { handleApiError } from '../utils/errorHandler';
import { isDepositAvailable } from '../types';
import type { DepositChanceResponse, WtsApiResult, DepositChanceParams } from '../types';

interface TransferPanelProps {
  className?: string;
}

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

/**
 * Transfer 패널 컴포넌트
 * 입금/출금 기능을 제공하는 패널
 */
export function TransferPanel({ className = '' }: TransferPanelProps) {
  const {
    activeTab,
    selectedCurrency,
    networkInfo,
    isLoading,
    error,
    setActiveTab,
    setSelectedCurrency,
    setNetworkInfo,
    setLoading,
    setError,
  } = useTransferStore();

  const addLog = useConsoleStore((state) => state.addLog);

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
      if (!currencyInfo || currencyInfo.networks.length === 0) {
        return;
      }

      const defaultNetwork = currencyInfo.networks[0];
      await fetchDepositChance(currency, defaultNetwork);
    },
    [setSelectedCurrency, addLog]
  );

  // 입금 가능 정보 조회
  const fetchDepositChance = useCallback(
    async (currency: string, netType: string) => {
      setLoading(true);
      setError(null);

      try {
        const params: DepositChanceParams = { currency, net_type: netType };
        const result = await invoke<WtsApiResult<DepositChanceResponse>>(
          'wts_get_deposit_chance',
          { params }
        );

        if (result.success && result.data) {
          setNetworkInfo(result.data);
          setSelectedNetwork(netType);
          addLog('INFO', 'DEPOSIT', `입금 정보 조회 완료: ${currency}/${netType}`);
        } else {
          handleApiError(result.error, 'DEPOSIT', '입금 정보 조회 실패');
          setError(result.error?.message || '입금 정보 조회 실패');
        }
      } catch (err) {
        handleApiError(err, 'DEPOSIT', '입금 정보 조회 실패');
        setError('입금 정보 조회 실패');
      } finally {
        setLoading(false);
      }
    },
    [setLoading, setError, setNetworkInfo, addLog]
  );

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
                  <span className="text-wts-muted text-xs mb-1 block">네트워크 선택</span>
                  <div className="flex flex-wrap gap-2">
                    {DEPOSIT_CURRENCIES.find((c) => c.code === selectedCurrency)?.networks.map(
                      (net) => (
                        <button
                          key={net}
                          onClick={() => fetchDepositChance(selectedCurrency, net)}
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
                      )
                    )}
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
                    <span className="text-wts-foreground">{networkInfo.network.name}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-wts-muted">입금 상태</span>
                    <span
                      className={
                        isDepositAvailable(networkInfo.deposit_state)
                          ? 'text-green-500'
                          : 'text-red-500'
                      }
                    >
                      {networkInfo.deposit_state === 'normal' ? '정상' : '중단'}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-wts-muted">확인 횟수</span>
                    <span className="text-wts-foreground">
                      {networkInfo.network.confirm_count}회
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-wts-muted">최소 입금</span>
                    <span className="text-wts-foreground font-mono">
                      {networkInfo.minimum} {networkInfo.currency}
                    </span>
                  </div>
                </div>
              )}
            </>
          )}

          {activeTab === 'withdraw' && (
            <div className="flex items-center justify-center h-32 text-wts-muted text-sm">
              출금 기능은 준비 중입니다
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

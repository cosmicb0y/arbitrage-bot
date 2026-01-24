import { create } from 'zustand';
import type { DepositChanceResponse } from '../types';

/**
 * Transfer 상태 인터페이스
 * 입금/출금 관련 상태 관리
 */
export interface TransferState {
  /** 활성 탭: deposit(입금) | withdraw(출금) */
  activeTab: 'deposit' | 'withdraw';
  /** 선택된 자산 코드 (예: "BTC", "ETH") */
  selectedCurrency: string | null;
  /** 선택된 네트워크 타입 (예: "BTC", "ETH", "TRX") */
  selectedNetwork: string | null;
  /** 네트워크 정보 (deposit chance 응답) */
  networkInfo: DepositChanceResponse | null;
  /** 로딩 상태 */
  isLoading: boolean;
  /** 에러 메시지 */
  error: string | null;

  // Actions
  /** 활성 탭 설정 */
  setActiveTab: (tab: 'deposit' | 'withdraw') => void;
  /** 자산 선택 (네트워크 정보 초기화 포함) */
  setSelectedCurrency: (currency: string | null) => void;
  /** 네트워크 선택 */
  setSelectedNetwork: (network: string | null) => void;
  /** 네트워크 정보 설정 */
  setNetworkInfo: (info: DepositChanceResponse | null) => void;
  /** 로딩 상태 설정 */
  setLoading: (loading: boolean) => void;
  /** 에러 설정 */
  setError: (error: string | null) => void;
  /** 상태 초기화 */
  reset: () => void;
}

/**
 * Transfer Store
 * 입금/출금 관련 상태 관리
 */
export const useTransferStore = create<TransferState>()((set) => ({
  activeTab: 'deposit',
  selectedCurrency: null,
  selectedNetwork: null,
  networkInfo: null,
  isLoading: false,
  error: null,

  setActiveTab: (activeTab) => set({ activeTab }),

  setSelectedCurrency: (selectedCurrency) =>
    set({
      selectedCurrency,
      selectedNetwork: null,
      networkInfo: null,
    }),

  setSelectedNetwork: (selectedNetwork) => set({ selectedNetwork }),

  setNetworkInfo: (networkInfo) => set({ networkInfo }),

  setLoading: (isLoading) => set({ isLoading }),

  setError: (error) => set({ error }),

  reset: () =>
    set({
      activeTab: 'deposit',
      selectedCurrency: null,
      selectedNetwork: null,
      networkInfo: null,
      isLoading: false,
      error: null,
    }),
}));

import { create } from 'zustand';
import type {
  DepositChanceResponse,
  DepositAddressResponse,
  WithdrawChanceResponse,
  WithdrawAddressResponse,
} from '../types';

/** 최대 주소 생성 재시도 횟수 */
export const MAX_GENERATE_RETRIES = 5;
/** 주소 생성 재시도 간격 (ms) */
export const GENERATE_RETRY_INTERVAL = 3000;

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

  /** 입금 주소 정보 */
  depositAddress: DepositAddressResponse | null;
  /** 주소 로딩 상태 */
  isAddressLoading: boolean;
  /** 주소 조회 에러 */
  addressError: string | null;

  /** 주소 생성 진행 중 */
  isGenerating: boolean;
  /** 생성 재시도 횟수 (0-5) */
  generateRetryCount: number;

  // 출금 상태 (WTS-5.2)
  /** 출금 가능 정보 */
  withdrawChanceInfo: WithdrawChanceResponse | null;
  /** 등록된 출금 주소 목록 */
  withdrawAddresses: WithdrawAddressResponse[];
  /** 선택된 출금 주소 */
  selectedWithdrawAddress: WithdrawAddressResponse | null;
  /** 출금 수량 */
  withdrawAmount: string;
  /** 출금 로딩 상태 */
  isWithdrawLoading: boolean;
  /** 출금 에러 */
  withdrawError: string | null;

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

  /** 입금 주소 설정 */
  setDepositAddress: (address: DepositAddressResponse | null) => void;
  /** 주소 로딩 설정 */
  setAddressLoading: (loading: boolean) => void;
  /** 주소 에러 설정 */
  setAddressError: (error: string | null) => void;

  /** 생성 진행 상태 설정 */
  setGenerating: (generating: boolean) => void;
  /** 생성 재시도 횟수 설정 */
  setGenerateRetryCount: (count: number) => void;
  /** 생성 상태 초기화 (isGenerating, generateRetryCount, addressError) */
  resetGenerateState: () => void;

  // 출금 Actions (WTS-5.2)
  /** 출금 가능 정보 설정 */
  setWithdrawChanceInfo: (info: WithdrawChanceResponse | null) => void;
  /** 출금 주소 목록 설정 */
  setWithdrawAddresses: (addresses: WithdrawAddressResponse[]) => void;
  /** 선택된 출금 주소 설정 */
  setSelectedWithdrawAddress: (address: WithdrawAddressResponse | null) => void;
  /** 출금 수량 설정 */
  setWithdrawAmount: (amount: string) => void;
  /** 출금 로딩 설정 */
  setWithdrawLoading: (loading: boolean) => void;
  /** 출금 에러 설정 */
  setWithdrawError: (error: string | null) => void;
  /** 출금 상태 초기화 */
  resetWithdrawState: () => void;

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

  depositAddress: null,
  isAddressLoading: false,
  addressError: null,

  isGenerating: false,
  generateRetryCount: 0,

  // 출금 초기 상태 (WTS-5.2)
  withdrawChanceInfo: null,
  withdrawAddresses: [],
  selectedWithdrawAddress: null,
  withdrawAmount: '',
  isWithdrawLoading: false,
  withdrawError: null,

  setActiveTab: (activeTab) => set({ activeTab }),

  setSelectedCurrency: (selectedCurrency) =>
    set((state) => ({
      ...state,
      selectedCurrency,
      selectedNetwork: null,
      networkInfo: null,
      // 자산 변경 시 주소 관련 상태도 초기화
      depositAddress: null,
      isAddressLoading: false,
      addressError: null,
      // 생성 상태도 초기화
      isGenerating: false,
      generateRetryCount: 0,
      // 출금 상태도 초기화 (WTS-5.2)
      withdrawChanceInfo: null,
      withdrawAddresses: [],
      selectedWithdrawAddress: null,
      withdrawAmount: '',
      isWithdrawLoading: false,
      withdrawError: null,
    })),

  setSelectedNetwork: (selectedNetwork) =>
    set((state) => ({
      ...state,
      selectedNetwork,
      // 네트워크 변경 시 주소 관련 상태 초기화 (재조회 필요)
      depositAddress: null,
      isAddressLoading: false,
      addressError: null,
      // 생성 상태도 초기화
      isGenerating: false,
      generateRetryCount: 0,
      // 출금 상태도 초기화 (WTS-5.2)
      withdrawChanceInfo: null,
      withdrawAddresses: [],
      selectedWithdrawAddress: null,
      withdrawAmount: '',
      isWithdrawLoading: false,
      withdrawError: null,
    })),

  setNetworkInfo: (networkInfo) => set({ networkInfo }),

  setLoading: (isLoading) => set({ isLoading }),

  setError: (error) => set({ error }),

  setDepositAddress: (depositAddress) => set({ depositAddress }),

  setAddressLoading: (isAddressLoading) => set({ isAddressLoading }),

  setAddressError: (addressError) => set({ addressError }),

  setGenerating: (isGenerating) => set({ isGenerating }),

  setGenerateRetryCount: (generateRetryCount) => set({ generateRetryCount }),

  resetGenerateState: () =>
    set({
      isGenerating: false,
      generateRetryCount: 0,
      // addressError는 초기화하지 않음 (에러 표시 유지를 위해)
    }),

  // 출금 액션 (WTS-5.2)
  setWithdrawChanceInfo: (withdrawChanceInfo) => set({ withdrawChanceInfo }),

  setWithdrawAddresses: (withdrawAddresses) => set({ withdrawAddresses }),

  setSelectedWithdrawAddress: (selectedWithdrawAddress) =>
    set({ selectedWithdrawAddress }),

  setWithdrawAmount: (withdrawAmount) => set({ withdrawAmount }),

  setWithdrawLoading: (isWithdrawLoading) => set({ isWithdrawLoading }),

  setWithdrawError: (withdrawError) => set({ withdrawError }),

  resetWithdrawState: () =>
    set({
      withdrawChanceInfo: null,
      withdrawAddresses: [],
      selectedWithdrawAddress: null,
      withdrawAmount: '',
      isWithdrawLoading: false,
      withdrawError: null,
    }),

  reset: () =>
    set({
      activeTab: 'deposit',
      selectedCurrency: null,
      selectedNetwork: null,
      networkInfo: null,
      isLoading: false,
      error: null,
      depositAddress: null,
      isAddressLoading: false,
      addressError: null,
      isGenerating: false,
      generateRetryCount: 0,
      // 출금 상태도 초기화 (WTS-5.2)
      withdrawChanceInfo: null,
      withdrawAddresses: [],
      selectedWithdrawAddress: null,
      withdrawAmount: '',
      isWithdrawLoading: false,
      withdrawError: null,
    }),
}));

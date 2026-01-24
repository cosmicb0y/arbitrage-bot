import { beforeEach, describe, expect, it } from 'vitest';
import { useTransferStore } from '../../stores/transferStore';
import type { DepositChanceResponse } from '../../types';

describe('useTransferStore', () => {
  beforeEach(() => {
    useTransferStore.getState().reset();
  });

  describe('초기 상태', () => {
    it('기본 activeTab은 deposit이다', () => {
      expect(useTransferStore.getState().activeTab).toBe('deposit');
    });

    it('기본 selectedCurrency는 null이다', () => {
      expect(useTransferStore.getState().selectedCurrency).toBeNull();
    });

    it('기본 selectedNetwork는 null이다', () => {
      expect(useTransferStore.getState().selectedNetwork).toBeNull();
    });

    it('기본 networkInfo는 null이다', () => {
      expect(useTransferStore.getState().networkInfo).toBeNull();
    });

    it('기본 isLoading은 false이다', () => {
      expect(useTransferStore.getState().isLoading).toBe(false);
    });

    it('기본 error는 null이다', () => {
      expect(useTransferStore.getState().error).toBeNull();
    });
  });

  describe('setActiveTab', () => {
    it('activeTab을 withdraw로 변경한다', () => {
      useTransferStore.getState().setActiveTab('withdraw');
      expect(useTransferStore.getState().activeTab).toBe('withdraw');
    });

    it('activeTab을 deposit으로 변경한다', () => {
      useTransferStore.getState().setActiveTab('withdraw');
      useTransferStore.getState().setActiveTab('deposit');
      expect(useTransferStore.getState().activeTab).toBe('deposit');
    });
  });

  describe('setSelectedCurrency', () => {
    it('자산을 선택한다', () => {
      useTransferStore.getState().setSelectedCurrency('BTC');
      expect(useTransferStore.getState().selectedCurrency).toBe('BTC');
    });

    it('자산 변경 시 selectedNetwork와 networkInfo를 초기화한다', () => {
      // 먼저 네트워크 정보 설정
      const mockNetworkInfo: DepositChanceResponse = {
        currency: 'ETH',
        net_type: 'ETH',
        network: {
          name: 'Ethereum',
          net_type: 'ETH',
          priority: 1,
          deposit_state: 'normal',
          confirm_count: 12,
        },
        deposit_state: 'normal',
        minimum: '0.01',
      };
      useTransferStore.setState({
        selectedCurrency: 'ETH',
        selectedNetwork: 'ETH',
        networkInfo: mockNetworkInfo,
      });

      // 자산 변경
      useTransferStore.getState().setSelectedCurrency('BTC');

      const state = useTransferStore.getState();
      expect(state.selectedCurrency).toBe('BTC');
      expect(state.selectedNetwork).toBeNull();
      expect(state.networkInfo).toBeNull();
    });

    it('자산을 null로 설정할 수 있다', () => {
      useTransferStore.getState().setSelectedCurrency('BTC');
      useTransferStore.getState().setSelectedCurrency(null);
      expect(useTransferStore.getState().selectedCurrency).toBeNull();
    });
  });

  describe('setSelectedNetwork', () => {
    it('네트워크를 선택한다', () => {
      useTransferStore.getState().setSelectedNetwork('ETH');
      expect(useTransferStore.getState().selectedNetwork).toBe('ETH');
    });

    it('네트워크를 null로 설정할 수 있다', () => {
      useTransferStore.getState().setSelectedNetwork('ETH');
      useTransferStore.getState().setSelectedNetwork(null);
      expect(useTransferStore.getState().selectedNetwork).toBeNull();
    });
  });

  describe('setNetworkInfo', () => {
    it('네트워크 정보를 설정한다', () => {
      const mockNetworkInfo: DepositChanceResponse = {
        currency: 'BTC',
        net_type: 'BTC',
        network: {
          name: 'Bitcoin',
          net_type: 'BTC',
          priority: 1,
          deposit_state: 'normal',
          confirm_count: 3,
        },
        deposit_state: 'normal',
        minimum: '0.0001',
      };

      useTransferStore.getState().setNetworkInfo(mockNetworkInfo);
      expect(useTransferStore.getState().networkInfo).toEqual(mockNetworkInfo);
    });

    it('네트워크 정보를 null로 설정할 수 있다', () => {
      useTransferStore.getState().setNetworkInfo(null);
      expect(useTransferStore.getState().networkInfo).toBeNull();
    });
  });

  describe('setLoading', () => {
    it('로딩 상태를 true로 설정한다', () => {
      useTransferStore.getState().setLoading(true);
      expect(useTransferStore.getState().isLoading).toBe(true);
    });

    it('로딩 상태를 false로 설정한다', () => {
      useTransferStore.getState().setLoading(true);
      useTransferStore.getState().setLoading(false);
      expect(useTransferStore.getState().isLoading).toBe(false);
    });
  });

  describe('setError', () => {
    it('에러 메시지를 설정한다', () => {
      useTransferStore.getState().setError('네트워크 오류');
      expect(useTransferStore.getState().error).toBe('네트워크 오류');
    });

    it('에러를 null로 설정할 수 있다', () => {
      useTransferStore.getState().setError('오류');
      useTransferStore.getState().setError(null);
      expect(useTransferStore.getState().error).toBeNull();
    });
  });

  describe('reset', () => {
    it('모든 상태를 초기값으로 리셋한다', () => {
      // 먼저 모든 상태를 변경
      useTransferStore.setState({
        activeTab: 'withdraw',
        selectedCurrency: 'ETH',
        selectedNetwork: 'ETH',
        networkInfo: {
          currency: 'ETH',
          net_type: 'ETH',
          network: {
            name: 'Ethereum',
            net_type: 'ETH',
            priority: 1,
            deposit_state: 'normal',
            confirm_count: 12,
          },
          deposit_state: 'normal',
          minimum: '0.01',
        },
        isLoading: true,
        error: '테스트 에러',
      });

      useTransferStore.getState().reset();

      const state = useTransferStore.getState();
      expect(state.activeTab).toBe('deposit');
      expect(state.selectedCurrency).toBeNull();
      expect(state.selectedNetwork).toBeNull();
      expect(state.networkInfo).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });
  });
});

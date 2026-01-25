import { beforeEach, describe, expect, it } from 'vitest';
import {
  useTransferStore,
  MAX_GENERATE_RETRIES,
  GENERATE_RETRY_INTERVAL,
} from '../../stores/transferStore';
import type {
  DepositChanceResponse,
  DepositAddressResponse,
  WithdrawChanceResponse,
  WithdrawAddressResponse,
} from '../../types';

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

    it('기본 depositAddress는 null이다', () => {
      expect(useTransferStore.getState().depositAddress).toBeNull();
    });

    it('기본 isAddressLoading은 false이다', () => {
      expect(useTransferStore.getState().isAddressLoading).toBe(false);
    });

    it('기본 addressError는 null이다', () => {
      expect(useTransferStore.getState().addressError).toBeNull();
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
        is_deposit_possible: true,
        deposit_impossible_reason: null,
        minimum_deposit_amount: 0.01,
        minimum_deposit_confirmations: 12,
        decimal_precision: 18,
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

    it('자산 변경 시 입금 주소 상태를 초기화한다', () => {
      useTransferStore.setState({
        depositAddress: {
          currency: 'BTC',
          net_type: 'BTC',
          deposit_address: 'address',
          secondary_address: null,
        },
        isAddressLoading: true,
        addressError: 'error',
      });

      useTransferStore.getState().setSelectedCurrency('ETH');

      const state = useTransferStore.getState();
      expect(state.depositAddress).toBeNull();
      expect(state.isAddressLoading).toBe(false);
      expect(state.addressError).toBeNull();
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

    it('네트워크 변경 시 입금 주소 상태를 초기화한다', () => {
      useTransferStore.setState({
        depositAddress: {
          currency: 'ETH',
          net_type: 'ETH',
          deposit_address: 'address',
          secondary_address: null,
        },
      });

      useTransferStore.getState().setSelectedNetwork('Arbitrum');

      const state = useTransferStore.getState();
      expect(state.selectedNetwork).toBe('Arbitrum');
      expect(state.depositAddress).toBeNull();
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
        is_deposit_possible: true,
        deposit_impossible_reason: null,
        minimum_deposit_amount: 0.0001,
        minimum_deposit_confirmations: 3,
        decimal_precision: 8,
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

  describe('입금 주소 상태 관리', () => {
    it('입금 주소를 설정한다', () => {
      const mockAddress: DepositAddressResponse = {
        currency: 'BTC',
        net_type: 'BTC',
        deposit_address: '1A2b3C...',
        secondary_address: null,
      };
      useTransferStore.getState().setDepositAddress(mockAddress);
      expect(useTransferStore.getState().depositAddress).toEqual(mockAddress);
    });

    it('주소 로딩 상태를 설정한다', () => {
      useTransferStore.getState().setAddressLoading(true);
      expect(useTransferStore.getState().isAddressLoading).toBe(true);
    });

    it('주소 에러를 설정한다', () => {
      useTransferStore.getState().setAddressError('주소 조회 실패');
      expect(useTransferStore.getState().addressError).toBe('주소 조회 실패');
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
          is_deposit_possible: true,
          deposit_impossible_reason: null,
          minimum_deposit_amount: 0.01,
          minimum_deposit_confirmations: 12,
          decimal_precision: 18,
        },
        isLoading: true,
        error: '테스트 에러',
        depositAddress: {
          currency: 'ETH',
          net_type: 'ETH',
          deposit_address: '0x...',
          secondary_address: null,
        },
        isAddressLoading: true,
        addressError: '주소 에러',
      });

      useTransferStore.getState().reset();

      const state = useTransferStore.getState();
      expect(state.activeTab).toBe('deposit');
      expect(state.selectedCurrency).toBeNull();
      expect(state.selectedNetwork).toBeNull();
      expect(state.networkInfo).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
      expect(state.depositAddress).toBeNull();
      expect(state.isAddressLoading).toBe(false);
      expect(state.addressError).toBeNull();
    });
  });

  describe('비동기 주소 생성 상수 (WTS-4.4)', () => {
    it('MAX_GENERATE_RETRIES는 5이다', () => {
      expect(MAX_GENERATE_RETRIES).toBe(5);
    });

    it('GENERATE_RETRY_INTERVAL은 3000ms(3초)이다', () => {
      expect(GENERATE_RETRY_INTERVAL).toBe(3000);
    });
  });

  describe('비동기 주소 생성 상태 관리 (WTS-4.4)', () => {
    beforeEach(() => {
      useTransferStore.getState().reset();
    });

    describe('초기 상태', () => {
      it('기본 isGenerating은 false이다', () => {
        expect(useTransferStore.getState().isGenerating).toBe(false);
      });

      it('기본 generateRetryCount는 0이다', () => {
        expect(useTransferStore.getState().generateRetryCount).toBe(0);
      });
    });

    describe('setGenerating', () => {
      it('isGenerating을 true로 설정한다', () => {
        useTransferStore.getState().setGenerating(true);
        expect(useTransferStore.getState().isGenerating).toBe(true);
      });

      it('isGenerating을 false로 설정한다', () => {
        useTransferStore.getState().setGenerating(true);
        useTransferStore.getState().setGenerating(false);
        expect(useTransferStore.getState().isGenerating).toBe(false);
      });
    });

    describe('setGenerateRetryCount', () => {
      it('재시도 횟수를 설정한다', () => {
        useTransferStore.getState().setGenerateRetryCount(3);
        expect(useTransferStore.getState().generateRetryCount).toBe(3);
      });

      it('재시도 횟수를 0에서 5까지 설정할 수 있다', () => {
        for (let i = 0; i <= 5; i++) {
          useTransferStore.getState().setGenerateRetryCount(i);
          expect(useTransferStore.getState().generateRetryCount).toBe(i);
        }
      });
    });

    describe('resetGenerateState', () => {
      it('생성 상태를 초기화하지만 에러는 유지한다', () => {
        // 상태 설정
        useTransferStore.getState().setGenerating(true);
        useTransferStore.getState().setGenerateRetryCount(3);
        useTransferStore.getState().setAddressError('에러 발생');

        // 초기화
        useTransferStore.getState().resetGenerateState();

        const state = useTransferStore.getState();
        expect(state.isGenerating).toBe(false);
        expect(state.generateRetryCount).toBe(0);
        // 에러는 유지되어야 함 (UI 표시를 위해)
        expect(state.addressError).toBe('에러 발생');
      });

      it('다른 상태에 영향을 주지 않는다', () => {
        // 다른 상태 설정
        useTransferStore.getState().setSelectedCurrency('BTC');
        useTransferStore.getState().setGenerating(true);

        // 생성 상태 초기화
        useTransferStore.getState().resetGenerateState();

        // 다른 상태는 유지
        expect(useTransferStore.getState().selectedCurrency).toBe('BTC');
      });
    });

    describe('자산/네트워크 변경 시 생성 상태 초기화', () => {
      it('자산 변경 시 생성 상태가 초기화된다', () => {
        useTransferStore.getState().setGenerating(true);
        useTransferStore.getState().setGenerateRetryCount(3);

        useTransferStore.getState().setSelectedCurrency('ETH');

        const state = useTransferStore.getState();
        expect(state.isGenerating).toBe(false);
        expect(state.generateRetryCount).toBe(0);
      });

      it('네트워크 변경 시 생성 상태가 초기화된다', () => {
        useTransferStore.getState().setGenerating(true);
        useTransferStore.getState().setGenerateRetryCount(2);

        useTransferStore.getState().setSelectedNetwork('TRX');

        const state = useTransferStore.getState();
        expect(state.isGenerating).toBe(false);
        expect(state.generateRetryCount).toBe(0);
      });
    });

    describe('reset에 생성 상태 포함', () => {
      it('reset 호출 시 생성 상태도 초기화된다', () => {
        useTransferStore.getState().setGenerating(true);
        useTransferStore.getState().setGenerateRetryCount(4);

        useTransferStore.getState().reset();

        const state = useTransferStore.getState();
        expect(state.isGenerating).toBe(false);
        expect(state.generateRetryCount).toBe(0);
      });
    });
  });

  // ============================================================================
  // 출금 상태 관리 (WTS-5.2)
  // ============================================================================

  describe('출금 상태 관리 (WTS-5.2)', () => {
    beforeEach(() => {
      useTransferStore.getState().reset();
    });

    const mockWithdrawChanceInfo: WithdrawChanceResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      member_level: {
        security_level: 3,
        fee_level: 0,
        email_verified: true,
        identity_auth_verified: true,
        bank_account_verified: true,
        two_factor_auth_verified: true,
        locked: false,
      },
      currency_info: {
        code: 'BTC',
        withdraw_fee: '0.0005',
        is_coin: true,
        wallet_state: 'working',
        wallet_support: ['default'],
      },
      account_info: {
        balance: '1.5',
        locked: '0.1',
        avg_buy_price: '50000000',
        avg_buy_price_modified: false,
        unit_currency: 'KRW',
      },
      withdraw_limit: {
        currency: 'BTC',
        minimum: '0.001',
        onetime: '10',
        daily: '100',
        remaining_daily: '99.5',
        remaining_daily_krw: '4975000000',
        fixed: 8,
        can_withdraw: true,
      },
    };

    const mockWithdrawAddress: WithdrawAddressResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      network_name: 'Bitcoin',
      withdraw_address: '1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa',
      secondary_address: null,
    };

    describe('초기 상태', () => {
      it('기본 withdrawChanceInfo는 null이다', () => {
        expect(useTransferStore.getState().withdrawChanceInfo).toBeNull();
      });

      it('기본 withdrawAddresses는 빈 배열이다', () => {
        expect(useTransferStore.getState().withdrawAddresses).toEqual([]);
      });

      it('기본 selectedWithdrawAddress는 null이다', () => {
        expect(useTransferStore.getState().selectedWithdrawAddress).toBeNull();
      });

      it('기본 withdrawAmount는 빈 문자열이다', () => {
        expect(useTransferStore.getState().withdrawAmount).toBe('');
      });

      it('기본 isWithdrawLoading은 false이다', () => {
        expect(useTransferStore.getState().isWithdrawLoading).toBe(false);
      });

      it('기본 withdrawError는 null이다', () => {
        expect(useTransferStore.getState().withdrawError).toBeNull();
      });
    });

    describe('setWithdrawChanceInfo', () => {
      it('출금 가능 정보를 설정한다', () => {
        useTransferStore.getState().setWithdrawChanceInfo(mockWithdrawChanceInfo);
        expect(useTransferStore.getState().withdrawChanceInfo).toEqual(mockWithdrawChanceInfo);
      });

      it('출금 가능 정보를 null로 설정할 수 있다', () => {
        useTransferStore.getState().setWithdrawChanceInfo(mockWithdrawChanceInfo);
        useTransferStore.getState().setWithdrawChanceInfo(null);
        expect(useTransferStore.getState().withdrawChanceInfo).toBeNull();
      });
    });

    describe('setWithdrawAddresses', () => {
      it('출금 주소 목록을 설정한다', () => {
        const addresses = [mockWithdrawAddress];
        useTransferStore.getState().setWithdrawAddresses(addresses);
        expect(useTransferStore.getState().withdrawAddresses).toEqual(addresses);
      });

      it('빈 배열로 설정할 수 있다', () => {
        useTransferStore.getState().setWithdrawAddresses([mockWithdrawAddress]);
        useTransferStore.getState().setWithdrawAddresses([]);
        expect(useTransferStore.getState().withdrawAddresses).toEqual([]);
      });
    });

    describe('setSelectedWithdrawAddress', () => {
      it('선택된 출금 주소를 설정한다', () => {
        useTransferStore.getState().setSelectedWithdrawAddress(mockWithdrawAddress);
        expect(useTransferStore.getState().selectedWithdrawAddress).toEqual(mockWithdrawAddress);
      });

      it('선택된 출금 주소를 null로 설정할 수 있다', () => {
        useTransferStore.getState().setSelectedWithdrawAddress(mockWithdrawAddress);
        useTransferStore.getState().setSelectedWithdrawAddress(null);
        expect(useTransferStore.getState().selectedWithdrawAddress).toBeNull();
      });
    });

    describe('setWithdrawAmount', () => {
      it('출금 수량을 설정한다', () => {
        useTransferStore.getState().setWithdrawAmount('0.5');
        expect(useTransferStore.getState().withdrawAmount).toBe('0.5');
      });

      it('빈 문자열로 설정할 수 있다', () => {
        useTransferStore.getState().setWithdrawAmount('0.5');
        useTransferStore.getState().setWithdrawAmount('');
        expect(useTransferStore.getState().withdrawAmount).toBe('');
      });
    });

    describe('setWithdrawLoading', () => {
      it('출금 로딩 상태를 true로 설정한다', () => {
        useTransferStore.getState().setWithdrawLoading(true);
        expect(useTransferStore.getState().isWithdrawLoading).toBe(true);
      });

      it('출금 로딩 상태를 false로 설정한다', () => {
        useTransferStore.getState().setWithdrawLoading(true);
        useTransferStore.getState().setWithdrawLoading(false);
        expect(useTransferStore.getState().isWithdrawLoading).toBe(false);
      });
    });

    describe('setWithdrawError', () => {
      it('출금 에러 메시지를 설정한다', () => {
        useTransferStore.getState().setWithdrawError('출금 실패');
        expect(useTransferStore.getState().withdrawError).toBe('출금 실패');
      });

      it('출금 에러를 null로 설정할 수 있다', () => {
        useTransferStore.getState().setWithdrawError('출금 실패');
        useTransferStore.getState().setWithdrawError(null);
        expect(useTransferStore.getState().withdrawError).toBeNull();
      });
    });

    describe('resetWithdrawState', () => {
      it('모든 출금 상태를 초기화한다', () => {
        // 출금 상태 설정
        useTransferStore.getState().setWithdrawChanceInfo(mockWithdrawChanceInfo);
        useTransferStore.getState().setWithdrawAddresses([mockWithdrawAddress]);
        useTransferStore.getState().setSelectedWithdrawAddress(mockWithdrawAddress);
        useTransferStore.getState().setWithdrawAmount('0.5');
        useTransferStore.getState().setWithdrawLoading(true);
        useTransferStore.getState().setWithdrawError('에러');

        // 출금 상태 초기화
        useTransferStore.getState().resetWithdrawState();

        const state = useTransferStore.getState();
        expect(state.withdrawChanceInfo).toBeNull();
        expect(state.withdrawAddresses).toEqual([]);
        expect(state.selectedWithdrawAddress).toBeNull();
        expect(state.withdrawAmount).toBe('');
        expect(state.isWithdrawLoading).toBe(false);
        expect(state.withdrawError).toBeNull();
      });

      it('입금 상태에는 영향을 주지 않는다', () => {
        // 입금 상태 설정
        useTransferStore.getState().setSelectedCurrency('BTC');
        useTransferStore.getState().setSelectedNetwork('BTC');

        // 출금 상태 초기화
        useTransferStore.getState().resetWithdrawState();

        // 입금 상태는 유지
        expect(useTransferStore.getState().selectedCurrency).toBe('BTC');
        expect(useTransferStore.getState().selectedNetwork).toBe('BTC');
      });
    });

    describe('자산/네트워크 변경 시 출금 상태 초기화', () => {
      it('자산 변경 시 출금 상태가 초기화된다', () => {
        // 출금 상태 설정
        useTransferStore.getState().setWithdrawChanceInfo(mockWithdrawChanceInfo);
        useTransferStore.getState().setWithdrawAddresses([mockWithdrawAddress]);
        useTransferStore.getState().setSelectedWithdrawAddress(mockWithdrawAddress);
        useTransferStore.getState().setWithdrawAmount('0.5');

        // 자산 변경
        useTransferStore.getState().setSelectedCurrency('ETH');

        const state = useTransferStore.getState();
        expect(state.withdrawChanceInfo).toBeNull();
        expect(state.withdrawAddresses).toEqual([]);
        expect(state.selectedWithdrawAddress).toBeNull();
        expect(state.withdrawAmount).toBe('');
      });

      it('네트워크 변경 시 출금 상태가 초기화된다', () => {
        // 출금 상태 설정
        useTransferStore.getState().setWithdrawChanceInfo(mockWithdrawChanceInfo);
        useTransferStore.getState().setWithdrawAddresses([mockWithdrawAddress]);
        useTransferStore.getState().setSelectedWithdrawAddress(mockWithdrawAddress);
        useTransferStore.getState().setWithdrawAmount('0.5');

        // 네트워크 변경
        useTransferStore.getState().setSelectedNetwork('TRX');

        const state = useTransferStore.getState();
        expect(state.withdrawChanceInfo).toBeNull();
        expect(state.withdrawAddresses).toEqual([]);
        expect(state.selectedWithdrawAddress).toBeNull();
        expect(state.withdrawAmount).toBe('');
      });
    });

    describe('reset에 출금 상태 포함', () => {
      it('reset 호출 시 출금 상태도 초기화된다', () => {
        // 출금 상태 설정
        useTransferStore.getState().setWithdrawChanceInfo(mockWithdrawChanceInfo);
        useTransferStore.getState().setWithdrawAddresses([mockWithdrawAddress]);
        useTransferStore.getState().setSelectedWithdrawAddress(mockWithdrawAddress);
        useTransferStore.getState().setWithdrawAmount('0.5');
        useTransferStore.getState().setWithdrawLoading(true);
        useTransferStore.getState().setWithdrawError('에러');

        useTransferStore.getState().reset();

        const state = useTransferStore.getState();
        expect(state.withdrawChanceInfo).toBeNull();
        expect(state.withdrawAddresses).toEqual([]);
        expect(state.selectedWithdrawAddress).toBeNull();
        expect(state.withdrawAmount).toBe('');
        expect(state.isWithdrawLoading).toBe(false);
        expect(state.withdrawError).toBeNull();
      });
    });
  });
});

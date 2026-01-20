import { beforeEach, describe, expect, it } from 'vitest';
import { useOrderStore } from '../../stores/orderStore';

describe('useOrderStore', () => {
  beforeEach(() => {
    useOrderStore.setState({
      orderType: 'limit',
      side: 'buy',
      price: '',
      quantity: '',
    });
  });

  describe('개별 액션', () => {
    it('setOrderType이 주문 유형을 변경한다', () => {
      useOrderStore.getState().setOrderType('market');
      expect(useOrderStore.getState().orderType).toBe('market');

      useOrderStore.getState().setOrderType('limit');
      expect(useOrderStore.getState().orderType).toBe('limit');
    });

    it('setSide가 주문 방향을 변경한다', () => {
      useOrderStore.getState().setSide('sell');
      expect(useOrderStore.getState().side).toBe('sell');

      useOrderStore.getState().setSide('buy');
      expect(useOrderStore.getState().side).toBe('buy');
    });

    it('setPrice가 가격을 설정한다', () => {
      useOrderStore.getState().setPrice('50000000');
      expect(useOrderStore.getState().price).toBe('50000000');
    });

    it('setQuantity가 수량을 설정한다', () => {
      useOrderStore.getState().setQuantity('0.5');
      expect(useOrderStore.getState().quantity).toBe('0.5');
    });
  });

  describe('setPriceFromOrderbook', () => {
    it('매도 호가(ask) 클릭 시 가격, 지정가, 매수 방향을 설정한다', () => {
      useOrderStore.getState().setPriceFromOrderbook(50000000, 'ask');

      const state = useOrderStore.getState();
      expect(state.price).toBe('50000000');
      expect(state.orderType).toBe('limit');
      expect(state.side).toBe('buy'); // ask 클릭 = 매수 의도
    });

    it('매수 호가(bid) 클릭 시 가격, 지정가, 매도 방향을 설정한다', () => {
      useOrderStore.getState().setPriceFromOrderbook(49900000, 'bid');

      const state = useOrderStore.getState();
      expect(state.price).toBe('49900000');
      expect(state.orderType).toBe('limit');
      expect(state.side).toBe('sell'); // bid 클릭 = 매도 의도
    });

    it('소수점 가격도 문자열로 변환한다', () => {
      useOrderStore.getState().setPriceFromOrderbook(0.00012345, 'ask');
      expect(useOrderStore.getState().price).toBe('0.00012345');
    });
  });

  describe('resetForm', () => {
    it('가격, 수량을 초기화하고 기본값으로 리셋한다', () => {
      // 먼저 값을 설정
      useOrderStore.setState({
        orderType: 'market',
        side: 'sell',
        price: '50000000',
        quantity: '0.5',
      });

      useOrderStore.getState().resetForm();

      const state = useOrderStore.getState();
      expect(state.price).toBe('');
      expect(state.quantity).toBe('');
      expect(state.orderType).toBe('limit');
      expect(state.side).toBe('buy');
    });
  });
});

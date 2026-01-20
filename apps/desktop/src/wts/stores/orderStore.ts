import { create } from 'zustand';
import type { OrderType, OrderSide } from '../types';

/**
 * Order Form 상태 인터페이스
 * 주문 폼의 상태를 관리하며, 오더북에서 호가 클릭 시 가격/방향 자동 설정을 지원합니다.
 */
export interface OrderState {
  /** 주문 유형: market(시장가) | limit(지정가) */
  orderType: OrderType;
  /** 주문 방향: buy(매수) | sell(매도) */
  side: OrderSide;
  /** 지정가 가격 (문자열) */
  price: string;
  /** 주문 수량 (문자열) */
  quantity: string;

  /** 주문 유형 설정 */
  setOrderType: (type: OrderType) => void;
  /** 주문 방향 설정 */
  setSide: (side: OrderSide) => void;
  /** 가격 설정 */
  setPrice: (price: string) => void;
  /** 수량 설정 */
  setQuantity: (quantity: string) => void;
  /**
   * 오더북에서 호가 클릭 시 호출
   * - 매도 호가(ask) 클릭 = 매수(buy) 의도
   * - 매수 호가(bid) 클릭 = 매도(sell) 의도
   * @param price 클릭한 호가 가격
   * @param clickedSide 클릭한 호가 타입 ('ask' | 'bid')
   */
  setPriceFromOrderbook: (price: number, clickedSide: 'ask' | 'bid') => void;
  /** 폼 초기화 (기본값: limit, buy, '', '') */
  resetForm: () => void;
}

/**
 * Order Store
 * 주문 폼 상태 관리
 */
export const useOrderStore = create<OrderState>()((set) => ({
  orderType: 'limit',
  side: 'buy',
  price: '',
  quantity: '',

  setOrderType: (orderType) => set({ orderType }),
  setSide: (side) => set({ side }),
  setPrice: (price) => set({ price }),
  setQuantity: (quantity) => set({ quantity }),

  // 오더북에서 호가 클릭 시 호출
  // 매도 호가(ask) 클릭 = 매수 의도, 매수 호가(bid) 클릭 = 매도 의도
  setPriceFromOrderbook: (price, clickedSide) => {
    const side: OrderSide = clickedSide === 'ask' ? 'buy' : 'sell';
    set({
      price: price.toString(),
      orderType: 'limit',
      side,
    });
  },

  resetForm: () =>
    set({
      price: '',
      quantity: '',
      orderType: 'limit',
      side: 'buy',
    }),
}));

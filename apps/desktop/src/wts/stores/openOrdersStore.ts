import { create } from 'zustand';
import type { UpbitMyOrderResponse } from '../types';

type WsStatus = 'connecting' | 'connected' | 'disconnected';

interface OpenOrdersState {
  /** 미체결 주문 목록 (state === 'wait' 인 주문만) */
  orders: UpbitMyOrderResponse[];
  /** WebSocket 연결 상태 */
  wsStatus: WsStatus;
  /** WebSocket 에러 메시지 */
  wsError: string | null;

  /** 주문 추가 또는 업데이트 */
  upsertOrder: (order: UpbitMyOrderResponse) => void;
  /** 주문 제거 (uuid로) */
  removeOrder: (uuid: string) => void;
  /** 전체 주문 목록 초기화 */
  clearOrders: () => void;
  /** WebSocket 연결 상태 설정 */
  setWsStatus: (status: WsStatus) => void;
  /** WebSocket 에러 설정 */
  setWsError: (error: string | null) => void;
}

export const useOpenOrdersStore = create<OpenOrdersState>()((set) => ({
  orders: [],
  wsStatus: 'disconnected',
  wsError: null,

  upsertOrder: (order) =>
    set((state) => {
      const existingIndex = state.orders.findIndex((o) => o.uuid === order.uuid);
      if (existingIndex >= 0) {
        // 기존 주문 업데이트
        const newOrders = [...state.orders];
        newOrders[existingIndex] = order;
        return { orders: newOrders };
      }
      // 새 주문 추가
      return { orders: [...state.orders, order] };
    }),

  removeOrder: (uuid) =>
    set((state) => ({
      orders: state.orders.filter((o) => o.uuid !== uuid),
    })),

  clearOrders: () => set({ orders: [] }),

  setWsStatus: (wsStatus) => set({ wsStatus }),

  setWsError: (wsError) => set({ wsError }),
}));

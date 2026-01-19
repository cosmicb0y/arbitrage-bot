import { create } from 'zustand';
import type { OrderbookEntry } from '../types';

type WsStatus = 'connecting' | 'connected' | 'disconnected';

interface OrderbookState {
  asks: OrderbookEntry[];
  bids: OrderbookEntry[];
  timestamp: number | null;
  wsStatus: WsStatus;
  wsError: string | null;
  setOrderbook: (
    asks: OrderbookEntry[],
    bids: OrderbookEntry[],
    timestamp: number
  ) => void;
  clearOrderbook: () => void;
  setWsStatus: (status: WsStatus) => void;
  setWsError: (error: string | null) => void;
}

export const useOrderbookStore = create<OrderbookState>()((set) => ({
  asks: [],
  bids: [],
  timestamp: null,
  wsStatus: 'disconnected',
  wsError: null,

  setOrderbook: (asks, bids, timestamp) => set({ asks, bids, timestamp }),
  clearOrderbook: () => set({ asks: [], bids: [], timestamp: null }),
  setWsStatus: (wsStatus) => set({ wsStatus }),
  setWsError: (wsError) => set({ wsError }),
}));

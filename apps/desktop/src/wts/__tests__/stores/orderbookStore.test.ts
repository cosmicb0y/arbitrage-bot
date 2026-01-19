import { describe, it, expect, beforeEach } from 'vitest';
import { useOrderbookStore } from '../../stores/orderbookStore';

describe('orderbookStore', () => {
  beforeEach(() => {
    // Reset store state before each test
    useOrderbookStore.setState({
      asks: [],
      bids: [],
      timestamp: null,
      wsStatus: 'disconnected',
      wsError: null,
    });
  });

  describe('initial state', () => {
    it('should have correct initial state', () => {
      const state = useOrderbookStore.getState();
      expect(state.asks).toEqual([]);
      expect(state.bids).toEqual([]);
      expect(state.timestamp).toBeNull();
      expect(state.wsStatus).toBe('disconnected');
      expect(state.wsError).toBeNull();
    });
  });

  describe('setOrderbook', () => {
    it('should update asks, bids, timestamp', () => {
      const asks = [{ price: 50100000, size: 0.5 }];
      const bids = [{ price: 50000000, size: 0.8 }];

      useOrderbookStore.getState().setOrderbook(asks, bids, 1704067200000);

      const state = useOrderbookStore.getState();
      expect(state.asks).toEqual(asks);
      expect(state.bids).toEqual(bids);
      expect(state.timestamp).toBe(1704067200000);
    });

    it('should handle multiple orderbook entries', () => {
      const asks = [
        { price: 50100000, size: 0.5 },
        { price: 50200000, size: 1.0 },
        { price: 50300000, size: 0.3 },
      ];
      const bids = [
        { price: 50000000, size: 0.8 },
        { price: 49900000, size: 1.2 },
        { price: 49800000, size: 0.6 },
      ];

      useOrderbookStore.getState().setOrderbook(asks, bids, 1704067200000);

      const state = useOrderbookStore.getState();
      expect(state.asks).toHaveLength(3);
      expect(state.bids).toHaveLength(3);
    });

    it('should overwrite previous orderbook data', () => {
      const asks1 = [{ price: 50100000, size: 0.5 }];
      const bids1 = [{ price: 50000000, size: 0.8 }];
      useOrderbookStore.getState().setOrderbook(asks1, bids1, 1704067200000);

      const asks2 = [{ price: 50150000, size: 0.6 }];
      const bids2 = [{ price: 50050000, size: 0.9 }];
      useOrderbookStore.getState().setOrderbook(asks2, bids2, 1704067201000);

      const state = useOrderbookStore.getState();
      expect(state.asks).toEqual(asks2);
      expect(state.bids).toEqual(bids2);
      expect(state.timestamp).toBe(1704067201000);
    });
  });

  describe('clearOrderbook', () => {
    it('should reset asks, bids, timestamp', () => {
      // Setup initial data
      useOrderbookStore.getState().setOrderbook(
        [{ price: 50100000, size: 0.5 }],
        [{ price: 50000000, size: 0.8 }],
        1704067200000
      );

      // Clear
      useOrderbookStore.getState().clearOrderbook();

      const state = useOrderbookStore.getState();
      expect(state.asks).toEqual([]);
      expect(state.bids).toEqual([]);
      expect(state.timestamp).toBeNull();
    });

    it('should not affect wsStatus', () => {
      useOrderbookStore.setState({ wsStatus: 'connected' });
      useOrderbookStore.getState().clearOrderbook();

      expect(useOrderbookStore.getState().wsStatus).toBe('connected');
    });

    it('should not affect wsError', () => {
      useOrderbookStore.setState({ wsError: '에러' });
      useOrderbookStore.getState().clearOrderbook();

      expect(useOrderbookStore.getState().wsError).toBe('에러');
    });
  });

  describe('setWsStatus', () => {
    it('should update wsStatus to connecting', () => {
      useOrderbookStore.getState().setWsStatus('connecting');
      expect(useOrderbookStore.getState().wsStatus).toBe('connecting');
    });

    it('should update wsStatus to connected', () => {
      useOrderbookStore.getState().setWsStatus('connected');
      expect(useOrderbookStore.getState().wsStatus).toBe('connected');
    });

    it('should update wsStatus to disconnected', () => {
      useOrderbookStore.setState({ wsStatus: 'connected' });
      useOrderbookStore.getState().setWsStatus('disconnected');
      expect(useOrderbookStore.getState().wsStatus).toBe('disconnected');
    });
  });

  describe('setWsError', () => {
    it('should update wsError message', () => {
      useOrderbookStore.getState().setWsError('오류 발생');
      expect(useOrderbookStore.getState().wsError).toBe('오류 발생');
    });

    it('should clear wsError', () => {
      useOrderbookStore.setState({ wsError: '에러' });
      useOrderbookStore.getState().setWsError(null);
      expect(useOrderbookStore.getState().wsError).toBeNull();
    });
  });
});

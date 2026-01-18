import { beforeEach, describe, expect, it } from 'vitest';
import { useWtsStore } from '../../stores/wtsStore';

describe('useWtsStore', () => {
  beforeEach(() => {
    useWtsStore.setState({
      selectedExchange: 'upbit',
      selectedMarket: null,
      connectionStatus: 'disconnected',
    });
  });

  it('초기 상태가 기대값과 일치한다', () => {
    const state = useWtsStore.getState();

    expect(state.selectedExchange).toBe('upbit');
    expect(state.selectedMarket).toBeNull();
    expect(state.connectionStatus).toBe('disconnected');
  });

  it('setExchange가 선택 거래소를 갱신한다', () => {
    const state = useWtsStore.getState();

    state.setExchange('upbit');

    expect(useWtsStore.getState().selectedExchange).toBe('upbit');
  });

  it('setMarket이 마켓을 갱신한다', () => {
    const state = useWtsStore.getState();

    state.setMarket('KRW-BTC');
    expect(useWtsStore.getState().selectedMarket).toBe('KRW-BTC');

    state.setMarket(null);
    expect(useWtsStore.getState().selectedMarket).toBeNull();
  });

  it('setConnectionStatus가 연결 상태를 갱신한다', () => {
    const state = useWtsStore.getState();

    state.setConnectionStatus('connected');

    expect(useWtsStore.getState().connectionStatus).toBe('connected');
  });
});

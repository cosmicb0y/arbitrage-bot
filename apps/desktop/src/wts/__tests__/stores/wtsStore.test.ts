import { beforeEach, describe, expect, it } from 'vitest';
import { useWtsStore } from '../../stores/wtsStore';

describe('useWtsStore', () => {
  beforeEach(() => {
    useWtsStore.setState({
      enabledExchanges: ['upbit'],
      selectedExchange: 'upbit',
      selectedMarket: null,
      connectionStatus: 'disconnected',
      lastConnectionError: null,
    });
  });

  it('초기 상태가 기대값과 일치한다', () => {
    const state = useWtsStore.getState();

    expect(state.enabledExchanges).toEqual(['upbit']);
    expect(state.selectedExchange).toBe('upbit');
    expect(state.selectedMarket).toBeNull();
    expect(state.connectionStatus).toBe('disconnected');
  });

  it('setExchange가 선택 거래소를 갱신한다', () => {
    const state = useWtsStore.getState();

    state.setExchange('upbit');

    expect(useWtsStore.getState().selectedExchange).toBe('upbit');
  });

  it('setEnabledExchanges가 활성 거래소 목록을 갱신한다', () => {
    const state = useWtsStore.getState();

    state.setEnabledExchanges(['upbit', 'bithumb']);

    expect(useWtsStore.getState().enabledExchanges).toEqual([
      'upbit',
      'bithumb',
    ]);
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

  it('lastConnectionError 초기값은 null이다', () => {
    const state = useWtsStore.getState();

    expect(state.lastConnectionError).toBeNull();
  });

  it('setConnectionError가 에러 메시지를 저장한다', () => {
    const state = useWtsStore.getState();

    state.setConnectionError('Network timeout');

    expect(useWtsStore.getState().lastConnectionError).toBe('Network timeout');
  });

  it('setConnectionError(null)이 에러를 초기화한다', () => {
    const state = useWtsStore.getState();

    state.setConnectionError('Some error');
    expect(useWtsStore.getState().lastConnectionError).toBe('Some error');

    state.setConnectionError(null);
    expect(useWtsStore.getState().lastConnectionError).toBeNull();
  });
});

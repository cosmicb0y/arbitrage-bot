import { beforeEach, describe, expect, it } from 'vitest';
import { useWtsStore } from '../../stores/wtsStore';
import { useConsoleStore } from '../../stores/consoleStore';
import { UPBIT_DEFAULT_MARKETS } from '../../types';

describe('useWtsStore', () => {
  beforeEach(() => {
    useWtsStore.setState({
      enabledExchanges: ['upbit'],
      selectedExchange: 'upbit',
      selectedMarket: null,
      connectionStatus: 'disconnected',
      lastConnectionError: null,
      availableMarkets: UPBIT_DEFAULT_MARKETS,
    });
    useConsoleStore.setState({ logs: [] });
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

  describe('마켓 관련 기능', () => {
    it('availableMarkets 초기값은 UPBIT_DEFAULT_MARKETS이다', () => {
      const state = useWtsStore.getState();
      expect(state.availableMarkets).toEqual(UPBIT_DEFAULT_MARKETS);
    });

    it('setAvailableMarkets가 마켓 목록을 업데이트한다', () => {
      const newMarkets = [
        { code: 'KRW-BTC', base: 'BTC', quote: 'KRW', displayName: '비트코인' },
      ];

      useWtsStore.getState().setAvailableMarkets(newMarkets);

      expect(useWtsStore.getState().availableMarkets).toEqual(newMarkets);
    });

    it('setMarket이 마켓 변경 시 콘솔에 로그를 기록한다', () => {
      useWtsStore.getState().setMarket('KRW-BTC');

      const logs = useConsoleStore.getState().logs;
      expect(logs.length).toBe(1);
      expect(logs[0].level).toBe('INFO');
      expect(logs[0].category).toBe('SYSTEM');
      expect(logs[0].message).toBe('마켓 변경: KRW-BTC');
    });

    it('setMarket이 유효하지 않은 마켓 코드면 상태를 변경하지 않는다', () => {
      useWtsStore.getState().setMarket('KRW-INVALID');

      expect(useWtsStore.getState().selectedMarket).toBeNull();
      expect(useConsoleStore.getState().logs.length).toBe(0);
    });

    it('setMarket이 동일한 마켓 선택 시 로그를 기록하지 않는다', () => {
      useWtsStore.getState().setMarket('KRW-BTC');
      useConsoleStore.setState({ logs: [] }); // 로그 초기화

      useWtsStore.getState().setMarket('KRW-BTC');

      expect(useConsoleStore.getState().logs.length).toBe(0);
    });

    it('setMarket이 null 설정 시 로그를 기록하지 않는다', () => {
      useWtsStore.getState().setMarket('KRW-BTC');
      useConsoleStore.setState({ logs: [] }); // 로그 초기화

      useWtsStore.getState().setMarket(null);

      expect(useConsoleStore.getState().logs.length).toBe(0);
    });

    it('setExchange가 거래소 변경 시 선택된 마켓을 null로 초기화한다', () => {
      useWtsStore.getState().setMarket('KRW-BTC');
      expect(useWtsStore.getState().selectedMarket).toBe('KRW-BTC');

      useWtsStore.getState().setExchange('bithumb');

      expect(useWtsStore.getState().selectedExchange).toBe('bithumb');
      expect(useWtsStore.getState().selectedMarket).toBeNull();
    });

    it('setExchange가 거래소 변경 시 마켓 목록을 갱신한다', () => {
      useWtsStore.getState().setExchange('bithumb');

      expect(useWtsStore.getState().availableMarkets).toEqual([]);
    });
  });
});

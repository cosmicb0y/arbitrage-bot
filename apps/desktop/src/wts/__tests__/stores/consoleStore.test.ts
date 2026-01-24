import { beforeEach, describe, expect, it } from 'vitest';
import { useConsoleStore } from '../../stores/consoleStore';
import { MAX_CONSOLE_LOGS } from '../../types';

describe('useConsoleStore', () => {
  beforeEach(() => {
    useConsoleStore.setState({ logs: [] });
  });

  it('로그를 FIFO 순서로 유지한다', () => {
    const state = useConsoleStore.getState();

    state.addLog('INFO', 'SYSTEM', 'first');
    state.addLog('SUCCESS', 'ORDER', 'second');

    const logs = useConsoleStore.getState().logs;

    expect(logs).toHaveLength(2);
    expect(logs[0].message).toBe('first');
    expect(logs[1].message).toBe('second');
  });

  it('최대 로그 수를 초과하면 오래된 로그를 제거한다', () => {
    const state = useConsoleStore.getState();

    for (let i = 0; i < MAX_CONSOLE_LOGS + 1; i += 1) {
      state.addLog('INFO', 'SYSTEM', `log-${i}`);
    }

    const logs = useConsoleStore.getState().logs;

    expect(logs).toHaveLength(MAX_CONSOLE_LOGS);
    expect(logs[0].message).toBe('log-1');
    expect(logs[logs.length - 1].message).toBe(`log-${MAX_CONSOLE_LOGS}`);
  });

  describe('WTS-3.6: 로그 성능 (AC: #4)', () => {
    it('addLog가 동기적으로 상태를 업데이트해야 한다', () => {
      const state = useConsoleStore.getState();

      state.addLog('SUCCESS', 'ORDER', '주문 체결');

      const logs = useConsoleStore.getState().logs;
      expect(logs).toHaveLength(1);
      expect(logs[0].message).toBe('주문 체결');
    });

    it('다수의 로그 추가 후 상태가 즉시 반영되어야 한다', () => {
      const state = useConsoleStore.getState();

      for (let i = 0; i < 100; i += 1) {
        state.addLog('INFO', 'SYSTEM', `log-${i}`);
      }

      const logs = useConsoleStore.getState().logs;
      expect(logs).toHaveLength(100);
    });

    it('로그에 타임스탬프가 정확히 기록되어야 한다', () => {
      const state = useConsoleStore.getState();
      const beforeAdd = Date.now();

      state.addLog('INFO', 'ORDER', 'test');

      const afterAdd = Date.now();
      const log = useConsoleStore.getState().logs[0];

      expect(log.timestamp).toBeGreaterThanOrEqual(beforeAdd);
      expect(log.timestamp).toBeLessThanOrEqual(afterAdd);
    });

    it('로그에 고유 ID가 생성되어야 한다', () => {
      const state = useConsoleStore.getState();

      state.addLog('INFO', 'ORDER', 'log1');
      state.addLog('INFO', 'ORDER', 'log2');

      const logs = useConsoleStore.getState().logs;
      expect(logs[0].id).not.toBe(logs[1].id);
    });
  });

  describe('WTS-3.6: 로그 데이터 구조', () => {
    it('로그에 필수 필드가 포함되어야 한다', () => {
      const state = useConsoleStore.getState();

      state.addLog('SUCCESS', 'ORDER', '주문 완료', { uuid: '123' });

      const log = useConsoleStore.getState().logs[0];

      expect(log).toHaveProperty('id');
      expect(log).toHaveProperty('timestamp');
      expect(log).toHaveProperty('level', 'SUCCESS');
      expect(log).toHaveProperty('category', 'ORDER');
      expect(log).toHaveProperty('message', '주문 완료');
      expect(log).toHaveProperty('detail');
      expect(log.detail).toEqual({ uuid: '123' });
    });

    it('detail 없이 로그 추가 시 undefined여야 한다', () => {
      const state = useConsoleStore.getState();

      state.addLog('INFO', 'SYSTEM', 'no detail');

      const log = useConsoleStore.getState().logs[0];
      expect(log.detail).toBeUndefined();
    });

    it('민감 정보 detail은 마스킹되어야 한다', () => {
      const state = useConsoleStore.getState();

      state.addLog('INFO', 'SYSTEM', 'mask', {
        api_key: 'secret',
        nested: { token: 'token-123' },
      });

      const log = useConsoleStore.getState().logs[0];
      const detail = log.detail as Record<string, unknown>;
      const nested = detail.nested as Record<string, unknown>;

      expect(detail.api_key).toBe('[MASKED]');
      expect(nested.token).toBe('[MASKED]');
    });
  });
});

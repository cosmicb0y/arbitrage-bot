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
});

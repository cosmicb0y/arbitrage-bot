import { create } from 'zustand';
import type {
  ConsoleState,
  ConsoleLogEntry,
  LogLevel,
  LogCategory,
} from '../types';
import { MAX_CONSOLE_LOGS } from '../types';

/**
 * 콘솔 스토어
 * 콘솔 로그 관리 (최대 1000개 FIFO)
 */
export const useConsoleStore = create<ConsoleState>()((set) => ({
  logs: [],

  addLog: (
    level: LogLevel,
    category: LogCategory,
    message: string,
    detail?: unknown
  ) =>
    set((state) => {
      const id =
        globalThis.crypto?.randomUUID?.() ??
        `${Date.now()}-${Math.random().toString(16).slice(2)}`;

      const newLog: ConsoleLogEntry = {
        id,
        timestamp: Date.now(),
        level,
        category,
        message,
        detail,
      };

      const logs = [...state.logs, newLog];
      const trimmedLogs =
        logs.length > MAX_CONSOLE_LOGS
          ? logs.slice(logs.length - MAX_CONSOLE_LOGS)
          : logs;

      return { logs: trimmedLogs };
    }),

  clearLogs: () => set({ logs: [] }),
}));

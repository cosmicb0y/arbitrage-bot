import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ConsolePanel } from '../../panels/ConsolePanel';
import { useConsoleStore } from '../../stores/consoleStore';

// Mock the store
vi.mock('../../stores/consoleStore', () => ({
  useConsoleStore: vi.fn(),
}));

describe('ConsolePanel', () => {
  const mockClearLogs = vi.fn();
  const mockAddLog = vi.fn();

  const createMockStore = (logs: ReturnType<typeof createTestLog>[] = []) => ({
    logs,
    addLog: mockAddLog,
    clearLogs: mockClearLogs,
  });

  const createTestLog = (
    overrides: {
      id?: string;
      timestamp?: number;
      level?: 'INFO' | 'SUCCESS' | 'ERROR' | 'WARN';
      category?: 'ORDER' | 'BALANCE' | 'DEPOSIT' | 'WITHDRAW' | 'SYSTEM';
      message?: string;
      detail?: unknown;
    } = {}
  ) => ({
    id: overrides.id || `test-${Math.random()}`,
    timestamp: overrides.timestamp || Date.now(),
    level: overrides.level || 'INFO',
    category: overrides.category || 'SYSTEM',
    message: overrides.message || 'Test message',
    detail: overrides.detail,
  });

  beforeEach(() => {
    vi.mocked(useConsoleStore).mockReturnValue(createMockStore());
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Empty State', () => {
    it('should display "No logs yet" when logs array is empty', () => {
      render(<ConsolePanel />);
      expect(screen.getByText('No logs yet')).toBeTruthy();
    });

    it('should render console panel with data-testid', () => {
      render(<ConsolePanel />);
      expect(screen.getByTestId('console-panel')).toBeTruthy();
    });
  });

  describe('Header Display (Task 3)', () => {
    it('should display "Console" title in header', () => {
      render(<ConsolePanel />);
      expect(screen.getByText('Console')).toBeTruthy();
    });

    it('should display log count "0 logs" when empty', () => {
      render(<ConsolePanel />);
      expect(screen.getByText('0 logs')).toBeTruthy();
    });

    it('should display correct log count when logs exist', () => {
      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([createTestLog(), createTestLog()])
      );
      render(<ConsolePanel />);
      expect(screen.getByText('2 logs')).toBeTruthy();
    });

    it('should have Clear button', () => {
      render(<ConsolePanel />);
      expect(screen.getByTitle('Clear logs')).toBeTruthy();
    });
  });

  describe('Log Rendering (AC #1, #5)', () => {
    it('should render logs from store', () => {
      const logs = [
        createTestLog({ message: 'First log' }),
        createTestLog({ message: 'Second log' }),
      ];
      vi.mocked(useConsoleStore).mockReturnValue(createMockStore(logs));

      render(<ConsolePanel />);
      expect(screen.getByText('First log')).toBeTruthy();
      expect(screen.getByText('Second log')).toBeTruthy();
    });

    it('should display timestamp in HH:mm:ss.SSS format', () => {
      const logs = [createTestLog()];
      vi.mocked(useConsoleStore).mockReturnValue(createMockStore(logs));

      render(<ConsolePanel />);
      expect(screen.getByText(/\d{2}:\d{2}:\d{2}\.\d{3}/)).toBeTruthy();
    });

    it('should display category badge', () => {
      const logs = [createTestLog({ category: 'ORDER' })];
      vi.mocked(useConsoleStore).mockReturnValue(createMockStore(logs));

      render(<ConsolePanel />);
      expect(screen.getByText('[ORDER]')).toBeTruthy();
    });
  });

  describe('Log Level Colors (AC #2)', () => {
    it('should apply text-wts-muted for INFO level', () => {
      const logs = [createTestLog({ level: 'INFO', message: 'Info message' })];
      vi.mocked(useConsoleStore).mockReturnValue(createMockStore(logs));

      const { container } = render(<ConsolePanel />);
      const logItem = container.querySelector('.text-wts-muted');
      expect(logItem).toBeTruthy();
    });

    it('should apply text-green-500 for SUCCESS level', () => {
      const logs = [
        createTestLog({ level: 'SUCCESS', message: 'Success message' }),
      ];
      vi.mocked(useConsoleStore).mockReturnValue(createMockStore(logs));

      const { container } = render(<ConsolePanel />);
      const logItem = container.querySelector('.text-green-500');
      expect(logItem).toBeTruthy();
    });

    it('should apply text-red-500 for ERROR level', () => {
      const logs = [
        createTestLog({ level: 'ERROR', message: 'Error message' }),
      ];
      vi.mocked(useConsoleStore).mockReturnValue(createMockStore(logs));

      const { container } = render(<ConsolePanel />);
      const logItem = container.querySelector('.text-red-500');
      expect(logItem).toBeTruthy();
    });

    it('should apply text-yellow-500 for WARN level', () => {
      const logs = [createTestLog({ level: 'WARN', message: 'Warn message' })];
      vi.mocked(useConsoleStore).mockReturnValue(createMockStore(logs));

      const { container } = render(<ConsolePanel />);
      const logItem = container.querySelector('.text-yellow-500');
      expect(logItem).toBeTruthy();
    });
  });

  describe('Clear Logs Button (Task 3)', () => {
    it('should call clearLogs when Clear button is clicked', () => {
      render(<ConsolePanel />);

      const clearButton = screen.getByTitle('Clear logs');
      fireEvent.click(clearButton);

      expect(mockClearLogs).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scrollable Log List (AC #4)', () => {
    it('should have overflow-y-auto class for scrollable area', () => {
      render(<ConsolePanel />);

      const consolePanel = screen.getByTestId('console-panel');
      const contentArea = consolePanel.querySelector('.overflow-y-auto');
      expect(contentArea).toBeTruthy();
    });
  });

  describe('Auto Scroll (AC #6)', () => {
    it('should auto-scroll to bottom when new logs are added', () => {
      vi.mocked(useConsoleStore).mockReturnValue(createMockStore([]));
      const { rerender } = render(<ConsolePanel />);

      const consolePanel = screen.getByTestId('console-panel');
      const contentArea = consolePanel.querySelector(
        '.wts-panel-content'
      ) as HTMLDivElement;

      Object.defineProperty(contentArea, 'scrollHeight', {
        value: 1000,
        configurable: true,
      });
      Object.defineProperty(contentArea, 'scrollTop', {
        value: 0,
        writable: true,
      });

      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([createTestLog({ message: 'New log' })])
      );
      rerender(<ConsolePanel />);

      expect(contentArea.scrollTop).toBe(1000);
    });

    it('should not auto-scroll when user scrolled away from bottom', () => {
      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([createTestLog({ message: 'Old log' })])
      );
      const { rerender } = render(<ConsolePanel />);

      const consolePanel = screen.getByTestId('console-panel');
      const contentArea = consolePanel.querySelector(
        '.wts-panel-content'
      ) as HTMLDivElement;

      Object.defineProperty(contentArea, 'scrollHeight', {
        value: 1000,
        configurable: true,
      });
      Object.defineProperty(contentArea, 'clientHeight', {
        value: 100,
        configurable: true,
      });
      Object.defineProperty(contentArea, 'scrollTop', {
        value: 0,
        writable: true,
      });

      fireEvent.scroll(contentArea);

      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([
          createTestLog({ message: 'Old log' }),
          createTestLog({ message: 'Another log' }),
        ])
      );
      rerender(<ConsolePanel />);

      expect(contentArea.scrollTop).toBe(0);
    });

    it('should re-enable auto-scroll after clearing logs', () => {
      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([createTestLog({ message: 'Old log' })])
      );
      const { rerender } = render(<ConsolePanel />);

      const consolePanel = screen.getByTestId('console-panel');
      const contentArea = consolePanel.querySelector(
        '.wts-panel-content'
      ) as HTMLDivElement;

      Object.defineProperty(contentArea, 'scrollHeight', {
        value: 1000,
        configurable: true,
      });
      Object.defineProperty(contentArea, 'clientHeight', {
        value: 100,
        configurable: true,
      });
      Object.defineProperty(contentArea, 'scrollTop', {
        value: 0,
        writable: true,
      });

      fireEvent.scroll(contentArea);
      fireEvent.click(screen.getByTitle('Clear logs'));

      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([createTestLog({ message: 'After clear' })])
      );
      rerender(<ConsolePanel />);

      expect(contentArea.scrollTop).toBe(1000);
    });
  });

  describe('Layout and Styling', () => {
    it('should apply wts-panel class', () => {
      render(<ConsolePanel />);

      const panel = screen.getByTestId('console-panel');
      expect(panel.className).toContain('wts-panel');
    });

    it('should apply wts-area-console class', () => {
      render(<ConsolePanel />);

      const panel = screen.getByTestId('console-panel');
      expect(panel.className).toContain('wts-area-console');
    });

    it('should have flex column layout', () => {
      render(<ConsolePanel />);

      const panel = screen.getByTestId('console-panel');
      expect(panel.className).toContain('flex');
      expect(panel.className).toContain('flex-col');
    });
  });

  describe('Custom className', () => {
    it('should accept and apply custom className', () => {
      render(<ConsolePanel className="custom-class" />);

      const panel = screen.getByTestId('console-panel');
      expect(panel.className).toContain('custom-class');
    });
  });

  describe('WTS-3.6: 자동 스크롤 동작 (AC #7)', () => {
    it('하단에서 50px 이내이면 자동 스크롤이 활성화되어야 한다', () => {
      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([createTestLog({ message: 'test' })])
      );
      const { rerender } = render(<ConsolePanel />);

      const consolePanel = screen.getByTestId('console-panel');
      const contentArea = consolePanel.querySelector(
        '.wts-panel-content'
      ) as HTMLDivElement;

      Object.defineProperty(contentArea, 'scrollHeight', {
        value: 1000,
        configurable: true,
      });
      Object.defineProperty(contentArea, 'clientHeight', {
        value: 200,
        configurable: true,
      });
      // scrollTop을 하단에서 40px 위치로 설정 (1000 - 200 - 40 = 760)
      Object.defineProperty(contentArea, 'scrollTop', {
        value: 760,
        writable: true,
      });

      // 스크롤 이벤트 발생
      fireEvent.scroll(contentArea);

      // 새 로그 추가
      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([
          createTestLog({ message: 'test' }),
          createTestLog({ message: 'new log' }),
        ])
      );
      rerender(<ConsolePanel />);

      // 하단에서 50px 이내이므로 자동 스크롤 활성화됨
      expect(contentArea.scrollTop).toBe(1000);
    });

    it('하단에서 50px 초과 위치이면 자동 스크롤이 비활성화되어야 한다', () => {
      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([createTestLog({ message: 'test' })])
      );
      const { rerender } = render(<ConsolePanel />);

      const consolePanel = screen.getByTestId('console-panel');
      const contentArea = consolePanel.querySelector(
        '.wts-panel-content'
      ) as HTMLDivElement;

      Object.defineProperty(contentArea, 'scrollHeight', {
        value: 1000,
        configurable: true,
      });
      Object.defineProperty(contentArea, 'clientHeight', {
        value: 200,
        configurable: true,
      });
      // scrollTop을 하단에서 100px 위치로 설정 (1000 - 200 - 100 = 700)
      Object.defineProperty(contentArea, 'scrollTop', {
        value: 700,
        writable: true,
      });

      // 스크롤 이벤트 발생
      fireEvent.scroll(contentArea);

      const originalScrollTop = contentArea.scrollTop;

      // 새 로그 추가
      vi.mocked(useConsoleStore).mockReturnValue(
        createMockStore([
          createTestLog({ message: 'test' }),
          createTestLog({ message: 'new log' }),
        ])
      );
      rerender(<ConsolePanel />);

      // 하단에서 50px 초과이므로 자동 스크롤 비활성화
      expect(contentArea.scrollTop).toBe(originalScrollTop);
    });
  });
});

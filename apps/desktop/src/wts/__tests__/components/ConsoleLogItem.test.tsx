import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ConsoleLogItem } from '../../components/ConsoleLogItem';
import type { ConsoleLogEntry } from '../../types';

describe('ConsoleLogItem', () => {
  const createLog = (
    overrides: Partial<ConsoleLogEntry> = {}
  ): ConsoleLogEntry => ({
    id: 'test-id',
    timestamp: new Date('2026-01-19T14:32:15.123').getTime(),
    level: 'INFO',
    category: 'SYSTEM',
    message: 'Test message',
    ...overrides,
  });

  describe('Timestamp Display (AC #1)', () => {
    it('should display timestamp in HH:mm:ss.SSS format', () => {
      const log = createLog();
      render(<ConsoleLogItem log={log} />);

      // Timestamp format pattern
      expect(screen.getByText(/\d{2}:\d{2}:\d{2}\.\d{3}/)).toBeTruthy();
    });
  });

  describe('Log Level Colors (AC #2)', () => {
    it('should apply text-wts-muted for INFO level', () => {
      const log = createLog({ level: 'INFO' });
      const { container } = render(<ConsoleLogItem log={log} />);

      // First child is the wrapper, look inside for the log line
      const logLine = container.querySelector('.text-wts-muted');
      expect(logLine).toBeTruthy();
    });

    it('should apply text-green-500 for SUCCESS level', () => {
      const log = createLog({ level: 'SUCCESS' });
      const { container } = render(<ConsoleLogItem log={log} />);

      const logLine = container.querySelector('.text-green-500');
      expect(logLine).toBeTruthy();
    });

    it('should apply text-red-500 for ERROR level', () => {
      const log = createLog({ level: 'ERROR' });
      const { container } = render(<ConsoleLogItem log={log} />);

      const logLine = container.querySelector('.text-red-500');
      expect(logLine).toBeTruthy();
    });

    it('should apply text-yellow-500 for WARN level', () => {
      const log = createLog({ level: 'WARN' });
      const { container } = render(<ConsoleLogItem log={log} />);

      const logLine = container.querySelector('.text-yellow-500');
      expect(logLine).toBeTruthy();
    });
  });

  describe('Category Badge', () => {
    it('should display category badge', () => {
      const log = createLog({ category: 'ORDER' });
      render(<ConsoleLogItem log={log} />);

      expect(screen.getByText('[ORDER]')).toBeTruthy();
    });

    it('should apply purple style for ORDER category', () => {
      const log = createLog({ category: 'ORDER' });
      render(<ConsoleLogItem log={log} />);

      const badge = screen.getByText('[ORDER]');
      expect(badge.className).toContain('purple');
    });

    it('should apply blue style for BALANCE category', () => {
      const log = createLog({ category: 'BALANCE' });
      render(<ConsoleLogItem log={log} />);

      const badge = screen.getByText('[BALANCE]');
      expect(badge.className).toContain('blue');
    });

    it('should apply cyan style for DEPOSIT category', () => {
      const log = createLog({ category: 'DEPOSIT' });
      render(<ConsoleLogItem log={log} />);

      const badge = screen.getByText('[DEPOSIT]');
      expect(badge.className).toContain('cyan');
    });

    it('should apply orange style for WITHDRAW category', () => {
      const log = createLog({ category: 'WITHDRAW' });
      render(<ConsoleLogItem log={log} />);

      const badge = screen.getByText('[WITHDRAW]');
      expect(badge.className).toContain('orange');
    });

    it('should apply gray style for SYSTEM category', () => {
      const log = createLog({ category: 'SYSTEM' });
      render(<ConsoleLogItem log={log} />);

      const badge = screen.getByText('[SYSTEM]');
      expect(badge.className).toContain('gray');
    });
  });

  describe('Message Display', () => {
    it('should display the log message', () => {
      const log = createLog({ message: 'Order placed successfully' });
      render(<ConsoleLogItem log={log} />);

      expect(screen.getByText('Order placed successfully')).toBeTruthy();
    });

    it('should handle long messages with break-words', () => {
      const log = createLog({
        message: 'A very long message that should wrap properly',
      });
      render(<ConsoleLogItem log={log} />);

      const messageElement = screen.getByText(/A very long message/);
      expect(messageElement.className).toContain('break-words');
    });
  });

  describe('Layout', () => {
    it('should use monospace font', () => {
      const log = createLog();
      const { container } = render(<ConsoleLogItem log={log} />);

      const logLine = container.querySelector('.font-mono');
      expect(logLine).toBeTruthy();
    });

    it('should use extra small text size', () => {
      const log = createLog();
      const { container } = render(<ConsoleLogItem log={log} />);

      const logLine = container.querySelector('.text-xs');
      expect(logLine).toBeTruthy();
    });
  });

  describe('Detail Expand/Collapse (Task 2.3)', () => {
    it('should not show expand button when no detail', () => {
      const log = createLog();
      render(<ConsoleLogItem log={log} />);

      expect(screen.queryByTitle('Show details')).toBeNull();
    });

    it('should show expand button when detail exists', () => {
      const log = createLog({ detail: { error: 'test error' } });
      render(<ConsoleLogItem log={log} />);

      expect(screen.getByTitle('Show details')).toBeTruthy();
    });

    it('should expand detail when button clicked', () => {
      const log = createLog({ detail: { error: 'test error' } });
      render(<ConsoleLogItem log={log} />);

      const expandButton = screen.getByTitle('Show details');
      fireEvent.click(expandButton);

      expect(screen.getByText(/test error/)).toBeTruthy();
    });

    it('should collapse detail when button clicked again', () => {
      const log = createLog({ detail: { error: 'test error' } });
      render(<ConsoleLogItem log={log} />);

      const expandButton = screen.getByTitle('Show details');
      fireEvent.click(expandButton);
      fireEvent.click(screen.getByTitle('Hide details'));

      expect(screen.queryByText(/test error/)).toBeNull();
    });

    it('should format object detail as JSON', () => {
      const log = createLog({ detail: { key: 'value' } });
      render(<ConsoleLogItem log={log} />);

      fireEvent.click(screen.getByTitle('Show details'));

      // JSON should be formatted
      expect(screen.getByText(/"key":/)).toBeTruthy();
    });

    it('should display string detail as-is', () => {
      const log = createLog({ detail: 'Plain error message' });
      render(<ConsoleLogItem log={log} />);

      fireEvent.click(screen.getByTitle('Show details'));

      expect(screen.getByText('Plain error message')).toBeTruthy();
    });
  });
});

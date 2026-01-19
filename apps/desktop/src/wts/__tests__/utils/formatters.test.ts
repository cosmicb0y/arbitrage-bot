import { describe, it, expect } from 'vitest';
import { formatLogTimestamp } from '../../utils/formatters';

describe('formatLogTimestamp', () => {
  it('should format timestamp as HH:mm:ss.SSS', () => {
    // 2026-01-19 14:32:15.123 UTC -> local time depends on timezone
    // Use a known timestamp and verify format pattern
    const timestamp = new Date('2026-01-19T14:32:15.123Z').getTime();
    const result = formatLogTimestamp(timestamp);

    // Verify format: HH:mm:ss.SSS (pattern match)
    expect(result).toMatch(/^\d{2}:\d{2}:\d{2}\.\d{3}$/);
  });

  it('should pad single digits with leading zeros', () => {
    // Create a timestamp at 01:02:03.004
    const date = new Date();
    date.setHours(1, 2, 3, 4);
    const result = formatLogTimestamp(date.getTime());

    expect(result).toBe('01:02:03.004');
  });

  it('should handle midnight correctly', () => {
    const date = new Date();
    date.setHours(0, 0, 0, 0);
    const result = formatLogTimestamp(date.getTime());

    expect(result).toBe('00:00:00.000');
  });

  it('should handle end of day correctly', () => {
    const date = new Date();
    date.setHours(23, 59, 59, 999);
    const result = formatLogTimestamp(date.getTime());

    expect(result).toBe('23:59:59.999');
  });

  it('should preserve milliseconds precision', () => {
    const date = new Date();
    date.setHours(12, 30, 45, 123);
    const result = formatLogTimestamp(date.getTime());

    expect(result).toContain('.123');
  });
});

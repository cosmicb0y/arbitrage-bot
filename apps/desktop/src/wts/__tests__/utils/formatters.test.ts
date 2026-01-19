import { describe, it, expect } from 'vitest';
import {
  formatLogTimestamp,
  formatCrypto,
  formatKrw,
  formatNumber,
} from '../../utils/formatters';

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

describe('formatCrypto', () => {
  it('should format crypto amount with trailing zeros removed', () => {
    expect(formatCrypto(0.5)).toBe('0.5');
    expect(formatCrypto(0.12345678)).toBe('0.12345678');
    expect(formatCrypto(0.1)).toBe('0.1');
    expect(formatCrypto(1.0)).toBe('1');
  });

  it('should handle zero', () => {
    expect(formatCrypto(0)).toBe('0');
  });

  it('should handle NaN and Infinity', () => {
    expect(formatCrypto(NaN)).toBe('0');
    expect(formatCrypto(Infinity)).toBe('0');
    expect(formatCrypto(-Infinity)).toBe('0');
  });

  it('should respect custom decimals parameter', () => {
    expect(formatCrypto(0.123456789, 4)).toBe('0.1235');
    expect(formatCrypto(0.1, 2)).toBe('0.1');
  });
});

describe('formatKrw', () => {
  it('should format KRW amount with ₩ prefix and thousand separators', () => {
    expect(formatKrw(25000000)).toBe('₩25,000,000');
    expect(formatKrw(1000)).toBe('₩1,000');
    expect(formatKrw(100)).toBe('₩100');
  });

  it('should round decimal amounts', () => {
    expect(formatKrw(1234.56)).toBe('₩1,235');
    expect(formatKrw(1234.4)).toBe('₩1,234');
  });

  it('should handle zero', () => {
    expect(formatKrw(0)).toBe('₩0');
  });

  it('should handle NaN and Infinity', () => {
    expect(formatKrw(NaN)).toBe('₩0');
    expect(formatKrw(Infinity)).toBe('₩0');
  });
});

describe('formatNumber', () => {
  it('should format number with thousand separators', () => {
    expect(formatNumber(1000000)).toBe('1,000,000');
    expect(formatNumber(1234)).toBe('1,234');
    expect(formatNumber(100)).toBe('100');
  });

  it('should round decimal amounts', () => {
    expect(formatNumber(1234.56)).toBe('1,235');
    expect(formatNumber(1234.4)).toBe('1,234');
  });

  it('should handle zero', () => {
    expect(formatNumber(0)).toBe('0');
  });

  it('should handle NaN and Infinity', () => {
    expect(formatNumber(NaN)).toBe('0');
    expect(formatNumber(Infinity)).toBe('0');
  });
});

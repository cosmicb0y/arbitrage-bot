import { describe, it, expect } from 'vitest';
import {
  formatLogTimestamp,
  formatCrypto,
  formatKrw,
  formatNumber,
  sanitizeLogDetail,
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

describe('sanitizeLogDetail (WTS-3.6 AC #5)', () => {
  describe('민감 정보 마스킹', () => {
    it('access_key를 마스킹해야 한다', () => {
      const input = { access_key: 'my-secret-key', uuid: '123' };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;

      expect(result.access_key).toBe('[MASKED]');
      expect(result.uuid).toBe('123');
    });

    it('secret_key를 마스킹해야 한다', () => {
      const input = { secret_key: 'super-secret', data: 'normal' };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;

      expect(result.secret_key).toBe('[MASKED]');
      expect(result.data).toBe('normal');
    });

    it('api_key를 마스킹해야 한다', () => {
      const input = { api_key: 'api123', status: 'ok' };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;

      expect(result.api_key).toBe('[MASKED]');
    });

    it('apikey (소문자)를 마스킹해야 한다', () => {
      const input = { apikey: 'api123' };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;

      expect(result.apikey).toBe('[MASKED]');
    });

    it('authorization을 마스킹해야 한다', () => {
      const input = { authorization: 'Bearer token123' };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;

      expect(result.authorization).toBe('[MASKED]');
    });

    it('password를 마스킹해야 한다', () => {
      const input = { password: 'mypassword' };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;

      expect(result.password).toBe('[MASKED]');
    });

    it('token을 마스킹해야 한다', () => {
      const input = { token: 'jwt-token', refresh_token: 'refresh' };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;

      expect(result.token).toBe('[MASKED]');
      expect(result.refresh_token).toBe('[MASKED]');
    });

    it('대소문자 무시하고 마스킹해야 한다', () => {
      const input = {
        ACCESS_KEY: 'key1',
        Secret_Key: 'key2',
        ApiKey: 'key3',
      };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;

      expect(result.ACCESS_KEY).toBe('[MASKED]');
      expect(result.Secret_Key).toBe('[MASKED]');
      expect(result.ApiKey).toBe('[MASKED]');
    });
  });

  describe('중첩 객체 마스킹', () => {
    it('중첩된 객체의 민감 정보도 마스킹해야 한다', () => {
      const input = {
        config: {
          api_key: 'nested-key',
          endpoint: 'https://api.example.com',
        },
        status: 'ok',
      };
      const result = sanitizeLogDetail(input) as Record<string, unknown>;
      const config = result.config as Record<string, unknown>;

      expect(config.api_key).toBe('[MASKED]');
      expect(config.endpoint).toBe('https://api.example.com');
      expect(result.status).toBe('ok');
    });

    it('배열 내 객체의 민감 정보도 마스킹해야 한다', () => {
      const input = [
        { token: 'token1', id: 1 },
        { token: 'token2', id: 2 },
      ];
      const result = sanitizeLogDetail(input) as Array<Record<string, unknown>>;

      expect(result[0].token).toBe('[MASKED]');
      expect(result[0].id).toBe(1);
      expect(result[1].token).toBe('[MASKED]');
      expect(result[1].id).toBe(2);
    });
  });

  describe('원시 타입 처리', () => {
    it('null을 그대로 반환해야 한다', () => {
      expect(sanitizeLogDetail(null)).toBeNull();
    });

    it('undefined를 그대로 반환해야 한다', () => {
      expect(sanitizeLogDetail(undefined)).toBeUndefined();
    });

    it('문자열을 그대로 반환해야 한다', () => {
      expect(sanitizeLogDetail('test string')).toBe('test string');
    });

    it('숫자를 그대로 반환해야 한다', () => {
      expect(sanitizeLogDetail(12345)).toBe(12345);
    });

    it('boolean을 그대로 반환해야 한다', () => {
      expect(sanitizeLogDetail(true)).toBe(true);
      expect(sanitizeLogDetail(false)).toBe(false);
    });
  });

  describe('일반 객체 필드 유지', () => {
    it('민감하지 않은 필드는 그대로 유지해야 한다', () => {
      const input = {
        uuid: 'order-123',
        side: 'bid',
        price: '50000000',
        volume: '0.001',
        market: 'KRW-BTC',
        state: 'done',
      };
      const result = sanitizeLogDetail(input);

      expect(result).toEqual(input);
    });
  });
});

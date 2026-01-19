import { describe, it, expect } from 'vitest';
import { LOG_LEVEL_STYLES, LOG_CATEGORY_STYLES } from '../../utils/consoleStyles';

describe('LOG_LEVEL_STYLES', () => {
  it('should have INFO styled with muted text', () => {
    expect(LOG_LEVEL_STYLES.INFO).toBe('text-wts-muted');
  });

  it('should have SUCCESS styled with green-500', () => {
    expect(LOG_LEVEL_STYLES.SUCCESS).toBe('text-green-500');
  });

  it('should have ERROR styled with red-500', () => {
    expect(LOG_LEVEL_STYLES.ERROR).toBe('text-red-500');
  });

  it('should have WARN styled with yellow-500', () => {
    expect(LOG_LEVEL_STYLES.WARN).toBe('text-yellow-500');
  });

  it('should have all four log levels defined', () => {
    const levels = Object.keys(LOG_LEVEL_STYLES);
    expect(levels).toEqual(['INFO', 'SUCCESS', 'ERROR', 'WARN']);
  });
});

describe('LOG_CATEGORY_STYLES', () => {
  it('should have ORDER styled with purple', () => {
    expect(LOG_CATEGORY_STYLES.ORDER).toContain('purple');
  });

  it('should have BALANCE styled with blue', () => {
    expect(LOG_CATEGORY_STYLES.BALANCE).toContain('blue');
  });

  it('should have DEPOSIT styled with cyan', () => {
    expect(LOG_CATEGORY_STYLES.DEPOSIT).toContain('cyan');
  });

  it('should have WITHDRAW styled with orange', () => {
    expect(LOG_CATEGORY_STYLES.WITHDRAW).toContain('orange');
  });

  it('should have SYSTEM styled with gray', () => {
    expect(LOG_CATEGORY_STYLES.SYSTEM).toContain('gray');
  });

  it('should have all five categories defined', () => {
    const categories = Object.keys(LOG_CATEGORY_STYLES);
    expect(categories).toEqual(['ORDER', 'BALANCE', 'DEPOSIT', 'WITHDRAW', 'SYSTEM']);
  });
});

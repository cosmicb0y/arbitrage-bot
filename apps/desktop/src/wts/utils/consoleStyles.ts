import type { LogLevel, LogCategory } from '../types';

/** 로그 레벨별 텍스트 색상 스타일 */
export const LOG_LEVEL_STYLES: Record<LogLevel, string> = {
  INFO: 'text-wts-muted',
  SUCCESS: 'text-green-500',
  ERROR: 'text-red-500',
  WARN: 'text-yellow-500',
};

/** 로그 카테고리별 배지 스타일 */
export const LOG_CATEGORY_STYLES: Record<LogCategory, string> = {
  ORDER: 'bg-purple-500/20 text-purple-400',
  BALANCE: 'bg-blue-500/20 text-blue-400',
  DEPOSIT: 'bg-cyan-500/20 text-cyan-400',
  WITHDRAW: 'bg-orange-500/20 text-orange-400',
  SYSTEM: 'bg-gray-500/20 text-gray-400',
};

/**
 * WTS 에러 처리 유틸리티
 * 콘솔 로깅 + 토스트 알림 통합 처리
 */

import { useConsoleStore } from '../stores/consoleStore';
import { useToastStore } from '../stores/toastStore';
import {
  getOrderErrorMessage,
  isRateLimitError,
  isNetworkError,
  isWithdrawActionRequiredError,
  WITHDRAW_ERROR_GUIDANCE,
} from '../types';
import type { LogCategory, WtsApiErrorResponse } from '../types';

function isNetworkMessage(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    lower.includes('network') ||
    lower.includes('timeout') ||
    lower.includes('timed out') ||
    lower.includes('connection') ||
    lower.includes('econn') ||
    message.includes('네트워크')
  );
}

function extractRemainingReq(detail?: Record<string, unknown>): string | undefined {
  if (!detail) return undefined;
  const remainingReq = detail['remaining_req'];
  return typeof remainingReq === 'string' ? remainingReq : undefined;
}

function getRateLimitMessage(category: LogCategory): string {
  switch (category) {
    case 'ORDER':
      return '주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.';
    case 'WITHDRAW':
      return '출금 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.';
    case 'DEPOSIT':
      return '입금 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.';
    case 'BALANCE':
      return '잔고 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.';
    default:
      return '요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.';
  }
}

function getRateLimitInfoMessage(category: LogCategory): string {
  if (category === 'ORDER') {
    return '주문 제한: 초당 8회. 잠시 후 다시 시도하세요.';
  }
  return '요청 제한으로 잠시 후 다시 시도하세요.';
}

/**
 * WTS API 에러 응답인지 확인
 */
export function isWtsApiError(error: unknown): error is WtsApiErrorResponse {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof (error as Record<string, unknown>).code === 'string'
  );
}

/**
 * API 에러 통합 처리
 * - 콘솔 ERROR 로깅
 * - 토스트 알림 표시
 * - Rate Limit 시 재시도 안내
 *
 * @param error 에러 객체 또는 WtsApiError 응답
 * @param category 로그 카테고리
 * @param context 에러 컨텍스트 (선택)
 */
export function handleApiError(
  error: unknown,
  category: LogCategory,
  context?: string
): void {
  if (category === 'WITHDRAW') {
    handleWithdrawError(error, context);
    return;
  }

  const addLog = useConsoleStore.getState().addLog;
  const showToast = useToastStore.getState().showToast;

  let errorCode = 'unknown';
  let errorMessage = '알 수 없는 오류가 발생했습니다';
  let logDetail: unknown = error;

  if (isWtsApiError(error)) {
    errorCode = error.code;
    errorMessage = getOrderErrorMessage(error.code, error.message);
    logDetail = error.detail ? { ...error.detail, code: error.code, message: error.message } : error;
  } else if (error instanceof Error) {
    logDetail = { name: error.name, message: error.message };
    if (isNetworkMessage(error.message)) {
      errorCode = 'network_error';
      errorMessage = getOrderErrorMessage(errorCode, error.message);
    } else {
      errorMessage = error.message;
    }
  } else if (typeof error === 'string') {
    logDetail = error;
    if (isNetworkMessage(error)) {
      errorCode = 'network_error';
      errorMessage = getOrderErrorMessage(errorCode, error);
    } else {
      errorMessage = error;
    }
  }

  if (isRateLimitError(errorCode) && category !== 'ORDER') {
    errorMessage = getRateLimitMessage(category);
  }

  const logMessage = context ? `${context}: ${errorMessage}` : errorMessage;

  // 에러 로깅
  addLog('ERROR', category, logMessage, logDetail);

  // 토스트 알림
  showToast('error', errorMessage);

  // Rate Limit 특별 안내
  if (isRateLimitError(errorCode)) {
    if (isWtsApiError(error)) {
      const remainingReq = extractRemainingReq(error.detail);
      if (remainingReq) {
        addLog('INFO', category, `Remaining-Req: ${remainingReq}`);
      }
    }
    addLog('INFO', category, getRateLimitInfoMessage(category));
  }

  // 네트워크 에러 시 추가 안내
  if (isNetworkError(errorCode)) {
    addLog('INFO', category, '네트워크 연결을 확인하고 다시 시도하세요.');
  }
}

/**
 * 에러 코드에서 상세 정보 추출
 */
export function getErrorDetails(errorCode: string): {
  isRateLimit: boolean;
  isNetwork: boolean;
  isAuth: boolean;
  isOrder: boolean;
} {
  const authCodes = ['missing_api_key', 'jwt_error', 'jwt_verification', 'no_authorization_ip', 'expired_access_key'];
  const orderCodes = ['insufficient_funds_bid', 'insufficient_funds_ask', 'under_min_total_bid', 'under_min_total_ask', 'invalid_volume', 'invalid_price', 'market_does_not_exist', 'invalid_side', 'invalid_ord_type'];

  return {
    isRateLimit: isRateLimitError(errorCode),
    isNetwork: isNetworkError(errorCode),
    isAuth: authCodes.includes(errorCode),
    isOrder: orderCodes.includes(errorCode),
  };
}

/**
 * 출금 에러 전용 처리 (WTS-5.5)
 * - 액션 필요 에러 (2FA, 미등록 주소): WARN 레벨 + 추가 안내
 * - 한도 에러 (over_daily_limit): WARN 레벨 + 추가 안내
 * - 기타 에러: ERROR 레벨
 *
 * @param error 에러 객체 또는 WtsApiError 응답
 * @param context 에러 컨텍스트 (선택)
 */
export function handleWithdrawError(
  error: unknown,
  context?: string
): void {
  const addLog = useConsoleStore.getState().addLog;
  const showToast = useToastStore.getState().showToast;

  let errorCode = 'unknown';
  let errorMessage = '알 수 없는 오류가 발생했습니다';
  let logDetail: unknown = error;

  if (isWtsApiError(error)) {
    errorCode = error.code;
    errorMessage = getOrderErrorMessage(error.code, error.message);
    logDetail = error.detail ? { ...error.detail, code: error.code, message: error.message } : error;
  } else if (error instanceof Error) {
    errorMessage = error.message;
    logDetail = { name: error.name, message: error.message };
  } else if (typeof error === 'string') {
    errorMessage = error;
    logDetail = error;
  }

  if (isRateLimitError(errorCode)) {
    errorMessage = getRateLimitMessage('WITHDRAW');
  }

  const logMessage = context ? `${context}: ${errorMessage}` : errorMessage;

  // 액션 필요 에러 (2FA, 미등록 주소): WARN 레벨
  if (isWithdrawActionRequiredError(errorCode)) {
    addLog('WARN', 'WITHDRAW', logMessage, logDetail);
    showToast('warning', errorMessage);

    // 추가 안내 메시지
    const guidance = WITHDRAW_ERROR_GUIDANCE[errorCode];
    if (guidance) {
      addLog('INFO', 'WITHDRAW', guidance);
    }
    return;
  }

  // 한도 에러: over_daily_limit은 WARN 레벨
  if (errorCode === 'over_daily_limit') {
    addLog('WARN', 'WITHDRAW', logMessage, logDetail);
    showToast('warning', errorMessage);

    const guidance = WITHDRAW_ERROR_GUIDANCE[errorCode];
    if (guidance) {
      addLog('INFO', 'WITHDRAW', guidance);
    }
    return;
  }

  // 기타 에러: ERROR 레벨
  addLog('ERROR', 'WITHDRAW', logMessage, logDetail);
  showToast('error', errorMessage);

  // Rate Limit 처리
  if (isRateLimitError(errorCode)) {
    if (isWtsApiError(error)) {
      const remainingReq = extractRemainingReq(error.detail);
      if (remainingReq) {
        addLog('INFO', 'WITHDRAW', `Remaining-Req: ${remainingReq}`);
      }
    }
    addLog('INFO', 'WITHDRAW', '요청 제한으로 잠시 후 다시 시도하세요.');
  }

  // 네트워크 에러 처리
  if (isNetworkError(errorCode)) {
    addLog('INFO', 'WITHDRAW', '네트워크 연결을 확인하고 다시 시도하세요.');
  }
}

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { isWtsApiError, handleApiError, getErrorDetails, handleWithdrawError } from '../../utils/errorHandler';
import { useConsoleStore } from '../../stores/consoleStore';
import { useToastStore } from '../../stores/toastStore';

// Mock stores
vi.mock('../../stores/consoleStore');
vi.mock('../../stores/toastStore');

describe('errorHandler', () => {
  const mockAddLog = vi.fn();
  const mockShowToast = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(useConsoleStore.getState).mockReturnValue({
      logs: [],
      addLog: mockAddLog,
      clearLogs: vi.fn(),
    });

    vi.mocked(useToastStore.getState).mockReturnValue({
      toasts: [],
      showToast: mockShowToast,
      removeToast: vi.fn(),
      clearToasts: vi.fn(),
    });
  });

  describe('isWtsApiError', () => {
    it('WtsApiErrorResponse 객체를 인식한다', () => {
      expect(isWtsApiError({ code: 'rate_limit', message: 'Too many requests' })).toBe(true);
    });

    it('code 속성이 없으면 false를 반환한다', () => {
      expect(isWtsApiError({ message: 'error' })).toBe(false);
    });

    it('null을 false로 처리한다', () => {
      expect(isWtsApiError(null)).toBe(false);
    });

    it('undefined를 false로 처리한다', () => {
      expect(isWtsApiError(undefined)).toBe(false);
    });

    it('문자열을 false로 처리한다', () => {
      expect(isWtsApiError('error')).toBe(false);
    });
  });

  describe('handleApiError', () => {
    it('WtsApiError에서 ERROR 로그와 토스트를 표시한다', () => {
      const error = { code: 'insufficient_funds_bid', message: 'Not enough balance' };

      handleApiError(error, 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'ORDER',
        '매수 가능 금액이 부족합니다',
        expect.anything()
      );
      expect(mockShowToast).toHaveBeenCalledWith('error', '매수 가능 금액이 부족합니다');
    });

    it('Error 객체에서 메시지를 추출한다', () => {
      const error = new Error('Unexpected failure');

      handleApiError(error, 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith('ERROR', 'ORDER', 'Unexpected failure', expect.anything());
      expect(mockShowToast).toHaveBeenCalledWith('error', 'Unexpected failure');
    });

    it('네트워크 오류 메시지를 네트워크 에러로 분류한다', () => {
      const error = new Error('Network failed');

      handleApiError(error, 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'ORDER',
        '네트워크 연결을 확인하세요',
        expect.anything()
      );
      expect(mockShowToast).toHaveBeenCalledWith('error', '네트워크 연결을 확인하세요');
    });

    it('문자열 에러를 처리한다', () => {
      handleApiError('Something went wrong', 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'ORDER',
        'Something went wrong',
        expect.anything()
      );
      expect(mockShowToast).toHaveBeenCalledWith('error', 'Something went wrong');
    });

    it('컨텍스트가 있으면 로그 메시지에 포함한다', () => {
      const error = { code: 'network_error', message: 'Connection failed' };

      handleApiError(error, 'ORDER', '주문 실패');

      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'ORDER',
        '주문 실패: 네트워크 연결을 확인하세요',
        expect.anything()
      );
    });

    it('Rate Limit 에러 시 재시도 안내 INFO 로그를 추가한다', () => {
      const error = { code: 'rate_limit', message: 'Too many requests' };

      handleApiError(error, 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'ORDER',
        expect.stringContaining('주문 요청'),
        expect.anything()
      );
      expect(mockAddLog).toHaveBeenCalledWith('INFO', 'ORDER', expect.stringContaining('초당 8회'));
    });

    it('too_many_requests 에러도 Rate Limit으로 처리한다', () => {
      const error = { code: 'too_many_requests', message: 'Rate limited' };

      handleApiError(error, 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith('INFO', 'ORDER', expect.stringContaining('초당 8회'));
    });

    it('WITHDRAW Rate Limit 에러는 출금 메시지를 사용한다', () => {
      const error = { code: 'rate_limit', message: 'Too many requests' };

      handleApiError(error, 'WITHDRAW');

      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'WITHDRAW',
        expect.stringContaining('출금 요청이 너무 빠릅니다'),
        expect.anything()
      );
      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'WITHDRAW',
        expect.stringContaining('요청 제한')
      );
    });

    it('네트워크 에러 시 네트워크 확인 안내를 추가한다', () => {
      const error = { code: 'network_error', message: 'Connection failed' };

      handleApiError(error, 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith('INFO', 'ORDER', expect.stringContaining('네트워크 연결'));
    });

    it('timeout_error도 네트워크 에러로 처리한다', () => {
      const error = { code: 'timeout_error', message: 'Request timed out' };

      handleApiError(error, 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith('INFO', 'ORDER', expect.stringContaining('네트워크 연결'));
    });

    it('Rate Limit 에러에 Remaining-Req가 있으면 추가 로그를 남긴다', () => {
      const error = {
        code: 'rate_limit',
        message: 'Too many requests',
        detail: { remaining_req: 'group=order; sec=2' },
      };

      handleApiError(error, 'ORDER');

      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'ORDER',
        expect.stringContaining('Remaining-Req: group=order; sec=2')
      );
    });
  });

  describe('getErrorDetails', () => {
    it('rate_limit 에러를 올바르게 분류한다', () => {
      const details = getErrorDetails('rate_limit');
      expect(details.isRateLimit).toBe(true);
      expect(details.isNetwork).toBe(false);
      expect(details.isAuth).toBe(false);
      expect(details.isOrder).toBe(false);
    });

    it('network_error 에러를 올바르게 분류한다', () => {
      const details = getErrorDetails('network_error');
      expect(details.isRateLimit).toBe(false);
      expect(details.isNetwork).toBe(true);
      expect(details.isAuth).toBe(false);
      expect(details.isOrder).toBe(false);
    });

    it('jwt_verification 에러를 인증 에러로 분류한다', () => {
      const details = getErrorDetails('jwt_verification');
      expect(details.isRateLimit).toBe(false);
      expect(details.isNetwork).toBe(false);
      expect(details.isAuth).toBe(true);
      expect(details.isOrder).toBe(false);
    });

    it('insufficient_funds_bid 에러를 주문 에러로 분류한다', () => {
      const details = getErrorDetails('insufficient_funds_bid');
      expect(details.isRateLimit).toBe(false);
      expect(details.isNetwork).toBe(false);
      expect(details.isAuth).toBe(false);
      expect(details.isOrder).toBe(true);
    });

    it('알 수 없는 에러 코드는 모두 false로 분류한다', () => {
      const details = getErrorDetails('unknown_error');
      expect(details.isRateLimit).toBe(false);
      expect(details.isNetwork).toBe(false);
      expect(details.isAuth).toBe(false);
      expect(details.isOrder).toBe(false);
    });
  });

  describe('handleWithdrawError (WTS-5.5)', () => {
    it('2FA 에러를 WARN 레벨로 기록한다', () => {
      const error = { code: 'two_factor_auth_required', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'WARN',
        'WITHDRAW',
        expect.stringContaining('2FA'),
        expect.anything()
      );
    });

    it('2FA 에러 시 warning 토스트를 표시한다', () => {
      const error = { code: 'two_factor_auth_required', message: 'test' };

      handleWithdrawError(error);

      expect(mockShowToast).toHaveBeenCalledWith('warning', expect.stringContaining('2FA'));
    });

    it('2FA 에러 시 추가 안내 메시지를 INFO 레벨로 기록한다', () => {
      const error = { code: 'two_factor_auth_required', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'WITHDRAW',
        expect.stringContaining('Upbit 모바일 앱')
      );
    });

    it('미등록 주소 에러를 WARN 레벨로 기록한다', () => {
      const error = { code: 'unregistered_withdraw_address', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'WARN',
        'WITHDRAW',
        expect.stringContaining('출금 주소'),
        expect.anything()
      );
    });

    it('미등록 주소 에러 시 등록 안내 URL을 INFO 레벨로 기록한다', () => {
      const error = { code: 'unregistered_withdraw_address', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'WITHDRAW',
        expect.stringContaining('upbit.com')
      );
    });

    it('withdraw_address_not_registered 에러를 WARN 레벨로 기록한다', () => {
      const error = { code: 'withdraw_address_not_registered', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'WARN',
        'WITHDRAW',
        expect.stringContaining('출금 주소'),
        expect.anything()
      );
    });

    it('withdraw_address_not_registered 에러 시 등록 안내 URL을 INFO 레벨로 기록한다', () => {
      const error = { code: 'withdraw_address_not_registered', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'WITHDRAW',
        expect.stringContaining('upbit.com')
      );
    });

    it('over_daily_limit 에러를 WARN 레벨로 기록한다', () => {
      const error = { code: 'over_daily_limit', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'WARN',
        'WITHDRAW',
        expect.stringContaining('한도'),
        expect.anything()
      );
    });

    it('over_daily_limit 에러 시 초기화 시간 안내를 INFO 레벨로 기록한다', () => {
      const error = { code: 'over_daily_limit', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'INFO',
        'WITHDRAW',
        expect.stringContaining('00:00')
      );
    });

    it('under_min_amount 에러를 ERROR 레벨로 기록한다', () => {
      const error = { code: 'under_min_amount', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'WITHDRAW',
        expect.anything(),
        expect.anything()
      );
    });

    it('under_min_amount 에러 시 error 토스트를 표시한다', () => {
      const error = { code: 'under_min_amount', message: 'test' };

      handleWithdrawError(error);

      expect(mockShowToast).toHaveBeenCalledWith('error', expect.anything());
    });

    it('알 수 없는 에러 코드는 ERROR 레벨로 기록한다', () => {
      const error = { code: 'unknown_withdraw_error', message: 'test' };

      handleWithdrawError(error);

      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'WITHDRAW',
        expect.anything(),
        expect.anything()
      );
    });

    it('컨텍스트가 있으면 로그 메시지에 포함한다', () => {
      const error = { code: 'two_factor_auth_required', message: 'test' };

      handleWithdrawError(error, '출금 요청 실패');

      expect(mockAddLog).toHaveBeenCalledWith(
        'WARN',
        'WITHDRAW',
        expect.stringContaining('출금 요청 실패'),
        expect.anything()
      );
    });
  });
});

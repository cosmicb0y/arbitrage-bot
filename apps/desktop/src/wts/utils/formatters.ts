/**
 * Unix timestamp를 HH:mm:ss.SSS 형식으로 변환
 * @param timestamp Unix timestamp (ms)
 * @returns HH:mm:ss.SSS 형식 문자열
 */
export function formatLogTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  const ms = date.getMilliseconds().toString().padStart(3, '0');
  return `${hours}:${minutes}:${seconds}.${ms}`;
}

/**
 * 암호화폐 수량 포맷 (소수점 이하 trailing zero 제거)
 * @param amount 수량
 * @param decimals 최대 소수점 자릿수 (기본 8)
 * @returns 포맷된 문자열
 */
export function formatCrypto(amount: number, decimals = 8): string {
  if (isNaN(amount) || !isFinite(amount)) return '0';
  // trailing zero 제거
  return amount.toFixed(decimals).replace(/\.?0+$/, '');
}

/**
 * KRW 금액 포맷 (천 단위 구분자 + ₩ 기호)
 * @param amount 금액
 * @returns 포맷된 문자열 (예: ₩25,000,000)
 */
export function formatKrw(amount: number): string {
  if (isNaN(amount) || !isFinite(amount)) return '₩0';
  return `₩${Math.round(amount).toLocaleString('ko-KR')}`;
}

/**
 * 숫자 포맷 (천 단위 구분자)
 * @param amount 금액
 * @returns 포맷된 문자열 (예: 1,000,000)
 */
export function formatNumber(amount: number): string {
  if (isNaN(amount) || !isFinite(amount)) return '0';
  return Math.round(amount).toLocaleString('ko-KR');
}

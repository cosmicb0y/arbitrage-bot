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

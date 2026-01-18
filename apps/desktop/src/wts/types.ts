/**
 * WTS (Web Trading System) TypeScript Types
 * Bloomberg Terminal 스타일 트레이딩 시스템 전용 타입 정의
 */

// ============================================================================
// Exchange Types
// ============================================================================

/** 지원 거래소 (MVP: Upbit) */
export type Exchange = 'upbit';

/** 연결 상태 */
export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected';

// ============================================================================
// Console Log Types
// ============================================================================

/** 로그 레벨 */
export type LogLevel = 'INFO' | 'SUCCESS' | 'ERROR' | 'WARN';

/** 로그 카테고리 */
export type LogCategory = 'ORDER' | 'BALANCE' | 'DEPOSIT' | 'WITHDRAW' | 'SYSTEM';

/** 콘솔 로그 엔트리 */
export interface ConsoleLogEntry {
  /** 고유 ID */
  id: string;
  /** Unix timestamp (ms) */
  timestamp: number;
  /** 로그 레벨 */
  level: LogLevel;
  /** 로그 카테고리 */
  category: LogCategory;
  /** 사용자 친화적 메시지 */
  message: string;
  /** API 응답 원본 (디버깅용) */
  detail?: unknown;
}

// ============================================================================
// WTS State Types
// ============================================================================

/** WTS 메인 상태 */
export interface WtsState {
  /** 선택된 거래소 */
  selectedExchange: Exchange;
  /** 거래소 선택 */
  setExchange: (exchange: Exchange) => void;

  /** 선택된 마켓 (예: 'KRW-BTC') */
  selectedMarket: string | null;
  /** 마켓 선택 */
  setMarket: (market: string | null) => void;

  /** 연결 상태 */
  connectionStatus: ConnectionStatus;
  /** 연결 상태 설정 */
  setConnectionStatus: (status: ConnectionStatus) => void;
}

/** 콘솔 상태 */
export interface ConsoleState {
  /** 로그 목록 */
  logs: ConsoleLogEntry[];
  /** 로그 추가 */
  addLog: (
    level: LogLevel,
    category: LogCategory,
    message: string,
    detail?: unknown
  ) => void;
  /** 로그 초기화 */
  clearLogs: () => void;
}

// ============================================================================
// Order Form Types (Future Use)
// ============================================================================

/** 주문 유형 */
export type OrderType = 'market' | 'limit';

/** 주문 방향 */
export type OrderSide = 'buy' | 'sell';

/** 주문 폼 상태 */
export interface OrderFormState {
  /** 주문 유형 */
  orderType: OrderType;
  /** 주문 방향 */
  side: OrderSide;
  /** 수량 */
  quantity: string;
  /** 가격 (지정가 주문 시) */
  price: string;
}

// ============================================================================
// Constants
// ============================================================================

/** 콘솔 최대 로그 수 (FIFO) */
export const MAX_CONSOLE_LOGS = 1000;

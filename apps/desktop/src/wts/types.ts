/**
 * WTS (Web Trading System) TypeScript Types
 * Bloomberg Terminal 스타일 트레이딩 시스템 전용 타입 정의
 */

// ============================================================================
// Exchange Types
// ============================================================================

/** 지원 거래소 (전체) */
export type Exchange =
  | 'upbit'
  | 'bithumb'
  | 'binance'
  | 'coinbase'
  | 'bybit'
  | 'gateio';

/** 활성화된 거래소 (MVP: Upbit만) */
export const ENABLED_EXCHANGES: readonly Exchange[] = ['upbit'] as const;

/** 거래소 메타데이터 */
export interface ExchangeMeta {
  name: string;
  shortKey: string;
  keyboardShortcut: number; // 1-6
}

/** 거래소 메타데이터 맵 */
export const EXCHANGE_META: Record<Exchange, ExchangeMeta> = {
  upbit: { name: 'Upbit', shortKey: 'UP', keyboardShortcut: 1 },
  bithumb: { name: 'Bithumb', shortKey: 'BT', keyboardShortcut: 2 },
  binance: { name: 'Binance', shortKey: 'BN', keyboardShortcut: 3 },
  coinbase: { name: 'Coinbase', shortKey: 'CB', keyboardShortcut: 4 },
  bybit: { name: 'Bybit', shortKey: 'BY', keyboardShortcut: 5 },
  gateio: { name: 'GateIO', shortKey: 'GT', keyboardShortcut: 6 },
};

/** 거래소 순서 (탭 표시 순서) */
export const EXCHANGE_ORDER: readonly Exchange[] = [
  'upbit',
  'bithumb',
  'binance',
  'coinbase',
  'bybit',
  'gateio',
] as const;

/** 연결 상태 */
export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected';

/** 연결 체크 결과 (Tauri 명령 응답) */
export interface ConnectionCheckResult {
  success: boolean;
  latency?: number; // ms
  error?: string;
}

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
  enabledExchanges: readonly Exchange[];
  setEnabledExchanges: (exchanges: readonly Exchange[]) => void;

  /** 선택된 거래소 */
  selectedExchange: Exchange;
  /** 거래소 선택 */
  setExchange: (exchange: Exchange) => void;

  /** 선택된 마켓 (예: 'KRW-BTC') */
  selectedMarket: string | null;
  /** 마켓 선택 */
  setMarket: (market: string | null) => void;

  /** 사용 가능한 마켓 목록 */
  availableMarkets: readonly Market[];
  /** 마켓 목록 설정 */
  setAvailableMarkets: (markets: readonly Market[]) => void;

  /** 연결 상태 */
  connectionStatus: ConnectionStatus;
  /** 연결 상태 설정 */
  setConnectionStatus: (status: ConnectionStatus) => void;

  /** 마지막 연결 에러 메시지 */
  lastConnectionError: string | null;
  /** 연결 에러 설정 */
  setConnectionError: (error: string | null) => void;
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
// Balance Types (Upbit API)
// ============================================================================

/** 잔고 엔트리 (Upbit API 응답) */
export interface BalanceEntry {
  /** 화폐 코드 (예: "BTC", "KRW") */
  currency: string;
  /** 가용 잔고 */
  balance: string;
  /** 잠금 잔고 (미체결 주문) */
  locked: string;
  /** 평균 매수가 */
  avg_buy_price: string;
  /** 평균 매수가 수정 여부 */
  avg_buy_price_modified: boolean;
  /** 평가 기준 화폐 (예: "KRW") */
  unit_currency: string;
}

/** WTS API 에러 응답 */
export interface WtsApiErrorResponse {
  code: string;
  message: string;
}

/** WTS API 응답 래퍼 */
export interface WtsApiResult<T> {
  success: boolean;
  data?: T;
  error?: WtsApiErrorResponse;
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
// Market Types
// ============================================================================

/** 마켓 코드 (예: "KRW-BTC") */
export type MarketCode = `${string}-${string}`;

/** 마켓 정보 */
export interface Market {
  /** 마켓 코드 (예: "KRW-BTC") */
  code: MarketCode;
  /** 기준 화폐 (예: "BTC") */
  base: string;
  /** 결제 화폐 (예: "KRW") */
  quote: string;
  /** 표시명 (예: "비트코인") */
  displayName?: string;
}

/** Upbit MVP 마켓 목록 */
export const UPBIT_DEFAULT_MARKETS: readonly Market[] = [
  { code: 'KRW-BTC', base: 'BTC', quote: 'KRW', displayName: '비트코인' },
  { code: 'KRW-ETH', base: 'ETH', quote: 'KRW', displayName: '이더리움' },
  { code: 'KRW-XRP', base: 'XRP', quote: 'KRW', displayName: '리플' },
  { code: 'KRW-SOL', base: 'SOL', quote: 'KRW', displayName: '솔라나' },
  { code: 'KRW-DOGE', base: 'DOGE', quote: 'KRW', displayName: '도지코인' },
  { code: 'KRW-ADA', base: 'ADA', quote: 'KRW', displayName: '에이다' },
  { code: 'KRW-AVAX', base: 'AVAX', quote: 'KRW', displayName: '아발란체' },
  { code: 'KRW-DOT', base: 'DOT', quote: 'KRW', displayName: '폴카닷' },
] as const;

// ============================================================================
// Constants
// ============================================================================

/** 콘솔 최대 로그 수 (FIFO) */
export const MAX_CONSOLE_LOGS = 1000;

// ============================================================================
// Orderbook Types
// ============================================================================

/** 오더북 호가 엔트리 */
export interface OrderbookEntry {
  /** 가격 */
  price: number;
  /** 수량 */
  size: number;
}

/** 오더북 상태 */
export interface OrderbookData {
  /** 매도 호가 (가격 오름차순) */
  asks: OrderbookEntry[];
  /** 매수 호가 (가격 내림차순) */
  bids: OrderbookEntry[];
  /** 타임스탬프 (ms) */
  timestamp: number | null;
}

/** Upbit 오더북 단위 (API 응답) */
export interface UpbitOrderbookUnit {
  ask_price: number;
  bid_price: number;
  ask_size: number;
  bid_size: number;
}

/** Upbit 오더북 WebSocket 응답 */
export interface UpbitOrderbookResponse {
  type: 'orderbook';
  code: string;
  timestamp: number;
  total_ask_size: number;
  total_bid_size: number;
  orderbook_units: UpbitOrderbookUnit[];
}

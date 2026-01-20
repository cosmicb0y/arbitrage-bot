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
// Order Form Types (UI)
// ============================================================================

/** 주문 유형 (UI용) */
export type OrderType = 'market' | 'limit';

/** 주문 방향 (UI용) */
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
// Order API Types (Upbit)
// ============================================================================

/** Upbit 주문 방향 (API용) */
export type UpbitOrderSide = 'bid' | 'ask';

/** Upbit 주문 유형 (API용) */
export type UpbitOrderType = 'limit' | 'price' | 'market';

/**
 * 주문 요청 파라미터 (Tauri 명령)
 *
 * @description
 * - limit (지정가): market, side, volume, price 모두 필요
 * - price (시장가 매수): market, side='bid', price(총액) 필요, volume 없음
 * - market (시장가 매도): market, side='ask', volume 필요, price 없음
 */
export interface OrderParams {
  /** 마켓 코드 (예: "KRW-BTC") */
  market: string;
  /** 주문 방향: bid(매수) | ask(매도) */
  side: UpbitOrderSide;
  /** 주문 수량 (시장가 매도 또는 지정가) */
  volume?: string;
  /** 주문 가격 (시장가 매수: 총액, 지정가: 단가) */
  price?: string;
  /** 주문 유형 */
  ord_type: UpbitOrderType;
}

/** 주문 응답 (Upbit API) */
export interface OrderResponse {
  /** 주문 고유 ID */
  uuid: string;
  /** 주문 방향 */
  side: string;
  /** 주문 유형 */
  ord_type: string;
  /** 주문 가격 */
  price: string | null;
  /** 주문 상태: wait, watch, done, cancel */
  state: string;
  /** 마켓 코드 */
  market: string;
  /** 주문 생성 시각 */
  created_at: string;
  /** 주문 수량 */
  volume: string | null;
  /** 미체결 수량 */
  remaining_volume: string | null;
  /** 예약 수수료 */
  reserved_fee: string;
  /** 미사용 수수료 */
  remaining_fee: string;
  /** 지불 수수료 */
  paid_fee: string;
  /** 잠금 금액/수량 */
  locked: string;
  /** 체결 수량 */
  executed_volume: string;
  /** 체결 횟수 */
  trades_count: number;
}

// ============================================================================
// Order Helpers
// ============================================================================

/**
 * UI 주문 방향 → Upbit API 주문 방향 변환
 */
export function toUpbitSide(side: OrderSide): UpbitOrderSide {
  return side === 'buy' ? 'bid' : 'ask';
}

/**
 * UI 주문 유형 + 방향 → Upbit API 주문 유형 변환
 *
 * @description
 * - 지정가 (limit): 'limit'
 * - 시장가 매수: 'price' (총액 지정)
 * - 시장가 매도: 'market' (수량 지정)
 */
export function toUpbitOrderType(
  orderType: OrderType,
  side: OrderSide
): UpbitOrderType {
  if (orderType === 'limit') return 'limit';
  // 시장가: 매수는 'price', 매도는 'market'
  return side === 'buy' ? 'price' : 'market';
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

// ============================================================================
// Order Error Codes (Upbit)
// ============================================================================

/** Upbit 주문 관련 에러 코드 → 한국어 메시지 매핑 */
export const UPBIT_ORDER_ERROR_MESSAGES: Record<string, string> = {
  // 인증 관련
  missing_api_key: 'API 키가 설정되지 않았습니다',
  jwt_error: 'JWT 토큰 생성에 실패했습니다',
  jwt_verification: 'JWT 인증에 실패했습니다',
  no_authorization_ip: '허용되지 않은 IP입니다',
  expired_access_key: '만료된 API 키입니다',
  // 네트워크/서버
  network_error: '네트워크 연결에 실패했습니다',
  rate_limit: '요청이 너무 많습니다. 잠시 후 다시 시도하세요',
  parse_error: '응답 파싱에 실패했습니다',
  // 주문 관련
  insufficient_funds_bid: '매수 가능 금액이 부족합니다',
  insufficient_funds_ask: '매도 가능 수량이 부족합니다',
  under_min_total_bid: '최소 주문금액(5,000원) 이상이어야 합니다',
  under_min_total_ask: '최소 주문금액(5,000원) 이상이어야 합니다',
  invalid_volume: '주문 수량이 올바르지 않습니다',
  invalid_price: '주문 가격이 올바르지 않습니다',
  market_does_not_exist: '존재하지 않는 마켓입니다',
  invalid_side: '주문 방향이 올바르지 않습니다',
  invalid_ord_type: '주문 유형이 올바르지 않습니다',
  validation_error: '잘못된 요청입니다',
};

/**
 * 에러 코드에 대한 한국어 메시지 반환
 * @param code 에러 코드
 * @param fallback 기본 메시지 (매핑 없을 때)
 */
export function getOrderErrorMessage(code: string, fallback?: string): string {
  return UPBIT_ORDER_ERROR_MESSAGES[code] ?? fallback ?? code;
}

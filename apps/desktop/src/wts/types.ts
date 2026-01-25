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
  detail?: Record<string, unknown>;
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
  network_error: '네트워크 연결을 확인하세요',
  timeout_error: '요청 시간이 초과되었습니다. 네트워크 상태를 확인하세요',
  connection_error: '서버에 연결할 수 없습니다. 네트워크 상태를 확인하세요',
  rate_limit: '주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.',
  too_many_requests: '주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.',
  parse_error: '응답 파싱에 실패했습니다',
  server_error: '서버 오류가 발생했습니다. 잠시 후 다시 시도하세요',
  service_unavailable: '서비스를 일시적으로 이용할 수 없습니다',
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
  invalid_query_payload: '요청 파라미터가 올바르지 않습니다',
  // 입금 관련 (WTS-4.1)
  coin_address_not_found: '입금 주소가 아직 생성되지 않았습니다',
  deposit_address_not_found: '입금 주소가 아직 생성되지 않았습니다',
  invalid_currency: '지원하지 않는 자산입니다',
  invalid_net_type: '지원하지 않는 네트워크입니다',
  deposit_paused: '현재 입금이 일시 중단되었습니다',
  deposit_suspended: '해당 자산의 입금이 중단되었습니다',
  address_generation_failed: '입금 주소 생성에 실패했습니다',
  // 출금 관련 (WTS-5.1)
  unregistered_withdraw_address: '출금 주소를 Upbit 웹에서 먼저 등록해주세요',
  withdraw_address_not_registered: '출금 주소를 Upbit 웹에서 먼저 등록해주세요',
  insufficient_funds_withdraw: '출금 가능 잔고가 부족합니다',
  under_min_amount: '최소 출금 수량 이상이어야 합니다',
  over_daily_limit: '일일 출금 한도를 초과했습니다',
  withdraw_suspended: '현재 출금이 일시 중단되었습니다',
  withdraw_disabled: '해당 자산의 출금이 비활성화되었습니다',
  wallet_not_working: '지갑 점검 중입니다. 잠시 후 다시 시도해주세요',
  two_factor_auth_required: 'Upbit 앱에서 2FA 인증이 필요합니다',
  invalid_withdraw_address: '유효하지 않은 출금 주소입니다',
  invalid_secondary_address: '유효하지 않은 보조 주소입니다 (태그/메모)',
  travel_rule_violation: '트래블룰 검증에 실패했습니다',
};

/**
 * 에러 코드에 대한 한국어 메시지 반환
 * @param code 에러 코드
 * @param fallback 기본 메시지 (매핑 없을 때)
 */
export function getOrderErrorMessage(code: string, fallback?: string): string {
  const mapped = UPBIT_ORDER_ERROR_MESSAGES[code];
  if (mapped) return mapped;
  if (fallback && /[가-힣]/.test(fallback)) return fallback;
  return '알 수 없는 오류가 발생했습니다';
}

/**
 * Rate Limit 관련 에러인지 확인
 * @param code 에러 코드
 */
export function isRateLimitError(code: string): boolean {
  return code === 'rate_limit' || code === 'too_many_requests';
}

/**
 * 네트워크 관련 에러인지 확인
 * @param code 에러 코드
 */
export function isNetworkError(code: string): boolean {
  return code === 'network_error' || code === 'timeout_error' || code === 'connection_error';
}

// ============================================================================
// Withdraw Error Classification (WTS-5.5)
// ============================================================================

/**
 * 액션 필요 출금 에러 코드 (WARN 레벨)
 * 사용자가 외부에서 조치 후 재시도 가능
 */
export const WITHDRAW_ACTION_REQUIRED_ERRORS = [
  'two_factor_auth_required',
  'unregistered_withdraw_address',
  'withdraw_address_not_registered',
] as const;

/**
 * 출금 한도 관련 에러 코드
 * 조건 변경 필요
 */
export const WITHDRAW_LIMIT_ERRORS = [
  'over_daily_limit',
  'under_min_amount',
  'insufficient_funds_withdraw',
] as const;

/**
 * 출금 에러별 추가 안내 메시지
 */
export const WITHDRAW_ERROR_GUIDANCE: Record<string, string> = {
  two_factor_auth_required: 'Upbit 모바일 앱에서 출금 인증을 완료한 후 다시 시도하세요.',
  unregistered_withdraw_address: 'https://upbit.com > 입출금 > 출금 > 출금주소관리에서 주소를 등록하세요.',
  withdraw_address_not_registered: 'https://upbit.com > 입출금 > 출금 > 출금주소관리에서 주소를 등록하세요.',
  over_daily_limit: '출금 한도는 매일 00:00(KST)에 초기화됩니다.',
};

/**
 * 액션 필요 출금 에러인지 확인
 * @param code 에러 코드
 */
export function isWithdrawActionRequiredError(code: string): boolean {
  return WITHDRAW_ACTION_REQUIRED_ERRORS.includes(code as typeof WITHDRAW_ACTION_REQUIRED_ERRORS[number]);
}

/**
 * 출금 한도 관련 에러인지 확인
 * @param code 에러 코드
 */
export function isWithdrawLimitError(code: string): boolean {
  return WITHDRAW_LIMIT_ERRORS.includes(code as typeof WITHDRAW_LIMIT_ERRORS[number]);
}

// ============================================================================
// Deposit API Types (Upbit) - WTS-4.1
// ============================================================================

/** 입금 주소 조회 파라미터 */
export interface DepositAddressParams {
  /** 자산 코드 (예: "BTC", "ETH") */
  currency: string;
  /** 네트워크 타입 (예: "BTC", "ETH", "TRX" 등) */
  net_type: string;
}

/** 입금 주소 조회 응답 */
export interface DepositAddressResponse {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 입금 주소 (null일 수 있음 - 생성 중) */
  deposit_address: string | null;
  /** 보조 주소 (XRP tag, EOS memo 등) */
  secondary_address: string | null;
}

/** 입금 가능 정보 파라미터 */
export interface DepositChanceParams {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
}

/** 입금 가능 정보 응답 (실제 Upbit API 응답 형식) */
export interface DepositChanceResponse {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 입금 가능 여부 */
  is_deposit_possible: boolean;
  /** 입금 불가능 사유 (가능하면 null) */
  deposit_impossible_reason: string | null;
  /** 최소 입금 수량 */
  minimum_deposit_amount: number;
  /** 최소 입금 확인 횟수 */
  minimum_deposit_confirmations: number;
  /** 소수점 정밀도 */
  decimal_precision: number;
}

/** 네트워크 정보 (프론트엔드 호환용) */
export interface DepositNetwork {
  /** 네트워크 이름 */
  name: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 우선순위 */
  priority: number;
  /** 입금 상태 */
  deposit_state: string;
  /** 확인 횟수 */
  confirm_count: number;
}

/** DepositChanceResponse에서 DepositNetwork 생성 */
export function toDepositNetwork(response: DepositChanceResponse): DepositNetwork {
  return {
    name: response.net_type,
    net_type: response.net_type,
    priority: 1,
    deposit_state: response.is_deposit_possible ? 'normal' : 'paused',
    confirm_count: response.minimum_deposit_confirmations,
  };
}

/** 입금 주소 생성 응답 - 비동기 생성 중 */
export interface GenerateAddressCreating {
  success: true;
  message: 'creating';
}

/** 입금 주소 생성 응답 (비동기) */
export type GenerateAddressResponse =
  | GenerateAddressCreating
  | DepositAddressResponse;

/** 입금 상태 타입 */
export type DepositState = 'normal' | 'paused' | 'suspended';

/** 입금 상태가 정상인지 확인 (레거시 호환) */
export function isDepositAvailable(state: string): boolean {
  return state === 'normal';
}

/** DepositChanceResponse로 입금 가능 여부 확인 */
export function isDepositPossible(response: DepositChanceResponse): boolean {
  return response.is_deposit_possible;
}

/** GenerateAddressResponse가 생성 중 상태인지 확인 */
export function isAddressGenerating(
  response: GenerateAddressResponse
): response is GenerateAddressCreating {
  return 'success' in response && response.message === 'creating';
}

// ============================================================================
// Withdraw API Types (Upbit) - WTS-5.1
// ============================================================================

/** 출금 요청 파라미터 */
export interface WithdrawParams {
  /** 자산 코드 (예: "BTC", "ETH") */
  currency: string;
  /** 네트워크 타입 (예: "BTC", "ETH", "TRX" 등) */
  net_type: string;
  /** 출금 수량 */
  amount: string;
  /** 출금 주소 (Upbit에 사전 등록 필수) */
  address: string;
  /** 보조 주소 (XRP tag, EOS memo 등) */
  secondary_address?: string | null;
  /** 트래블룰 거래 유형 ("default" 또는 "internal") */
  transaction_type?: string;
}

/** 출금 응답 */
export interface WithdrawResponse {
  /** 응답 타입 (항상 "withdraw") */
  type: string;
  /** 출금 고유 식별자 */
  uuid: string;
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 트랜잭션 ID (블록체인 TXID, 처리 전에는 null) */
  txid: string | null;
  /** 출금 상태 */
  state: WithdrawState;
  /** 출금 생성 시각 */
  created_at: string;
  /** 출금 완료 시각 (완료 전에는 null) */
  done_at: string | null;
  /** 출금 수량 */
  amount: string;
  /** 출금 수수료 */
  fee: string;
  /** 트래블룰 거래 유형 */
  transaction_type: string;
}

/** 출금 상태 */
export type WithdrawState =
  | 'submitting'
  | 'submitted'
  | 'almost_accepted'
  | 'rejected'
  | 'accepted'
  | 'processing'
  | 'done'
  | 'canceled';

/** 출금 가능 정보 파라미터 */
export interface WithdrawChanceParams {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
}

/** 회원 레벨 정보 */
export interface WithdrawMemberLevel {
  security_level: number;
  fee_level: number;
  email_verified: boolean;
  identity_auth_verified: boolean;
  bank_account_verified: boolean;
  two_factor_auth_verified: boolean;
  locked: boolean;
}

/** 자산 정보 (출금용) */
export interface WithdrawCurrencyInfo {
  code: string;
  withdraw_fee: string;
  is_coin: boolean;
  wallet_state: string;
  wallet_support: string[];
}

/** 계좌 정보 (출금용) */
export interface WithdrawAccountInfo {
  balance: string;
  locked: string;
  avg_buy_price: string;
  avg_buy_price_modified: boolean;
  unit_currency: string;
}

/** 출금 한도 정보 */
export interface WithdrawLimitInfo {
  currency: string;
  minimum: string;
  onetime: string;
  daily: string;
  remaining_daily: string;
  remaining_daily_krw: string;
  fixed: number;
  can_withdraw: boolean;
}

/** 출금 가능 정보 응답 */
export interface WithdrawChanceResponse {
  currency: string;
  net_type: string;
  member_level: WithdrawMemberLevel;
  currency_info: WithdrawCurrencyInfo;
  account_info: WithdrawAccountInfo;
  withdraw_limit: WithdrawLimitInfo;
}

/** 출금 허용 주소 응답 */
export interface WithdrawAddressResponse {
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 네트워크 이름 */
  network_name: string;
  /** 출금 주소 */
  withdraw_address: string;
  /** 보조 주소 */
  secondary_address: string | null;
}

/** 출금 조회 파라미터 */
export interface GetWithdrawParams {
  /** 출금 UUID (uuid 또는 txid 중 하나 필수) */
  uuid?: string;
  /** 트랜잭션 ID */
  txid?: string;
}

/** 출금 상태가 완료인지 확인 */
export function isWithdrawComplete(state: WithdrawState): boolean {
  return state === 'done';
}

/** 출금 상태가 진행 중인지 확인 */
export function isWithdrawPending(state: WithdrawState): boolean {
  return ['submitting', 'submitted', 'almost_accepted', 'accepted', 'processing'].includes(state);
}

/** 출금 상태가 실패인지 확인 */
export function isWithdrawFailed(state: WithdrawState): boolean {
  return ['rejected', 'canceled'].includes(state);
}

/** 출금 상태 한국어 메시지 매핑 */
export const WITHDRAW_STATE_MESSAGES: Record<WithdrawState, string> = {
  submitting: '출금 요청 제출 중...',
  submitted: '출금 요청이 제출되었습니다',
  almost_accepted: '출금 요청이 곧 승인됩니다',
  accepted: '출금 요청이 승인되었습니다',
  processing: '블록체인 전송 처리 중...',
  done: '출금이 완료되었습니다',
  rejected: '출금 요청이 거부되었습니다',
  canceled: '출금이 취소되었습니다',
};

// ============================================================================
// Withdraw Confirm Dialog Types (WTS-5.3)
// ============================================================================

/** 출금 확인 다이얼로그에 표시할 정보 */
export interface WithdrawConfirmInfo {
  /** 자산 코드 (예: "BTC") */
  currency: string;
  /** 네트워크 타입 (예: "BTC") */
  net_type: string;
  /** 출금 주소 */
  address: string;
  /** 보조 주소 (XRP tag, EOS memo 등) */
  secondary_address: string | null;
  /** 출금 수량 */
  amount: string;
  /** 출금 수수료 */
  fee: string;
  /** 실수령액 (amount - fee) */
  receivable: string;
}

// ============================================================================
// Withdraw Result Dialog Types (WTS-5.4)
// ============================================================================

/** 출금 결과 다이얼로그에 표시할 정보 */
export interface WithdrawResultInfo {
  /** 출금 고유 식별자 */
  uuid: string;
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 출금 상태 */
  state: WithdrawState;
  /** 출금 수량 */
  amount: string;
  /** 출금 수수료 */
  fee: string;
  /** 트랜잭션 ID (블록체인 TXID, 처리 전에는 null) */
  txid: string | null;
  /** 출금 생성 시각 */
  created_at: string;
}

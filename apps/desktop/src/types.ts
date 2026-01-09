// TypeScript types matching Rust structures

export interface PriceData {
  exchange: string;
  symbol: string;
  pair_id: number;
  price: number;
  bid: number;
  ask: number;
  volume_24h: number;
  timestamp: number;
  // Quote currency (e.g., "USDT", "USDC", "USD", "KRW")
  quote?: string;
}

// USD-like stablecoin quote type
export type UsdlikeQuote = "USDT" | "USDC" | "BUSD";

// USD-like premium (same stablecoin comparison)
export interface UsdlikePremium {
  // Premium in basis points
  bps: number;
  // Which stablecoin was used for comparison
  quote: UsdlikeQuote;
}

export interface ArbitrageOpportunity {
  id: number;
  symbol: string;
  source_exchange: string;
  target_exchange: string;
  // Quote currency at source exchange (e.g., "USDT", "USDC", "KRW")
  source_quote: string;
  // Quote currency at target exchange (e.g., "USDT", "USDC", "KRW")
  target_quote: string;
  source_price: number;
  target_price: number;
  // Raw premium in basis points (direct price comparison)
  premium_bps: number;
  // USD-like premium: same stablecoin comparison (USDT vs USDT or USDC vs USDC)
  // For KRW markets, converts to overseas market's quote currency.
  usdlike_premium?: UsdlikePremium;
  // Kimchi premium: USD price comparison (KRW via forex rate)
  kimchi_premium_bps: number;
  net_profit_bps: number;
  confidence_score: number;
  timestamp: number;
  // Common networks available for transfer (canonical names)
  common_networks?: string[];
  // Whether this opportunity has a viable transfer path
  has_transfer_path?: boolean;
  // Whether wallet status data is available for this opportunity
  wallet_status_known?: boolean;
  // Orderbook depth at source (ask size - quantity available to buy)
  source_depth?: number;
  // Orderbook depth at target (bid size - quantity available to sell)
  target_depth?: number;
  // Optimal trade size from depth walking algorithm
  optimal_size?: number;
  // Expected profit at optimal size (after fees)
  optimal_profit?: number;
  // Reason for optimal_size value: "ok" | "no_orderbook" | "not_profitable"
  optimal_size_reason?: "ok" | "no_orderbook" | "not_profitable";
  // Raw price from source exchange in original quote currency (e.g., KRW for Korean exchanges)
  source_raw_price?: number;
  // Raw price from target exchange in original quote currency
  target_raw_price?: number;
}

export interface BotStats {
  uptime_secs: number;
  price_updates: number;
  opportunities_detected: number;
  trades_executed: number;
  is_running: boolean;
}

export interface ExecutionConfig {
  mode: string;
  min_premium_bps: number;
  max_slippage_bps: number;
  dry_run: boolean;
}

export interface ExchangeRate {
  usd_krw: number;
  upbit_usdt_krw: number;
  bithumb_usdt_krw: number;
  api_rate?: number;
  usdt_usd: number;
  usdc_usd: number;
  timestamp: number;
}

export interface MarketInfo {
  base: string;
  symbol: string;
  exchange: string;
}

export interface CommonMarkets {
  common_bases: string[];
  markets: Record<string, MarketInfo[]>;
  exchanges: string[];
  timestamp: number;
}

export interface ExchangeCredentials {
  api_key: string;
  secret_key: string;
}

export interface CoinbaseCredentials {
  api_key_id: string;
  secret_key: string;
}

export interface Credentials {
  binance: ExchangeCredentials;
  coinbase: CoinbaseCredentials;
  upbit: ExchangeCredentials;
  bithumb: ExchangeCredentials;
  bybit: ExchangeCredentials;
}

// Wallet and deposit/withdraw status
export type WalletStatus = "working" | "withdraw_only" | "deposit_only" | "suspended";

export interface NetworkStatus {
  network: string;
  name: string;
  deposit_enabled: boolean;
  withdraw_enabled: boolean;
  min_withdraw: number;
  withdraw_fee: number;
  confirms_required: number;
}

export interface AssetWalletStatus {
  asset: string;
  name: string;
  networks: NetworkStatus[];
  can_deposit: boolean;
  can_withdraw: boolean;
}

export interface AssetBalance {
  asset: string;
  free: number;
  locked: number;
  total: number;
  usd_value?: number;
}

export interface ExchangeWalletInfo {
  exchange: string;
  balances: AssetBalance[];
  wallet_status: AssetWalletStatus[];
  last_updated: number;
}

// WebSocket wallet status data (from server)
export interface WsWalletStatusData {
  exchanges: ExchangeWalletStatus[];
  timestamp: number;
}

// Exchange wallet status (without balances, from server)
export interface ExchangeWalletStatus {
  exchange: string;
  wallet_status: AssetWalletStatus[];
  last_updated: number;
}

// Symbol mapping for handling same-symbol-different-coin cases
export interface SymbolMapping {
  exchange: string;
  symbol: string;
  canonical_name: string;
  exclude: boolean;
  notes?: string;
}

export interface SymbolMappings {
  mappings: SymbolMapping[];
}

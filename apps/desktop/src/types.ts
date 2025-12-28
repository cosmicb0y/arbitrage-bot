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
}

export interface ArbitrageOpportunity {
  id: number;
  symbol: string;
  source_exchange: string;
  target_exchange: string;
  source_price: number;
  target_price: number;
  premium_bps: number;
  net_profit_bps: number;
  confidence_score: number;
  timestamp: number;
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
  api_rate?: number;
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

export interface Credentials {
  binance: ExchangeCredentials;
  coinbase: ExchangeCredentials;
  upbit: ExchangeCredentials;
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

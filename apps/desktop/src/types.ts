// TypeScript types matching Rust structures

export interface PriceData {
  exchange: string;
  symbol: string;
  pair_id: number;
  price: number;
  bid: number;
  ask: number;
  timestamp: number;
}

export interface ArbitrageOpportunity {
  id: number;
  source_exchange: string;
  target_exchange: string;
  source_price: number;
  target_price: number;
  premium_bps: number;
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

import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState, useCallback, useRef } from "react";
import type {
  PriceData,
  ArbitrageOpportunity,
  BotStats,
  ExecutionConfig,
  ExchangeRate,
  CommonMarkets,
  Credentials,
  ExchangeWalletInfo,
  WsWalletStatusData,
  ExchangeWalletStatus,
} from "../types";

// Check if running inside Tauri
const isTauri = () => {
  return typeof window !== "undefined" && "__TAURI__" in window;
};

// WebSocket server URL for browser fallback
const WS_SERVER_URL = "ws://127.0.0.1:9001/ws";

// WebSocket message types from CLI server
interface WsServerMessage {
  type: "price" | "prices" | "stats" | "opportunity" | "opportunities" | "exchange_rate" | "common_markets" | "wallet_status";
  data: PriceData | PriceData[] | BotStats | ArbitrageOpportunity | ArbitrageOpportunity[] | ExchangeRate | CommonMarkets | WsWalletStatusData;
}

// Shared WebSocket connection for browser mode
type MessageHandler = (msg: WsServerMessage) => void;

class WebSocketManager {
  private ws: WebSocket | null = null;
  private handlers: Set<MessageHandler> = new Set();
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private isConnecting = false;
  private isClosed = false;
  // Cache for initial sync messages - new subscribers get these immediately
  private messageCache: Map<WsServerMessage['type'], WsServerMessage> = new Map();

  connect() {
    if (this.isConnecting || (this.ws && this.ws.readyState === WebSocket.OPEN)) {
      return;
    }

    this.isConnecting = true;
    this.isClosed = false;

    const ws = new WebSocket(WS_SERVER_URL);

    ws.onopen = () => {
      this.ws = ws;
      this.isConnecting = false;
    };

    ws.onmessage = (event) => {
      try {
        const msg: WsServerMessage = JSON.parse(event.data);
        // Cache messages for late subscribers
        this.messageCache.set(msg.type, msg);
        this.handlers.forEach((handler) => handler(msg));
      } catch (e) {
        // Ignore parse errors
      }
    };

    ws.onclose = () => {
      this.ws = null;
      this.isConnecting = false;

      if (!this.isClosed && this.handlers.size > 0) {
        console.log("WebSocket disconnected, reconnecting in 2s...");
        this.reconnectTimeout = setTimeout(() => this.connect(), 2000);
      }
    };

    ws.onerror = () => {
      // Error handling is done in onclose
    };
  }

  subscribe(handler: MessageHandler) {
    this.handlers.add(handler);
    if (this.handlers.size === 1) {
      this.connect();
    }
    // Replay cached messages to new subscriber
    this.messageCache.forEach((msg) => {
      handler(msg);
    });
    return () => {
      this.handlers.delete(handler);
      if (this.handlers.size === 0) {
        this.close();
      }
    };
  }

  close() {
    this.isClosed = true;
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    // Clear cache on close
    this.messageCache.clear();
  }
}

// Singleton WebSocket manager
const wsManager = new WebSocketManager();

export function usePrices() {
  const [prices, setPrices] = useState<PriceData[]>([]);
  // Use Map for O(1) lookups and batch updates
  const priceMapRef = useRef<Map<string, PriceData>>(new Map());
  const pendingUpdateRef = useRef<boolean>(false);

  useEffect(() => {
    // Flush pending updates at 10 FPS (every 100ms) to reduce React re-renders
    const flushUpdates = () => {
      if (pendingUpdateRef.current) {
        pendingUpdateRef.current = false;
        setPrices(Array.from(priceMapRef.current.values()));
      }
    };
    const flushInterval = setInterval(flushUpdates, 100);

    // Browser fallback: use shared WebSocket
    if (!isTauri()) {
      const unsubscribe = wsManager.subscribe((msg) => {
        if (msg.type === "prices") {
          const newPrices = msg.data as PriceData[];
          priceMapRef.current.clear();
          for (const p of newPrices) {
            priceMapRef.current.set(`${p.exchange}-${p.pair_id}`, p);
          }
          pendingUpdateRef.current = true;
        } else if (msg.type === "price") {
          const price = msg.data as PriceData;
          priceMapRef.current.set(`${price.exchange}-${price.pair_id}`, price);
          pendingUpdateRef.current = true;
        }
      });

      return () => {
        clearInterval(flushInterval);
        unsubscribe();
      };
    }

    // Tauri mode: use IPC events
    let unlistenBatch: UnlistenFn | undefined;
    let unlistenSingle: UnlistenFn | undefined;

    const setup = async () => {
      unlistenBatch = await listen<PriceData[]>("price_update", (event) => {
        priceMapRef.current.clear();
        for (const p of event.payload) {
          priceMapRef.current.set(`${p.exchange}-${p.pair_id}`, p);
        }
        pendingUpdateRef.current = true;
      });

      unlistenSingle = await listen<PriceData>("price", (event) => {
        const p = event.payload;
        priceMapRef.current.set(`${p.exchange}-${p.pair_id}`, p);
        pendingUpdateRef.current = true;
      });

      try {
        const data = await invoke<PriceData[]>("get_prices");
        if (data.length > 0) {
          for (const p of data) {
            priceMapRef.current.set(`${p.exchange}-${p.pair_id}`, p);
          }
          pendingUpdateRef.current = true;
        }
      } catch (e) {
        console.error("Failed to fetch initial prices:", e);
      }
    };

    setup();

    return () => {
      clearInterval(flushInterval);
      if (unlistenBatch) unlistenBatch();
      if (unlistenSingle) unlistenSingle();
    };
  }, []);

  return prices;
}

export function useOpportunities() {
  const [opportunities, setOpportunities] = useState<ArbitrageOpportunity[]>(
    []
  );
  // Use Map for O(1) lookups and batch updates
  const oppMapRef = useRef<Map<string, ArbitrageOpportunity>>(new Map());
  const pendingUpdateRef = useRef<boolean>(false);

  useEffect(() => {
    const getOppKey = (opp: ArbitrageOpportunity) =>
      `${opp.symbol}-${opp.source_exchange}-${opp.target_exchange}`;

    // Max age for opportunities (10 seconds)
    const MAX_AGE_MS = 10_000;

    // Flush pending updates at 10 FPS (every 100ms)
    const flushUpdates = () => {
      if (pendingUpdateRef.current) {
        pendingUpdateRef.current = false;
        const now = Date.now();
        // Remove stale opportunities
        for (const [key, opp] of oppMapRef.current) {
          if (now - opp.timestamp > MAX_AGE_MS) {
            oppMapRef.current.delete(key);
          }
        }
        // Sort by premium_bps descending, limit to 50
        const sorted = Array.from(oppMapRef.current.values())
          .sort((a, b) => b.premium_bps - a.premium_bps)
          .slice(0, 50);
        setOpportunities(sorted);
      }
    };
    const flushInterval = setInterval(flushUpdates, 100);

    // Browser fallback
    if (!isTauri()) {
      const unsubscribe = wsManager.subscribe((msg) => {
        if (msg.type === "opportunity") {
          const opp = msg.data as ArbitrageOpportunity;
          oppMapRef.current.set(getOppKey(opp), opp);
          pendingUpdateRef.current = true;
        } else if (msg.type === "opportunities") {
          const opps = msg.data as ArbitrageOpportunity[];
          oppMapRef.current.clear();
          for (const opp of opps) {
            oppMapRef.current.set(getOppKey(opp), opp);
          }
          pendingUpdateRef.current = true;
        }
      });

      return () => {
        clearInterval(flushInterval);
        unsubscribe();
      };
    }

    // Tauri mode
    let unlistenNew: UnlistenFn | undefined;
    let unlistenBatch: UnlistenFn | undefined;

    const setup = async () => {
      // Listen for single new opportunity
      unlistenNew = await listen<ArbitrageOpportunity>(
        "new_opportunity",
        (event) => {
          const opp = event.payload;
          oppMapRef.current.set(getOppKey(opp), opp);
          pendingUpdateRef.current = true;
        }
      );

      // Listen for batch opportunities (initial sync)
      unlistenBatch = await listen<ArbitrageOpportunity[]>(
        "opportunities",
        (event) => {
          oppMapRef.current.clear();
          for (const opp of event.payload) {
            oppMapRef.current.set(getOppKey(opp), opp);
          }
          pendingUpdateRef.current = true;
        }
      );

      try {
        const data = await invoke<ArbitrageOpportunity[]>("get_opportunities");
        for (const opp of data) {
          oppMapRef.current.set(getOppKey(opp), opp);
        }
        pendingUpdateRef.current = true;
      } catch (e) {
        console.error("Failed to fetch initial opportunities:", e);
      }
    };

    setup();

    return () => {
      clearInterval(flushInterval);
      if (unlistenNew) unlistenNew();
      if (unlistenBatch) unlistenBatch();
    };
  }, []);

  const executeOpportunity = useCallback(
    async (id: number, amount: number): Promise<string> => {
      return invoke<string>("execute_opportunity", {
        opportunityId: id,
        amount,
      });
    },
    []
  );

  return { opportunities, executeOpportunity };
}

export function useStats() {
  const [stats, setStats] = useState<BotStats>({
    uptime_secs: 0,
    price_updates: 0,
    opportunities_detected: 0,
    trades_executed: 0,
    is_running: false,
  });

  useEffect(() => {
    // Browser fallback
    if (!isTauri()) {
      const unsubscribe = wsManager.subscribe((msg) => {
        if (msg.type === "stats") {
          setStats(msg.data as BotStats);
        }
      });

      return unsubscribe;
    }

    // Tauri mode
    let unlisten: UnlistenFn | undefined;

    const setup = async () => {
      unlisten = await listen<BotStats>("stats", (event) => {
        setStats(event.payload);
      });

      try {
        const data = await invoke<BotStats>("get_stats");
        setStats(data);
      } catch (e) {
        console.error("Failed to fetch initial stats:", e);
      }
    };

    setup();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return stats;
}

export function useBotControl() {
  const start = useCallback(async () => {
    if (!isTauri()) return;
    try {
      await invoke("start_bot");
    } catch (e) {
      console.error("Failed to start bot:", e);
    }
  }, []);

  const stop = useCallback(async () => {
    if (!isTauri()) return;
    try {
      await invoke("stop_bot");
    } catch (e) {
      console.error("Failed to stop bot:", e);
    }
  }, []);

  return { start, stop };
}

export function useConfig() {
  const [config, setConfig] = useState<ExecutionConfig>({
    mode: "alert",
    min_premium_bps: 30,
    max_slippage_bps: 50,
    dry_run: true,
  });

  useEffect(() => {
    if (!isTauri()) return;

    const fetchConfig = async () => {
      try {
        const data = await invoke<ExecutionConfig>("get_config");
        setConfig(data);
      } catch (e) {
        console.error("Failed to fetch config:", e);
      }
    };

    fetchConfig();
  }, []);

  const updateConfig = useCallback(async (newConfig: ExecutionConfig) => {
    if (!isTauri()) return;
    try {
      await invoke("update_config", { config: newConfig });
      setConfig(newConfig);
    } catch (e) {
      console.error("Failed to update config:", e);
    }
  }, []);

  return { config, updateConfig };
}

export function useExchangeRate() {
  const [exchangeRate, setExchangeRate] = useState<ExchangeRate | null>(null);

  useEffect(() => {
    // Browser fallback
    if (!isTauri()) {
      const unsubscribe = wsManager.subscribe((msg) => {
        if (msg.type === "exchange_rate") {
          setExchangeRate(msg.data as ExchangeRate);
        }
      });

      return unsubscribe;
    }

    // Tauri mode
    let unlisten: UnlistenFn | undefined;

    const setup = async () => {
      unlisten = await listen<ExchangeRate>("exchange_rate", (event) => {
        setExchangeRate(event.payload);
      });

      try {
        const data = await invoke<ExchangeRate>("get_exchange_rate");
        if (data && data.usd_krw > 0) {
          setExchangeRate(data);
        }
      } catch (e) {
        console.error("Failed to fetch initial exchange rate:", e);
      }
    };

    setup();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return exchangeRate;
}

// Cache for common markets (persists across component remounts)
let cachedCommonMarkets: CommonMarkets | null = null;

export function useCommonMarkets() {
  const [commonMarkets, setCommonMarkets] = useState<CommonMarkets | null>(cachedCommonMarkets);

  useEffect(() => {
    // Browser fallback
    if (!isTauri()) {
      const unsubscribe = wsManager.subscribe((msg) => {
        if (msg.type === "common_markets") {
          cachedCommonMarkets = msg.data as CommonMarkets;
          setCommonMarkets(cachedCommonMarkets);
        }
      });

      return unsubscribe;
    }

    // Tauri mode
    let unlisten: UnlistenFn | undefined;

    const setup = async () => {
      unlisten = await listen<CommonMarkets>("common_markets", (event) => {
        cachedCommonMarkets = event.payload;
        setCommonMarkets(cachedCommonMarkets);
      });

      try {
        const data = await invoke<CommonMarkets>("get_common_markets");
        if (data && data.common_bases.length > 0) {
          cachedCommonMarkets = data;
          setCommonMarkets(cachedCommonMarkets);
        }
      } catch (e) {
        console.error("Failed to fetch initial common markets:", e);
      }
    };

    setup();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return commonMarkets;
}

const emptyCredentials: Credentials = {
  binance: { api_key: "", secret_key: "" },
  coinbase: { api_key: "", secret_key: "" },
  upbit: { api_key: "", secret_key: "" },
};

export function useCredentials() {
  const [credentials, setCredentials] = useState<Credentials>(emptyCredentials);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!isTauri()) {
      setLoading(false);
      return;
    }

    const fetchCredentials = async () => {
      try {
        const data = await invoke<Credentials>("get_credentials");
        setCredentials(data);
      } catch (e) {
        console.error("Failed to fetch credentials:", e);
      } finally {
        setLoading(false);
      }
    };

    fetchCredentials();
  }, []);

  const saveCredentials = useCallback(async (creds: Credentials): Promise<boolean> => {
    if (!isTauri()) return false;
    try {
      await invoke("save_credentials", { creds });
      // Reload masked credentials after save
      const data = await invoke<Credentials>("get_credentials");
      setCredentials(data);
      return true;
    } catch (e) {
      console.error("Failed to save credentials:", e);
      return false;
    }
  }, []);

  return { credentials, saveCredentials, loading };
}

export function useWalletInfo(exchange?: string) {
  const [wallets, setWallets] = useState<ExchangeWalletInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchWallets = useCallback(async () => {
    if (!isTauri()) return;
    setLoading(true);
    setError(null);

    try {
      if (exchange) {
        const data = await invoke<ExchangeWalletInfo>("get_wallet_info", { exchange });
        setWallets([data]);
      } else {
        const data = await invoke<ExchangeWalletInfo[]>("get_all_wallets");
        setWallets(data);
      }
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e);
      setError(errMsg);
      console.error("Failed to fetch wallet info:", e);
    } finally {
      setLoading(false);
    }
  }, [exchange]);

  return { wallets, loading, error, fetchWallets };
}

// Cache for wallet status (persists across component remounts)
let cachedWalletStatus: ExchangeWalletStatus[] = [];

/**
 * Hook to receive wallet status updates via WebSocket from the server.
 * This is for the Markets component which only needs deposit/withdraw status.
 * Updated every 5 minutes from the server.
 */
export function useWalletStatus() {
  const [walletStatus, setWalletStatus] = useState<ExchangeWalletStatus[]>(cachedWalletStatus);

  useEffect(() => {
    // Browser fallback: use shared WebSocket
    if (!isTauri()) {
      const unsubscribe = wsManager.subscribe((msg) => {
        if (msg.type === "wallet_status") {
          const data = msg.data as WsWalletStatusData;
          cachedWalletStatus = data.exchanges;
          setWalletStatus(cachedWalletStatus);
        }
      });

      return unsubscribe;
    }

    // Tauri mode: listen for wallet_status events
    let unlisten: UnlistenFn | undefined;

    const setup = async () => {
      unlisten = await listen<WsWalletStatusData>("wallet_status", (event) => {
        cachedWalletStatus = event.payload.exchanges;
        setWalletStatus(cachedWalletStatus);
      });

      // Fetch initial wallet status from cache
      try {
        const data = await invoke<WsWalletStatusData | null>("get_wallet_status");
        if (data && data.exchanges.length > 0) {
          cachedWalletStatus = data.exchanges;
          setWalletStatus(cachedWalletStatus);
        }
      } catch (e) {
        console.error("Failed to fetch initial wallet status:", e);
      }
    };

    setup();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return walletStatus;
}

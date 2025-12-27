import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState, useCallback } from "react";
import type {
  PriceData,
  ArbitrageOpportunity,
  BotStats,
  ExecutionConfig,
  ExchangeRate,
} from "../types";

// Check if running inside Tauri
const isTauri = () => {
  return typeof window !== "undefined" && "__TAURI__" in window;
};

// WebSocket server URL for browser fallback
const WS_SERVER_URL = "ws://127.0.0.1:9001/ws";

// WebSocket message types from CLI server
interface WsServerMessage {
  type: "price" | "prices" | "stats" | "opportunity" | "exchange_rate";
  data: PriceData | PriceData[] | BotStats | ArbitrageOpportunity | ExchangeRate;
}

// Shared WebSocket connection for browser mode
type MessageHandler = (msg: WsServerMessage) => void;

class WebSocketManager {
  private ws: WebSocket | null = null;
  private handlers: Set<MessageHandler> = new Set();
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private isConnecting = false;
  private isClosed = false;

  connect() {
    if (this.isConnecting || (this.ws && this.ws.readyState === WebSocket.OPEN)) {
      return;
    }

    this.isConnecting = true;
    this.isClosed = false;

    const ws = new WebSocket(WS_SERVER_URL);

    ws.onopen = () => {
      console.log("Connected to CLI server WebSocket");
      this.ws = ws;
      this.isConnecting = false;
    };

    ws.onmessage = (event) => {
      try {
        const msg: WsServerMessage = JSON.parse(event.data);
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
  }
}

// Singleton WebSocket manager
const wsManager = new WebSocketManager();

export function usePrices() {
  const [prices, setPrices] = useState<PriceData[]>([]);

  useEffect(() => {
    // Browser fallback: use shared WebSocket
    if (!isTauri()) {
      const unsubscribe = wsManager.subscribe((msg) => {
        if (msg.type === "prices") {
          setPrices(msg.data as PriceData[]);
        } else if (msg.type === "price") {
          const price = msg.data as PriceData;
          setPrices((prev) => {
            const key = `${price.exchange}-${price.pair_id}`;
            const existing = prev.findIndex(
              (p) => `${p.exchange}-${p.pair_id}` === key
            );
            if (existing >= 0) {
              const updated = [...prev];
              updated[existing] = price;
              return updated;
            }
            return [...prev, price];
          });
        }
      });

      return unsubscribe;
    }

    // Tauri mode: use IPC events
    let unlistenBatch: UnlistenFn | undefined;
    let unlistenSingle: UnlistenFn | undefined;

    const setup = async () => {
      unlistenBatch = await listen<PriceData[]>("price_update", (event) => {
        setPrices(event.payload);
      });

      unlistenSingle = await listen<PriceData>("price", (event) => {
        setPrices((prev) => {
          const key = `${event.payload.exchange}-${event.payload.pair_id}`;
          const existing = prev.findIndex(
            (p) => `${p.exchange}-${p.pair_id}` === key
          );
          if (existing >= 0) {
            const updated = [...prev];
            updated[existing] = event.payload;
            return updated;
          }
          return [...prev, event.payload];
        });
      });

      try {
        const data = await invoke<PriceData[]>("get_prices");
        if (data.length > 0) {
          setPrices(data);
        }
      } catch (e) {
        console.error("Failed to fetch initial prices:", e);
      }
    };

    setup();

    return () => {
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

  useEffect(() => {
    // Browser fallback
    if (!isTauri()) {
      const unsubscribe = wsManager.subscribe((msg) => {
        if (msg.type === "opportunity") {
          const opp = msg.data as ArbitrageOpportunity;
          setOpportunities((prev) => {
            const updated = [opp, ...prev];
            return updated.slice(0, 50);
          });
        }
      });

      return unsubscribe;
    }

    // Tauri mode
    let unlisten: UnlistenFn | undefined;

    const setup = async () => {
      unlisten = await listen<ArbitrageOpportunity>(
        "new_opportunity",
        (event) => {
          setOpportunities((prev) => {
            const updated = [event.payload, ...prev];
            return updated.slice(0, 50);
          });
        }
      );

      try {
        const data = await invoke<ArbitrageOpportunity[]>("get_opportunities");
        setOpportunities(data);
      } catch (e) {
        console.error("Failed to fetch initial opportunities:", e);
      }
    };

    setup();

    return () => {
      if (unlisten) unlisten();
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

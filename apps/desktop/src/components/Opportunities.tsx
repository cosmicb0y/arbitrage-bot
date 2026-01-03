import { useState, useEffect, useRef, useMemo } from "react";
import { useOpportunities, usePrices, useWalletStatus, useExchangeRate } from "../hooks/useTauri";

type OppKey = string;
type ChangeType = "up" | "down" | null;

interface PriceChanges {
  source: ChangeType;
  target: ChangeType;
  spread: ChangeType;
}

const getOppKey = (opp: { symbol: string; source_exchange: string; target_exchange: string }) =>
  `${opp.symbol}-${opp.source_exchange}-${opp.target_exchange}`;

// Format elapsed time since opportunity was detected
const formatElapsedTime = (timestampMs: number): string => {
  const now = Date.now();
  const elapsedMs = now - timestampMs;

  if (elapsedMs < 0) return "0s";

  const seconds = Math.floor(elapsedMs / 1000);
  if (seconds < 60) return `${seconds}s`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;

  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
};

type PremiumMode = "kimchi" | "tether";

function Opportunities() {
  const { opportunities: rawOpportunities, executeOpportunity } = useOpportunities();
  const prices = usePrices();
  const walletStatuses = useWalletStatus();
  const exchangeRate = useExchangeRate();
  const [executing, setExecuting] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [priceChanges, setPriceChanges] = useState<Map<OppKey, PriceChanges>>(new Map());
  const prevDataRef = useRef<Map<OppKey, { source: number; target: number; spread: number }>>(new Map());
  const [minVolume, setMinVolume] = useState<number>(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [premiumMode, setPremiumMode] = useState<PremiumMode>("tether");
  const [pathOnly, setPathOnly] = useState<boolean>(false);
  const [, setTick] = useState(0); // Force re-render for elapsed time updates

  // Update elapsed time display every second
  useEffect(() => {
    const interval = setInterval(() => {
      setTick((t) => t + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  // Build wallet status lookup: exchange -> asset -> { canDeposit, canWithdraw }
  const walletStatusMap = useMemo(() => {
    const map: Record<string, Record<string, { canDeposit: boolean; canWithdraw: boolean }>> = {};
    for (const status of walletStatuses) {
      map[status.exchange] = {};
      for (const assetStatus of status.wallet_status) {
        map[status.exchange][assetStatus.asset] = {
          canDeposit: assetStatus.can_deposit,
          canWithdraw: assetStatus.can_withdraw,
        };
      }
    }
    return map;
  }, [walletStatuses]);

  // Build volume map by exchange+symbol (for filtering both source and target)
  const volumeByExchangeSymbol = useMemo(() => {
    const map: Record<string, number> = {};
    for (const price of prices) {
      const key = `${price.exchange}:${price.symbol}`;
      map[key] = price.volume_24h || 0;
    }
    return map;
  }, [prices]);

  // Helper to get sort premium based on mode
  const getSortPremium = (opp: typeof rawOpportunities[0]): number => {
    const hasKRW = opp.source_quote === "KRW" || opp.target_quote === "KRW";
    if (!hasKRW) return opp.premium_bps;

    if (premiumMode === "kimchi") {
      return opp.kimchi_premium_bps ?? opp.premium_bps;
    } else if (premiumMode === "tether") {
      return opp.tether_premium_bps ?? opp.premium_bps;
    }
    return opp.premium_bps;
  };

  // Sort and filter opportunities (show all, no premium mode filtering)
  const opportunities = useMemo(() => {
    const query = searchQuery.toUpperCase();
    return [...rawOpportunities]
      .filter((opp) => {
        // Search filter: match symbol or exchange names
        if (query) {
          const matchesSymbol = opp.symbol.toUpperCase().includes(query);
          const matchesSource = opp.source_exchange.toUpperCase().includes(query);
          const matchesTarget = opp.target_exchange.toUpperCase().includes(query);
          if (!matchesSymbol && !matchesSource && !matchesTarget) return false;
        }
        // Volume filter: both source and target exchanges must meet the volume threshold
        if (minVolume > 0) {
          const sourceKey = `${opp.source_exchange}:${opp.symbol}`;
          const targetKey = `${opp.target_exchange}:${opp.symbol}`;
          const sourceVolume = volumeByExchangeSymbol[sourceKey] || 0;
          const targetVolume = volumeByExchangeSymbol[targetKey] || 0;
          if (sourceVolume < minVolume || targetVolume < minVolume) return false;
        }
        // Path filter: only show opportunities with transfer path
        if (pathOnly && !opp.has_transfer_path) return false;
        return true;
      })
      .sort((a, b) => getSortPremium(b) - getSortPremium(a));
  }, [rawOpportunities, searchQuery, minVolume, volumeByExchangeSymbol, premiumMode, pathOnly]);

  // Detect price changes
  useEffect(() => {
    const newChanges = new Map<OppKey, PriceChanges>();
    const prevData = prevDataRef.current;
    const currentKeys = new Set<OppKey>();

    for (const opp of opportunities) {
      const key = getOppKey(opp);
      currentKeys.add(key);
      const prev = prevData.get(key);

      if (prev) {
        const sourceChange: ChangeType = opp.source_price > prev.source ? "up" : opp.source_price < prev.source ? "down" : null;
        const targetChange: ChangeType = opp.target_price > prev.target ? "up" : opp.target_price < prev.target ? "down" : null;
        const spreadChange: ChangeType = opp.premium_bps > prev.spread ? "up" : opp.premium_bps < prev.spread ? "down" : null;

        if (sourceChange || targetChange || spreadChange) {
          newChanges.set(key, { source: sourceChange, target: targetChange, spread: spreadChange });
        }
      }

      prevData.set(key, { source: opp.source_price, target: opp.target_price, spread: opp.premium_bps });
    }

    // Clean up old entries that are no longer in opportunities (prevent memory leak)
    for (const key of prevData.keys()) {
      if (!currentKeys.has(key)) {
        prevData.delete(key);
      }
    }

    if (newChanges.size > 0) {
      setPriceChanges(newChanges);
      const timer = setTimeout(() => setPriceChanges(new Map()), 800);
      return () => clearTimeout(timer);
    }
  }, [opportunities]);

  const getChangeClass = (change: ChangeType | undefined): string => {
    if (change === "up") return "bg-success-500/30 rounded px-1 -mx-1";
    if (change === "down") return "bg-danger-500/30 rounded px-1 -mx-1";
    return "";
  };

  const handleExecute = async (id: number) => {
    setExecuting(id);
    setMessage(null);
    try {
      const result = await executeOpportunity(id, 1000);
      setMessage(result);
    } catch (err) {
      setMessage(`Error: ${err}`);
    } finally {
      setExecuting(null);
    }
  };

  const formatPrice = (price: number): string => {
    // Determine decimal places based on price magnitude
    // Show all significant digits up to reasonable precision
    if (price >= 10000) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      });
    } else if (price >= 1000) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 3,
      });
    } else if (price >= 100) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 4,
      });
    } else if (price >= 10) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 5,
      });
    } else if (price >= 1) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 4,
        maximumFractionDigits: 6,
      });
    } else if (price >= 0.01) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 4,
        maximumFractionDigits: 6,
      });
    } else if (price >= 0.0001) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 6,
        maximumFractionDigits: 8,
      });
    } else {
      // Very small prices (< 0.0001): show up to 10 decimal places
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 8,
        maximumFractionDigits: 10,
      });
    }
  };

  // Format KRW price (adaptive decimals based on magnitude)
  const formatKrwPrice = (price: number): string => {
    // For very small prices (< 1 KRW), show appropriate decimal places
    if (price < 0.0001) {
      // Extremely small: show up to 8 decimal places
      return price.toLocaleString("ko-KR", {
        minimumFractionDigits: 6,
        maximumFractionDigits: 8,
      });
    } else if (price < 0.01) {
      // Very small: show up to 6 decimal places
      return price.toLocaleString("ko-KR", {
        minimumFractionDigits: 4,
        maximumFractionDigits: 6,
      });
    } else if (price < 1) {
      // Small: show up to 4 decimal places
      return price.toLocaleString("ko-KR", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 4,
      });
    } else if (price < 100) {
      // Medium: show up to 2 decimal places
      return price.toLocaleString("ko-KR", {
        minimumFractionDigits: 0,
        maximumFractionDigits: 2,
      });
    } else {
      // Large prices: no decimals
      return price.toLocaleString("ko-KR", {
        minimumFractionDigits: 0,
        maximumFractionDigits: 0,
      });
    }
  };

  // Get USDT/KRW rate for an exchange (for reverse conversion)
  const getUsdtKrwForExchange = (exchange: string): number => {
    if (!exchangeRate) return 0;
    if (exchange === "Upbit") return exchangeRate.upbit_usdt_krw;
    if (exchange === "Bithumb") return exchangeRate.bithumb_usdt_krw;
    return 0;
  };

  // Convert USD price back to original quote currency
  const convertToRawPrice = (
    usdPrice: number,
    exchange: string,
    quote: string
  ): { rawPrice: number; currency: string; symbol: string } | null => {
    // For Korean exchanges with KRW quote, convert back to KRW
    if (quote === "KRW") {
      const usdtKrw = getUsdtKrwForExchange(exchange);
      if (usdtKrw > 0) {
        // usdPrice was converted from KRW: krw_price / usdt_krw * usdt_usd = usd_price
        // So: krw_price = usd_price / usdt_usd * usdt_krw
        const usdtUsd = exchangeRate?.usdt_usd || 1;
        const krwPrice = (usdPrice / usdtUsd) * usdtKrw;
        return { rawPrice: krwPrice, currency: "₩", symbol: "KRW" };
      }
    }
    // For USDT quote, convert back to USDT
    if (quote === "USDT") {
      const usdtUsd = exchangeRate?.usdt_usd || 1;
      if (usdtUsd > 0 && usdtUsd !== 1) {
        const usdtPrice = usdPrice / usdtUsd;
        return { rawPrice: usdtPrice, currency: "", symbol: "USDT" };
      }
      // If usdt_usd is ~1, show as USDT but no conversion needed
      return { rawPrice: usdPrice, currency: "", symbol: "USDT" };
    }
    // For USDC quote, convert back to USDC
    if (quote === "USDC") {
      const usdcUsd = exchangeRate?.usdc_usd || 1;
      if (usdcUsd > 0 && usdcUsd !== 1) {
        const usdcPrice = usdPrice / usdcUsd;
        return { rawPrice: usdcPrice, currency: "", symbol: "USDC" };
      }
      // If usdc_usd is ~1, show as USDC but no conversion needed
      return { rawPrice: usdPrice, currency: "", symbol: "USDC" };
    }
    // For USD quote, no conversion needed
    return null;
  };

  // Get the appropriate premium value based on mode and whether it's a KRW trade
  const getDisplayPremium = (opp: typeof opportunities[0]): number => {
    const hasKRW = opp.source_quote === "KRW" || opp.target_quote === "KRW";

    if (!hasKRW) {
      // Non-KRW trades always show raw premium
      return opp.premium_bps;
    }

    // KRW trades: show 김프 or 테프 based on mode
    if (premiumMode === "kimchi") {
      return opp.kimchi_premium_bps ?? opp.premium_bps;
    }
    // tether mode
    return opp.tether_premium_bps ?? opp.premium_bps;
  };

  // Get premium label for display (only for KRW trades)
  const getPremiumLabel = (opp: typeof opportunities[0]): string | null => {
    const hasKRW = opp.source_quote === "KRW" || opp.target_quote === "KRW";
    if (!hasKRW) return null;
    return premiumMode === "kimchi" ? "김프" : "테프";
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h2 className="text-lg font-semibold">Arbitrage Opportunities</h2>
        <div className="flex items-center gap-4">
          {/* Path Only Toggle */}
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={pathOnly}
              onChange={(e) => setPathOnly(e.target.checked)}
              className="w-4 h-4 rounded border-dark-600 bg-dark-700 text-primary-500 focus:ring-primary-500 focus:ring-offset-0"
            />
            <span className="text-sm text-gray-400">Path만</span>
          </label>
          {/* Premium Mode Toggle (김프/테프 for KRW trades) */}
          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-500">KRW:</span>
            <div className="flex bg-dark-700 rounded-lg p-0.5">
              <button
                onClick={() => setPremiumMode("kimchi")}
                className={`px-3 py-1 text-sm rounded-md transition-colors ${
                  premiumMode === "kimchi"
                    ? "bg-yellow-600 text-white"
                    : "text-gray-400 hover:text-white"
                }`}
              >
                김프
              </button>
              <button
                onClick={() => setPremiumMode("tether")}
                className={`px-3 py-1 text-sm rounded-md transition-colors ${
                  premiumMode === "tether"
                    ? "bg-green-600 text-white"
                    : "text-gray-400 hover:text-white"
                }`}
              >
                테프
              </button>
            </div>
          </div>
          <input
            type="text"
            placeholder="Search symbol or exchange..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-48 px-3 py-1.5 bg-dark-700 border border-dark-600 rounded text-sm text-white placeholder-gray-500 focus:outline-none focus:border-primary-500"
          />
          <select
            value={minVolume}
            onChange={(e) => setMinVolume(Number(e.target.value))}
            className="px-3 py-1.5 bg-dark-700 border border-dark-600 rounded text-sm text-white focus:outline-none focus:border-primary-500"
          >
            <option value={0}>All Volumes</option>
            <option value={100000}>Vol &gt; $100K</option>
            <option value={1000000}>Vol &gt; $1M</option>
            <option value={10000000}>Vol &gt; $10M</option>
            <option value={100000000}>Vol &gt; $100M</option>
            <option value={1000000000}>Vol &gt; $1B</option>
          </select>
          <span className="text-sm text-gray-400">
            {opportunities.length} opportunities found
          </span>
        </div>
      </div>

      {message && (
        <div className="bg-primary-600/20 border border-primary-500 rounded-lg p-3 text-sm">
          {message}
        </div>
      )}

      <div className="bg-dark-800 rounded-lg border border-dark-700 overflow-hidden">
        <table className="w-full table-fixed">
          <colgroup>
            <col className="w-20" />
            <col className="w-72" />
            <col className="w-24" />
            <col className="w-28" />
            <col className="w-28" />
            <col className="w-40" />
            <col className="w-24" />
            <col className="w-24" />
          </colgroup>
          <thead className="bg-dark-700">
            <tr>
              <th className="text-left text-gray-400 text-sm p-4">Asset</th>
              <th className="text-left text-gray-400 text-sm p-4">Route</th>
              <th className="text-center text-gray-400 text-sm p-4">Transfer</th>
              <th className="text-right text-gray-400 text-sm p-4">Buy Price</th>
              <th className="text-right text-gray-400 text-sm p-4">Sell Price</th>
              <th className="text-right text-gray-400 text-sm p-4">Depth</th>
              <th className="text-right text-gray-400 text-sm p-4">Spread</th>
              <th className="text-center text-gray-400 text-sm p-4">Action</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-dark-700">
            {opportunities.length > 0 ? (
              opportunities.map((opp, index) => {
                const oppKey = getOppKey(opp);
                const changes = priceChanges.get(oppKey);
                // source_exchange에서 출금, target_exchange로 입금
                const sourceStatus = walletStatusMap[opp.source_exchange]?.[opp.symbol];
                const targetStatus = walletStatusMap[opp.target_exchange]?.[opp.symbol];
                const canWithdrawFromSource = sourceStatus?.canWithdraw;
                const canDepositToTarget = targetStatus?.canDeposit;
                return (
                  <tr
                    key={`${opp.id}-${index}`}
                    className="hover:bg-dark-700/50"
                  >
                  <td className="p-4">
                    <span className="text-primary-400 font-bold text-lg">
                      {opp.symbol}
                    </span>
                  </td>
                  <td className="p-4">
                    <div className="flex items-center space-x-2">
                      <div className="flex items-center gap-1">
                        <span className="text-success-500 font-medium">
                          {opp.source_exchange}
                        </span>
                        <span className={`text-xs px-1 py-0.5 rounded ${
                          opp.source_quote === 'KRW' ? 'bg-yellow-500/20 text-yellow-400' :
                          opp.source_quote === 'USDC' ? 'bg-blue-500/20 text-blue-400' :
                          opp.source_quote === 'USDT' ? 'bg-green-500/20 text-green-400' :
                          'bg-gray-500/20 text-gray-400'
                        }`}>
                          {opp.source_quote || 'USD'}
                        </span>
                      </div>
                      <span className="text-gray-500">→</span>
                      <div className="flex items-center gap-1">
                        <span className="text-primary-500 font-medium">
                          {opp.target_exchange}
                        </span>
                        <span className={`text-xs px-1 py-0.5 rounded ${
                          opp.target_quote === 'KRW' ? 'bg-yellow-500/20 text-yellow-400' :
                          opp.target_quote === 'USDC' ? 'bg-blue-500/20 text-blue-400' :
                          opp.target_quote === 'USDT' ? 'bg-green-500/20 text-green-400' :
                          'bg-gray-500/20 text-gray-400'
                        }`}>
                          {opp.target_quote || 'USD'}
                        </span>
                      </div>
                    </div>
                  </td>
                  <td className="p-4 text-center">
                    <div className="flex flex-col items-center gap-0.5">
                      <div className="flex items-center justify-center gap-1">
                        <span
                          className={`text-xs px-1.5 py-0.5 rounded ${
                            canWithdrawFromSource === undefined
                              ? "bg-gray-600 text-gray-400"
                              : canWithdrawFromSource
                                ? "bg-success-500/20 text-success-400"
                                : "bg-danger-500/20 text-danger-400"
                          }`}
                          title={`${opp.source_exchange} withdraw`}
                        >
                          W{canWithdrawFromSource === undefined ? "?" : canWithdrawFromSource ? "✓" : "✕"}
                        </span>
                        <span className="text-gray-600">→</span>
                        <span
                          className={`text-xs px-1.5 py-0.5 rounded ${
                            canDepositToTarget === undefined
                              ? "bg-gray-600 text-gray-400"
                              : canDepositToTarget
                                ? "bg-success-500/20 text-success-400"
                                : "bg-danger-500/20 text-danger-400"
                          }`}
                          title={`${opp.target_exchange} deposit`}
                        >
                          D{canDepositToTarget === undefined ? "?" : canDepositToTarget ? "✓" : "✕"}
                        </span>
                      </div>
                      {/* Show transfer path status - uses common network matching */}
                      {/* Distinguish: unknown (loading) vs no path (blocked) vs has path (available) */}
                      <span
                        className={`text-xs px-1.5 py-0.5 rounded ${
                          !opp.wallet_status_known
                            ? "bg-gray-600 text-gray-400"
                            : opp.has_transfer_path
                              ? "bg-success-500/20 text-success-400"
                              : "bg-danger-500/20 text-danger-400"
                        }`}
                        title={
                          !opp.wallet_status_known
                            ? "Wallet status loading..."
                            : opp.common_networks?.length
                              ? `Common networks: ${opp.common_networks.join(", ")}`
                              : "No common network available"
                        }
                      >
                        {!opp.wallet_status_known
                          ? "? Loading"
                          : opp.has_transfer_path
                            ? "✓ Path"
                            : "✕ Blocked"}
                      </span>
                    </div>
                  </td>
                  <td className="p-4 text-right font-mono">
                    {(() => {
                      const rawPrice = convertToRawPrice(opp.source_price, opp.source_exchange, opp.source_quote);
                      if (rawPrice) {
                        // Show raw price first, then USD conversion
                        const isKRW = rawPrice.symbol === "KRW";
                        return (
                          <div className="flex flex-col items-end">
                            <span className={getChangeClass(changes?.source)}>
                              {isKRW
                                ? `${rawPrice.currency}${formatKrwPrice(rawPrice.rawPrice)}`
                                : `${formatPrice(rawPrice.rawPrice)} ${rawPrice.symbol}`
                              }
                            </span>
                            <span className="text-xs text-gray-500">
                              ${formatPrice(opp.source_price)}
                            </span>
                          </div>
                        );
                      }
                      // USD quote: show USD only
                      return (
                        <span className={getChangeClass(changes?.source)}>
                          ${formatPrice(opp.source_price)}
                        </span>
                      );
                    })()}
                  </td>
                  <td className="p-4 text-right font-mono">
                    {(() => {
                      const rawPrice = convertToRawPrice(opp.target_price, opp.target_exchange, opp.target_quote);
                      if (rawPrice) {
                        // Show raw price first, then USD conversion
                        const isKRW = rawPrice.symbol === "KRW";
                        return (
                          <div className="flex flex-col items-end">
                            <span className={getChangeClass(changes?.target)}>
                              {isKRW
                                ? `${rawPrice.currency}${formatKrwPrice(rawPrice.rawPrice)}`
                                : `${formatPrice(rawPrice.rawPrice)} ${rawPrice.symbol}`
                              }
                            </span>
                            <span className="text-xs text-gray-500">
                              ${formatPrice(opp.target_price)}
                            </span>
                          </div>
                        );
                      }
                      // USD quote: show USD only
                      return (
                        <span className={getChangeClass(changes?.target)}>
                          ${formatPrice(opp.target_price)}
                        </span>
                      );
                    })()}
                  </td>
                  <td className="p-4 text-right">
                    {(() => {
                      const sourceDepth = opp.source_depth ?? 0;
                      const targetDepth = opp.target_depth ?? 0;
                      // Calculate USD value: depth * price
                      const sourceUsd = sourceDepth > 0 ? sourceDepth * opp.source_price : 0;
                      const targetUsd = targetDepth > 0 ? targetDepth * opp.target_price : 0;
                      // Format USD value (compact: $1.2K, $15K, $1.2M)
                      const formatUsd = (value: number) => {
                        if (value >= 1_000_000) return `$${(value / 1_000_000).toFixed(1)}M`;
                        if (value >= 1_000) return `$${(value / 1_000).toFixed(1)}K`;
                        return `$${value.toFixed(0)}`;
                      };
                      return (
                        <div className="flex flex-col items-end text-xs font-mono">
                          <span className="text-gray-400" title={`Buy depth: ${sourceDepth.toFixed(4)} @ $${opp.source_price.toFixed(2)}`}>
                            B: {sourceDepth > 0 ? <><span>{sourceDepth.toFixed(4)}</span> <span className="text-gray-500">({formatUsd(sourceUsd)})</span></> : "-"}
                          </span>
                          <span className="text-gray-400" title={`Sell depth: ${targetDepth.toFixed(4)} @ $${opp.target_price.toFixed(2)}`}>
                            S: {targetDepth > 0 ? <><span>{targetDepth.toFixed(4)}</span> <span className="text-gray-500">({formatUsd(targetUsd)})</span></> : "-"}
                          </span>
                        </div>
                      );
                    })()}
                  </td>
                  <td className="p-4 text-right">
                    {(() => {
                      const displayPremium = getDisplayPremium(opp);
                      const premiumLabel = getPremiumLabel(opp);
                      return (
                        <div className="flex flex-col items-end">
                          <span
                            className={`font-mono font-bold ${
                              displayPremium >= 50
                                ? "text-success-500"
                                : displayPremium >= 30
                                  ? "text-yellow-500"
                                  : "text-gray-400"
                            } ${getChangeClass(changes?.spread)}`}
                          >
                            {displayPremium >= 0 ? "+" : ""}{(displayPremium / 100).toFixed(2)}%
                          </span>
                          {premiumLabel && (
                            <span className={`text-xs mt-0.5 ${
                              premiumMode === "kimchi" ? "text-yellow-500" : "text-green-500"
                            }`}>
                              {premiumLabel}
                            </span>
                          )}
                        </div>
                      );
                    })()}
                  </td>
                  <td className="p-4 text-center">
                    <button
                      onClick={() => handleExecute(opp.id)}
                      disabled={executing === opp.id}
                      className={`px-3 py-1 rounded text-sm font-medium ${
                        executing === opp.id
                          ? "bg-gray-600 cursor-not-allowed"
                          : "bg-primary-600 hover:bg-primary-500"
                      }`}
                    >
                      {executing === opp.id ? "..." : "Execute"}
                    </button>
                  </td>
                </tr>
                );
              })
            ) : (
              <tr>
                <td colSpan={8} className="p-8 text-center text-gray-400">
                  No opportunities detected. The bot is monitoring price differences
                  across exchanges.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="text-sm text-gray-500">
        Spread shows the price difference between exchanges as a percentage.
      </div>
    </div>
  );
}

export default Opportunities;

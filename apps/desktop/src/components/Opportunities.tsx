import { useState, useEffect, useRef, useMemo } from "react";
import { useOpportunities, usePrices, useWalletStatus } from "../hooks/useTauri";

type OppKey = string;
type ChangeType = "up" | "down" | null;

interface PriceChanges {
  source: ChangeType;
  target: ChangeType;
  spread: ChangeType;
}

const getOppKey = (opp: { symbol: string; source_exchange: string; target_exchange: string }) =>
  `${opp.symbol}-${opp.source_exchange}-${opp.target_exchange}`;

function Opportunities() {
  const { opportunities: rawOpportunities, executeOpportunity } = useOpportunities();
  const prices = usePrices();
  const walletStatuses = useWalletStatus();
  const [executing, setExecuting] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [priceChanges, setPriceChanges] = useState<Map<OppKey, PriceChanges>>(new Map());
  const prevDataRef = useRef<Map<OppKey, { source: number; target: number; spread: number }>>(new Map());
  const [minVolume, setMinVolume] = useState<number>(0);
  const [searchQuery, setSearchQuery] = useState("");

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

  // Build volume map by symbol
  const volumeBySymbol = useMemo(() => {
    const map: Record<string, number> = {};
    for (const price of prices) {
      if (!map[price.symbol]) {
        map[price.symbol] = 0;
      }
      map[price.symbol] += price.volume_24h || 0;
    }
    return map;
  }, [prices]);

  // Sort and filter opportunities
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
        // Volume filter
        if (minVolume > 0) {
          const volume = volumeBySymbol[opp.symbol] || 0;
          if (volume < minVolume) return false;
        }
        return true;
      })
      .sort((a, b) => b.premium_bps - a.premium_bps);
  }, [rawOpportunities, searchQuery, minVolume, volumeBySymbol]);

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
    if (price >= 10000) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      });
    }
    return price.toLocaleString("en-US", {
      minimumFractionDigits: 4,
      maximumFractionDigits: 4,
    });
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h2 className="text-lg font-semibold">Arbitrage Opportunities</h2>
        <div className="flex items-center gap-4">
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
            <col className="w-24" />
            <col className="w-48" />
            <col className="w-24" />
            <col className="w-28" />
            <col className="w-28" />
            <col className="w-24" />
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
              <th className="text-right text-gray-400 text-sm p-4">Spread</th>
              <th className="text-center text-gray-400 text-sm p-4">Confidence</th>
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
                      <span className="text-success-500 font-medium">
                        {opp.source_exchange}
                      </span>
                      <span className="text-gray-500">→</span>
                      <span className="text-primary-500 font-medium">
                        {opp.target_exchange}
                      </span>
                    </div>
                  </td>
                  <td className="p-4 text-center">
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
                  </td>
                  <td className="p-4 text-right font-mono">
                    <span className={getChangeClass(changes?.source)}>
                      ${formatPrice(opp.source_price)}
                    </span>
                  </td>
                  <td className="p-4 text-right font-mono">
                    <span className={getChangeClass(changes?.target)}>
                      ${formatPrice(opp.target_price)}
                    </span>
                  </td>
                  <td className="p-4 text-right">
                    <span
                      className={`font-mono font-bold ${
                        opp.premium_bps >= 50
                          ? "text-success-500"
                          : opp.premium_bps >= 30
                            ? "text-yellow-500"
                            : "text-gray-400"
                      } ${getChangeClass(changes?.spread)}`}
                    >
                      +{(opp.premium_bps / 100).toFixed(2)}%
                    </span>
                  </td>
                  <td className="p-4 text-center">
                    <div className="flex items-center justify-center">
                      <div
                        className={`w-2 h-2 rounded-full mr-2 ${
                          opp.confidence_score >= 70
                            ? "bg-success-500"
                            : opp.confidence_score >= 40
                              ? "bg-yellow-500"
                              : "bg-danger-500"
                        }`}
                      />
                      <span className="text-gray-400 text-sm">
                        {opp.confidence_score}%
                      </span>
                    </div>
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

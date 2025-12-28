import { useState, useEffect, useMemo } from "react";
import { useCommonMarkets, useWalletInfo } from "../hooks/useTauri";
import type { AssetWalletStatus } from "../types";

type FilterMode = "all" | "common" | "partial";

function Markets() {
  const commonMarkets = useCommonMarkets();
  const { wallets, fetchWallets } = useWalletInfo();
  const [searchTerm, setSearchTerm] = useState("");
  const [filterMode, setFilterMode] = useState<FilterMode>("all");

  // Fetch wallet status on mount
  useEffect(() => {
    fetchWallets();
  }, [fetchWallets]);

  // Build lookup map: exchange -> asset -> status
  const walletStatusMap = useMemo(() => {
    const map: Record<string, Record<string, AssetWalletStatus>> = {};
    for (const wallet of wallets) {
      map[wallet.exchange] = {};
      for (const status of wallet.wallet_status) {
        map[wallet.exchange][status.asset] = status;
      }
    }
    return map;
  }, [wallets]);

  if (!commonMarkets) {
    return (
      <div className="space-y-4">
        <h2 className="text-lg font-semibold">Common Markets</h2>
        <div className="bg-dark-800 rounded-lg border border-dark-700 p-8 text-center text-gray-400">
          Loading markets from exchanges...
        </div>
      </div>
    );
  }

  const exchangeCount = commonMarkets.exchanges.length;

  // Calculate exchange count for each base
  const basesWithCount = commonMarkets.common_bases.map((base) => ({
    base,
    count: commonMarkets.markets[base]?.length || 0,
  }));

  // Filter and sort
  const filteredBases = basesWithCount
    .filter(({ base }) => base.toLowerCase().includes(searchTerm.toLowerCase()))
    .filter(({ count }) => {
      if (filterMode === "common") return count === exchangeCount;
      if (filterMode === "partial") return count < exchangeCount;
      return true;
    })
    .sort((a, b) => {
      // Sort by count descending, then alphabetically
      if (b.count !== a.count) return b.count - a.count;
      return a.base.localeCompare(b.base);
    });

  const commonCount = basesWithCount.filter((b) => b.count === exchangeCount).length;
  const partialCount = basesWithCount.filter((b) => b.count < exchangeCount).length;

  const timeSince = (ms: number): string => {
    const seconds = Math.floor((Date.now() - ms) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ago`;
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h2 className="text-lg font-semibold">Tradable Markets</h2>
        <div className="flex items-center space-x-4">
          <span className="text-sm text-gray-400">
            {filteredBases.length} of {commonMarkets.common_bases.length} markets
          </span>
          <span className="text-sm text-gray-500">
            Updated {timeSince(commonMarkets.timestamp)}
          </span>
        </div>
      </div>

      {/* Filter buttons */}
      <div className="flex items-center space-x-2">
        <button
          onClick={() => setFilterMode("all")}
          className={`px-3 py-1.5 text-sm rounded-lg transition-colors ${
            filterMode === "all"
              ? "bg-primary-600 text-white"
              : "bg-dark-700 text-gray-400 hover:bg-dark-600"
          }`}
        >
          All ({commonMarkets.common_bases.length})
        </button>
        <button
          onClick={() => setFilterMode("common")}
          className={`px-3 py-1.5 text-sm rounded-lg transition-colors ${
            filterMode === "common"
              ? "bg-green-600 text-white"
              : "bg-dark-700 text-gray-400 hover:bg-dark-600"
          }`}
        >
          All Exchanges ({commonCount})
        </button>
        <button
          onClick={() => setFilterMode("partial")}
          className={`px-3 py-1.5 text-sm rounded-lg transition-colors ${
            filterMode === "partial"
              ? "bg-yellow-600 text-white"
              : "bg-dark-700 text-gray-400 hover:bg-dark-600"
          }`}
        >
          Partial ({partialCount})
        </button>
      </div>

      <div className="flex items-center space-x-4">
        <input
          type="text"
          placeholder="Search markets..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="flex-1 bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-sm focus:outline-none focus:border-primary-500"
        />
        <div className="text-sm text-gray-400">
          Exchanges: {commonMarkets.exchanges.join(", ")}
        </div>
      </div>

      <div className="bg-dark-800 rounded-lg border border-dark-700 overflow-hidden">
        <table className="w-full">
          <thead className="bg-dark-700">
            <tr>
              <th className="text-left text-gray-400 text-sm p-4">Asset</th>
              <th className="text-left text-gray-400 text-sm p-4">Coverage</th>
              {commonMarkets.exchanges.map((exchange) => (
                <th key={exchange} className="text-left text-gray-400 text-sm p-4">
                  {exchange}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-dark-700">
            {filteredBases.length > 0 ? (
              filteredBases.map(({ base, count }) => {
                const markets = commonMarkets.markets[base] || [];
                const isComplete = count === exchangeCount;
                return (
                  <tr key={base} className="hover:bg-dark-700/50">
                    <td className="p-4">
                      <span className="text-primary-400 font-bold">{base}</span>
                    </td>
                    <td className="p-4">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                          isComplete
                            ? "bg-green-900/50 text-green-400"
                            : "bg-yellow-900/50 text-yellow-400"
                        }`}
                      >
                        {count}/{exchangeCount}
                      </span>
                    </td>
                    {commonMarkets.exchanges.map((exchange) => {
                      const market = markets.find((m) => m.exchange === exchange);
                      const status = walletStatusMap[exchange]?.[base];
                      return (
                        <td key={exchange} className="p-4">
                          {market ? (
                            <div className="flex items-center space-x-2">
                              <span className="text-gray-300 font-mono text-sm">
                                {market.symbol}
                              </span>
                              {status && (
                                <span className="flex items-center space-x-0.5 text-xs">
                                  <span
                                    className={status.can_deposit ? "text-success-500" : "text-danger-500"}
                                    title={status.can_deposit ? "Deposit OK" : "Deposit Disabled"}
                                  >
                                    D
                                  </span>
                                  <span className="text-gray-600">/</span>
                                  <span
                                    className={status.can_withdraw ? "text-success-500" : "text-danger-500"}
                                    title={status.can_withdraw ? "Withdraw OK" : "Withdraw Disabled"}
                                  >
                                    W
                                  </span>
                                </span>
                              )}
                            </div>
                          ) : (
                            <span className="text-gray-600">-</span>
                          )}
                        </td>
                      );
                    })}
                  </tr>
                );
              })
            ) : (
              <tr>
                <td
                  colSpan={commonMarkets.exchanges.length + 2}
                  className="p-8 text-center text-gray-400"
                >
                  No markets found matching "{searchTerm}"
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="text-sm text-gray-500 space-y-1">
        <div>Markets available on 2 or more exchanges. Green badge = available on all exchanges.</div>
        <div>
          <span className="text-success-500">D</span>/<span className="text-success-500">W</span> = Deposit/Withdraw enabled,{" "}
          <span className="text-danger-500">D</span>/<span className="text-danger-500">W</span> = disabled
        </div>
      </div>
    </div>
  );
}

export default Markets;

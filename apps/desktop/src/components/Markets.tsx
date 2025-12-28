import { useState, useEffect, useMemo } from "react";
import { useCommonMarkets, useWalletInfo } from "../hooks/useTauri";
import type { AssetWalletStatus, NetworkStatus } from "../types";

type FilterMode = "all" | "common" | "partial";
type WalletStatusType = "normal" | "partial" | "suspended" | "unknown";

// Calculate wallet status based on all networks
function getWalletStatus(status: AssetWalletStatus | undefined): {
  type: WalletStatusType;
  depositNetworks: NetworkStatus[];
  withdrawNetworks: NetworkStatus[];
} {
  if (!status || status.networks.length === 0) {
    return { type: "unknown", depositNetworks: [], withdrawNetworks: [] };
  }

  const depositNetworks = status.networks.filter((n) => n.deposit_enabled);
  const withdrawNetworks = status.networks.filter((n) => n.withdraw_enabled);
  const totalNetworks = status.networks.length;

  const allDeposit = depositNetworks.length === totalNetworks;
  const allWithdraw = withdrawNetworks.length === totalNetworks;
  const noDeposit = depositNetworks.length === 0;
  const noWithdraw = withdrawNetworks.length === 0;

  let type: WalletStatusType;
  if (allDeposit && allWithdraw) {
    type = "normal";
  } else if (noDeposit && noWithdraw) {
    type = "suspended";
  } else {
    type = "partial";
  }

  return { type, depositNetworks, withdrawNetworks };
}

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
                      const walletInfo = getWalletStatus(status);

                      const statusConfig = {
                        normal: {
                          label: "Normal",
                          className: "bg-success-500/20 text-success-400 border-success-500/30",
                        },
                        partial: {
                          label: "Partial",
                          className: "bg-yellow-500/20 text-yellow-400 border-yellow-500/30",
                        },
                        suspended: {
                          label: "Suspended",
                          className: "bg-danger-500/20 text-danger-400 border-danger-500/30",
                        },
                        unknown: {
                          label: "-",
                          className: "bg-gray-500/20 text-gray-500 border-gray-500/30",
                        },
                      };

                      const config = statusConfig[walletInfo.type];
                      const totalNetworks = status?.networks.length || 0;

                      // Build tooltip with network details
                      const tooltipLines: string[] = [];
                      if (status && status.networks.length > 0) {
                        tooltipLines.push(`Networks (${totalNetworks}):`);
                        status.networks.forEach((n) => {
                          const d = n.deposit_enabled ? "D" : "-";
                          const w = n.withdraw_enabled ? "W" : "-";
                          tooltipLines.push(`  ${n.name}: ${d}/${w}`);
                        });
                        tooltipLines.push("");
                        tooltipLines.push(
                          `Deposit: ${walletInfo.depositNetworks.length}/${totalNetworks}`
                        );
                        tooltipLines.push(
                          `Withdraw: ${walletInfo.withdrawNetworks.length}/${totalNetworks}`
                        );
                      }

                      return (
                        <td key={exchange} className="p-4">
                          {market ? (
                            <div className="flex flex-col gap-1">
                              <span className="text-gray-300 font-mono text-sm">
                                {market.symbol}
                              </span>
                              <span
                                className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs border ${config.className}`}
                                title={tooltipLines.join("\n")}
                              >
                                {walletInfo.type !== "unknown" && (
                                  <span className="mr-1 opacity-70">
                                    {walletInfo.depositNetworks.length}/{totalNetworks}
                                  </span>
                                )}
                                {config.label}
                              </span>
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
        <div className="flex items-center gap-4">
          <span>
            <span className="text-success-400">Normal</span> = All networks OK
          </span>
          <span>
            <span className="text-yellow-400">Partial</span> = Some networks suspended
          </span>
          <span>
            <span className="text-danger-400">Suspended</span> = All networks suspended
          </span>
        </div>
        <div className="text-gray-600">Hover over status badge to see network details</div>
      </div>
    </div>
  );
}

export default Markets;

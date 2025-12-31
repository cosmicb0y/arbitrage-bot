import { useState, useMemo, Fragment, useEffect, useRef } from "react";
import { useCommonMarkets, useWalletStatus } from "../hooks/useTauri";
import type { AssetWalletStatus, NetworkStatus } from "../types";

type FilterMode = "all" | "common" | "partial";
type StatusType = "normal" | "partial" | "suspended" | "unknown";

// Calculate deposit/withdraw status separately
function getDepositWithdrawStatus(status: AssetWalletStatus | undefined): {
  depositStatus: StatusType;
  withdrawStatus: StatusType;
  depositNetworks: NetworkStatus[];
  withdrawNetworks: NetworkStatus[];
  allNetworks: NetworkStatus[];
} {
  if (!status || status.networks.length === 0) {
    return {
      depositStatus: "unknown",
      withdrawStatus: "unknown",
      depositNetworks: [],
      withdrawNetworks: [],
      allNetworks: [],
    };
  }

  const depositNetworks = status.networks.filter((n) => n.deposit_enabled);
  const withdrawNetworks = status.networks.filter((n) => n.withdraw_enabled);
  const totalNetworks = status.networks.length;

  // Deposit status
  let depositStatus: StatusType;
  if (depositNetworks.length === totalNetworks) {
    depositStatus = "normal";
  } else if (depositNetworks.length === 0) {
    depositStatus = "suspended";
  } else {
    depositStatus = "partial";
  }

  // Withdraw status
  let withdrawStatus: StatusType;
  if (withdrawNetworks.length === totalNetworks) {
    withdrawStatus = "normal";
  } else if (withdrawNetworks.length === 0) {
    withdrawStatus = "suspended";
  } else {
    withdrawStatus = "partial";
  }

  return {
    depositStatus,
    withdrawStatus,
    depositNetworks,
    withdrawNetworks,
    allNetworks: status.networks,
  };
}

// Status badge component
function StatusBadge({ status }: { status: StatusType }) {
  const config = {
    normal: {
      label: "Normal",
      bgClass: "bg-success-500/20",
      textClass: "text-success-400",
      icon: "✓",
    },
    partial: {
      label: "Partial",
      bgClass: "bg-yellow-500/20",
      textClass: "text-yellow-400",
      icon: "!",
    },
    suspended: {
      label: "Suspended",
      bgClass: "bg-danger-500/20",
      textClass: "text-danger-400",
      icon: "✕",
    },
    unknown: {
      label: "-",
      bgClass: "bg-gray-500/20",
      textClass: "text-gray-500",
      icon: "",
    },
  };

  const c = config[status];
  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs ${c.bgClass} ${c.textClass}`}
    >
      {c.icon && <span>{c.icon}</span>}
      {c.label}
    </span>
  );
}

function Markets() {
  const commonMarkets = useCommonMarkets();
  // Use WebSocket-based wallet status (auto-updates every 5 minutes from server)
  const walletStatuses = useWalletStatus();
  const [searchTerm, setSearchTerm] = useState("");
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  // Track expanded rows: "asset-exchange" format
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set());
  // Track which exchanges to show (empty = all)
  const [selectedExchanges, setSelectedExchanges] = useState<Set<string>>(new Set());
  const [showExchangeFilter, setShowExchangeFilter] = useState(false);
  const filterRef = useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (filterRef.current && !filterRef.current.contains(event.target as Node)) {
        setShowExchangeFilter(false);
      }
    };
    if (showExchangeFilter) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [showExchangeFilter]);

  const toggleRow = (key: string) => {
    setExpandedRows((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  const toggleExchange = (exchange: string) => {
    setSelectedExchanges((prev) => {
      const next = new Set(prev);
      if (next.has(exchange)) {
        next.delete(exchange);
      } else {
        next.add(exchange);
      }
      return next;
    });
  };

  const selectAllExchanges = () => {
    setSelectedExchanges(new Set());
  };

  const clearAllExchanges = () => {
    if (commonMarkets) {
      setSelectedExchanges(new Set(commonMarkets.exchanges));
    }
  };

  // Build lookup map: exchange -> asset -> status
  const walletStatusMap = useMemo(() => {
    const map: Record<string, Record<string, AssetWalletStatus>> = {};
    for (const status of walletStatuses) {
      map[status.exchange] = {};
      for (const assetStatus of status.wallet_status) {
        map[status.exchange][assetStatus.asset] = assetStatus;
      }
    }
    return map;
  }, [walletStatuses]);

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

  // Filter exchanges to display
  const visibleExchanges = useMemo(() => {
    if (selectedExchanges.size === 0) {
      return commonMarkets.exchanges;
    }
    return commonMarkets.exchanges.filter((e) => !selectedExchanges.has(e));
  }, [commonMarkets.exchanges, selectedExchanges]);

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
        <div className="relative" ref={filterRef}>
          <button
            onClick={() => setShowExchangeFilter(!showExchangeFilter)}
            className={`px-3 py-2 text-sm rounded-lg border transition-colors flex items-center gap-2 ${
              selectedExchanges.size > 0
                ? "bg-primary-600 border-primary-500 text-white"
                : "bg-dark-700 border-dark-600 text-gray-400 hover:bg-dark-600"
            }`}
          >
            <span>Exchanges</span>
            <span className="text-xs bg-dark-600 px-1.5 py-0.5 rounded">
              {visibleExchanges.length}/{exchangeCount}
            </span>
            <span className={`transition-transform ${showExchangeFilter ? "rotate-180" : ""}`}>
              ▼
            </span>
          </button>
          {showExchangeFilter && (
            <div className="absolute right-0 mt-2 w-56 bg-dark-800 border border-dark-600 rounded-lg shadow-xl z-50">
              <div className="p-2 border-b border-dark-600 flex justify-between">
                <button
                  onClick={selectAllExchanges}
                  className="text-xs text-primary-400 hover:text-primary-300"
                >
                  Show All
                </button>
                <button
                  onClick={clearAllExchanges}
                  className="text-xs text-gray-400 hover:text-gray-300"
                >
                  Hide All
                </button>
              </div>
              <div className="p-2 space-y-1 max-h-64 overflow-y-auto">
                {commonMarkets.exchanges.map((exchange) => {
                  const isVisible = !selectedExchanges.has(exchange);
                  return (
                    <label
                      key={exchange}
                      className="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-dark-700 cursor-pointer"
                    >
                      <input
                        type="checkbox"
                        checked={isVisible}
                        onChange={() => toggleExchange(exchange)}
                        className="w-4 h-4 rounded border-dark-500 bg-dark-600 text-primary-500 focus:ring-primary-500 focus:ring-offset-0"
                      />
                      <span className={isVisible ? "text-gray-200" : "text-gray-500"}>
                        {exchange}
                      </span>
                    </label>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="bg-dark-800 rounded-lg border border-dark-700 overflow-hidden overflow-x-auto">
        <table className="w-full table-fixed">
          <thead className="bg-dark-700">
            <tr>
              <th className="text-left text-gray-400 text-sm p-4 w-24">Asset</th>
              <th className="text-left text-gray-400 text-sm p-4 w-20">Coverage</th>
              {visibleExchanges.map((exchange) => (
                <th key={exchange} className="text-center text-gray-400 text-sm p-4 w-40" colSpan={2}>
                  <div>{exchange}</div>
                  <div className="flex justify-center gap-4 mt-1 text-xs font-normal">
                    <span className="w-16 text-center">Deposit</span>
                    <span className="w-16 text-center">Withdraw</span>
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-dark-700">
            {filteredBases.length > 0 ? (
              filteredBases.map(({ base, count }) => {
                const markets = commonMarkets.markets[base] || [];
                const isComplete = count === exchangeCount;

                // Collect status info for visible exchanges
                const exchangeStatusInfo = visibleExchanges.map((exchange) => {
                  const market = markets.find((m) => m.exchange === exchange);
                  const status = walletStatusMap[exchange]?.[base];
                  const info = getDepositWithdrawStatus(status);
                  return { exchange, market, status, info };
                });

                // Check if any exchange has networks to expand
                const hasNetworks = exchangeStatusInfo.some(
                  (e) => e.info.allNetworks.length > 0
                );

                return (
                  <Fragment key={base}>
                    <tr
                      className={`hover:bg-dark-700/50 ${hasNetworks ? "cursor-pointer" : ""}`}
                      onClick={() => hasNetworks && toggleRow(base)}
                    >
                      <td className="p-4">
                        <div className="flex items-center gap-2">
                          {hasNetworks && (
                            <span
                              className={`text-gray-500 transition-transform ${
                                expandedRows.has(base) ? "rotate-90" : ""
                              }`}
                            >
                              ▶
                            </span>
                          )}
                          <span className="text-primary-400 font-bold">{base}</span>
                        </div>
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
                      {exchangeStatusInfo.map(({ exchange, market, info }) => {
                        if (!market) {
                          return (
                            <Fragment key={exchange}>
                              <td className="p-4 text-center w-20">
                                <span className="text-gray-600">-</span>
                              </td>
                              <td className="p-4 text-center w-20">
                                <span className="text-gray-600">-</span>
                              </td>
                            </Fragment>
                          );
                        }

                        return (
                          <Fragment key={exchange}>
                            <td className="p-4 text-center w-20">
                              <StatusBadge status={info.depositStatus} />
                            </td>
                            <td className="p-4 text-center w-20">
                              <StatusBadge status={info.withdrawStatus} />
                            </td>
                          </Fragment>
                        );
                      })}
                    </tr>
                    {/* Expanded network details */}
                    {expandedRows.has(base) && (
                      <tr key={`${base}-expanded`} className="bg-dark-750">
                        <td colSpan={2 + visibleExchanges.length * 2} className="p-0">
                          <div className="px-6 py-3 bg-dark-850">
                            <div className="text-xs text-gray-400 mb-2">Network Details</div>
                            <div className="grid gap-4" style={{ gridTemplateColumns: `repeat(${visibleExchanges.length}, 1fr)` }}>
                              {exchangeStatusInfo.map(({ exchange, market, info }) => (
                                <div key={exchange} className="space-y-1">
                                  <div className="text-xs font-medium text-gray-300 mb-2">
                                    {exchange}
                                  </div>
                                  {!market ? (
                                    <div className="text-xs text-gray-600">Not available</div>
                                  ) : info.allNetworks.length === 0 ? (
                                    <div className="text-xs text-gray-600">No network info</div>
                                  ) : (
                                    info.allNetworks.map((network) => {
                                    // Show network ID if name differs from ID (helps distinguish duplicates)
                                    const showNetworkId = network.name !== network.network;
                                    return (
                                      <div
                                        key={network.network}
                                        className="flex items-center justify-between text-xs bg-dark-700 rounded px-2 py-1.5"
                                      >
                                        <span className="text-gray-300">
                                          {network.name}
                                          {showNetworkId && (
                                            <span className="text-gray-500 ml-1">({network.network})</span>
                                          )}
                                        </span>
                                        <div className="flex items-center gap-3">
                                          <span
                                            className={`px-1.5 py-0.5 rounded ${
                                              network.deposit_enabled
                                                ? "bg-success-500/20 text-success-400"
                                                : "bg-danger-500/20 text-danger-400"
                                            }`}
                                          >
                                            D:{network.deposit_enabled ? "✓" : "✕"}
                                          </span>
                                          <span
                                            className={`px-1.5 py-0.5 rounded ${
                                              network.withdraw_enabled
                                                ? "bg-success-500/20 text-success-400"
                                                : "bg-danger-500/20 text-danger-400"
                                            }`}
                                          >
                                            W:{network.withdraw_enabled ? "✓" : "✕"}
                                          </span>
                                        </div>
                                      </div>
                                    );
                                    })
                                  )}
                                </div>
                              ))}
                            </div>
                          </div>
                        </td>
                      </tr>
                    )}
                  </Fragment>
                );
              })
            ) : (
              <tr>
                <td
                  colSpan={2 + visibleExchanges.length * 2}
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
        <div className="text-gray-600">Click on a row to expand network details</div>
      </div>
    </div>
  );
}

export default Markets;

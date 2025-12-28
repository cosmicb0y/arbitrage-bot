import { useState, useEffect } from "react";
import { useWalletInfo } from "../hooks/useTauri";
import type { ExchangeWalletInfo } from "../types";

function Wallets() {
  const { wallets, loading, error, fetchWallets } = useWalletInfo();
  const [selectedExchange, setSelectedExchange] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [showOnlyWithBalance, setShowOnlyWithBalance] = useState(true);

  useEffect(() => {
    fetchWallets();
  }, [fetchWallets]);

  const formatBalance = (value: number): string => {
    if (value === 0) return "0";
    if (value < 0.00001) return value.toExponential(2);
    if (value < 1) return value.toFixed(6);
    if (value < 1000) return value.toFixed(4);
    return value.toLocaleString("en-US", { maximumFractionDigits: 2 });
  };

  const getStatusColor = (canDeposit: boolean, canWithdraw: boolean): string => {
    if (canDeposit && canWithdraw) return "text-success-500";
    if (canDeposit || canWithdraw) return "text-yellow-500";
    return "text-danger-500";
  };

  const getStatusText = (canDeposit: boolean, canWithdraw: boolean): string => {
    if (canDeposit && canWithdraw) return "Active";
    if (canDeposit) return "Deposit Only";
    if (canWithdraw) return "Withdraw Only";
    return "Suspended";
  };

  const selectedWallet = selectedExchange
    ? wallets.find((w) => w.exchange === selectedExchange)
    : null;

  // Filter and combine data for display
  const getFilteredAssets = (wallet: ExchangeWalletInfo) => {
    const query = searchQuery.toUpperCase();
    const balanceMap = new Map(wallet.balances.map((b) => [b.asset, b]));
    const statusMap = new Map(wallet.wallet_status.map((s) => [s.asset, s]));

    // Get all unique assets
    const allAssets = new Set([
      ...wallet.balances.map((b) => b.asset),
      ...wallet.wallet_status.map((s) => s.asset),
    ]);

    return Array.from(allAssets)
      .filter((asset) => {
        if (query && !asset.includes(query)) return false;
        if (showOnlyWithBalance) {
          const balance = balanceMap.get(asset);
          if (!balance || balance.total === 0) return false;
        }
        return true;
      })
      .map((asset) => ({
        asset,
        balance: balanceMap.get(asset),
        status: statusMap.get(asset),
      }))
      .sort((a, b) => {
        // Sort by balance (descending)
        const balA = a.balance?.total || 0;
        const balB = b.balance?.total || 0;
        return balB - balA;
      });
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h2 className="text-lg font-semibold">Wallet & Deposit/Withdraw Status</h2>
        <button
          onClick={fetchWallets}
          disabled={loading}
          className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
            loading
              ? "bg-dark-600 text-gray-500 cursor-not-allowed"
              : "bg-primary-600 hover:bg-primary-500 text-white"
          }`}
        >
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {error && (
        <div className="bg-danger-500/20 border border-danger-500 rounded-lg p-3 text-sm text-danger-500">
          {error}
        </div>
      )}

      {/* Exchange Tabs */}
      <div className="flex space-x-2 border-b border-dark-700">
        {wallets.map((wallet) => (
          <button
            key={wallet.exchange}
            onClick={() => setSelectedExchange(wallet.exchange)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              selectedExchange === wallet.exchange
                ? "border-primary-500 text-primary-400"
                : "border-transparent text-gray-400 hover:text-white"
            }`}
          >
            {wallet.exchange}
            <span className="ml-2 text-xs text-gray-500">
              ({wallet.balances.length} assets)
            </span>
          </button>
        ))}
        {wallets.length === 0 && !loading && (
          <div className="px-4 py-2 text-sm text-gray-500">
            No wallets configured. Add API credentials in Settings.
          </div>
        )}
      </div>

      {/* Filters */}
      {selectedWallet && (
        <div className="flex gap-4 items-center">
          <input
            type="text"
            placeholder="Search assets..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="flex-1 max-w-xs px-3 py-2 bg-dark-700 border border-dark-600 rounded text-sm text-white placeholder-gray-500 focus:outline-none focus:border-primary-500"
          />
          <label className="flex items-center gap-2 text-sm text-gray-400">
            <input
              type="checkbox"
              checked={showOnlyWithBalance}
              onChange={(e) => setShowOnlyWithBalance(e.target.checked)}
              className="rounded bg-dark-700 border-dark-600 text-primary-500 focus:ring-primary-500"
            />
            Show only with balance
          </label>
        </div>
      )}

      {/* Wallet Content */}
      {selectedWallet && (
        <div className="bg-dark-800 rounded-lg border border-dark-700 overflow-hidden">
          <table className="w-full">
            <thead className="bg-dark-700">
              <tr>
                <th className="text-left px-4 py-3 text-sm font-medium text-gray-400">
                  Asset
                </th>
                <th className="text-right px-4 py-3 text-sm font-medium text-gray-400">
                  Available
                </th>
                <th className="text-right px-4 py-3 text-sm font-medium text-gray-400">
                  Locked
                </th>
                <th className="text-right px-4 py-3 text-sm font-medium text-gray-400">
                  Total
                </th>
                <th className="text-center px-4 py-3 text-sm font-medium text-gray-400">
                  Deposit
                </th>
                <th className="text-center px-4 py-3 text-sm font-medium text-gray-400">
                  Withdraw
                </th>
                <th className="text-center px-4 py-3 text-sm font-medium text-gray-400">
                  Status
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-dark-700">
              {getFilteredAssets(selectedWallet).map(({ asset, balance, status }) => (
                <tr key={asset} className="hover:bg-dark-700/50">
                  <td className="px-4 py-3">
                    <span className="font-medium text-white">{asset}</span>
                    {status?.name && status.name !== asset && (
                      <span className="ml-2 text-xs text-gray-500">{status.name}</span>
                    )}
                  </td>
                  <td className="px-4 py-3 text-right font-mono text-sm text-gray-300">
                    {balance ? formatBalance(balance.free) : "-"}
                  </td>
                  <td className="px-4 py-3 text-right font-mono text-sm text-gray-500">
                    {balance ? formatBalance(balance.locked) : "-"}
                  </td>
                  <td className="px-4 py-3 text-right font-mono text-sm text-white">
                    {balance ? formatBalance(balance.total) : "-"}
                  </td>
                  <td className="px-4 py-3 text-center">
                    {status ? (
                      <span
                        className={
                          status.can_deposit ? "text-success-500" : "text-danger-500"
                        }
                      >
                        {status.can_deposit ? "●" : "○"}
                      </span>
                    ) : (
                      <span className="text-gray-500">-</span>
                    )}
                  </td>
                  <td className="px-4 py-3 text-center">
                    {status ? (
                      <span
                        className={
                          status.can_withdraw ? "text-success-500" : "text-danger-500"
                        }
                      >
                        {status.can_withdraw ? "●" : "○"}
                      </span>
                    ) : (
                      <span className="text-gray-500">-</span>
                    )}
                  </td>
                  <td className="px-4 py-3 text-center">
                    {status ? (
                      <span
                        className={`text-xs ${getStatusColor(
                          status.can_deposit,
                          status.can_withdraw
                        )}`}
                      >
                        {getStatusText(status.can_deposit, status.can_withdraw)}
                      </span>
                    ) : (
                      <span className="text-xs text-gray-500">Unknown</span>
                    )}
                  </td>
                </tr>
              ))}
              {getFilteredAssets(selectedWallet).length === 0 && (
                <tr>
                  <td colSpan={7} className="px-4 py-8 text-center text-gray-500">
                    No assets found
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      {/* Network Details Modal/Section could be added here */}
      {selectedWallet && (
        <div className="text-xs text-gray-500">
          Last updated:{" "}
          {new Date(selectedWallet.last_updated).toLocaleString()}
        </div>
      )}
    </div>
  );
}

export default Wallets;

import { useStats, useBotControl, useDebugStats } from "../hooks/useTauri";

interface HeaderProps {
  activeTab: string;
  onTabChange: (tab: "dashboard" | "opportunities" | "markets" | "wallets" | "settings") => void;
}

function Header({ activeTab, onTabChange }: HeaderProps) {
  const stats = useStats();
  const { start, stop } = useBotControl();
  const { stats: debugStats, fetchStats } = useDebugStats();

  const formatUptime = (secs: number): string => {
    const hours = Math.floor(secs / 3600);
    const mins = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    return `${hours.toString().padStart(2, "0")}:${mins.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  };

  return (
    <header className="bg-dark-800 border-b border-dark-700 px-4 py-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-6">
          <h1 className="text-xl font-bold text-primary-500">Arbitrage Bot</h1>

          <nav className="flex space-x-1">
            {(["dashboard", "opportunities", "markets", "wallets", "settings"] as const).map(
              (tab) => (
                <button
                  key={tab}
                  onClick={() => onTabChange(tab)}
                  className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                    activeTab === tab
                      ? "bg-primary-600 text-white"
                      : "text-gray-400 hover:text-white hover:bg-dark-700"
                  }`}
                >
                  {tab.charAt(0).toUpperCase() + tab.slice(1)}
                </button>
              )
            )}
          </nav>
        </div>

        <div className="flex items-center space-x-6">
          {/* Stats */}
          <div className="flex items-center space-x-4 text-sm">
            <div className="text-gray-400">
              Uptime:{" "}
              <span className="text-white font-mono">
                {formatUptime(stats.uptime_secs)}
              </span>
            </div>
            <div className="text-gray-400">
              Prices:{" "}
              <span className="text-white font-mono">
                {stats.price_updates.toLocaleString()}
              </span>
            </div>
            <div className="text-gray-400">
              Opps:{" "}
              <span className="text-success-500 font-mono">
                {stats.opportunities_detected}
              </span>
            </div>
            <div className="text-gray-400">
              Trades:{" "}
              <span className="text-primary-500 font-mono">
                {stats.trades_executed}
              </span>
            </div>
          </div>

          {/* Bot Control */}
          <button
            onClick={() => (stats.is_running ? stop() : start())}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              stats.is_running
                ? "bg-danger-600 hover:bg-danger-500 text-white"
                : "bg-success-600 hover:bg-success-500 text-white"
            }`}
          >
            {stats.is_running ? "Stop" : "Start"}
          </button>

          {/* Status Indicator */}
          <div className="flex items-center space-x-2">
            <div
              className={`w-3 h-3 rounded-full ${
                stats.is_running ? "bg-success-500 animate-pulse" : "bg-gray-500"
              }`}
            />
            <span className="text-sm text-gray-400">
              {stats.is_running ? "Running" : "Stopped"}
            </span>
          </div>

          {/* Debug Button */}
          <button
            onClick={fetchStats}
            className="px-2 py-1 text-xs bg-dark-700 hover:bg-dark-600 text-gray-400 rounded"
            title="Check debug stats (see console)"
          >
            Debug
          </button>
        </div>
      </div>

      {/* Debug Stats Panel (shown after clicking Debug button) */}
      {debugStats && (
        <div className="bg-dark-900 border-t border-dark-700 px-4 py-2 text-xs font-mono text-gray-400">
          <span className="mr-4">Backend: prices={debugStats.prices_count}</span>
          <span className="mr-4">opportunities={debugStats.opportunities_count}</span>
          <span className="mr-4">messages={debugStats.message_count.toLocaleString()}</span>
          <span className="mr-4">common_markets={debugStats.has_common_markets ? "yes" : "no"}</span>
          <span>wallet_status={debugStats.has_wallet_status ? "yes" : "no"}</span>
        </div>
      )}
    </header>
  );
}

export default Header;

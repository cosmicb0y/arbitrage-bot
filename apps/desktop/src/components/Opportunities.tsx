import { useState } from "react";
import { useOpportunities } from "../hooks/useTauri";

function Opportunities() {
  const { opportunities, executeOpportunity } = useOpportunities();
  const [executing, setExecuting] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);

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
        <h2 className="text-lg font-semibold">Arbitrage Opportunities</h2>
        <span className="text-sm text-gray-400">
          {opportunities.length} opportunities found
        </span>
      </div>

      {message && (
        <div className="bg-primary-600/20 border border-primary-500 rounded-lg p-3 text-sm">
          {message}
        </div>
      )}

      <div className="bg-dark-800 rounded-lg border border-dark-700 overflow-hidden">
        <table className="w-full">
          <thead className="bg-dark-700">
            <tr>
              <th className="text-left text-gray-400 text-sm p-4">Route</th>
              <th className="text-right text-gray-400 text-sm p-4">Buy Price</th>
              <th className="text-right text-gray-400 text-sm p-4">Sell Price</th>
              <th className="text-right text-gray-400 text-sm p-4">Spread</th>
              <th className="text-right text-gray-400 text-sm p-4">Time</th>
              <th className="text-center text-gray-400 text-sm p-4">Action</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-dark-700">
            {opportunities.length > 0 ? (
              opportunities.map((opp, index) => (
                <tr key={`${opp.id}-${index}`} className="hover:bg-dark-700/50">
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
                  <td className="p-4 text-right font-mono">
                    ${formatPrice(opp.source_price)}
                  </td>
                  <td className="p-4 text-right font-mono">
                    ${formatPrice(opp.target_price)}
                  </td>
                  <td className="p-4 text-right">
                    <span
                      className={`font-mono font-bold ${
                        opp.premium_bps >= 50
                          ? "text-success-500"
                          : opp.premium_bps >= 30
                            ? "text-yellow-500"
                            : "text-gray-400"
                      }`}
                    >
                      +{(opp.premium_bps / 100).toFixed(2)}%
                    </span>
                  </td>
                  <td className="p-4 text-right text-gray-400 text-sm">
                    {timeSince(opp.timestamp)}
                  </td>
                  <td className="p-4 text-center">
                    <button
                      onClick={() => handleExecute(opp.id)}
                      disabled={executing === opp.id}
                      className={`px-3 py-1 rounded text-sm font-medium transition-colors ${
                        executing === opp.id
                          ? "bg-gray-600 cursor-not-allowed"
                          : "bg-primary-600 hover:bg-primary-500"
                      }`}
                    >
                      {executing === opp.id ? "..." : "Execute"}
                    </button>
                  </td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan={6} className="p-8 text-center text-gray-400">
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

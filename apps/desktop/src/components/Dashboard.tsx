import { useMemo, useState } from "react";
import { usePrices, useOpportunities, useExchangeRate, useCommonMarkets } from "../hooks/useTauri";
import type { PriceData } from "../types";

function Dashboard() {
  const prices = usePrices();
  const { opportunities } = useOpportunities();
  const exchangeRate = useExchangeRate();
  const commonMarkets = useCommonMarkets();
  const [selectedSymbol, setSelectedSymbol] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [minVolume, setMinVolume] = useState<number>(0);

  // Group prices by symbol
  const pricesBySymbol = useMemo(() => {
    const grouped: Record<string, PriceData[]> = {};
    for (const price of prices) {
      if (!grouped[price.symbol]) {
        grouped[price.symbol] = [];
      }
      grouped[price.symbol].push(price);
    }
    return grouped;
  }, [prices]);

  // All available symbols (from commonMarkets, with price data merged)
  const allSymbols = useMemo(() => {
    const symbolSet = new Set<string>();
    // Add symbols from commonMarkets (common_bases are the symbols)
    if (commonMarkets) {
      for (const symbol of commonMarkets.common_bases) {
        symbolSet.add(symbol);
      }
    }
    // Also add symbols from prices (in case some are not in commonMarkets)
    for (const symbol of Object.keys(pricesBySymbol)) {
      symbolSet.add(symbol);
    }
    return symbolSet;
  }, [commonMarkets, pricesBySymbol]);

  // Helper to calculate total volume for a symbol
  const getSymbolVolume = (symbol: string): number => {
    const prices = pricesBySymbol[symbol];
    if (!prices) return 0;
    return prices.reduce((sum, p) => sum + (p.volume_24h || 0), 0);
  };

  // Available symbols sorted by spread (descending) and filtered by search and volume
  // Includes symbols from commonMarkets even if no price data yet
  const symbols = useMemo(() => {
    const query = searchQuery.toUpperCase();
    return Array.from(allSymbols)
      .filter((symbol) => {
        if (!symbol.toUpperCase().includes(query)) return false;
        if (minVolume > 0) {
          const totalVolume = getSymbolVolume(symbol);
          if (totalVolume < minVolume) return false;
        }
        return true;
      })
      .sort((a, b) => {
        const pricesA = pricesBySymbol[a];
        const pricesB = pricesBySymbol[b];
        // Symbols with price data come first
        if (pricesA && !pricesB) return -1;
        if (!pricesA && pricesB) return 1;
        if (!pricesA || !pricesB) return a.localeCompare(b);
        // Sort by spread (descending)
        const maxA = Math.max(...pricesA.map(p => p.price));
        const minA = Math.min(...pricesA.map(p => p.price));
        const maxB = Math.max(...pricesB.map(p => p.price));
        const minB = Math.min(...pricesB.map(p => p.price));
        const spreadA = minA > 0 ? (maxA - minA) / minA : 0;
        const spreadB = minB > 0 ? (maxB - minB) / minB : 0;
        return spreadB - spreadA;
      });
  }, [allSymbols, pricesBySymbol, searchQuery, minVolume]);

  // Auto-select first symbol if none selected
  const activeSymbol = selectedSymbol && allSymbols.has(selectedSymbol)
    ? selectedSymbol
    : symbols[0] || null;

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

  const formatVolume = (volume: number): string => {
    if (volume >= 1_000_000_000) {
      return `$${(volume / 1_000_000_000).toFixed(2)}B`;
    }
    if (volume >= 1_000_000) {
      return `$${(volume / 1_000_000).toFixed(2)}M`;
    }
    if (volume >= 1_000) {
      return `$${(volume / 1_000).toFixed(2)}K`;
    }
    return `$${volume.toFixed(2)}`;
  };

  return (
    <div className="space-y-6">
      {/* Exchange Rate Banner */}
      {exchangeRate && (
        <section className="bg-dark-800 rounded-lg p-4 border border-dark-700">
          <div className="flex justify-between items-center">
            <div className="flex items-center space-x-8">
              <div>
                <div className="text-gray-500 text-xs mb-1">Upbit USDT/KRW</div>
                <span className="font-mono text-xl text-primary-400">
                  ₩{exchangeRate.usd_krw.toLocaleString("ko-KR", {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2,
                  })}
                </span>
              </div>
              {exchangeRate.api_rate && (
                <div>
                  <div className="text-gray-500 text-xs mb-1">USD/KRW (API)</div>
                  <span className="font-mono text-xl text-gray-300">
                    ₩{exchangeRate.api_rate.toLocaleString("ko-KR", {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 2,
                    })}
                  </span>
                </div>
              )}
              {exchangeRate.api_rate && (
                <div>
                  <div className="text-gray-500 text-xs mb-1">Premium</div>
                  <span className={`font-mono text-xl ${
                    exchangeRate.usd_krw > exchangeRate.api_rate
                      ? "text-success-500"
                      : "text-danger-500"
                  }`}>
                    {exchangeRate.usd_krw > exchangeRate.api_rate ? "+" : ""}
                    {(((exchangeRate.usd_krw - exchangeRate.api_rate) / exchangeRate.api_rate) * 100).toFixed(2)}%
                  </span>
                </div>
              )}
            </div>
            <span className="text-xs text-gray-500">
              Updated: {new Date(exchangeRate.timestamp).toLocaleTimeString()}
            </span>
          </div>
        </section>
      )}

      {/* Market List + Premium Matrix */}
      <section className="flex gap-4 items-start">
        {/* Left: Market List */}
        <div className="w-64 flex-shrink-0">
          <h2 className="text-lg font-semibold mb-4">Markets</h2>
          <div className="mb-2 space-y-2">
            <input
              type="text"
              placeholder="Search markets..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full px-3 py-2 bg-dark-700 border border-dark-600 rounded text-sm text-white placeholder-gray-500 focus:outline-none focus:border-primary-500"
            />
            <select
              value={minVolume}
              onChange={(e) => setMinVolume(Number(e.target.value))}
              className="w-full px-3 py-2 bg-dark-700 border border-dark-600 rounded text-sm text-white focus:outline-none focus:border-primary-500"
            >
              <option value={0}>All Volumes</option>
              <option value={100000}>Vol &gt; $100K</option>
              <option value={1000000}>Vol &gt; $1M</option>
              <option value={10000000}>Vol &gt; $10M</option>
              <option value={100000000}>Vol &gt; $100M</option>
              <option value={1000000000}>Vol &gt; $1B</option>
            </select>
          </div>
          <div className="bg-dark-800 rounded-lg border border-dark-700 overflow-hidden max-h-[456px] overflow-y-auto">
              {symbols.map((symbol) => {
                const exchangePrices = pricesBySymbol[symbol];
                const hasPriceData = exchangePrices && exchangePrices.length > 0;
                const maxPrice = hasPriceData ? Math.max(...exchangePrices.map(p => p.price)) : 0;
                const minPrice = hasPriceData ? Math.min(...exchangePrices.map(p => p.price)) : 0;
                const spread = maxPrice > 0 ? ((maxPrice - minPrice) / minPrice) * 100 : 0;
                const totalVolume = hasPriceData ? exchangePrices.reduce((sum, p) => sum + (p.volume_24h || 0), 0) : 0;

                return (
                  <button
                    key={symbol}
                    onClick={() => setSelectedSymbol(symbol)}
                    className={`w-full px-4 py-3 text-left transition-colors border-b border-dark-700 last:border-b-0 ${
                      activeSymbol === symbol
                        ? "bg-primary-500/20 border-l-2 border-l-primary-500"
                        : "hover:bg-dark-700"
                    }`}
                  >
                    <div className="flex justify-between items-center">
                      <span className={`font-medium ${activeSymbol === symbol ? "text-primary-400" : hasPriceData ? "text-white" : "text-gray-500"}`}>
                        {symbol}
                      </span>
                      {hasPriceData ? (
                        <span className={`text-sm font-mono ${spread >= 0.5 ? "text-success-500" : "text-gray-500"}`}>
                          {spread.toFixed(2)}%
                        </span>
                      ) : (
                        <span className="text-xs text-gray-600">no data</span>
                      )}
                    </div>
                    {totalVolume > 0 && (
                      <div className="text-xs text-gray-500 mt-1">
                        Vol: {formatVolume(totalVolume)}
                      </div>
                    )}
                  </button>
                );
              })}
          </div>
        </div>

        {/* Right: Premium Matrix + Price Details */}
        <div className="flex-1">
          <h2 className="text-lg font-semibold mb-4">
            Premium Matrix {activeSymbol && <span className="text-primary-400">- {activeSymbol}</span>}
          </h2>
          <div className="bg-dark-800 rounded-lg p-4 border border-dark-700">
            {activeSymbol && pricesBySymbol[activeSymbol] ? (
              <>
                <PremiumMatrix prices={pricesBySymbol[activeSymbol]} />
                <div className="mt-4 pt-4 border-t border-dark-700">
                  <h3 className="text-sm text-gray-400 mb-3">Exchange Prices</h3>
                  <div className="grid grid-cols-3 gap-4">
                    {pricesBySymbol[activeSymbol].map((p) => (
                      <div key={p.exchange} className="text-sm">
                        <span className="text-gray-400">{p.exchange}</span>
                        <div className="font-mono text-white">${formatPrice(p.price)}</div>
                        <div className="text-xs text-gray-500">
                          Bid: ${formatPrice(p.bid)} / Ask: ${formatPrice(p.ask)}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </>
            ) : activeSymbol ? (
              <div className="text-gray-400 text-center py-8">
                <div className="text-lg mb-2">{activeSymbol}</div>
                <div className="text-sm">No live price data available</div>
                <div className="text-xs mt-2 text-gray-500">
                  This market exists on all exchanges but is not in the top 50 by volume.
                </div>
              </div>
            ) : (
              <div className="text-gray-400 text-center py-8">
                Select a market from the list
              </div>
            )}
          </div>
        </div>
      </section>

      {/* Recent Opportunities */}
      <section>
        <h2 className="text-lg font-semibold mb-4">Recent Opportunities</h2>
        <div className="bg-dark-800 rounded-lg border border-dark-700">
          {opportunities.length > 0 ? (
            <div className="divide-y divide-dark-700">
              {opportunities.slice(0, 5).map((opp, index) => (
                <div
                  key={`${opp.id}-${index}`}
                  className="p-4 flex justify-between items-center"
                >
                  <div className="flex items-center space-x-4">
                    <span className="text-primary-400 font-bold min-w-[48px]">
                      {opp.symbol}
                    </span>
                    <div>
                      <span className="text-success-500">{opp.source_exchange}</span>
                      <span className="text-gray-400 mx-2">→</span>
                      <span className="text-primary-500">{opp.target_exchange}</span>
                    </div>
                  </div>
                  <div className="flex items-center space-x-4">
                    <div className="text-gray-400 text-sm">
                      ${formatPrice(opp.source_price)} → ${formatPrice(opp.target_price)}
                    </div>
                    <span
                      className={`font-mono font-bold ${
                        opp.premium_bps >= 50
                          ? "text-success-500"
                          : "text-yellow-500"
                      }`}
                    >
                      +{(opp.premium_bps / 100).toFixed(2)}%
                    </span>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-gray-400 text-center py-8">
              No opportunities detected yet
            </div>
          )}
        </div>
      </section>
    </div>
  );
}

interface PremiumMatrixProps {
  prices: PriceData[];
}

function PremiumMatrix({ prices }: PremiumMatrixProps) {
  const exchanges = prices.map((p) => p.exchange);

  const getPremium = (buyIdx: number, sellIdx: number): number => {
    if (buyIdx === sellIdx) return 0;
    const buyPrice = prices[buyIdx]?.price || 0;
    const sellPrice = prices[sellIdx]?.price || 0;
    if (buyPrice === 0) return 0;
    return Math.round(((sellPrice - buyPrice) / buyPrice) * 10000);
  };

  const getColor = (bps: number): string => {
    if (bps >= 50) return "bg-success-500 text-white";
    if (bps >= 30) return "bg-yellow-500 text-dark-900";
    if (bps > 0) return "bg-dark-700 text-gray-300";
    if (bps < -30) return "bg-danger-500 text-white";
    return "bg-dark-700 text-gray-500";
  };

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr>
            <th className="text-left text-gray-400 text-sm p-2">Buy ↓ / Sell →</th>
            {exchanges.map((ex) => (
              <th key={ex} className="text-center text-gray-400 text-sm p-2">
                {ex}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {exchanges.map((buyEx, buyIdx) => (
            <tr key={buyEx}>
              <td className="text-gray-400 text-sm p-2">{buyEx}</td>
              {exchanges.map((_, sellIdx) => {
                const premium = getPremium(buyIdx, sellIdx);
                return (
                  <td key={sellIdx} className="p-1">
                    <div
                      className={`text-center text-sm font-mono py-2 px-3 rounded ${getColor(premium)}`}
                    >
                      {buyIdx === sellIdx ? "-" : `${premium >= 0 ? "+" : ""}${(premium / 100).toFixed(2)}%`}
                    </div>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default Dashboard;

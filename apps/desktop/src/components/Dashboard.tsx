import { useMemo, useState } from "react";
import { usePrices, useOpportunities, useExchangeRate } from "../hooks/useTauri";
import type { PriceData } from "../types";

function Dashboard() {
  const prices = usePrices();
  const { opportunities } = useOpportunities();
  const exchangeRate = useExchangeRate();
  const [selectedSymbol, setSelectedSymbol] = useState<string | null>(null);

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

  // Available symbols sorted by spread (descending)
  const symbols = useMemo(() => {
    return Object.keys(pricesBySymbol).sort((a, b) => {
      const pricesA = pricesBySymbol[a];
      const pricesB = pricesBySymbol[b];
      const maxA = Math.max(...pricesA.map(p => p.price));
      const minA = Math.min(...pricesA.map(p => p.price));
      const maxB = Math.max(...pricesB.map(p => p.price));
      const minB = Math.min(...pricesB.map(p => p.price));
      const spreadA = minA > 0 ? (maxA - minA) / minA : 0;
      const spreadB = minB > 0 ? (maxB - minB) / minB : 0;
      return spreadB - spreadA;
    });
  }, [pricesBySymbol]);

  // Auto-select first symbol if none selected
  const activeSymbol = selectedSymbol && pricesBySymbol[selectedSymbol]
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
          <div className="bg-dark-800 rounded-lg border border-dark-700 overflow-hidden max-h-[500px] overflow-y-auto">
              {symbols.map((symbol) => {
                const exchangePrices = pricesBySymbol[symbol];
                const maxPrice = Math.max(...exchangePrices.map(p => p.price));
                const minPrice = Math.min(...exchangePrices.map(p => p.price));
                const spread = maxPrice > 0 ? ((maxPrice - minPrice) / minPrice) * 100 : 0;

                return (
                  <button
                    key={symbol}
                    onClick={() => setSelectedSymbol(symbol)}
                    className={`w-full px-4 py-3 flex justify-between items-center text-left transition-colors border-b border-dark-700 last:border-b-0 ${
                      activeSymbol === symbol
                        ? "bg-primary-500/20 border-l-2 border-l-primary-500"
                        : "hover:bg-dark-700"
                    }`}
                  >
                    <span className={`font-medium ${activeSymbol === symbol ? "text-primary-400" : "text-white"}`}>
                      {symbol}
                    </span>
                    <span className={`text-sm font-mono ${spread >= 0.5 ? "text-success-500" : "text-gray-500"}`}>
                      {spread.toFixed(2)}%
                    </span>
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

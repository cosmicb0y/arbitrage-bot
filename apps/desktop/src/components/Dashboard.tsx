import { useMemo, useState } from "react";
import { usePrices, useOpportunities, useExchangeRate, useCommonMarkets } from "../hooks/useTauri";
import type { PriceData, ExchangeRate } from "../types";

type PremiumMode = "kimchi" | "tether";
type QuoteFilter = "all" | "USDT" | "USDC";

/**
 * Generate trading page URL for each exchange
 */
function getExchangeTradeUrl(exchange: string, symbol: string, quote: string): string | null {
  const q = quote || "USD";

  switch (exchange) {
    case "Binance":
      // https://www.binance.com/en/trade/BTC_USDT?type=spot
      return `https://www.binance.com/en/trade/${symbol}_${q}?type=spot`;

    case "Upbit":
      // https://upbit.com/exchange?code=CRIX.UPBIT.KRW-BTC
      return `https://upbit.com/exchange?code=CRIX.UPBIT.${q}-${symbol}`;

    case "Bithumb":
      // https://www.bithumb.com/react/trade/order/BTC-KRW
      return `https://www.bithumb.com/react/trade/order/${symbol}-${q}`;

    case "Coinbase":
      // https://www.coinbase.com/advanced-trade/spot/BTC-USD
      return `https://www.coinbase.com/advanced-trade/spot/${symbol}-${q}`;

    case "Gate.io":
      // https://www.gate.com/trade/BTC_USDT
      return `https://www.gate.com/trade/${symbol}_${q}`;

    case "Bybit":
      // https://www.bybit.com/en/trade/spot/BTC/USDT
      return `https://www.bybit.com/en/trade/spot/${symbol}/${q}`;

    case "Kraken":
      // https://pro.kraken.com/app/trade/btc-usd
      return `https://pro.kraken.com/app/trade/${symbol.toLowerCase()}-${q.toLowerCase()}`;

    case "OKX":
      // https://www.okx.com/trade-spot/btc-usdt
      return `https://www.okx.com/trade-spot/${symbol.toLowerCase()}-${q.toLowerCase()}`;

    default:
      return null;
  }
}

function Dashboard() {
  const prices = usePrices();
  const { opportunities } = useOpportunities();
  const exchangeRate = useExchangeRate();
  const commonMarkets = useCommonMarkets();
  const [selectedSymbol, setSelectedSymbol] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [minVolume, setMinVolume] = useState<number>(0);
  const [premiumMode, setPremiumMode] = useState<PremiumMode>("tether");
  const [quoteFilter, setQuoteFilter] = useState<QuoteFilter>("all");

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
            <div className="flex items-center space-x-6">
              {/* Upbit USDT/KRW */}
              <div>
                <div className="text-gray-500 text-xs mb-1">Upbit USDT/KRW</div>
                <span className="font-mono text-xl text-primary-400">
                  ₩{(exchangeRate.upbit_usdt_krw || exchangeRate.usd_krw).toLocaleString("ko-KR", {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2,
                  })}
                </span>
              </div>
              {/* Bithumb USDT/KRW */}
              {exchangeRate.bithumb_usdt_krw > 0 && (
                <div>
                  <div className="text-gray-500 text-xs mb-1">Bithumb USDT/KRW</div>
                  <span className="font-mono text-xl text-primary-400">
                    ₩{exchangeRate.bithumb_usdt_krw.toLocaleString("ko-KR", {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 2,
                    })}
                  </span>
                </div>
              )}
              {/* Kimchi Premium - using Upbit rate vs API rate */}
              {exchangeRate.api_rate && (exchangeRate.upbit_usdt_krw > 0 || exchangeRate.usd_krw > 0) && (() => {
                const upbitRate = exchangeRate.upbit_usdt_krw || exchangeRate.usd_krw;
                return (
                  <>
                    <div className="border-l border-dark-600 pl-6">
                      <div className="text-gray-500 text-xs mb-1">USD/KRW (API)</div>
                      <span className="font-mono text-lg text-gray-300">
                        ₩{exchangeRate.api_rate.toLocaleString("ko-KR", {
                          minimumFractionDigits: 2,
                          maximumFractionDigits: 2,
                        })}
                      </span>
                    </div>
                    <div>
                      <div className="text-gray-500 text-xs mb-1">Kimchi Premium</div>
                      <span className={`font-mono text-xl font-bold ${
                        upbitRate > exchangeRate.api_rate
                          ? "text-success-500"
                          : "text-danger-500"
                      }`}>
                        {upbitRate > exchangeRate.api_rate ? "+" : ""}
                        {(((upbitRate - exchangeRate.api_rate) / exchangeRate.api_rate) * 100).toFixed(2)}%
                      </span>
                    </div>
                  </>
                );
              })()}
              {/* Stablecoin rates */}
              <div className="border-l border-dark-600 pl-6 flex items-center space-x-4">
                <div>
                  <div className="text-gray-500 text-xs mb-1">USDT/USD</div>
                  <span className="font-mono text-lg text-gray-300">
                    ${exchangeRate.usdt_usd?.toFixed(4) || "1.0000"}
                  </span>
                </div>
                <div>
                  <div className="text-gray-500 text-xs mb-1">USDC/USD</div>
                  <span className="font-mono text-lg text-gray-300">
                    ${exchangeRate.usdc_usd?.toFixed(4) || "1.0000"}
                  </span>
                </div>
              </div>
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
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold">
              Premium Matrix {activeSymbol && <span className="text-primary-400">- {activeSymbol}</span>}
            </h2>
            <div className="flex items-center gap-4">
              {/* Quote Filter */}
              <div className="flex items-center gap-1 bg-dark-700 rounded-lg p-1">
                <button
                  onClick={() => setQuoteFilter("all")}
                  className={`px-2 py-1 rounded text-xs font-medium transition-colors ${
                    quoteFilter === "all"
                      ? "bg-gray-500 text-white"
                      : "text-gray-400 hover:text-white"
                  }`}
                >
                  All
                </button>
                <button
                  onClick={() => setQuoteFilter("USDT")}
                  className={`px-2 py-1 rounded text-xs font-medium transition-colors ${
                    quoteFilter === "USDT"
                      ? "bg-green-500 text-dark-900"
                      : "text-gray-400 hover:text-white"
                  }`}
                >
                  USDT
                </button>
                <button
                  onClick={() => setQuoteFilter("USDC")}
                  className={`px-2 py-1 rounded text-xs font-medium transition-colors ${
                    quoteFilter === "USDC"
                      ? "bg-blue-500 text-white"
                      : "text-gray-400 hover:text-white"
                  }`}
                >
                  USDC
                </button>
              </div>
              {/* Premium Mode Toggle */}
              <div className="flex items-center gap-1 bg-dark-700 rounded-lg p-1">
                <button
                  onClick={() => setPremiumMode("kimchi")}
                  className={`px-3 py-1.5 rounded text-sm font-medium transition-colors ${
                    premiumMode === "kimchi"
                      ? "bg-yellow-500 text-dark-900"
                      : "text-gray-400 hover:text-white"
                  }`}
                >
                  김프
                </button>
                <button
                  onClick={() => setPremiumMode("tether")}
                  className={`px-3 py-1.5 rounded text-sm font-medium transition-colors ${
                    premiumMode === "tether"
                      ? "bg-green-500 text-dark-900"
                      : "text-gray-400 hover:text-white"
                  }`}
                >
                  테프
                </button>
              </div>
            </div>
          </div>
          <div className="bg-dark-800 rounded-lg p-4 border border-dark-700">
            {activeSymbol && pricesBySymbol[activeSymbol] ? (
              <>
                <PremiumMatrix
                  prices={pricesBySymbol[activeSymbol]}
                  premiumMode={premiumMode}
                  exchangeRate={exchangeRate}
                  quoteFilter={quoteFilter}
                  symbol={activeSymbol}
                />
                <div className="mt-4 pt-4 border-t border-dark-700">
                  <h3 className="text-sm text-gray-400 mb-3">Exchange Prices</h3>
                  <div className="grid grid-cols-3 gap-4">
                    {pricesBySymbol[activeSymbol]
                      .filter((p) => {
                        if (quoteFilter === "all") return true;
                        const quote = p.quote || "USD";
                        // KRW is always included
                        if (quote === "KRW") return true;
                        // USDT filter matches USDT and USD (treat USD as USDT)
                        if (quoteFilter === "USDT") return quote === "USDT" || quote === "USD";
                        // USDC filter matches only USDC
                        if (quoteFilter === "USDC") return quote === "USDC";
                        return true;
                      })
                      .map((p) => (
                      <div key={`${p.exchange}-${p.quote || 'USD'}`} className="text-sm">
                        <div className="flex items-center gap-2">
                          <span className="text-gray-400">{p.exchange}</span>
                          <span className={`text-xs px-1.5 py-0.5 rounded ${
                            p.quote === 'KRW' ? 'bg-yellow-500/20 text-yellow-400' :
                            p.quote === 'USDC' ? 'bg-blue-500/20 text-blue-400' :
                            p.quote === 'USDT' ? 'bg-green-500/20 text-green-400' :
                            'bg-gray-500/20 text-gray-400'
                          }`}>
                            {p.quote || 'USD'}
                          </span>
                        </div>
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
  premiumMode: PremiumMode;
  exchangeRate: ExchangeRate | null;
  quoteFilter: QuoteFilter;
  symbol: string;
}

function PremiumMatrix({ prices, premiumMode, exchangeRate, quoteFilter, symbol }: PremiumMatrixProps) {
  // Filter prices by quote currency (KRW is always included)
  const filteredPrices = useMemo(() => {
    if (quoteFilter === "all") return prices;
    return prices.filter((p) => {
      const quote = p.quote || "USD";
      // KRW is always included
      if (quote === "KRW") return true;
      // USDT filter matches USDT and USD (treat USD as USDT)
      if (quoteFilter === "USDT") return quote === "USDT" || quote === "USD";
      // USDC filter matches only USDC
      if (quoteFilter === "USDC") return quote === "USDC";
      return true;
    });
  }, [prices, quoteFilter]);

  // Create unique key for each exchange + quote combination
  const priceEntries = filteredPrices.map((p) => ({
    ...p,
    label: `${p.exchange}${p.quote && p.quote !== 'USD' ? ` (${p.quote})` : ''}`,
    isKrw: p.quote === 'KRW',
  }));

  /**
   * Calculate premium between buy and sell positions
   * - Raw premium: direct USD price comparison (all prices are already in USD)
   * - Kimchi premium: for KRW trades, adjust using bank rate vs USDT rate
   * - Tether premium: same as raw (prices already converted via USDT/KRW)
   */
  const getPremium = (buyIdx: number, sellIdx: number): number => {
    if (buyIdx === sellIdx) return 0;

    const buyEntry = priceEntries[buyIdx];
    const sellEntry = priceEntries[sellIdx];
    const buyPrice = buyEntry?.price || 0;
    const sellPrice = sellEntry?.price || 0;

    if (buyPrice === 0) return 0;

    // Raw premium (direct USD comparison)
    const rawPremium = Math.round(((sellPrice - buyPrice) / buyPrice) * 10000);

    // If neither side is KRW, return raw premium
    const buyIsKrw = buyEntry?.isKrw;
    const sellIsKrw = sellEntry?.isKrw;

    if (!buyIsKrw && !sellIsKrw) {
      return rawPremium;
    }

    // Tether premium = raw premium (already converted via USDT/KRW)
    if (premiumMode === "tether") {
      return rawPremium;
    }

    // Kimchi premium: adjust for bank rate vs USDT rate difference
    // KRW prices were converted: krw_original / usdt_krw = price_usd_stored
    // Kimchi should use: krw_original / usd_krw = price_usd_kimchi
    // Ratio: price_usd_kimchi / price_usd_stored = usdt_krw / usd_krw
    const usdKrw = exchangeRate?.api_rate;
    const usdtKrw = exchangeRate?.upbit_usdt_krw || exchangeRate?.usd_krw;

    if (!usdKrw || !usdtKrw || usdKrw <= 0) {
      return rawPremium;
    }

    const rateRatio = usdtKrw / usdKrw;

    // Adjust KRW price to what it would be using bank rate
    let adjustedBuyPrice = buyPrice;
    let adjustedSellPrice = sellPrice;

    if (buyIsKrw) {
      adjustedBuyPrice = buyPrice * rateRatio;
    }
    if (sellIsKrw) {
      adjustedSellPrice = sellPrice * rateRatio;
    }

    if (adjustedBuyPrice === 0) return rawPremium;

    return Math.round(((adjustedSellPrice - adjustedBuyPrice) / adjustedBuyPrice) * 10000);
  };

  const getColor = (bps: number): string => {
    if (bps >= 50) return "bg-success-500 text-white";
    if (bps >= 30) return "bg-yellow-500 text-dark-900";
    if (bps > 0) return "bg-dark-700 text-gray-300";
    if (bps < -30) return "bg-danger-500 text-white";
    return "bg-dark-700 text-gray-500";
  };

  if (priceEntries.length === 0) {
    return (
      <div className="text-gray-400 text-center py-8">
        No {quoteFilter} markets available for this symbol
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr>
            <th className="text-left text-gray-400 text-sm p-2">Buy ↓ / Sell →</th>
            {priceEntries.map((entry) => {
              const tradeUrl = getExchangeTradeUrl(entry.exchange, symbol, entry.quote || "USD");
              return (
                <th key={entry.label} className="text-center text-gray-400 text-sm p-2">
                  {tradeUrl ? (
                    <a
                      href={tradeUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="hover:text-primary-400 transition-colors cursor-pointer"
                      title={`Open ${entry.exchange} ${symbol}/${entry.quote || 'USD'}`}
                    >
                      {entry.exchange}
                    </a>
                  ) : (
                    <div>{entry.exchange}</div>
                  )}
                  {entry.quote && entry.quote !== 'USD' && (
                    <div className={`text-xs ${
                      entry.quote === 'KRW' ? 'text-yellow-400' :
                      entry.quote === 'USDC' ? 'text-blue-400' :
                      entry.quote === 'USDT' ? 'text-green-400' :
                      'text-gray-500'
                    }`}>
                      {entry.quote}
                    </div>
                  )}
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {priceEntries.map((buyEntry, buyIdx) => {
            const rowTradeUrl = getExchangeTradeUrl(buyEntry.exchange, symbol, buyEntry.quote || "USD");
            return (
            <tr key={buyEntry.label}>
              <td className="text-gray-400 text-sm p-2">
                {rowTradeUrl ? (
                  <a
                    href={rowTradeUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="hover:text-primary-400 transition-colors cursor-pointer"
                    title={`Open ${buyEntry.exchange} ${symbol}/${buyEntry.quote || 'USD'}`}
                  >
                    {buyEntry.exchange}
                  </a>
                ) : (
                  <div>{buyEntry.exchange}</div>
                )}
                {buyEntry.quote && buyEntry.quote !== 'USD' && (
                  <div className={`text-xs ${
                    buyEntry.quote === 'KRW' ? 'text-yellow-400' :
                    buyEntry.quote === 'USDC' ? 'text-blue-400' :
                    buyEntry.quote === 'USDT' ? 'text-green-400' :
                    'text-gray-500'
                  }`}>
                    {buyEntry.quote}
                  </div>
                )}
              </td>
              {priceEntries.map((_, sellIdx) => {
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
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export default Dashboard;

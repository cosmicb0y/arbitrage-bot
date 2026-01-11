import { useMemo, useState } from "react";
import { usePrices, useOpportunities, useExchangeRate, useCommonMarkets, usePremiumMatrix } from "../hooks/useTauri";
import type { PriceData, ExchangeRate, PremiumMatrixData } from "../types";

type PremiumMode = "kimchi" | "tether";
type QuoteFilter = "all" | "USDT" | "USDC";

/**
 * Generate trading page URL for each exchange
 */
function getExchangeTradeUrl(exchange: string, symbol: string, quote: string): string | null {
  const q = quote || "USD";

  // Exchange names come from Rust Debug format (e.g., "GateIO", "Okx")
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

    case "GateIO":
      // https://www.gate.com/trade/BTC_USDT
      return `https://www.gate.com/trade/${symbol}_${q}`;

    case "Bybit":
      // https://www.bybit.com/en/trade/spot/BTC/USDT
      return `https://www.bybit.com/en/trade/spot/${symbol}/${q}`;

    case "Kraken":
      // https://pro.kraken.com/app/trade/btc-usd
      return `https://pro.kraken.com/app/trade/${symbol.toLowerCase()}-${q.toLowerCase()}`;

    case "Okx":
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
  const premiumMatrices = usePremiumMatrix();
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

  // Helper to calculate minimum volume for a symbol (across all exchanges)
  // For arbitrage, we need sufficient liquidity on ALL exchanges
  const getSymbolMinVolume = (symbol: string): number => {
    const prices = pricesBySymbol[symbol];
    if (!prices || prices.length === 0) return 0;
    const volumes = prices.map(p => p.volume_24h || 0);
    return Math.min(...volumes);
  };

  // Helper to calculate total volume for a symbol (for display only)
  const getSymbolTotalVolume = (symbol: string): number => {
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
        // Volume filter: use minimum volume across exchanges (like Opportunities)
        if (minVolume > 0) {
          const minExchangeVolume = getSymbolMinVolume(symbol);
          if (minExchangeVolume < minVolume) return false;
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
        // Sort by max premium from server matrix (descending)
        const matrixA = premiumMatrices.get(a);
        const matrixB = premiumMatrices.get(b);
        const getMaxPremium = (m: PremiumMatrixData | undefined) => {
          if (!m || m.entries.length === 0) return 0;
          const premiums = m.entries.map(e =>
            premiumMode === "kimchi" ? e.kimchi_premium_bps : e.tether_premium_bps
          );
          return Math.max(...premiums);
        };
        const spreadA = getMaxPremium(matrixA);
        const spreadB = getMaxPremium(matrixB);
        return spreadB - spreadA;
      });
  }, [allSymbols, pricesBySymbol, searchQuery, minVolume, premiumMatrices, premiumMode]);

  // Auto-select first symbol if none selected
  const activeSymbol = selectedSymbol && allSymbols.has(selectedSymbol)
    ? selectedSymbol
    : symbols[0] || null;

  const formatPrice = (price: number): string => {
    // Determine decimal places based on price magnitude
    // Show all significant digits up to reasonable precision
    if (price >= 10000) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      });
    } else if (price >= 1000) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 3,
      });
    } else if (price >= 100) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 4,
      });
    } else if (price >= 10) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 5,
      });
    } else if (price >= 1) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 4,
        maximumFractionDigits: 6,
      });
    } else if (price >= 0.01) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 4,
        maximumFractionDigits: 6,
      });
    } else if (price >= 0.0001) {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 6,
        maximumFractionDigits: 8,
      });
    } else {
      // Very small prices (< 0.0001): show up to 10 decimal places
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 8,
        maximumFractionDigits: 10,
      });
    }
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
          <div className="flex flex-col gap-3">
            {/* Top row: KRW rates and Kimchi Premium */}
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
              </div>
              <span className="text-xs text-gray-500">
                Updated: {new Date(exchangeRate.timestamp).toLocaleTimeString()}
              </span>
            </div>

            {/* Bottom row: Stablecoin prices by exchange */}
            <StablecoinPrices prices={prices} />
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
                // Use server premium matrix for spread calculation (more accurate)
                const matrix = premiumMatrices.get(symbol);
                let spread = 0;
                let hasMatrix = false;
                if (matrix && matrix.entries.length > 0) {
                  hasMatrix = true;
                  // Get max premium from server-calculated matrix (based on current premiumMode)
                  const premiums = matrix.entries.map(e =>
                    premiumMode === "kimchi" ? e.kimchi_premium_bps : e.tether_premium_bps
                  );
                  spread = Math.max(...premiums) / 100; // bps to %
                }
                const totalVolume = hasPriceData ? getSymbolTotalVolume(symbol) : 0;

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
                        <span className={`text-sm font-mono ${spread >= 0.5 ? "text-success-500" : hasMatrix ? "text-gray-500" : "text-gray-600"}`}>
                          {hasMatrix ? `${spread.toFixed(2)}%` : "..."}
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
                  serverMatrix={premiumMatrices.get(activeSymbol)}
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
  serverMatrix?: PremiumMatrixData;
}

function PremiumMatrix({ prices, premiumMode, exchangeRate, quoteFilter, symbol, serverMatrix }: PremiumMatrixProps) {
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
   * Get premium from server-calculated matrix data.
   * Returns the premium for the given buy/sell exchange pair.
   */
  const getServerPremium = (buyExchange: string, sellExchange: string, buyQuote: string, sellQuote: string): number | null => {
    if (!serverMatrix) {
      // Debug: log when serverMatrix is not available
      // console.debug(`[PremiumMatrix] No serverMatrix for ${symbol}`);
      return null;
    }

    // Server uses Debug format for exchange (e.g., "Upbit"), which should match client
    // Server uses as_str() for quote (e.g., "KRW", "USDT"), which should match client
    const entry = serverMatrix.entries.find(
      (e) => e.buy_exchange === buyExchange && e.sell_exchange === sellExchange
        && e.buy_quote === buyQuote && e.sell_quote === sellQuote
    );

    if (!entry) {
      return null;
    }

    return premiumMode === "kimchi" ? entry.kimchi_premium_bps : entry.tether_premium_bps;
  };

  /**
   * Calculate premium between buy and sell positions
   * - First tries to use server-calculated premium (more accurate)
   * - Falls back to client-side calculation if server data not available
   */
  const getPremium = (buyIdx: number, sellIdx: number): number => {
    if (buyIdx === sellIdx) return 0;

    const buyEntry = priceEntries[buyIdx];
    const sellEntry = priceEntries[sellIdx];

    // Try to get server-calculated premium first
    const buyQuote = buyEntry?.quote || "USD";
    const sellQuote = sellEntry?.quote || "USD";
    const serverPremium = getServerPremium(buyEntry?.exchange, sellEntry?.exchange, buyQuote, sellQuote);
    if (serverPremium !== null) {
      return serverPremium;
    }

    // Fallback to client-side calculation
    // Use USD-normalized prices for comparison
    // For KRW markets, we MUST have price_usd; raw KRW price is not comparable
    const buyIsKrw = buyEntry?.isKrw;
    const sellIsKrw = sellEntry?.isKrw;

    let buyPriceUsd: number;
    let sellPriceUsd: number;

    if (buyIsKrw) {
      // KRW market: must use price_usd, cannot fall back to raw price
      if (buyEntry?.price_usd == null) return 0;
      buyPriceUsd = buyEntry.price_usd;
    } else {
      buyPriceUsd = buyEntry?.price_usd ?? buyEntry?.price ?? 0;
    }

    if (sellIsKrw) {
      // KRW market: must use price_usd, cannot fall back to raw price
      if (sellEntry?.price_usd == null) return 0;
      sellPriceUsd = sellEntry.price_usd;
    } else {
      sellPriceUsd = sellEntry?.price_usd ?? sellEntry?.price ?? 0;
    }

    if (buyPriceUsd === 0) return 0;

    // Raw premium using USD prices
    const rawPremium = Math.round(((sellPriceUsd - buyPriceUsd) / buyPriceUsd) * 10000);

    // If neither side is KRW, return raw premium
    if (!buyIsKrw && !sellIsKrw) {
      return rawPremium;
    }

    // Tether premium = raw premium (price_usd is already converted via USDT/KRW)
    if (premiumMode === "tether") {
      return rawPremium;
    }

    // Kimchi premium: adjust for bank rate vs USDT rate difference
    const usdKrw = exchangeRate?.api_rate;
    const usdtKrw = exchangeRate?.upbit_usdt_krw || exchangeRate?.usd_krw;

    if (!usdKrw || !usdtKrw || usdKrw <= 0) {
      return rawPremium;
    }

    const rateRatio = usdtKrw / usdKrw;

    let adjustedBuyPrice = buyPriceUsd;
    let adjustedSellPrice = sellPriceUsd;

    if (buyIsKrw) {
      adjustedBuyPrice = buyPriceUsd * rateRatio;
    }
    if (sellIsKrw) {
      adjustedSellPrice = sellPriceUsd * rateRatio;
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

interface StablecoinPricesProps {
  prices: PriceData[];
}

interface StablecoinPrice {
  exchange: string;
  price: number;
  source: "direct" | "derived";  // direct = actual pair, derived = calculated from cross pair
  quote: string;  // Original quote currency
}

/**
 * Display USDT and USDC prices by exchange
 * Combines direct pairs and derived prices from cross-stablecoin pairs
 */
function StablecoinPrices({ prices }: StablecoinPricesProps) {
  // Collect all stablecoin-related prices and derive USD equivalents
  const stablecoinData = useMemo(() => {
    // Direct pairs
    const usdtUsdDirect: StablecoinPrice[] = [];  // USDT/USD
    const usdcUsdDirect: StablecoinPrice[] = [];  // USDC/USD
    const usdcUsdtPairs: PriceData[] = [];        // USDC/USDT (for deriving)
    const usdtUsdcPairs: PriceData[] = [];        // USDT/USDC (for deriving)

    for (const p of prices) {
      if (p.quote === "KRW") continue;

      if (p.symbol === "USDT") {
        if (p.quote === "USD") {
          usdtUsdDirect.push({ exchange: p.exchange, price: p.price, source: "direct", quote: "USD" });
        } else if (p.quote === "USDC") {
          usdtUsdcPairs.push(p);
        }
      } else if (p.symbol === "USDC") {
        if (p.quote === "USD") {
          usdcUsdDirect.push({ exchange: p.exchange, price: p.price, source: "direct", quote: "USD" });
        } else if (p.quote === "USDT") {
          usdcUsdtPairs.push(p);
        }
      }
    }

    // Build exchange sets for direct prices
    const usdtUsdExchanges = new Set(usdtUsdDirect.map(p => p.exchange));
    const usdcUsdExchanges = new Set(usdcUsdDirect.map(p => p.exchange));

    // Get reference USDT/USD price (average of direct prices, or 1.0 if none)
    const avgUsdtUsd = usdtUsdDirect.length > 0
      ? usdtUsdDirect.reduce((sum, p) => sum + p.price, 0) / usdtUsdDirect.length
      : 1.0;

    // Get reference USDC/USD price (average of direct prices, or 1.0 if none)
    const avgUsdcUsd = usdcUsdDirect.length > 0
      ? usdcUsdDirect.reduce((sum, p) => sum + p.price, 0) / usdcUsdDirect.length
      : 1.0;

    // Derive USDT/USD from USDT/USDC pairs (USDT/USD = USDT/USDC * USDC/USD)
    const usdtUsdDerived: StablecoinPrice[] = [];
    for (const p of usdtUsdcPairs) {
      if (!usdtUsdExchanges.has(p.exchange)) {
        usdtUsdDerived.push({
          exchange: p.exchange,
          price: p.price * avgUsdcUsd,
          source: "derived",
          quote: "USDC",
        });
      }
    }

    // Also derive USDT/USD from USDC/USDT pairs (inverse: USDT/USD = (1/USDC_USDT) * USDT/USD_ref)
    // This handles exchanges like Bybit that only have USDC/USDT, not USDT/USDC
    for (const p of usdcUsdtPairs) {
      if (!usdtUsdExchanges.has(p.exchange) && !usdtUsdDerived.some(d => d.exchange === p.exchange)) {
        // USDC/USDT price means 1 USDC = p.price USDT
        // So 1 USDT = (1/p.price) USDC
        // USDT/USD = (1/p.price) * avgUsdcUsd
        const usdtInUsdc = 1 / p.price;
        usdtUsdDerived.push({
          exchange: p.exchange,
          price: usdtInUsdc * avgUsdcUsd,
          source: "derived",
          quote: "USDC",
        });
      }
    }

    // Derive USDC/USD from USDC/USDT pairs (USDC/USD = USDC/USDT * USDT/USD)
    const usdcUsdDerived: StablecoinPrice[] = [];
    for (const p of usdcUsdtPairs) {
      if (!usdcUsdExchanges.has(p.exchange)) {
        usdcUsdDerived.push({
          exchange: p.exchange,
          price: p.price * avgUsdtUsd,
          source: "derived",
          quote: "USDT",
        });
      }
    }

    // Also derive USDC/USD from USDT/USDC pairs (inverse)
    // Note: Exclude Coinbase since it treats USD = USDC at 1:1
    for (const p of usdtUsdcPairs) {
      if (p.exchange === "Coinbase") continue; // Coinbase USDC/USD is always $1.0000
      if (!usdcUsdExchanges.has(p.exchange) && !usdcUsdDerived.some(d => d.exchange === p.exchange)) {
        // USDT/USDC price means 1 USDT = p.price USDC
        // So 1 USDC = (1/p.price) USDT
        // USDC/USD = (1/p.price) * avgUsdtUsd
        const usdcInUsdt = 1 / p.price;
        usdcUsdDerived.push({
          exchange: p.exchange,
          price: usdcInUsdt * avgUsdtUsd,
          source: "derived",
          quote: "USDT",
        });
      }
    }

    // Special case: Coinbase treats USD = USDC at 1:1
    // Coinbase's USD pairs are essentially USDC pairs, so USDC/USD = $1.0000
    if (!usdcUsdExchanges.has("Coinbase")) {
      usdcUsdDirect.push({ exchange: "Coinbase", price: 1.0, source: "direct", quote: "USD" });
    }

    // Combine and sort
    const usdtUsd = [...usdtUsdDirect, ...usdtUsdDerived].sort((a, b) =>
      a.exchange.localeCompare(b.exchange)
    );
    const usdcUsd = [...usdcUsdDirect, ...usdcUsdDerived].sort((a, b) =>
      a.exchange.localeCompare(b.exchange)
    );

    return { usdtUsd, usdcUsd };
  }, [prices]);

  const hasAny = stablecoinData.usdtUsd.length > 0 || stablecoinData.usdcUsd.length > 0;

  if (!hasAny) {
    return null;
  }

  return (
    <div className="border-t border-dark-600 pt-3">
      <div className="flex gap-8">
        {/* USDT/USD prices */}
        {stablecoinData.usdtUsd.length > 0 && (
          <div>
            <div className="text-gray-500 text-xs mb-2 font-medium">USDT/USD</div>
            <div className="flex flex-wrap gap-4">
              {stablecoinData.usdtUsd.map((p) => (
                <div key={`${p.exchange}-usdt-usd`} className="flex items-center gap-2">
                  <span className="text-gray-400 text-sm">{p.exchange}</span>
                  {p.source === "derived" && (
                    <span className="text-gray-600 text-xs" title={`Derived from ${p.quote} pair`}>
                      *
                    </span>
                  )}
                  <span className={`font-mono text-sm ${
                    p.price >= 1.0 ? "text-green-400" : "text-red-400"
                  }`}>
                    ${p.price.toFixed(4)}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* USDC/USD prices */}
        {stablecoinData.usdcUsd.length > 0 && (
          <div>
            <div className="text-gray-500 text-xs mb-2 font-medium">USDC/USD</div>
            <div className="flex flex-wrap gap-4">
              {stablecoinData.usdcUsd.map((p) => (
                <div key={`${p.exchange}-usdc-usd`} className="flex items-center gap-2">
                  <span className="text-gray-400 text-sm">{p.exchange}</span>
                  {p.source === "derived" && (
                    <span className="text-gray-600 text-xs" title={`Derived from ${p.quote} pair`}>
                      *
                    </span>
                  )}
                  <span className={`font-mono text-sm ${
                    p.price >= 1.0 ? "text-blue-400" : "text-red-400"
                  }`}>
                    ${p.price.toFixed(4)}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
      <div className="text-gray-600 text-xs mt-2">* derived from cross-stablecoin pair</div>
    </div>
  );
}

export default Dashboard;

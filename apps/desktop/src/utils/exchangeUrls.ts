/**
 * Generate trading page URL for each exchange
 */
export function getExchangeTradeUrl(exchange: string, symbol: string, quote: string): string | null {
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

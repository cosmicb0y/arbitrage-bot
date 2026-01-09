use arbitrage_core::{symbol_to_pair_id, Exchange, FixedPoint, PriceTick};
use serde::Deserialize;

use super::ExchangeAdapter;
use crate::FeedError;

pub struct BybitAdapter;

#[derive(Debug, Deserialize)]
struct BybitTickerData {
    symbol: String,
    #[serde(rename = "lastPrice")]
    last_price: String,
    #[serde(rename = "bid1Price", default)]
    bid1_price: String,
    #[serde(rename = "ask1Price", default)]
    ask1_price: String,
    #[serde(rename = "volume24h", default)]
    _volume_24h: String,
    #[serde(rename = "turnover24h", default)]
    turnover_24h: String,
}

#[derive(Debug, Deserialize)]
struct BybitTickerMessage {
    #[serde(rename = "topic")]
    _topic: String,
    #[serde(rename = "type")]
    _msg_type: String,
    data: BybitTickerData,
    #[serde(rename = "ts")]
    _ts: u64,
}

#[derive(Debug, Deserialize)]
struct BybitOrderbookData {
    s: String,
    b: Vec<[String; 2]>,
    a: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
struct BybitOrderbookMessage {
    #[serde(rename = "topic")]
    _topic: String,
    #[serde(rename = "type")]
    msg_type: String,
    data: BybitOrderbookData,
    #[serde(rename = "ts")]
    _ts: u64,
}

impl ExchangeAdapter for BybitAdapter {
    fn exchange() -> Exchange {
        Exchange::Bybit
    }

    fn ws_url() -> &'static str {
        "wss://stream.bybit.com/v5/public/spot"
    }

    fn extract_base_quote(symbol: &str) -> Option<(String, String)> {
        let symbol = symbol.to_uppercase();
        const QUOTES: &[&str] = &["USDT", "USDC"];
        for quote in QUOTES {
            if let Some(base) = symbol.strip_suffix(quote) {
                return Some((base.to_string(), (*quote).to_string()));
            }
        }
        None
    }

    fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        let mut messages = Vec::new();

        for chunk in symbols.chunks(10) {
            let topics: Vec<String> = chunk
                .iter()
                .map(|s| format!("\"orderbook.50.{}\"", s.to_uppercase()))
                .collect();
            messages.push(format!(
                r#"{{"op": "subscribe", "args": [{}]}}"#,
                topics.join(", ")
            ));
        }

        messages
    }
}

impl BybitAdapter {
    pub fn symbol_to_pair_id(symbol: &str) -> Option<u32> {
        Self::extract_base_symbol(symbol).map(|base| symbol_to_pair_id(&base))
    }

    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let msg: BybitTickerMessage = serde_json::from_str(json)?;

        let price = msg
            .data
            .last_price
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = msg.data.bid1_price.parse::<f64>().unwrap_or(price);
        let ask = msg.data.ask1_price.parse::<f64>().unwrap_or(price);

        Ok(PriceTick::new(
            Exchange::Bybit,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ))
    }

    pub fn parse_ticker_auto(json: &str) -> Result<PriceTick, FeedError> {
        let (tick, _) = Self::parse_ticker_with_symbol(json)?;
        Ok(tick)
    }

    pub fn parse_ticker_with_symbol(json: &str) -> Result<(PriceTick, String), FeedError> {
        let (tick, base, _quote) = Self::parse_ticker_with_base_quote(json)?;
        Ok((tick, base))
    }

    pub fn parse_ticker_with_base_quote(
        json: &str,
    ) -> Result<(PriceTick, String, String), FeedError> {
        let msg: BybitTickerMessage = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&msg.data.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", msg.data.symbol)))?;
        let pair_id = symbol_to_pair_id(&base);

        let price = msg
            .data
            .last_price
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = msg.data.bid1_price.parse::<f64>().unwrap_or(price);
        let ask = msg.data.ask1_price.parse::<f64>().unwrap_or(price);
        let volume_usd = msg.data.turnover_24h.parse::<f64>().unwrap_or(0.0);

        Ok((
            PriceTick::new(
                Exchange::Bybit,
                pair_id,
                FixedPoint::from_f64(price),
                FixedPoint::from_f64(bid),
                FixedPoint::from_f64(ask),
            )
            .with_volume_24h(FixedPoint::from_f64(volume_usd)),
            base,
            quote,
        ))
    }

    pub fn parse_orderbook_with_base_quote(
        json: &str,
    ) -> Result<(PriceTick, String, String), FeedError> {
        let msg: BybitOrderbookMessage = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&msg.data.s)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", msg.data.s)))?;
        let pair_id = symbol_to_pair_id(&base);

        let best_bid = msg
            .data
            .b
            .first()
            .ok_or_else(|| FeedError::ParseError("No bid in orderbook".to_string()))?;
        let bid = best_bid[0]
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid price".to_string()))?;
        let bid_size = best_bid[1].parse::<f64>().unwrap_or(0.0);

        let best_ask = msg
            .data
            .a
            .first()
            .ok_or_else(|| FeedError::ParseError("No ask in orderbook".to_string()))?;
        let ask = best_ask[0]
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid ask price".to_string()))?;
        let ask_size = best_ask[1].parse::<f64>().unwrap_or(0.0);

        let mid = (bid + ask) / 2.0;

        Ok((
            PriceTick::new(
                Exchange::Bybit,
                pair_id,
                FixedPoint::from_f64(mid),
                FixedPoint::from_f64(bid),
                FixedPoint::from_f64(ask),
            )
            .with_sizes(
                FixedPoint::from_f64(bid_size),
                FixedPoint::from_f64(ask_size),
            ),
            base,
            quote,
        ))
    }

    /// Parse orderbook message with snapshot/delta indicator.
    /// Returns (tick, base, quote, bids, asks, is_snapshot).
    /// - is_snapshot=true: Full orderbook replacement
    /// - is_snapshot=false: Delta update (apply changes to existing orderbook)
    pub fn parse_orderbook_full(
        json: &str,
    ) -> Result<(PriceTick, String, String, Vec<(f64, f64)>, Vec<(f64, f64)>, bool), FeedError> {
        let msg: BybitOrderbookMessage = serde_json::from_str(json)?;

        let is_snapshot = msg.msg_type == "snapshot";

        let (base, quote) = Self::extract_base_quote(&msg.data.s)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", msg.data.s)))?;
        let pair_id = symbol_to_pair_id(&base);

        let bids: Vec<(f64, f64)> = msg
            .data
            .b
            .iter()
            .filter_map(|level| {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            })
            .collect();

        let asks: Vec<(f64, f64)> = msg
            .data
            .a
            .iter()
            .filter_map(|level| {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            })
            .collect();

        let (bid, bid_size) = bids.first().copied().unwrap_or((0.0, 0.0));
        let (ask, ask_size) = asks.first().copied().unwrap_or((0.0, 0.0));
        let mid = if bid > 0.0 && ask > 0.0 {
            (bid + ask) / 2.0
        } else {
            bid.max(ask)
        };

        Ok((
            PriceTick::new(
                Exchange::Bybit,
                pair_id,
                FixedPoint::from_f64(mid),
                FixedPoint::from_f64(bid),
                FixedPoint::from_f64(ask),
            )
            .with_sizes(
                FixedPoint::from_f64(bid_size),
                FixedPoint::from_f64(ask_size),
            ),
            base,
            quote,
            bids,
            asks,
            is_snapshot,
        ))
    }

    pub fn is_orderbook_message(json: &str) -> bool {
        json.contains("\"topic\":\"orderbook.")
    }

    pub fn subscribe_message(symbols: &[String]) -> String {
        let topics: Vec<String> = symbols
            .iter()
            .take(10)
            .map(|s| format!("\"tickers.{}\"", s.to_uppercase()))
            .collect();

        format!(r#"{{"op": "subscribe", "args": [{}]}}"#, topics.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bybit_parse_ticker() {
        let json = r#"{
            "topic": "tickers.BTCUSDT",
            "type": "snapshot",
            "data": {
                "symbol": "BTCUSDT",
                "lastPrice": "50000.00",
                "highPrice24h": "51000.00",
                "lowPrice24h": "49000.00",
                "prevPrice24h": "49500.00",
                "volume24h": "1000.00",
                "turnover24h": "50000000.00",
                "price24hPcnt": "0.01"
            },
            "ts": 1700000000000
        }"#;

        let (tick, symbol) = BybitAdapter::parse_ticker_with_symbol(json).unwrap();
        assert_eq!(tick.exchange(), Exchange::Bybit);
        assert_eq!(symbol, "BTC");
        assert!((tick.price().to_f64() - 50000.0).abs() < 0.01);
        assert!((tick.bid().to_f64() - 50000.0).abs() < 0.01);
        assert!((tick.ask().to_f64() - 50000.0).abs() < 0.01);
    }

    #[test]
    fn test_bybit_subscribe_message() {
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let msg = BybitAdapter::subscribe_message(&symbols);
        assert!(msg.contains("subscribe"));
        assert!(msg.contains("tickers.BTCUSDT"));
        assert!(msg.contains("tickers.ETHUSDT"));
    }

    #[test]
    fn test_bybit_extract_quote_currency() {
        assert_eq!(
            BybitAdapter::extract_quote_currency("BTCUSDT"),
            Some("USDT".to_string())
        );
        assert_eq!(
            BybitAdapter::extract_quote_currency("BTCUSDC"),
            Some("USDC".to_string())
        );
        assert_eq!(BybitAdapter::extract_quote_currency("BTCEUR"), None);
    }
}

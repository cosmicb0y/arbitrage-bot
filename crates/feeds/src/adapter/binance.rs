use arbitrage_core::{symbol_to_pair_id, Exchange, FixedPoint, PriceTick};
use serde::Deserialize;

use super::ExchangeAdapter;
use crate::FeedError;

pub struct BinanceAdapter;

#[derive(Debug, Deserialize)]
struct BinanceTicker {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "b")]
    bid: String,
    #[serde(rename = "a")]
    ask: String,
    #[serde(rename = "v", default)]
    volume: String,
    #[allow(dead_code)]
    #[serde(rename = "E", default)]
    event_time: u64,
}

#[derive(Debug, Deserialize)]
struct BinanceBookTicker {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "b")]
    bid: String,
    #[serde(rename = "B")]
    bid_qty: String,
    #[serde(rename = "a")]
    ask: String,
    #[serde(rename = "A")]
    ask_qty: String,
}

impl ExchangeAdapter for BinanceAdapter {
    fn exchange() -> Exchange {
        Exchange::Binance
    }

    fn ws_url() -> &'static str {
        "wss://stream.binance.com:9443/ws"
    }

    fn extract_base_quote(symbol: &str) -> Option<(String, String)> {
        let symbol = symbol.to_uppercase();
        const QUOTES: &[&str] = &["USDT", "USDC", "BUSD", "USD"];
        for quote in QUOTES {
            if let Some(base) = symbol.strip_suffix(quote) {
                return Some((base.to_string(), (*quote).to_string()));
            }
        }
        None
    }

    fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        let mut messages = Vec::new();
        let mut id = 1;

        for chunk in symbols.chunks(50) {
            let depth_streams: Vec<String> = chunk
                .iter()
                .map(|s| format!("\"{}@depth20@100ms\"", s.to_lowercase()))
                .collect();

            messages.push(format!(
                r#"{{"method": "SUBSCRIBE", "params": [{}], "id": {}}}"#,
                depth_streams.join(", "),
                id
            ));
            id += 1;
        }

        messages
    }
}

impl BinanceAdapter {
    pub fn symbol_to_pair_id(symbol: &str) -> Option<u32> {
        Self::extract_base_symbol(symbol).map(|base| symbol_to_pair_id(&base))
    }

    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: BinanceTicker = serde_json::from_str(json)?;

        let price = ticker
            .close
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = ticker
            .bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker
            .ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Ok(PriceTick::new(
            Exchange::Binance,
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
        let ticker: BinanceTicker = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&ticker.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", ticker.symbol)))?;
        let pair_id = symbol_to_pair_id(&base);

        let price = ticker
            .close
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = ticker
            .bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker
            .ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let volume = ticker.volume.parse::<f64>().unwrap_or(0.0);
        let volume_usd = volume * price;

        Ok((
            PriceTick::new(
                Exchange::Binance,
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

    pub fn parse_book_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: BinanceBookTicker = serde_json::from_str(json)?;

        let bid = ticker
            .bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker
            .ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let mid = (bid + ask) / 2.0;

        Ok(PriceTick::new(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(mid),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ))
    }

    pub fn parse_book_ticker_auto(json: &str) -> Result<PriceTick, FeedError> {
        let ticker: BinanceBookTicker = serde_json::from_str(json)?;

        let pair_id = Self::symbol_to_pair_id(&ticker.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", ticker.symbol)))?;

        let bid = ticker
            .bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker
            .ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let mid = (bid + ask) / 2.0;

        Ok(PriceTick::new(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(mid),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ))
    }

    pub fn parse_book_ticker_with_base_quote(
        json: &str,
    ) -> Result<(PriceTick, String, String), FeedError> {
        let ticker: BinanceBookTicker = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&ticker.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", ticker.symbol)))?;
        let pair_id = symbol_to_pair_id(&base);

        let bid = ticker
            .bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker
            .ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid_size = ticker.bid_qty.parse::<f64>().unwrap_or(0.0);
        let ask_size = ticker.ask_qty.parse::<f64>().unwrap_or(0.0);
        let mid = (bid + ask) / 2.0;

        Ok((
            PriceTick::new(
                Exchange::Binance,
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

    pub fn is_book_ticker_message(json: &str) -> bool {
        json.contains("\"B\":") && json.contains("\"A\":") && !json.contains("\"c\":")
    }

    pub fn is_partial_depth_message(json: &str) -> bool {
        json.contains("\"bids\":")
            && json.contains("\"asks\":")
            && json.contains("\"lastUpdateId\":")
    }

    pub fn parse_partial_depth(
        json: &str,
    ) -> Result<(String, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
        #[derive(Debug, Deserialize)]
        struct BinancePartialDepth {
            #[serde(rename = "lastUpdateId")]
            _last_update_id: u64,
            bids: Vec<[String; 2]>,
            asks: Vec<[String; 2]>,
        }

        let depth: BinancePartialDepth = if json.contains("\"stream\":") {
            #[derive(Debug, Deserialize)]
            struct StreamWrapper {
                #[allow(dead_code)]
                stream: String,
                data: BinancePartialDepth,
            }
            let wrapper: StreamWrapper = serde_json::from_str(json)?;
            wrapper.data
        } else {
            serde_json::from_str(json)?
        };

        let bids: Vec<(f64, f64)> = depth
            .bids
            .iter()
            .filter_map(|[price, qty]| {
                let p = price.parse::<f64>().ok()?;
                let q = qty.parse::<f64>().ok()?;
                Some((p, q))
            })
            .collect();

        let asks: Vec<(f64, f64)> = depth
            .asks
            .iter()
            .filter_map(|[price, qty]| {
                let p = price.parse::<f64>().ok()?;
                let q = qty.parse::<f64>().ok()?;
                Some((p, q))
            })
            .collect();

        let symbol = if json.contains("\"stream\":") {
            #[derive(Debug, Deserialize)]
            struct StreamOnly {
                stream: String,
            }
            let s: StreamOnly = serde_json::from_str(json)?;
            s.stream
                .split('@')
                .next()
                .map(|s| s.to_uppercase())
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok((symbol, bids, asks))
    }

    pub fn parse_partial_depth_with_base_quote(
        json: &str,
    ) -> Result<(PriceTick, String, String, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
        let (symbol, bids, asks) = Self::parse_partial_depth(json)?;

        if bids.is_empty() || asks.is_empty() {
            return Err(FeedError::ParseError("Empty orderbook".to_string()));
        }

        let (base, quote) = Self::extract_base_quote(&symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", symbol)))?;
        let pair_id = symbol_to_pair_id(&base);

        let best_bid = bids[0].0;
        let best_ask = asks[0].0;
        let bid_size = bids[0].1;
        let ask_size = asks[0].1;
        let mid = (best_bid + best_ask) / 2.0;

        let tick = PriceTick::new(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(mid),
            FixedPoint::from_f64(best_bid),
            FixedPoint::from_f64(best_ask),
        )
        .with_sizes(
            FixedPoint::from_f64(bid_size),
            FixedPoint::from_f64(ask_size),
        );

        Ok((tick, base, quote, bids, asks))
    }

    pub fn subscribe_message(symbols: &[String]) -> String {
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("\"{}@ticker\"", s.to_lowercase()))
            .collect();

        format!(
            r#"{{"method": "SUBSCRIBE", "params": [{}], "id": 1}}"#,
            streams.join(", ")
        )
    }

    pub fn ws_url_combined(symbols: &[String]) -> String {
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@depth20@100ms", s.to_lowercase()))
            .collect();
        format!(
            "wss://stream.binance.com:9443/stream?streams={}",
            streams.join("/")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_parse_ticker() {
        let json = r#"{
            "e": "24hrTicker",
            "s": "BTCUSDT",
            "c": "50000.00",
            "b": "49999.00",
            "a": "50001.00",
            "v": "1000.00",
            "E": 1700000000000
        }"#;

        let tick = BinanceAdapter::parse_ticker(json, 1).unwrap();
        assert_eq!(tick.exchange(), Exchange::Binance);
        assert_eq!(tick.pair_id(), 1);
        assert!((tick.price().to_f64() - 50000.0).abs() < 0.01);
        assert!((tick.bid().to_f64() - 49999.0).abs() < 0.01);
        assert!((tick.ask().to_f64() - 50001.0).abs() < 0.01);
    }

    #[test]
    fn test_binance_parse_book_ticker() {
        let json = r#"{
            "s": "ETHUSDT",
            "b": "3000.00",
            "B": "10.5",
            "a": "3001.00",
            "A": "8.2"
        }"#;

        let tick = BinanceAdapter::parse_book_ticker(json, 2).unwrap();
        assert_eq!(tick.exchange(), Exchange::Binance);
        assert_eq!(tick.pair_id(), 2);
        assert!((tick.bid().to_f64() - 3000.0).abs() < 0.01);
        assert!((tick.ask().to_f64() - 3001.0).abs() < 0.01);
    }

    #[test]
    fn test_adapter_subscribe_message() {
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let msg = BinanceAdapter::subscribe_message(&symbols);
        assert!(msg.contains("SUBSCRIBE"));
        assert!(msg.contains("btcusdt@ticker"));
        assert!(msg.contains("ethusdt@ticker"));
    }

    #[test]
    fn test_binance_extract_quote_currency() {
        assert_eq!(
            BinanceAdapter::extract_quote_currency("BTCUSDT"),
            Some("USDT".to_string())
        );
        assert_eq!(
            BinanceAdapter::extract_quote_currency("BTCUSDC"),
            Some("USDC".to_string())
        );
        assert_eq!(
            BinanceAdapter::extract_quote_currency("BTCBUSD"),
            Some("BUSD".to_string())
        );
        assert_eq!(
            BinanceAdapter::extract_quote_currency("ETHUSDT"),
            Some("USDT".to_string())
        );
        assert_eq!(BinanceAdapter::extract_quote_currency("INVALID"), None);
    }

    #[test]
    fn test_binance_extract_base_quote() {
        assert_eq!(
            BinanceAdapter::extract_base_quote("BTCUSDT"),
            Some(("BTC".to_string(), "USDT".to_string()))
        );
        assert_eq!(
            BinanceAdapter::extract_base_quote("ETHUSDC"),
            Some(("ETH".to_string(), "USDC".to_string()))
        );
        assert_eq!(
            BinanceAdapter::extract_base_quote("SOLBUSD"),
            Some(("SOL".to_string(), "BUSD".to_string()))
        );
    }
}

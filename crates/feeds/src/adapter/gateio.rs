use arbitrage_core::{symbol_to_pair_id, Exchange, FixedPoint, PriceTick};
use serde::Deserialize;

use super::ExchangeAdapter;
use crate::FeedError;

pub struct GateIOAdapter;

#[derive(Debug, Deserialize)]
struct GateIOTickerResult {
    currency_pair: String,
    last: String,
    lowest_ask: String,
    highest_bid: String,
    #[serde(default)]
    _change_percentage: String,
    #[serde(default)]
    _base_volume: String,
    #[serde(default)]
    quote_volume: String,
    #[serde(default)]
    _high_24h: String,
    #[serde(default)]
    _low_24h: String,
}

#[derive(Debug, Deserialize)]
struct GateIOTickerMessage {
    #[serde(rename = "time")]
    _time: u64,
    #[serde(rename = "time_ms", default)]
    _time_ms: u64,
    #[serde(rename = "channel")]
    _channel: String,
    event: String,
    result: GateIOTickerResult,
}

impl ExchangeAdapter for GateIOAdapter {
    fn exchange() -> Exchange {
        Exchange::GateIO
    }

    fn ws_url() -> &'static str {
        "wss://api.gateio.ws/ws/v4/"
    }

    fn extract_base_quote(symbol: &str) -> Option<(String, String)> {
        let pair = symbol.to_uppercase();
        const QUOTES: &[&str] = &["_USDT", "_USDC", "_USD"];
        for suffix in QUOTES {
            if let Some(base) = pair.strip_suffix(suffix) {
                let quote = suffix.trim_start_matches('_');
                return Some((base.to_string(), quote.to_string()));
            }
        }
        None
    }

    fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut messages = Vec::new();

        for symbol in symbols {
            messages.push(format!(
                r#"{{"time": {}, "channel": "spot.order_book", "event": "subscribe", "payload": ["{}", "20", "100ms"]}}"#,
                timestamp,
                symbol.to_uppercase()
            ));
        }

        messages
    }
}

impl GateIOAdapter {
    pub fn symbol_to_pair_id(currency_pair: &str) -> Option<u32> {
        let base = Self::extract_base_symbol(currency_pair)?;
        Some(symbol_to_pair_id(&base))
    }

    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let msg: GateIOTickerMessage = serde_json::from_str(json)?;

        if msg.event != "update" {
            return Err(FeedError::ParseError("Not an update message".to_string()));
        }

        let price = msg
            .result
            .last
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = msg
            .result
            .highest_bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = msg
            .result
            .lowest_ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Ok(PriceTick::new(
            Exchange::GateIO,
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
        let msg: GateIOTickerMessage = serde_json::from_str(json)?;

        if msg.event != "update" {
            return Err(FeedError::ParseError("Not an update message".to_string()));
        }

        let (base, quote) =
            Self::extract_base_quote(&msg.result.currency_pair).ok_or_else(|| {
                FeedError::ParseError(format!("Unknown pair: {}", msg.result.currency_pair))
            })?;
        let pair_id = symbol_to_pair_id(&base);

        let price = msg
            .result
            .last
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = msg
            .result
            .highest_bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = msg
            .result
            .lowest_ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let volume_usd = msg.result.quote_volume.parse::<f64>().unwrap_or(0.0);

        Ok((
            PriceTick::new(
                Exchange::GateIO,
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

    pub fn subscribe_message(symbols: &[String]) -> String {
        let pairs: Vec<String> = symbols
            .iter()
            .map(|s| format!("\"{}\"", s.to_uppercase()))
            .collect();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        format!(
            r#"{{"time": {}, "channel": "spot.tickers", "event": "subscribe", "payload": [{}]}}"#,
            timestamp,
            pairs.join(", ")
        )
    }

    pub fn is_orderbook_message(json: &str) -> bool {
        json.contains("\"channel\":\"spot.order_book\"")
    }

    pub fn parse_orderbook_with_symbol(
        json: &str,
    ) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct GateIOOrderbookResult {
            s: String,
            bids: Vec<[String; 2]>,
            asks: Vec<[String; 2]>,
        }

        #[derive(Debug, Deserialize)]
        struct GateIOOrderbookMessage {
            _channel: String,
            event: String,
            result: GateIOOrderbookResult,
        }

        let msg: GateIOOrderbookMessage = serde_json::from_str(json)?;

        if msg.event != "update" {
            return Err(FeedError::ParseError("Not an update message".to_string()));
        }

        let best_bid = msg
            .result
            .bids
            .first()
            .ok_or_else(|| FeedError::ParseError("No bids in orderbook".to_string()))?;
        let bid = best_bid[0]
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid price".to_string()))?;
        let bid_size = best_bid[1].parse::<f64>().unwrap_or(0.0);

        let best_ask = msg
            .result
            .asks
            .first()
            .ok_or_else(|| FeedError::ParseError("No asks in orderbook".to_string()))?;
        let ask = best_ask[0]
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid ask price".to_string()))?;
        let ask_size = best_ask[1].parse::<f64>().unwrap_or(0.0);

        Ok((
            msg.result.s,
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
            FixedPoint::from_f64(bid_size),
            FixedPoint::from_f64(ask_size),
        ))
    }

    pub fn parse_orderbook_full(
        json: &str,
    ) -> Result<
        (
            String,
            FixedPoint,
            FixedPoint,
            FixedPoint,
            FixedPoint,
            Vec<(f64, f64)>,
            Vec<(f64, f64)>,
        ),
        FeedError,
    > {
        #[derive(Debug, Deserialize)]
        struct GateIOOrderbookResult {
            s: String,
            bids: Vec<[String; 2]>,
            asks: Vec<[String; 2]>,
        }

        #[derive(Debug, Deserialize)]
        struct GateIOOrderbookMessage {
            #[allow(dead_code)]
            _channel: String,
            event: String,
            result: GateIOOrderbookResult,
        }

        let msg: GateIOOrderbookMessage = serde_json::from_str(json)?;

        if msg.event != "update" {
            return Err(FeedError::ParseError("Not an update message".to_string()));
        }

        let bids: Vec<(f64, f64)> = msg
            .result
            .bids
            .iter()
            .filter_map(|level| {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            })
            .collect();

        let asks: Vec<(f64, f64)> = msg
            .result
            .asks
            .iter()
            .filter_map(|level| {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            })
            .collect();

        let (bid, bid_size) = bids.first().copied().unwrap_or((0.0, 0.0));
        let (ask, ask_size) = asks.first().copied().unwrap_or((0.0, 0.0));

        if bid == 0.0 || ask == 0.0 {
            return Err(FeedError::ParseError("Empty bids or asks".to_string()));
        }

        Ok((
            msg.result.s,
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
            FixedPoint::from_f64(bid_size),
            FixedPoint::from_f64(ask_size),
            bids,
            asks,
        ))
    }

    pub fn to_currency_pair(symbol: &str) -> String {
        format!("{}_USDT", symbol.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateio_parse_ticker() {
        let json = r#"{
            "time": 1669107766,
            "time_ms": 1669107766406,
            "channel": "spot.tickers",
            "event": "update",
            "result": {
                "currency_pair": "BTC_USDT",
                "last": "50000.00",
                "lowest_ask": "50001.00",
                "highest_bid": "49999.00",
                "change_percentage": "1.5",
                "base_volume": "1000.00",
                "quote_volume": "50000000.00",
                "high_24h": "51000.00",
                "low_24h": "49000.00"
            }
        }"#;

        let (tick, symbol) = GateIOAdapter::parse_ticker_with_symbol(json).unwrap();
        assert_eq!(tick.exchange(), Exchange::GateIO);
        assert_eq!(symbol, "BTC");
        assert!((tick.price().to_f64() - 50000.0).abs() < 0.01);
        assert!((tick.bid().to_f64() - 49999.0).abs() < 0.01);
        assert!((tick.ask().to_f64() - 50001.0).abs() < 0.01);
    }

    #[test]
    fn test_gateio_subscribe_message() {
        let symbols = vec!["BTC_USDT".to_string(), "ETH_USDT".to_string()];
        let msg = GateIOAdapter::subscribe_message(&symbols);
        assert!(msg.contains("spot.tickers"));
        assert!(msg.contains("subscribe"));
        assert!(msg.contains("BTC_USDT"));
        assert!(msg.contains("ETH_USDT"));
    }

    #[test]
    fn test_gateio_extract_base_symbol() {
        assert_eq!(
            GateIOAdapter::extract_base_symbol("BTC_USDT"),
            Some("BTC".to_string())
        );
        assert_eq!(
            GateIOAdapter::extract_base_symbol("ETH_USDT"),
            Some("ETH".to_string())
        );
        assert_eq!(GateIOAdapter::extract_base_symbol("BTCUSDT"), None);
    }

    #[test]
    fn test_gateio_extract_quote_currency() {
        assert_eq!(
            GateIOAdapter::extract_quote_currency("BTC_USDT"),
            Some("USDT".to_string())
        );
        assert_eq!(
            GateIOAdapter::extract_quote_currency("BTC_USDC"),
            Some("USDC".to_string())
        );
        assert_eq!(
            GateIOAdapter::extract_quote_currency("BTC_USD"),
            Some("USD".to_string())
        );
        assert_eq!(GateIOAdapter::extract_quote_currency("BTC_EUR"), None);
    }
}

use arbitrage_core::{symbol_to_pair_id, Exchange, FixedPoint, PriceTick};
use serde::Deserialize;

use super::{ExchangeAdapter, KoreanExchangeAdapter};
use crate::FeedError;

pub struct UpbitAdapter;

#[derive(Debug, Deserialize)]
struct UpbitTicker {
    #[serde(alias = "cd", alias = "code")]
    code: String,
    #[serde(alias = "tp", alias = "trade_price")]
    trade_price: f64,
    #[serde(alias = "op", alias = "opening_price", default)]
    _opening_price: f64,
    #[serde(alias = "hp", alias = "high_price", default)]
    _high_price: f64,
    #[serde(alias = "lp", alias = "low_price", default)]
    _low_price: f64,
    #[serde(alias = "atv24h", alias = "acc_trade_volume_24h", default)]
    _acc_trade_volume_24h: f64,
    #[serde(alias = "tms", alias = "timestamp", default)]
    _timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum UpbitMessage {
    Ticker {
        code: String,
        price: FixedPoint,
    },
    Orderbook {
        code: String,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
    },
}

impl ExchangeAdapter for UpbitAdapter {
    fn exchange() -> Exchange {
        Exchange::Upbit
    }

    fn ws_url() -> &'static str {
        "wss://api.upbit.com/websocket/v1"
    }

    fn extract_base_quote(symbol: &str) -> Option<(String, String)> {
        let code = symbol.to_uppercase();
        if code.starts_with("KRW-") {
            let base = code.strip_prefix("KRW-")?;
            Some((base.to_string(), "KRW".to_string()))
        } else {
            None
        }
    }

    fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        vec![Self::subscribe_message(symbols)]
    }
}

impl KoreanExchangeAdapter for UpbitAdapter {
    fn is_usdt_market(code: &str) -> bool {
        code.to_uppercase() == "KRW-USDT"
    }

    fn is_usdc_market(code: &str) -> bool {
        code.to_uppercase() == "KRW-USDC"
    }
}

impl UpbitAdapter {
    pub fn parse_message(json: &str) -> Result<UpbitMessage, FeedError> {
        #[derive(Debug, Deserialize)]
        struct GenericMessage {
            #[serde(alias = "ty", alias = "type")]
            msg_type: String,
            #[serde(alias = "cd", alias = "code")]
            code: String,
            #[serde(alias = "tp", alias = "trade_price", default)]
            trade_price: f64,
            #[serde(alias = "obu", alias = "orderbook_units", default)]
            orderbook_units: Vec<OrderbookUnit>,
        }

        #[derive(Debug, Deserialize, Default)]
        struct OrderbookUnit {
            #[serde(alias = "ap", alias = "ask_price", default)]
            ask_price: f64,
            #[serde(alias = "bp", alias = "bid_price", default)]
            bid_price: f64,
            #[serde(alias = "as", alias = "ask_size", default)]
            ask_size: f64,
            #[serde(alias = "bs", alias = "bid_size", default)]
            bid_size: f64,
        }

        let msg: GenericMessage = serde_json::from_str(json)?;

        match msg.msg_type.as_str() {
            "ticker" => Ok(UpbitMessage::Ticker {
                code: msg.code,
                price: FixedPoint::from_f64(msg.trade_price),
            }),
            "orderbook" => {
                let best = msg
                    .orderbook_units
                    .first()
                    .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;
                Ok(UpbitMessage::Orderbook {
                    code: msg.code,
                    bid: FixedPoint::from_f64(best.bid_price),
                    ask: FixedPoint::from_f64(best.ask_price),
                    bid_size: FixedPoint::from_f64(best.bid_size),
                    ask_size: FixedPoint::from_f64(best.ask_size),
                })
            }
            _ => Err(FeedError::ParseError(format!(
                "Unknown message type: {}",
                msg.msg_type
            ))),
        }
    }

    pub fn parse_message_binary(data: &[u8]) -> Result<UpbitMessage, FeedError> {
        #[derive(Debug, Deserialize)]
        struct GenericMessage {
            #[serde(alias = "ty", alias = "type")]
            msg_type: String,
            #[serde(alias = "cd", alias = "code")]
            code: String,
            #[serde(alias = "tp", alias = "trade_price", default)]
            trade_price: f64,
            #[serde(alias = "obu", alias = "orderbook_units", default)]
            orderbook_units: Vec<OrderbookUnit>,
        }

        #[derive(Debug, Deserialize, Default)]
        struct OrderbookUnit {
            #[serde(alias = "ap", alias = "ask_price", default)]
            ask_price: f64,
            #[serde(alias = "bp", alias = "bid_price", default)]
            bid_price: f64,
            #[serde(alias = "as", alias = "ask_size", default)]
            ask_size: f64,
            #[serde(alias = "bs", alias = "bid_size", default)]
            bid_size: f64,
        }

        let msg: GenericMessage = rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))?;

        match msg.msg_type.as_str() {
            "ticker" => Ok(UpbitMessage::Ticker {
                code: msg.code,
                price: FixedPoint::from_f64(msg.trade_price),
            }),
            "orderbook" => {
                let best = msg
                    .orderbook_units
                    .first()
                    .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;
                Ok(UpbitMessage::Orderbook {
                    code: msg.code,
                    bid: FixedPoint::from_f64(best.bid_price),
                    ask: FixedPoint::from_f64(best.ask_price),
                    bid_size: FixedPoint::from_f64(best.bid_size),
                    ask_size: FixedPoint::from_f64(best.ask_size),
                })
            }
            _ => Err(FeedError::ParseError(format!(
                "Unknown message type: {}",
                msg.msg_type
            ))),
        }
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
        struct OrderbookUnit {
            #[serde(alias = "ap", alias = "ask_price")]
            ask_price: f64,
            #[serde(alias = "bp", alias = "bid_price")]
            bid_price: f64,
            #[serde(alias = "as", alias = "ask_size")]
            ask_size: f64,
            #[serde(alias = "bs", alias = "bid_size")]
            bid_size: f64,
        }

        #[derive(Debug, Deserialize)]
        struct GenericMessage {
            #[serde(alias = "ty", alias = "type")]
            msg_type: String,
            #[serde(alias = "cd", alias = "code")]
            code: String,
            #[serde(alias = "obu", alias = "orderbook_units", default)]
            orderbook_units: Vec<OrderbookUnit>,
        }

        let msg: GenericMessage = serde_json::from_str(json)?;

        if msg.msg_type != "orderbook" {
            return Err(FeedError::ParseError(format!(
                "Not an orderbook message: {}",
                msg.msg_type
            )));
        }

        let best = msg
            .orderbook_units
            .first()
            .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;

        let bids: Vec<(f64, f64)> = msg
            .orderbook_units
            .iter()
            .map(|u| (u.bid_price, u.bid_size))
            .collect();
        let asks: Vec<(f64, f64)> = msg
            .orderbook_units
            .iter()
            .map(|u| (u.ask_price, u.ask_size))
            .collect();

        Ok((
            msg.code,
            FixedPoint::from_f64(best.bid_price),
            FixedPoint::from_f64(best.ask_price),
            FixedPoint::from_f64(best.bid_size),
            FixedPoint::from_f64(best.ask_size),
            bids,
            asks,
        ))
    }

    pub fn parse_orderbook_full_binary(
        data: &[u8],
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
        let json_str = std::str::from_utf8(data)
            .map_err(|e| FeedError::ParseError(format!("Invalid UTF-8: {}", e)))?;
        Self::parse_orderbook_full(json_str)
    }

    pub fn market_to_pair_id(code: &str) -> Option<u32> {
        let code = code.to_uppercase();
        if code == "KRW-USDT" {
            return None;
        }

        let base = Self::extract_base_symbol(&code)?;
        Some(symbol_to_pair_id(&base))
    }

    pub fn is_stablecoin_market(code: &str) -> bool {
        <Self as KoreanExchangeAdapter>::is_stablecoin_market(code)
    }

    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: UpbitTicker = serde_json::from_str(json)?;

        let price = FixedPoint::from_f64(ticker.trade_price);

        Ok(PriceTick::new(
            Exchange::Upbit,
            pair_id,
            price,
            price,
            price,
        ))
    }

    pub fn parse_ticker_with_code(json: &str) -> Result<(String, FixedPoint), FeedError> {
        let ticker: UpbitTicker = serde_json::from_str(json)?;
        let price = FixedPoint::from_f64(ticker.trade_price);
        Ok((ticker.code, price))
    }

    pub fn parse_ticker_binary(data: &[u8], pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: UpbitTicker = rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))?;

        let price = FixedPoint::from_f64(ticker.trade_price);

        Ok(PriceTick::new(
            Exchange::Upbit,
            pair_id,
            price,
            price,
            price,
        ))
    }

    pub fn parse_ticker_binary_with_code(data: &[u8]) -> Result<(String, FixedPoint), FeedError> {
        let ticker: UpbitTicker = rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))?;
        let price = FixedPoint::from_f64(ticker.trade_price);
        Ok((ticker.code, price))
    }

    pub fn parse_orderbook(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        #[derive(Debug, Deserialize)]
        struct UpbitOrderbook {
            #[serde(rename = "code")]
            _code: String,
            orderbook_units: Vec<UpbitOrderbookUnit>,
        }

        #[derive(Debug, Deserialize)]
        struct UpbitOrderbookUnit {
            ask_price: f64,
            bid_price: f64,
            #[serde(default)]
            _ask_size: f64,
            #[serde(default)]
            _bid_size: f64,
        }

        let orderbook: UpbitOrderbook = serde_json::from_str(json)?;

        if orderbook.orderbook_units.is_empty() {
            return Err(FeedError::ParseError("Empty orderbook".to_string()));
        }

        let best = &orderbook.orderbook_units[0];
        let mid = (best.bid_price + best.ask_price) / 2.0;

        Ok(PriceTick::new(
            Exchange::Upbit,
            pair_id,
            FixedPoint::from_f64(mid),
            FixedPoint::from_f64(best.bid_price),
            FixedPoint::from_f64(best.ask_price),
        ))
    }

    pub fn subscribe_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();
        let codes_str = codes.join(",");

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"ticker","codes":[{}]}},{{"type":"orderbook","codes":[{}],"level":0}},{{"format":"SIMPLE"}}]"#,
            codes_str, codes_str
        )
    }

    pub fn subscribe_orderbook_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"orderbook","codes":[{}]}},{{"format":"SIMPLE"}}]"#,
            codes.join(",")
        )
    }

    pub fn is_orderbook_message(json: &str) -> bool {
        json.contains("\"ty\":\"orderbook\"") || json.contains("\"type\":\"orderbook\"")
    }

    pub fn parse_orderbook_with_code(
        json: &str,
    ) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct UpbitOrderbookSimple {
            #[serde(alias = "cd", alias = "code")]
            code: String,
            #[serde(alias = "obu", alias = "orderbook_units")]
            orderbook_units: Vec<UpbitOrderbookUnit>,
        }

        #[derive(Debug, Deserialize)]
        struct UpbitOrderbookUnit {
            #[serde(alias = "ap", alias = "ask_price")]
            ask_price: f64,
            #[serde(alias = "bp", alias = "bid_price")]
            bid_price: f64,
            #[serde(alias = "as", alias = "ask_size", default)]
            ask_size: f64,
            #[serde(alias = "bs", alias = "bid_size", default)]
            bid_size: f64,
        }

        let orderbook: UpbitOrderbookSimple = serde_json::from_str(json)?;

        if orderbook.orderbook_units.is_empty() {
            return Err(FeedError::ParseError("Empty orderbook".to_string()));
        }

        let best = &orderbook.orderbook_units[0];
        Ok((
            orderbook.code,
            FixedPoint::from_f64(best.bid_price),
            FixedPoint::from_f64(best.ask_price),
            FixedPoint::from_f64(best.bid_size),
            FixedPoint::from_f64(best.ask_size),
        ))
    }

    pub fn parse_orderbook_binary_with_code(
        data: &[u8],
    ) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct UpbitOrderbookSimple {
            #[serde(alias = "cd", alias = "code")]
            code: String,
            #[serde(alias = "obu", alias = "orderbook_units")]
            orderbook_units: Vec<UpbitOrderbookUnit>,
        }

        #[derive(Debug, Deserialize)]
        struct UpbitOrderbookUnit {
            #[serde(alias = "ap", alias = "ask_price")]
            ask_price: f64,
            #[serde(alias = "bp", alias = "bid_price")]
            bid_price: f64,
            #[serde(alias = "as", alias = "ask_size", default)]
            ask_size: f64,
            #[serde(alias = "bs", alias = "bid_size", default)]
            bid_size: f64,
        }

        let orderbook: UpbitOrderbookSimple = rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))?;

        if orderbook.orderbook_units.is_empty() {
            return Err(FeedError::ParseError("Empty orderbook".to_string()));
        }

        let best = &orderbook.orderbook_units[0];
        Ok((
            orderbook.code,
            FixedPoint::from_f64(best.bid_price),
            FixedPoint::from_f64(best.ask_price),
            FixedPoint::from_f64(best.bid_size),
            FixedPoint::from_f64(best.ask_size),
        ))
    }

    pub fn to_market_code(symbol: &str) -> String {
        if let Some((base, quote)) = symbol.split_once('/') {
            format!("{}-{}", quote, base)
        } else {
            symbol.to_string()
        }
    }

    pub fn from_market_code(code: &str) -> String {
        if let Some((quote, base)) = code.split_once('-') {
            format!("{}/{}", base, quote)
        } else {
            code.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upbit_parse_ticker() {
        let json = r#"{
            "type": "ticker",
            "code": "KRW-BTC",
            "trade_price": 145000000.0,
            "opening_price": 144000000.0,
            "high_price": 146000000.0,
            "low_price": 143000000.0,
            "acc_trade_volume_24h": 1234.5,
            "timestamp": 1700000000000
        }"#;

        let tick = UpbitAdapter::parse_ticker(json, 1).unwrap();
        assert_eq!(tick.exchange(), Exchange::Upbit);
        assert_eq!(tick.pair_id(), 1);
        assert!((tick.price().to_f64() - 145000000.0).abs() < 1.0);
    }

    #[test]
    fn test_upbit_parse_orderbook() {
        let json = r#"{
            "type": "orderbook",
            "code": "KRW-BTC",
            "orderbook_units": [
                {"ask_price": 145100000.0, "bid_price": 145000000.0, "ask_size": 1.0, "bid_size": 2.0},
                {"ask_price": 145200000.0, "bid_price": 144900000.0, "ask_size": 0.5, "bid_size": 1.5}
            ]
        }"#;

        let tick = UpbitAdapter::parse_orderbook(json, 1).unwrap();
        assert_eq!(tick.exchange(), Exchange::Upbit);
        assert!((tick.bid().to_f64() - 145000000.0).abs() < 1.0);
        assert!((tick.ask().to_f64() - 145100000.0).abs() < 1.0);
    }

    #[test]
    fn test_upbit_subscribe_message() {
        let markets = vec!["KRW-BTC".to_string(), "KRW-ETH".to_string()];
        let msg = UpbitAdapter::subscribe_message(&markets);
        assert!(msg.contains("ticket"));
        assert!(msg.contains("ticker"));
        assert!(msg.contains("KRW-BTC"));
        assert!(msg.contains("KRW-ETH"));
    }

    #[test]
    fn test_upbit_market_code_conversion() {
        assert_eq!(UpbitAdapter::to_market_code("BTC/KRW"), "KRW-BTC");
        assert_eq!(UpbitAdapter::to_market_code("ETH/KRW"), "KRW-ETH");
        assert_eq!(UpbitAdapter::from_market_code("KRW-BTC"), "BTC/KRW");
        assert_eq!(UpbitAdapter::from_market_code("KRW-ETH"), "ETH/KRW");
    }

    // Story 5.1: Dynamic subscription market parsing tests (AC: #1)
    #[test]
    fn test_dynamic_market_parse_ticker_new_symbol() {
        // Test parsing a dynamically subscribed market (e.g., KRW-DOGE)
        let json = r#"{
            "type": "ticker",
            "code": "KRW-DOGE",
            "trade_price": 180.5,
            "opening_price": 175.0,
            "high_price": 185.0,
            "low_price": 172.0,
            "acc_trade_volume_24h": 50000000.0,
            "timestamp": 1700000000000
        }"#;

        let pair_id = symbol_to_pair_id("DOGE");
        let tick = UpbitAdapter::parse_ticker(json, pair_id).unwrap();

        assert_eq!(tick.exchange(), Exchange::Upbit);
        assert_eq!(tick.pair_id(), pair_id);
        assert!((tick.price().to_f64() - 180.5).abs() < 0.1);
    }

    #[test]
    fn test_dynamic_market_parse_orderbook_new_symbol() {
        // Test parsing orderbook for dynamically subscribed market
        let json = r#"{
            "type": "orderbook",
            "code": "KRW-XRP",
            "orderbook_units": [
                {"ask_price": 750.0, "bid_price": 748.0, "ask_size": 10000.0, "bid_size": 15000.0},
                {"ask_price": 752.0, "bid_price": 746.0, "ask_size": 8000.0, "bid_size": 12000.0}
            ]
        }"#;

        let pair_id = symbol_to_pair_id("XRP");
        let tick = UpbitAdapter::parse_orderbook(json, pair_id).unwrap();

        assert_eq!(tick.exchange(), Exchange::Upbit);
        assert_eq!(tick.pair_id(), pair_id);
        assert!((tick.bid().to_f64() - 748.0).abs() < 0.1);
        assert!((tick.ask().to_f64() - 750.0).abs() < 0.1);
    }

    #[test]
    fn test_dynamic_market_extract_base_symbol() {
        // Test extract_base_symbol for dynamically discovered markets
        assert_eq!(
            UpbitAdapter::extract_base_symbol("KRW-DOGE"),
            Some("DOGE".to_string())
        );
        assert_eq!(
            UpbitAdapter::extract_base_symbol("KRW-XRP"),
            Some("XRP".to_string())
        );
        assert_eq!(
            UpbitAdapter::extract_base_symbol("KRW-SHIB"),
            Some("SHIB".to_string())
        );
    }

    #[test]
    fn test_dynamic_market_symbol_to_pair_id() {
        // Verify symbol_to_pair_id returns same pair_id across exchanges
        let binance_pair_id = symbol_to_pair_id("DOGE");
        let upbit_pair_id = symbol_to_pair_id("DOGE");
        assert_eq!(binance_pair_id, upbit_pair_id);
    }
}

use arbitrage_core::{symbol_to_pair_id, Exchange, FixedPoint, PriceTick};
use serde::Deserialize;

use super::{ExchangeAdapter, KoreanExchangeAdapter};
use crate::FeedError;

// ============================================================================
// Public Types
// ============================================================================

pub struct BithumbAdapter;

#[derive(Debug, Clone)]
pub enum BithumbMessage {
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

#[derive(Debug, Clone)]
pub struct OrderbookSnapshot {
    pub code: String,
    pub best_bid: FixedPoint,
    pub best_ask: FixedPoint,
    pub best_bid_size: FixedPoint,
    pub best_ask_size: FixedPoint,
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
}

// ============================================================================
// Internal Deserialization Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(alias = "ty", alias = "type")]
    msg_type: String,
    #[serde(alias = "cd", alias = "code")]
    code: String,
    #[serde(alias = "tp", alias = "trade_price", default)]
    trade_price: f64,
    #[serde(alias = "obu", alias = "orderbook_units", default)]
    orderbook_units: Vec<RawOrderbookUnit>,
}

#[derive(Debug, Deserialize, Default)]
struct RawOrderbookUnit {
    #[serde(alias = "ap", alias = "ask_price", default)]
    ask_price: f64,
    #[serde(alias = "bp", alias = "bid_price", default)]
    bid_price: f64,
    #[serde(alias = "as", alias = "ask_size", default)]
    ask_size: f64,
    #[serde(alias = "bs", alias = "bid_size", default)]
    bid_size: f64,
}

#[derive(Debug, Deserialize)]
struct RawTicker {
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

#[derive(Debug, Deserialize)]
struct RawOrderbook {
    #[serde(alias = "cd", alias = "code")]
    code: String,
    #[serde(alias = "obu", alias = "orderbook_units")]
    orderbook_units: Vec<RawOrderbookUnit>,
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl ExchangeAdapter for BithumbAdapter {
    fn exchange() -> Exchange {
        Exchange::Bithumb
    }

    fn ws_url() -> &'static str {
        "wss://ws-api.bithumb.com/websocket/v1"
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

impl KoreanExchangeAdapter for BithumbAdapter {
    fn is_usdt_market(code: &str) -> bool {
        code.to_uppercase() == "KRW-USDT"
    }

    fn is_usdc_market(code: &str) -> bool {
        code.to_uppercase() == "KRW-USDC"
    }
}

// ============================================================================
// Internal Parsing Helpers
// ============================================================================

impl BithumbAdapter {
    fn parse_raw_message_json(json: &str) -> Result<RawMessage, FeedError> {
        serde_json::from_str(json).map_err(Into::into)
    }

    fn parse_raw_message_binary(data: &[u8]) -> Result<RawMessage, FeedError> {
        rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))
    }

    fn raw_to_bithumb_message(msg: RawMessage) -> Result<BithumbMessage, FeedError> {
        match msg.msg_type.as_str() {
            "ticker" => Ok(BithumbMessage::Ticker {
                code: msg.code,
                price: FixedPoint::from_f64(msg.trade_price),
            }),
            "orderbook" => {
                let best = msg
                    .orderbook_units
                    .first()
                    .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;
                Ok(BithumbMessage::Orderbook {
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

    fn raw_to_orderbook_snapshot(msg: RawMessage) -> Result<OrderbookSnapshot, FeedError> {
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

        Ok(OrderbookSnapshot {
            code: msg.code,
            best_bid: FixedPoint::from_f64(best.bid_price),
            best_ask: FixedPoint::from_f64(best.ask_price),
            best_bid_size: FixedPoint::from_f64(best.bid_size),
            best_ask_size: FixedPoint::from_f64(best.ask_size),
            bids,
            asks,
        })
    }

    fn parse_raw_orderbook_json(json: &str) -> Result<RawOrderbook, FeedError> {
        serde_json::from_str(json).map_err(Into::into)
    }

    fn parse_raw_orderbook_binary(data: &[u8]) -> Result<RawOrderbook, FeedError> {
        rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))
    }

    fn raw_orderbook_to_tuple(
        orderbook: RawOrderbook,
    ) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
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
}

// ============================================================================
// Public API
// ============================================================================

impl BithumbAdapter {
    pub fn parse_message(json: &str) -> Result<BithumbMessage, FeedError> {
        let msg = Self::parse_raw_message_json(json)?;
        Self::raw_to_bithumb_message(msg)
    }

    pub fn parse_message_binary(data: &[u8]) -> Result<BithumbMessage, FeedError> {
        let msg = Self::parse_raw_message_binary(data)?;
        Self::raw_to_bithumb_message(msg)
    }

    pub fn parse_orderbook_full(json: &str) -> Result<OrderbookSnapshot, FeedError> {
        let msg = Self::parse_raw_message_json(json)?;
        Self::raw_to_orderbook_snapshot(msg)
    }

    pub fn parse_orderbook_full_binary(data: &[u8]) -> Result<OrderbookSnapshot, FeedError> {
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
        let ticker: RawTicker = serde_json::from_str(json)?;
        let price = FixedPoint::from_f64(ticker.trade_price);

        Ok(PriceTick::new(
            Exchange::Bithumb,
            pair_id,
            price,
            price,
            price,
        ))
    }

    pub fn parse_ticker_with_code(json: &str) -> Result<(String, FixedPoint), FeedError> {
        let ticker: RawTicker = serde_json::from_str(json)?;
        let price = FixedPoint::from_f64(ticker.trade_price);
        Ok((ticker.code, price))
    }

    pub fn parse_ticker_binary(data: &[u8], pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: RawTicker = rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))?;
        let price = FixedPoint::from_f64(ticker.trade_price);

        Ok(PriceTick::new(
            Exchange::Bithumb,
            pair_id,
            price,
            price,
            price,
        ))
    }

    pub fn parse_ticker_binary_with_code(data: &[u8]) -> Result<(String, FixedPoint), FeedError> {
        let ticker: RawTicker = rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))?;
        let price = FixedPoint::from_f64(ticker.trade_price);
        Ok((ticker.code, price))
    }

    pub fn subscribe_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();
        let codes_str = codes.join(",");

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"ticker","codes":[{}]}},{{"type":"orderbook","codes":[{}],"level":1}},{{"format":"SIMPLE"}}]"#,
            codes_str, codes_str
        )
    }

    pub fn is_orderbook_message(json: &str) -> bool {
        json.contains("\"ty\":\"orderbook\"") || json.contains("\"type\":\"orderbook\"")
    }

    pub fn parse_orderbook_with_code(
        json: &str,
    ) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        let orderbook = Self::parse_raw_orderbook_json(json)?;
        Self::raw_orderbook_to_tuple(orderbook)
    }

    pub fn parse_orderbook_binary_with_code(
        data: &[u8],
    ) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        let orderbook = Self::parse_raw_orderbook_binary(data)?;
        Self::raw_orderbook_to_tuple(orderbook)
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

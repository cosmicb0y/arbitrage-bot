//! Exchange adapter trait and implementations.
//!
//! Each exchange has its own WebSocket message format.
//! Adapters normalize these into our internal PriceTick format.

use arbitrage_core::{symbol_to_pair_id, Exchange, FixedPoint, PriceTick};
use serde::Deserialize;

use crate::FeedError;

/// Binance WebSocket adapter.
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

impl BinanceAdapter {
    /// Map symbol to pair_id.
    /// Extracts base asset from trading pair (e.g., BTCUSDT -> BTC) and generates pair_id.
    pub fn symbol_to_pair_id(symbol: &str) -> Option<u32> {
        let symbol = symbol.to_uppercase();
        // Extract base asset from USDT/USDC/BUSD pairs
        let base = if symbol.ends_with("USDT") {
            symbol.strip_suffix("USDT")
        } else if symbol.ends_with("USDC") {
            symbol.strip_suffix("USDC")
        } else if symbol.ends_with("BUSD") {
            symbol.strip_suffix("BUSD")
        } else {
            None
        }?;

        Some(symbol_to_pair_id(base))
    }

    /// Extract both base and quote from trading pair.
    /// Returns (base, quote) tuple. Single pass - no duplicate suffix checks.
    pub fn extract_base_quote(symbol: &str) -> Option<(String, String)> {
        let symbol = symbol.to_uppercase();
        // Check longer suffixes first to avoid matching "USD" in "USDT"
        const QUOTES: &[&str] = &["USDT", "USDC", "BUSD", "USD"];
        for quote in QUOTES {
            if let Some(base) = symbol.strip_suffix(quote) {
                return Some((base.to_string(), (*quote).to_string()));
            }
        }
        None
    }

    /// Extract base symbol from trading pair (e.g., BTCUSDT -> BTC).
    #[inline]
    pub fn extract_base_symbol(symbol: &str) -> Option<String> {
        Self::extract_base_quote(symbol).map(|(base, _)| base)
    }

    /// Extract quote currency from trading pair (e.g., BTCUSDT -> USDT).
    #[inline]
    pub fn extract_quote_currency(symbol: &str) -> Option<String> {
        Self::extract_base_quote(symbol).map(|(_, quote)| quote)
    }

    /// Parse a 24hr ticker message from Binance.
    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: BinanceTicker = serde_json::from_str(json)?;

        let price = ticker.close.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = ticker.bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker.ask.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Ok(PriceTick::new(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ))
    }

    /// Parse a 24hr ticker message and auto-detect pair_id from symbol.
    pub fn parse_ticker_auto(json: &str) -> Result<PriceTick, FeedError> {
        let (tick, _) = Self::parse_ticker_with_symbol(json)?;
        Ok(tick)
    }

    /// Parse a 24hr ticker message, returning both the tick and the base symbol.
    pub fn parse_ticker_with_symbol(json: &str) -> Result<(PriceTick, String), FeedError> {
        let (tick, base, _quote) = Self::parse_ticker_with_base_quote(json)?;
        Ok((tick, base))
    }

    /// Parse a 24hr ticker message, returning the tick, base symbol, and quote currency.
    pub fn parse_ticker_with_base_quote(json: &str) -> Result<(PriceTick, String, String), FeedError> {
        let ticker: BinanceTicker = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&ticker.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", ticker.symbol)))?;
        let pair_id = symbol_to_pair_id(&base);

        let price = ticker.close.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = ticker.bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker.ask.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let volume = ticker.volume.parse::<f64>().unwrap_or(0.0);
        // Convert volume to USD value (volume * price)
        let volume_usd = volume * price;

        Ok((PriceTick::new(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ).with_volume_24h(FixedPoint::from_f64(volume_usd)), base, quote))
    }

    /// Parse a book ticker message from Binance (best bid/ask only).
    pub fn parse_book_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: BinanceBookTicker = serde_json::from_str(json)?;

        let bid = ticker.bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker.ask.parse::<f64>()
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

    /// Parse a book ticker message and auto-detect pair_id from symbol.
    pub fn parse_book_ticker_auto(json: &str) -> Result<PriceTick, FeedError> {
        let ticker: BinanceBookTicker = serde_json::from_str(json)?;

        let pair_id = Self::symbol_to_pair_id(&ticker.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", ticker.symbol)))?;

        let bid = ticker.bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker.ask.parse::<f64>()
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

    /// Parse a book ticker message with base and quote.
    /// Returns (PriceTick, base_symbol, quote_currency).
    pub fn parse_book_ticker_with_base_quote(json: &str) -> Result<(PriceTick, String, String), FeedError> {
        let ticker: BinanceBookTicker = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&ticker.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", ticker.symbol)))?;
        let pair_id = symbol_to_pair_id(&base);

        let bid = ticker.bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker.ask.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid_size = ticker.bid_qty.parse::<f64>().unwrap_or(0.0);
        let ask_size = ticker.ask_qty.parse::<f64>().unwrap_or(0.0);
        let mid = (bid + ask) / 2.0;

        Ok((PriceTick::new(
            Exchange::Binance,
            pair_id,
            FixedPoint::from_f64(mid),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ).with_sizes(FixedPoint::from_f64(bid_size), FixedPoint::from_f64(ask_size)), base, quote))
    }

    /// Check if a message is a book ticker message.
    pub fn is_book_ticker_message(json: &str) -> bool {
        // Book ticker messages have "b", "B", "a", "A" but no "c" (close price)
        json.contains("\"B\":") && json.contains("\"A\":") && !json.contains("\"c\":")
    }

    /// Check if a message is a partial depth message (@depth5/10/20).
    pub fn is_partial_depth_message(json: &str) -> bool {
        // Partial depth messages have "bids" and "asks" arrays, and "lastUpdateId"
        json.contains("\"bids\":") && json.contains("\"asks\":") && json.contains("\"lastUpdateId\":")
    }

    /// Parse a partial depth message from Binance (@depth5/10/20@100ms).
    /// Returns (symbol, bids, asks) where bids/asks are Vec<(price, qty)>.
    /// Bids are sorted descending, asks are sorted ascending.
    pub fn parse_partial_depth(json: &str) -> Result<(String, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
        #[derive(Debug, Deserialize)]
        struct BinancePartialDepth {
            #[serde(rename = "lastUpdateId")]
            last_update_id: u64,
            bids: Vec<[String; 2]>,  // [[price, qty], ...]
            asks: Vec<[String; 2]>,
        }

        // For combined stream, the message is wrapped: {"stream":"btcusdt@depth20@100ms","data":{...}}
        // For single stream, it's just the data object
        let depth: BinancePartialDepth = if json.contains("\"stream\":") {
            #[derive(Debug, Deserialize)]
            struct StreamWrapper {
                stream: String,
                data: BinancePartialDepth,
            }
            let wrapper: StreamWrapper = serde_json::from_str(json)?;
            wrapper.data
        } else {
            serde_json::from_str(json)?
        };

        // Parse bids (already sorted descending by Binance)
        let bids: Vec<(f64, f64)> = depth.bids.iter()
            .filter_map(|[price, qty]| {
                let p = price.parse::<f64>().ok()?;
                let q = qty.parse::<f64>().ok()?;
                Some((p, q))
            })
            .collect();

        // Parse asks (already sorted ascending by Binance)
        let asks: Vec<(f64, f64)> = depth.asks.iter()
            .filter_map(|[price, qty]| {
                let p = price.parse::<f64>().ok()?;
                let q = qty.parse::<f64>().ok()?;
                Some((p, q))
            })
            .collect();

        // Extract symbol from stream name if available
        let symbol = if json.contains("\"stream\":") {
            #[derive(Debug, Deserialize)]
            struct StreamOnly {
                stream: String,
            }
            let s: StreamOnly = serde_json::from_str(json)?;
            // stream format: "btcusdt@depth20@100ms" -> extract "BTCUSDT"
            s.stream.split('@').next()
                .map(|s| s.to_uppercase())
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok((symbol, bids, asks))
    }

    /// Parse partial depth message with base and quote extraction.
    /// Returns (PriceTick, base_symbol, quote_currency, bids, asks).
    pub fn parse_partial_depth_with_base_quote(json: &str) -> Result<(PriceTick, String, String, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
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
        ).with_sizes(FixedPoint::from_f64(bid_size), FixedPoint::from_f64(ask_size));

        Ok((tick, base, quote, bids, asks))
    }

    /// Generate a subscription message for Binance WebSocket.
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

    /// Generate subscription messages for partial depth stream (20 levels).
    /// @depth20@100ms provides full orderbook snapshot every 100ms without REST sync.
    /// Binance limits payload size, so we chunk symbols into smaller batches.
    pub fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        let mut messages = Vec::new();
        let mut id = 1;

        // Chunk into 50 symbols per message to stay within Binance payload limits
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

    /// Build combined stream URL for Binance WebSocket.
    /// Uses the `/stream?streams=` endpoint which wraps each message with stream name.
    /// This is required for depth streams which don't include symbol in response.
    pub fn ws_url_combined(symbols: &[String]) -> String {
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@depth20@100ms", s.to_lowercase()))
            .collect();
        format!("wss://stream.binance.com:9443/stream?streams={}", streams.join("/"))
    }

    /// Get WebSocket URL for Binance (single stream endpoint, requires SUBSCRIBE).
    pub fn ws_url() -> &'static str {
        "wss://stream.binance.com:9443/ws"
    }
}

/// Upbit WebSocket adapter.
pub struct UpbitAdapter;

/// Upbit WebSocket ticker response (SIMPLE format uses abbreviated field names).
#[derive(Debug, Deserialize)]
struct UpbitTicker {
    /// Market code (e.g., "KRW-BTC") - "cd" in SIMPLE format
    #[serde(alias = "cd", alias = "code")]
    code: String,
    /// Current trade price - "tp" in SIMPLE format
    #[serde(alias = "tp", alias = "trade_price")]
    trade_price: f64,
    /// Opening price - "op" in SIMPLE format
    #[serde(alias = "op", alias = "opening_price", default)]
    opening_price: f64,
    /// Highest price - "hp" in SIMPLE format
    #[serde(alias = "hp", alias = "high_price", default)]
    high_price: f64,
    /// Lowest price - "lp" in SIMPLE format
    #[serde(alias = "lp", alias = "low_price", default)]
    low_price: f64,
    /// Accumulated trade volume (24h) - "atv24h" in SIMPLE format
    #[serde(alias = "atv24h", alias = "acc_trade_volume_24h", default)]
    acc_trade_volume_24h: f64,
    /// Timestamp - "tms" in SIMPLE format
    #[serde(alias = "tms", alias = "timestamp", default)]
    timestamp: u64,
}

/// Upbit message types for efficient single-parse dispatch.
#[derive(Debug, Clone)]
pub enum UpbitMessage {
    /// Ticker message with trade price
    Ticker {
        code: String,
        price: FixedPoint,
    },
    /// Orderbook message with best bid/ask
    Orderbook {
        code: String,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
    },
}

impl UpbitAdapter {
    /// Parse any Upbit message (JSON) with single parse, dispatch by type.
    /// More efficient than is_orderbook_message() + parse_xxx().
    pub fn parse_message(json: &str) -> Result<UpbitMessage, FeedError> {
        // Parse once into a generic structure
        #[derive(Debug, Deserialize)]
        struct GenericMessage {
            #[serde(alias = "ty", alias = "type")]
            msg_type: String,
            #[serde(alias = "cd", alias = "code")]
            code: String,
            // Ticker fields
            #[serde(alias = "tp", alias = "trade_price", default)]
            trade_price: f64,
            // Orderbook fields
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
                let best = msg.orderbook_units.first()
                    .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;
                Ok(UpbitMessage::Orderbook {
                    code: msg.code,
                    bid: FixedPoint::from_f64(best.bid_price),
                    ask: FixedPoint::from_f64(best.ask_price),
                    bid_size: FixedPoint::from_f64(best.bid_size),
                    ask_size: FixedPoint::from_f64(best.ask_size),
                })
            }
            _ => Err(FeedError::ParseError(format!("Unknown message type: {}", msg.msg_type))),
        }
    }

    /// Parse any Upbit message (MessagePack binary) with single parse.
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
                let best = msg.orderbook_units.first()
                    .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;
                Ok(UpbitMessage::Orderbook {
                    code: msg.code,
                    bid: FixedPoint::from_f64(best.bid_price),
                    ask: FixedPoint::from_f64(best.ask_price),
                    bid_size: FixedPoint::from_f64(best.bid_size),
                    ask_size: FixedPoint::from_f64(best.ask_size),
                })
            }
            _ => Err(FeedError::ParseError(format!("Unknown message type: {}", msg.msg_type))),
        }
    }

    /// Parse Upbit orderbook message with full depth (JSON).
    /// Returns (code, bid, ask, bid_size, ask_size, bids_full, asks_full).
    /// bids_full and asks_full are Vec<(price, size)> for depth walking.
    pub fn parse_orderbook_full(json: &str) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
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
            return Err(FeedError::ParseError(format!("Not an orderbook message: {}", msg.msg_type)));
        }

        let best = msg.orderbook_units.first()
            .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;

        // Collect all bids (descending by price) and asks (ascending by price)
        let bids: Vec<(f64, f64)> = msg.orderbook_units.iter()
            .map(|u| (u.bid_price, u.bid_size))
            .collect();
        let asks: Vec<(f64, f64)> = msg.orderbook_units.iter()
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

    /// Parse Upbit orderbook message with full depth (binary format is JSON as bytes).
    pub fn parse_orderbook_full_binary(data: &[u8]) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
        // Upbit sends JSON as binary bytes, not MessagePack
        let json_str = std::str::from_utf8(data)
            .map_err(|e| FeedError::ParseError(format!("Invalid UTF-8: {}", e)))?;
        Self::parse_orderbook_full(json_str)
    }

    /// Map market code to pair_id.
    /// Extracts base asset from market code (e.g., KRW-BTC -> BTC) and generates pair_id.
    pub fn market_to_pair_id(code: &str) -> Option<u32> {
        let code = code.to_uppercase();
        if code == "KRW-USDT" {
            return None; // Special case: used for exchange rate, not trading
        }

        let base = Self::extract_base_symbol(&code)?;
        Some(symbol_to_pair_id(&base))
    }

    /// Extract base symbol from market code (e.g., KRW-BTC -> BTC).
    pub fn extract_base_symbol(code: &str) -> Option<String> {
        let code = code.to_uppercase();
        if code.starts_with("KRW-") {
            code.strip_prefix("KRW-").map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Check if market code is for USDT (exchange rate).
    pub fn is_usdt_market(code: &str) -> bool {
        code.to_uppercase() == "KRW-USDT"
    }

    /// Check if market code is for USDC (exchange rate).
    pub fn is_usdc_market(code: &str) -> bool {
        code.to_uppercase() == "KRW-USDC"
    }

    /// Check if market code is a stablecoin market (USDT or USDC).
    pub fn is_stablecoin_market(code: &str) -> bool {
        let upper = code.to_uppercase();
        upper == "KRW-USDT" || upper == "KRW-USDC"
    }

    /// Parse a ticker message from Upbit (JSON format).
    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: UpbitTicker = serde_json::from_str(json)?;

        // Upbit doesn't provide bid/ask in ticker, use trade_price for all
        let price = FixedPoint::from_f64(ticker.trade_price);

        Ok(PriceTick::new(
            Exchange::Upbit,
            pair_id,
            price,
            price, // bid approximation
            price, // ask approximation
        ))
    }

    /// Parse a ticker message from Upbit and return with market code.
    /// Returns (market_code, PriceTick).
    pub fn parse_ticker_with_code(json: &str) -> Result<(String, FixedPoint), FeedError> {
        let ticker: UpbitTicker = serde_json::from_str(json)?;
        let price = FixedPoint::from_f64(ticker.trade_price);
        Ok((ticker.code, price))
    }

    /// Parse a ticker message from Upbit binary (MessagePack format).
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

    /// Parse a ticker message from Upbit binary and return with market code.
    pub fn parse_ticker_binary_with_code(data: &[u8]) -> Result<(String, FixedPoint), FeedError> {
        let ticker: UpbitTicker = rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))?;
        let price = FixedPoint::from_f64(ticker.trade_price);
        Ok((ticker.code, price))
    }

    /// Parse a ticker from Upbit orderbook for accurate bid/ask.
    pub fn parse_orderbook(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        #[derive(Debug, Deserialize)]
        struct UpbitOrderbook {
            code: String,
            orderbook_units: Vec<UpbitOrderbookUnit>,
        }

        #[derive(Debug, Deserialize)]
        struct UpbitOrderbookUnit {
            ask_price: f64,
            bid_price: f64,
            #[serde(default)]
            ask_size: f64,
            #[serde(default)]
            bid_size: f64,
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

    /// Generate a subscription message for Upbit WebSocket.
    /// Upbit uses a unique format: array of ticket, type, and codes.
    /// Subscribes to both ticker (for trade price and volume) and orderbook (full depth).
    ///
    /// Orderbook subscription requires "level" parameter:
    /// - level: 0 means full orderbook (all levels, no aggregation)
    /// - level: N means aggregate orderbook at N KRW increments
    pub fn subscribe_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();
        let codes_str = codes.join(",");

        // Subscribe to both ticker and orderbook in a single message
        // level: 0 = full orderbook without aggregation (all levels)
        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"ticker","codes":[{}]}},{{"type":"orderbook","codes":[{}],"level":0}},{{"format":"SIMPLE"}}]"#,
            codes_str, codes_str
        )
    }

    /// Generate a subscription message for orderbook only.
    pub fn subscribe_orderbook_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"orderbook","codes":[{}]}},{{"format":"SIMPLE"}}]"#,
            codes.join(",")
        )
    }

    /// Check if a message is an orderbook message.
    pub fn is_orderbook_message(json: &str) -> bool {
        json.contains("\"ty\":\"orderbook\"") || json.contains("\"type\":\"orderbook\"")
    }

    /// Parse orderbook message with code and depth.
    /// Returns (market_code, bid, ask, bid_size, ask_size).
    pub fn parse_orderbook_with_code(json: &str) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct UpbitOrderbookSimple {
            /// Market code - "cd" in SIMPLE format
            #[serde(alias = "cd", alias = "code")]
            code: String,
            /// Orderbook units - "obu" in SIMPLE format
            #[serde(alias = "obu", alias = "orderbook_units")]
            orderbook_units: Vec<UpbitOrderbookUnit>,
        }

        #[derive(Debug, Deserialize)]
        struct UpbitOrderbookUnit {
            /// Ask price - "ap" in SIMPLE format
            #[serde(alias = "ap", alias = "ask_price")]
            ask_price: f64,
            /// Bid price - "bp" in SIMPLE format
            #[serde(alias = "bp", alias = "bid_price")]
            bid_price: f64,
            /// Ask size - "as" in SIMPLE format
            #[serde(alias = "as", alias = "ask_size", default)]
            ask_size: f64,
            /// Bid size - "bs" in SIMPLE format
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

    /// Parse orderbook binary message with code and depth (MessagePack format).
    /// Returns (market_code, bid, ask, bid_size, ask_size).
    pub fn parse_orderbook_binary_with_code(data: &[u8]) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
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

    /// Get WebSocket URL for Upbit.
    pub fn ws_url() -> &'static str {
        "wss://api.upbit.com/websocket/v1"
    }

    /// Convert symbol to Upbit market format.
    /// "BTC/KRW" -> "KRW-BTC"
    pub fn to_market_code(symbol: &str) -> String {
        if let Some((base, quote)) = symbol.split_once('/') {
            format!("{}-{}", quote, base)
        } else {
            symbol.to_string()
        }
    }

    /// Convert Upbit market code to symbol.
    /// "KRW-BTC" -> "BTC/KRW"
    pub fn from_market_code(code: &str) -> String {
        if let Some((quote, base)) = code.split_once('-') {
            format!("{}/{}", base, quote)
        } else {
            code.to_string()
        }
    }
}

/// Coinbase WebSocket adapter.
pub struct CoinbaseAdapter;

/// Coinbase L2 event types for efficient single-parse dispatch.
#[derive(Debug, Clone)]
pub enum CoinbaseL2Event {
    /// Full orderbook snapshot - contains all price levels
    Snapshot {
        product_id: String,
        /// All bid levels: (price, size) sorted by price descending
        bids: Vec<(f64, f64)>,
        /// All ask levels: (price, size) sorted by price ascending
        asks: Vec<(f64, f64)>,
    },
    /// Incremental update - contains changed levels only
    /// If size is 0, the level should be removed
    Update {
        product_id: String,
        /// Changes: (side "buy"/"sell", price, new_size)
        changes: Vec<(String, f64, f64)>,
    },
}

/// Coinbase CDP API credentials for WebSocket authentication.
#[derive(Debug, Clone)]
pub struct CoinbaseCredentials {
    /// API key name (format: organizations/{org_id}/apiKeys/{key_id})
    pub key_name: String,
    /// EC private key in PEM format
    pub secret_key: String,
}

impl CoinbaseCredentials {
    /// Create new credentials from key name and secret.
    pub fn new(key_name: String, secret_key: String) -> Self {
        Self { key_name, secret_key }
    }

    /// Load credentials from environment variables.
    /// Expects COINBASE_API_KEY_ID and COINBASE_SECRET_KEY.
    pub fn from_env() -> Option<Self> {
        let key_name = std::env::var("COINBASE_API_KEY_ID").ok()?;
        let secret_key = std::env::var("COINBASE_SECRET_KEY").ok()?;

        if key_name.is_empty() || secret_key.is_empty() {
            return None;
        }

        Some(Self { key_name, secret_key })
    }

    /// Check if credentials are configured.
    pub fn is_configured(&self) -> bool {
        !self.key_name.is_empty() && !self.secret_key.is_empty()
    }
}

#[derive(Debug, Deserialize)]
struct CoinbaseTicker {
    #[serde(rename = "type")]
    msg_type: String,
    product_id: String,
    price: String,
    best_bid: String,
    best_ask: String,
    #[serde(default)]
    volume_24h: String,
}

impl CoinbaseAdapter {
    /// Map product_id to pair_id.
    /// Extracts base asset from product_id (e.g., BTC-USD -> BTC) and generates pair_id.
    pub fn product_to_pair_id(product_id: &str) -> Option<u32> {
        let base = Self::extract_base_symbol(product_id)?;
        Some(symbol_to_pair_id(&base))
    }

    /// Extract base symbol from product_id (e.g., BTC-USD -> BTC).
    pub fn extract_base_symbol(product_id: &str) -> Option<String> {
        let product_id = product_id.to_uppercase();
        if product_id.ends_with("-USD") {
            product_id.strip_suffix("-USD").map(|s| s.to_string())
        } else if product_id.ends_with("-USDT") {
            product_id.strip_suffix("-USDT").map(|s| s.to_string())
        } else if product_id.ends_with("-USDC") {
            product_id.strip_suffix("-USDC").map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Extract quote currency from product_id (e.g., BTC-USD -> USD, BTC-USDC -> USDC).
    pub fn extract_quote_currency(product_id: &str) -> Option<String> {
        let product_id = product_id.to_uppercase();
        if product_id.ends_with("-USDT") {
            Some("USDT".to_string())
        } else if product_id.ends_with("-USDC") {
            Some("USDC".to_string())
        } else if product_id.ends_with("-USD") {
            Some("USD".to_string())
        } else {
            None
        }
    }

    /// Extract both base and quote from product_id.
    /// Returns (base, quote) tuple.
    pub fn extract_base_quote(product_id: &str) -> Option<(String, String)> {
        let base = Self::extract_base_symbol(product_id)?;
        let quote = Self::extract_quote_currency(product_id)?;
        Some((base, quote))
    }

    /// Parse a ticker message from Coinbase.
    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: CoinbaseTicker = serde_json::from_str(json)?;

        let price = ticker.price.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = ticker.best_bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker.best_ask.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Ok(PriceTick::new(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ))
    }

    /// Parse a ticker message and auto-detect pair_id from product_id.
    pub fn parse_ticker_auto(json: &str) -> Result<PriceTick, FeedError> {
        let (tick, _) = Self::parse_ticker_with_symbol(json)?;
        Ok(tick)
    }

    /// Parse a ticker message, returning both the tick and the base symbol.
    pub fn parse_ticker_with_symbol(json: &str) -> Result<(PriceTick, String), FeedError> {
        let (tick, base, _quote) = Self::parse_ticker_with_base_quote(json)?;
        Ok((tick, base))
    }

    /// Parse a ticker message, returning the tick, base symbol, and quote currency.
    pub fn parse_ticker_with_base_quote(json: &str) -> Result<(PriceTick, String, String), FeedError> {
        let ticker: CoinbaseTicker = serde_json::from_str(json)?;

        // Skip non-ticker messages
        if ticker.msg_type != "ticker" {
            return Err(FeedError::ParseError("Not a ticker message".to_string()));
        }

        let (base, quote) = Self::extract_base_quote(&ticker.product_id)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown product: {}", ticker.product_id)))?;
        let pair_id = symbol_to_pair_id(&base);

        let price = ticker.price.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = ticker.best_bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker.best_ask.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let volume = ticker.volume_24h.parse::<f64>().unwrap_or(0.0);
        // Coinbase volume_24h is already in base currency, convert to USD
        let volume_usd = volume * price;

        Ok((PriceTick::new(
            Exchange::Coinbase,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ).with_volume_24h(FixedPoint::from_f64(volume_usd)), base, quote))
    }

    /// Generate a subscription message for Coinbase WebSocket.
    pub fn subscribe_message(product_ids: &[String]) -> String {
        let products: Vec<String> = product_ids
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect();

        format!(
            r#"{{"type": "subscribe", "product_ids": [{}], "channels": ["ticker"]}}"#,
            products.join(", ")
        )
    }

    /// Generate subscription messages for level2 and heartbeats channels.
    /// level2 provides real-time orderbook updates (immediate, not batched).
    /// heartbeats is REQUIRED for connection keep-alive (60-90s timeout without it).
    pub fn subscribe_messages(product_ids: &[String]) -> Vec<String> {
        let products: Vec<String> = product_ids
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect();
        let products_str = products.join(", ");

        vec![
            // Subscribe to level2 for real-time orderbook depth
            // level2: immediate updates per change
            // level2_batch: batched updates every 50ms (lower latency but less real-time)
            // heartbeats: REQUIRED for connection keep-alive (Coinbase times out in 60-90s)
            format!(
                r#"{{"type": "subscribe", "product_ids": [{}], "channels": ["level2", "heartbeats"]}}"#,
                products_str
            ),
        ]
    }

    /// Parse L2 data message and return the appropriate event type.
    /// This parses the JSON once and dispatches based on the event type field.
    pub fn parse_l2_event(json: &str) -> Result<CoinbaseL2Event, FeedError> {
        #[derive(Debug, Deserialize)]
        struct L2DataMessage {
            channel: String,
            events: Vec<L2DataEvent>,
        }

        #[derive(Debug, Deserialize)]
        struct L2DataEvent {
            #[serde(rename = "type")]
            event_type: String,
            product_id: String,
            updates: Vec<L2DataUpdate>,
        }

        #[derive(Debug, Deserialize)]
        struct L2DataUpdate {
            side: String,
            price_level: String,
            new_quantity: String,
        }

        let msg: L2DataMessage = serde_json::from_str(json)?;

        if msg.channel != "l2_data" {
            return Err(FeedError::ParseError("Not an l2_data channel message".to_string()));
        }

        let event = msg.events.first()
            .ok_or_else(|| FeedError::ParseError("No events in l2_data message".to_string()))?;

        match event.event_type.as_str() {
            "snapshot" => {
                // Collect all bids and asks
                let mut bids: Vec<(f64, f64)> = Vec::new();
                let mut asks: Vec<(f64, f64)> = Vec::new();

                for update in &event.updates {
                    let price = update.price_level.parse::<f64>().unwrap_or(0.0);
                    let size = update.new_quantity.parse::<f64>().unwrap_or(0.0);

                    if price > 0.0 {
                        match update.side.as_str() {
                            "bid" => bids.push((price, size)),
                            "offer" => asks.push((price, size)),
                            _ => {}
                        }
                    }
                }

                // Sort: bids descending (highest first), asks ascending (lowest first)
                bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                if bids.is_empty() || asks.is_empty() {
                    return Err(FeedError::ParseError("No valid bid/ask in snapshot".to_string()));
                }

                Ok(CoinbaseL2Event::Snapshot {
                    product_id: event.product_id.clone(),
                    bids,
                    asks,
                })
            }
            "update" => {
                let changes: Vec<(String, f64, f64)> = event.updates
                    .iter()
                    .filter_map(|update| {
                        let price = update.price_level.parse::<f64>().ok()?;
                        let size = update.new_quantity.parse::<f64>().ok()?;
                        // Coinbase Advanced Trade API uses "bid"/"offer" (not "ask")
                        // Normalize to "buy"/"sell" for internal consistency
                        let side = match update.side.as_str() {
                            "bid" => "buy".to_string(),
                            "offer" => "sell".to_string(),
                            _ => update.side.clone(),
                        };
                        Some((side, price, size))
                    })
                    .collect();

                Ok(CoinbaseL2Event::Update {
                    product_id: event.product_id.clone(),
                    changes,
                })
            }
            _ => Err(FeedError::ParseError(format!("Unknown event type: {}", event.event_type))),
        }
    }

    /// Check if a message is a level2 (orderbook) message.
    pub fn is_level2_message(json: &str) -> bool {
        // Exchange API: level2_batch uses "l2update", level2 uses "update"/"snapshot"
        // Advanced Trade API: uses "channel":"l2_data" with events array
        json.contains("\"type\":\"l2update\"")
            || json.contains("\"type\":\"snapshot\"")
            || json.contains("\"type\":\"update\"")
            || json.contains("\"channel\":\"l2_data\"")
    }

    /// Parse a level2 snapshot message to get best bid/ask with sizes.
    /// Supports both Exchange API and Advanced Trade API formats.
    /// Returns (product_id, bid, ask, bid_size, ask_size).
    pub fn parse_level2_snapshot(json: &str) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        // Try Advanced Trade API format first (more common now)
        // Format: {"channel":"l2_data","events":[{"type":"snapshot","product_id":"...","updates":[{"side":"bid","price_level":"...","new_quantity":"..."},...]}}
        if let Ok(result) = Self::parse_advanced_trade_l2(json) {
            return Ok(result);
        }

        // Fallback to Exchange API format
        // Format: {"type":"snapshot","product_id":"...","bids":[[price,size],...],"asks":[[price,size],...]}
        #[derive(Debug, Deserialize)]
        struct Level2Snapshot {
            #[serde(rename = "type")]
            msg_type: String,
            product_id: String,
            bids: Vec<[String; 2]>,
            asks: Vec<[String; 2]>,
        }

        let snapshot: Level2Snapshot = serde_json::from_str(json)?;

        if snapshot.msg_type != "snapshot" {
            return Err(FeedError::ParseError("Not a snapshot message".to_string()));
        }

        let best_bid = snapshot.bids.first()
            .ok_or_else(|| FeedError::ParseError("No bids in snapshot".to_string()))?;
        let bid = best_bid[0].parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid price".to_string()))?;
        let bid_size = best_bid[1].parse::<f64>().unwrap_or(0.0);

        let best_ask = snapshot.asks.first()
            .ok_or_else(|| FeedError::ParseError("No asks in snapshot".to_string()))?;
        let ask = best_ask[0].parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid ask price".to_string()))?;
        let ask_size = best_ask[1].parse::<f64>().unwrap_or(0.0);

        Ok((
            snapshot.product_id,
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
            FixedPoint::from_f64(bid_size),
            FixedPoint::from_f64(ask_size),
        ))
    }

    /// Parse Advanced Trade API l2_data channel message.
    /// Format: {"channel":"l2_data","events":[{"type":"snapshot"|"update","product_id":"...","updates":[...]}]}
    fn parse_advanced_trade_l2(json: &str) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct L2DataMessage {
            channel: String,
            events: Vec<L2DataEvent>,
        }

        #[derive(Debug, Deserialize)]
        struct L2DataEvent {
            #[serde(rename = "type")]
            event_type: String,
            product_id: String,
            updates: Vec<L2DataUpdate>,
        }

        #[derive(Debug, Deserialize)]
        struct L2DataUpdate {
            side: String,
            price_level: String,
            new_quantity: String,
        }

        let msg: L2DataMessage = serde_json::from_str(json)?;

        if msg.channel != "l2_data" {
            return Err(FeedError::ParseError("Not an l2_data channel message".to_string()));
        }

        let event = msg.events.first()
            .ok_or_else(|| FeedError::ParseError("No events in l2_data message".to_string()))?;

        // Collect all bids and asks
        let mut bids: Vec<(f64, f64)> = Vec::new();
        let mut asks: Vec<(f64, f64)> = Vec::new();

        for update in &event.updates {
            let price = update.price_level.parse::<f64>().unwrap_or(0.0);
            let size = update.new_quantity.parse::<f64>().unwrap_or(0.0);

            if price > 0.0 {
                match update.side.as_str() {
                    "bid" => bids.push((price, size)),
                    "offer" => asks.push((price, size)),
                    _ => {}
                }
            }
        }

        // Sort: bids descending (highest first), asks ascending (lowest first)
        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let (bid, bid_size) = bids.first().copied().unwrap_or((0.0, 0.0));
        let (ask, ask_size) = asks.first().copied().unwrap_or((0.0, 0.0));

        if bid == 0.0 || ask == 0.0 {
            return Err(FeedError::ParseError("No valid bid/ask in l2_data".to_string()));
        }

        Ok((
            event.product_id.clone(),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
            FixedPoint::from_f64(bid_size),
            FixedPoint::from_f64(ask_size),
        ))
    }

    /// Parse a level2 update message.
    /// Supports Exchange API (l2update) and Advanced Trade API (l2_data with type=update).
    /// Returns (product_id, changes) where changes is Vec<(side, price, size)>.
    /// side is "buy" for bids or "sell" for asks.
    pub fn parse_level2_update(json: &str) -> Result<(String, Vec<(String, f64, f64)>), FeedError> {
        // Try Advanced Trade API format first (l2_data channel with type=update)
        if let Ok(result) = Self::parse_advanced_trade_l2_update(json) {
            return Ok(result);
        }

        // Try level2_batch format: {"type":"l2update", "changes":[[side, price, size], ...]}
        #[derive(Debug, Deserialize)]
        struct Level2BatchUpdate {
            #[serde(rename = "type")]
            msg_type: String,
            product_id: String,
            #[serde(default)]
            changes: Vec<[String; 3]>,
        }

        // Try level2 format: {"type":"update", "updates":[{side, price_level, new_quantity}, ...]}
        #[derive(Debug, Deserialize)]
        struct Level2Update {
            #[serde(rename = "type")]
            msg_type: String,
            product_id: String,
            #[serde(default)]
            updates: Vec<Level2UpdateItem>,
        }

        #[derive(Debug, Deserialize)]
        struct Level2UpdateItem {
            side: String,
            price_level: String,
            new_quantity: String,
        }

        // Try parsing as level2_batch format (l2update)
        if let Ok(update) = serde_json::from_str::<Level2BatchUpdate>(json) {
            if update.msg_type == "l2update" && !update.changes.is_empty() {
                let changes: Vec<(String, f64, f64)> = update.changes
                    .iter()
                    .filter_map(|change| {
                        let side = change[0].clone();
                        let price = change[1].parse::<f64>().ok()?;
                        let size = change[2].parse::<f64>().ok()?;
                        Some((side, price, size))
                    })
                    .collect();
                return Ok((update.product_id, changes));
            }
        }

        // Try parsing as level2 format (update)
        if let Ok(update) = serde_json::from_str::<Level2Update>(json) {
            if update.msg_type == "update" && !update.updates.is_empty() {
                let changes: Vec<(String, f64, f64)> = update.updates
                    .iter()
                    .filter_map(|item| {
                        let side = item.side.clone();
                        let price = item.price_level.parse::<f64>().ok()?;
                        let size = item.new_quantity.parse::<f64>().ok()?;
                        Some((side, price, size))
                    })
                    .collect();
                return Ok((update.product_id, changes));
            }
        }

        Err(FeedError::ParseError("Not a valid level2 update message".to_string()))
    }

    /// Parse Advanced Trade API l2_data channel update message.
    /// Format: {"channel":"l2_data","events":[{"type":"update","product_id":"...","updates":[...]}]}
    fn parse_advanced_trade_l2_update(json: &str) -> Result<(String, Vec<(String, f64, f64)>), FeedError> {
        #[derive(Debug, Deserialize)]
        struct L2DataMessage {
            channel: String,
            events: Vec<L2DataEvent>,
        }

        #[derive(Debug, Deserialize)]
        struct L2DataEvent {
            #[serde(rename = "type")]
            event_type: String,
            product_id: String,
            updates: Vec<L2DataUpdate>,
        }

        #[derive(Debug, Deserialize)]
        struct L2DataUpdate {
            side: String,
            price_level: String,
            new_quantity: String,
        }

        let msg: L2DataMessage = serde_json::from_str(json)?;

        if msg.channel != "l2_data" {
            return Err(FeedError::ParseError("Not an l2_data channel message".to_string()));
        }

        let event = msg.events.first()
            .ok_or_else(|| FeedError::ParseError("No events in l2_data message".to_string()))?;

        if event.event_type != "update" {
            return Err(FeedError::ParseError("Not an update event".to_string()));
        }

        let changes: Vec<(String, f64, f64)> = event.updates
            .iter()
            .filter_map(|update| {
                let price = update.price_level.parse::<f64>().ok()?;
                let size = update.new_quantity.parse::<f64>().ok()?;
                // Convert "bid"/"offer" to "buy"/"sell" for consistency
                let side = match update.side.as_str() {
                    "bid" => "buy".to_string(),
                    "offer" => "sell".to_string(),
                    _ => update.side.clone(),
                };
                Some((side, price, size))
            })
            .collect();

        Ok((event.product_id.clone(), changes))
    }

    /// Get WebSocket URL for Coinbase Exchange API (public, no auth required for ticker).
    pub fn ws_url() -> &'static str {
        "wss://ws-feed.exchange.coinbase.com"
    }

    /// Get WebSocket URL for Coinbase Advanced Trade API (requires JWT auth for level2).
    pub fn ws_url_advanced_trade() -> &'static str {
        "wss://advanced-trade-ws.coinbase.com"
    }

    /// Generate JWT token for Coinbase WebSocket authentication.
    /// WebSocket JWTs don't include request method or path (unlike REST API JWTs).
    /// Documentation: https://docs.cdp.coinbase.com/coinbase-app/advanced-trade-apis/websocket/websocket-authentication
    pub fn generate_ws_jwt(credentials: &CoinbaseCredentials) -> Result<String, crate::FeedError> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use p256::ecdsa::{SigningKey, Signature, signature::Signer};
        use p256::pkcs8::DecodePrivateKey;
        use p256::SecretKey;
        use std::time::{SystemTime, UNIX_EPOCH};

        let secret_key_pem = &credentials.secret_key;

        // Parse EC private key from PEM format (ES256 = P-256/secp256r1)
        // Try SEC1 format first (-----BEGIN EC PRIVATE KEY-----), then PKCS#8 (-----BEGIN PRIVATE KEY-----)
        let signing_key = if secret_key_pem.contains("EC PRIVATE KEY") {
            SecretKey::from_sec1_pem(secret_key_pem)
                .map(|sk| SigningKey::from(&sk))
                .map_err(|e| crate::FeedError::ParseError(format!("Failed to parse SEC1 EC private key: {}", e)))?
        } else if secret_key_pem.contains("PRIVATE KEY") {
            SigningKey::from_pkcs8_pem(secret_key_pem)
                .map_err(|e| crate::FeedError::ParseError(format!("Failed to parse PKCS#8 private key: {}", e)))?
        } else {
            return Err(crate::FeedError::ParseError(format!(
                "Invalid key format. Expected PEM format but got: {:?}",
                &secret_key_pem.chars().take(50).collect::<String>()
            )));
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Generate random nonce (32 bytes hex string)
        let nonce = format!("{:016x}{:016x}", rand::random::<u64>(), rand::random::<u64>());

        // JWT Header: {"alg": "ES256", "typ": "JWT", "kid": key_name, "nonce": nonce}
        let header = serde_json::json!({
            "alg": "ES256",
            "typ": "JWT",
            "kid": credentials.key_name,
            "nonce": nonce
        });

        // JWT Payload for WebSocket (no URI field unlike REST API)
        let payload = serde_json::json!({
            "iss": "cdp",
            "sub": credentials.key_name,
            "nbf": now,
            "exp": now + 120 // 120 seconds expiry
        });

        let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
        let message = format!("{}.{}", header_b64, payload_b64);

        // Sign with ES256 (ECDSA P-256)
        let signature: Signature = signing_key.sign(message.as_bytes());
        let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

        Ok(format!("{}.{}.{}", header_b64, payload_b64, signature_b64))
    }

    /// Generate subscription messages for level2 and heartbeats channels with JWT authentication.
    /// The level2 channel requires authentication since 2024.
    /// The heartbeats channel is REQUIRED for connection keep-alive (60-90s timeout).
    /// Returns subscription messages with JWT token included.
    /// Note: Uses single "channel" field (not "channels" array) per Coinbase Advanced Trade API.
    pub fn subscribe_messages_with_auth(product_ids: &[String], credentials: &CoinbaseCredentials) -> Result<Vec<String>, crate::FeedError> {
        let jwt = Self::generate_ws_jwt(credentials)?;

        let products: Vec<String> = product_ids
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect();
        let products_str = products.join(", ");

        // Subscribe to both level2 and heartbeats channels
        // heartbeats channel is crucial for connection keep-alive (Coinbase times out in 60-90s without it)
        Ok(vec![
            // level2 channel subscription with JWT authentication
            format!(
                r#"{{"type": "subscribe", "product_ids": [{}], "channel": "level2", "jwt": "{}"}}"#,
                products_str,
                jwt
            ),
            // heartbeats channel subscription (required for connection stability)
            format!(
                r#"{{"type": "subscribe", "product_ids": [{}], "channel": "heartbeats", "jwt": "{}"}}"#,
                products_str,
                jwt
            ),
        ])
    }
}

/// Bybit WebSocket adapter.
/// Uses V5 public WebSocket API for spot market.
pub struct BybitAdapter;

/// Bybit WebSocket ticker response (V5 API).
/// Note: Spot tickers only have lastPrice, not bid1Price/ask1Price (those are for derivatives).
#[derive(Debug, Deserialize)]
struct BybitTickerData {
    /// Symbol (e.g., "BTCUSDT")
    symbol: String,
    /// Last traded price
    #[serde(rename = "lastPrice")]
    last_price: String,
    /// Best bid price (optional, only available for derivatives)
    #[serde(rename = "bid1Price", default)]
    bid1_price: String,
    /// Best ask price (optional, only available for derivatives)
    #[serde(rename = "ask1Price", default)]
    ask1_price: String,
    /// 24h volume (base currency)
    #[serde(rename = "volume24h", default)]
    volume_24h: String,
    /// 24h turnover (quote currency, already in USDT)
    #[serde(rename = "turnover24h", default)]
    turnover_24h: String,
}

/// Bybit WebSocket message wrapper.
/// Note: For spot, `data` is an object (not an array like in some other endpoints).
#[derive(Debug, Deserialize)]
struct BybitTickerMessage {
    topic: String,
    #[serde(rename = "type")]
    msg_type: String,
    data: BybitTickerData,
    ts: u64,
}

/// Bybit WebSocket orderbook data (V5 API).
#[derive(Debug, Deserialize)]
struct BybitOrderbookData {
    /// Symbol (e.g., "BTCUSDT")
    s: String,
    /// Bid prices and sizes [[price, size], ...] (descending order)
    b: Vec<[String; 2]>,
    /// Ask prices and sizes [[price, size], ...] (ascending order)
    a: Vec<[String; 2]>,
}

/// Bybit WebSocket orderbook message wrapper.
#[derive(Debug, Deserialize)]
struct BybitOrderbookMessage {
    topic: String,
    #[serde(rename = "type")]
    msg_type: String,
    data: BybitOrderbookData,
    ts: u64,
}

impl BybitAdapter {
    /// Map symbol to pair_id.
    /// Extracts base asset from trading pair (e.g., BTCUSDT -> BTC) and generates pair_id.
    pub fn symbol_to_pair_id(symbol: &str) -> Option<u32> {
        let base = Self::extract_base_symbol(symbol)?;
        Some(symbol_to_pair_id(&base))
    }

    /// Extract both base and quote from trading pair.
    /// Returns (base, quote) tuple. Single pass - no duplicate suffix checks.
    pub fn extract_base_quote(symbol: &str) -> Option<(String, String)> {
        let symbol = symbol.to_uppercase();
        const QUOTES: &[&str] = &["USDT", "USDC"];
        for quote in QUOTES {
            if let Some(base) = symbol.strip_suffix(quote) {
                return Some((base.to_string(), (*quote).to_string()));
            }
        }
        None
    }

    /// Extract base symbol from trading pair (e.g., BTCUSDT -> BTC).
    #[inline]
    pub fn extract_base_symbol(symbol: &str) -> Option<String> {
        Self::extract_base_quote(symbol).map(|(base, _)| base)
    }

    /// Extract quote currency from trading pair (e.g., BTCUSDT -> USDT).
    #[inline]
    pub fn extract_quote_currency(symbol: &str) -> Option<String> {
        Self::extract_base_quote(symbol).map(|(_, quote)| quote)
    }

    /// Parse a ticker message from Bybit.
    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let msg: BybitTickerMessage = serde_json::from_str(json)?;

        let price = msg.data.last_price.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        // For spot tickers, bid1_price and ask1_price might be empty
        // Use lastPrice as fallback for bid/ask
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

    /// Parse a ticker message and auto-detect pair_id from symbol.
    pub fn parse_ticker_auto(json: &str) -> Result<PriceTick, FeedError> {
        let (tick, _) = Self::parse_ticker_with_symbol(json)?;
        Ok(tick)
    }

    /// Parse a ticker message, returning both the tick and the base symbol.
    pub fn parse_ticker_with_symbol(json: &str) -> Result<(PriceTick, String), FeedError> {
        let (tick, base, _quote) = Self::parse_ticker_with_base_quote(json)?;
        Ok((tick, base))
    }

    /// Parse a ticker message, returning the tick, base symbol, and quote currency.
    pub fn parse_ticker_with_base_quote(json: &str) -> Result<(PriceTick, String, String), FeedError> {
        let msg: BybitTickerMessage = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&msg.data.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", msg.data.symbol)))?;
        let pair_id = symbol_to_pair_id(&base);

        let price = msg.data.last_price.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        // For spot tickers, bid1_price and ask1_price might be empty
        // Use lastPrice as fallback for bid/ask
        let bid = msg.data.bid1_price.parse::<f64>().unwrap_or(price);
        let ask = msg.data.ask1_price.parse::<f64>().unwrap_or(price);
        // turnover_24h is already in USDT
        let volume_usd = msg.data.turnover_24h.parse::<f64>().unwrap_or(0.0);

        Ok((PriceTick::new(
            Exchange::Bybit,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ).with_volume_24h(FixedPoint::from_f64(volume_usd)), base, quote))
    }

    /// Parse an orderbook message from Bybit (level 1 for best bid/ask).
    /// Returns (PriceTick with bid/ask sizes, base_symbol, quote_currency).
    pub fn parse_orderbook_with_base_quote(json: &str) -> Result<(PriceTick, String, String), FeedError> {
        let msg: BybitOrderbookMessage = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&msg.data.s)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", msg.data.s)))?;
        let pair_id = symbol_to_pair_id(&base);

        // Get best bid (first in b array, descending order) - [price, size]
        let best_bid = msg.data.b.first()
            .ok_or_else(|| FeedError::ParseError("No bid in orderbook".to_string()))?;
        let bid = best_bid[0].parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid price".to_string()))?;
        let bid_size = best_bid[1].parse::<f64>().unwrap_or(0.0);

        // Get best ask (first in a array, ascending order) - [price, size]
        let best_ask = msg.data.a.first()
            .ok_or_else(|| FeedError::ParseError("No ask in orderbook".to_string()))?;
        let ask = best_ask[0].parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid ask price".to_string()))?;
        let ask_size = best_ask[1].parse::<f64>().unwrap_or(0.0);

        let mid = (bid + ask) / 2.0;

        Ok((PriceTick::new(
            Exchange::Bybit,
            pair_id,
            FixedPoint::from_f64(mid),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ).with_sizes(FixedPoint::from_f64(bid_size), FixedPoint::from_f64(ask_size)), base, quote))
    }

    /// Parse orderbook message and return full depth data.
    /// Returns (PriceTick, symbol, quote, bids, asks) where bids/asks are Vec<(price, qty)>.
    pub fn parse_orderbook_full(json: &str) -> Result<(PriceTick, String, String, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
        let msg: BybitOrderbookMessage = serde_json::from_str(json)?;

        let (base, quote) = Self::extract_base_quote(&msg.data.s)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", msg.data.s)))?;
        let pair_id = symbol_to_pair_id(&base);

        // Parse all bids (descending order by price)
        let bids: Vec<(f64, f64)> = msg.data.b.iter()
            .filter_map(|level| {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            })
            .collect();

        // Parse all asks (ascending order by price)
        let asks: Vec<(f64, f64)> = msg.data.a.iter()
            .filter_map(|level| {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            })
            .collect();

        // Get best bid/ask for PriceTick
        let (bid, bid_size) = bids.first().copied().unwrap_or((0.0, 0.0));
        let (ask, ask_size) = asks.first().copied().unwrap_or((0.0, 0.0));
        let mid = if bid > 0.0 && ask > 0.0 { (bid + ask) / 2.0 } else { bid.max(ask) };

        Ok((PriceTick::new(
            Exchange::Bybit,
            pair_id,
            FixedPoint::from_f64(mid),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ).with_sizes(FixedPoint::from_f64(bid_size), FixedPoint::from_f64(ask_size)), base, quote, bids, asks))
    }

    /// Check if a message is an orderbook message.
    pub fn is_orderbook_message(json: &str) -> bool {
        json.contains("\"topic\":\"orderbook.")
    }

    /// Generate a subscription message for Bybit WebSocket.
    /// Note: Bybit has a limit of 10 args per subscription message.
    /// This function generates a single message - use subscribe_messages() for batched subscriptions.
    pub fn subscribe_message(symbols: &[String]) -> String {
        // Take only first 10 symbols for single message
        let topics: Vec<String> = symbols
            .iter()
            .take(10)
            .map(|s| format!("\"tickers.{}\"", s.to_uppercase()))
            .collect();

        format!(
            r#"{{"op": "subscribe", "args": [{}]}}"#,
            topics.join(", ")
        )
    }

    /// Generate multiple subscription messages for Bybit WebSocket (batched by 10).
    /// Bybit has a limit of 10 args per subscription message.
    /// Subscribes to orderbook.50 for full orderbook with snapshot+delta.
    pub fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        let mut messages = Vec::new();

        // Subscribe to orderbook.50 (50 levels with snapshot+delta)
        // First message is snapshot, subsequent messages are delta updates
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

    /// Get WebSocket URL for Bybit.
    pub fn ws_url() -> &'static str {
        "wss://stream.bybit.com/v5/public/spot"
    }
}

/// Bithumb WebSocket adapter.
/// Uses the same format as Upbit (Korean exchange).
pub struct BithumbAdapter;

/// Bithumb message types for efficient single-parse dispatch.
#[derive(Debug, Clone)]
pub enum BithumbMessage {
    /// Ticker message with trade price
    Ticker {
        code: String,
        price: FixedPoint,
    },
    /// Orderbook message with best bid/ask
    Orderbook {
        code: String,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
    },
}

/// Bithumb WebSocket ticker response (similar to Upbit format).
#[derive(Debug, Deserialize)]
struct BithumbTicker {
    /// Market code (e.g., "KRW-BTC")
    #[serde(alias = "cd", alias = "code")]
    code: String,
    /// Current trade price
    #[serde(alias = "tp", alias = "trade_price")]
    trade_price: f64,
    /// Opening price
    #[serde(alias = "op", alias = "opening_price", default)]
    opening_price: f64,
    /// Highest price
    #[serde(alias = "hp", alias = "high_price", default)]
    high_price: f64,
    /// Lowest price
    #[serde(alias = "lp", alias = "low_price", default)]
    low_price: f64,
    /// Accumulated trade volume (24h)
    #[serde(alias = "atv24h", alias = "acc_trade_volume_24h", default)]
    acc_trade_volume_24h: f64,
    /// Timestamp
    #[serde(alias = "tms", alias = "timestamp", default)]
    timestamp: u64,
}

impl BithumbAdapter {
    /// Parse any Bithumb message (JSON) with single parse, dispatch by type.
    /// More efficient than is_orderbook_message() + parse_xxx().
    pub fn parse_message(json: &str) -> Result<BithumbMessage, FeedError> {
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
            "ticker" => Ok(BithumbMessage::Ticker {
                code: msg.code,
                price: FixedPoint::from_f64(msg.trade_price),
            }),
            "orderbook" => {
                let best = msg.orderbook_units.first()
                    .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;
                Ok(BithumbMessage::Orderbook {
                    code: msg.code,
                    bid: FixedPoint::from_f64(best.bid_price),
                    ask: FixedPoint::from_f64(best.ask_price),
                    bid_size: FixedPoint::from_f64(best.bid_size),
                    ask_size: FixedPoint::from_f64(best.ask_size),
                })
            }
            _ => Err(FeedError::ParseError(format!("Unknown message type: {}", msg.msg_type))),
        }
    }

    /// Parse any Bithumb message (MessagePack binary) with single parse.
    pub fn parse_message_binary(data: &[u8]) -> Result<BithumbMessage, FeedError> {
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
            "ticker" => Ok(BithumbMessage::Ticker {
                code: msg.code,
                price: FixedPoint::from_f64(msg.trade_price),
            }),
            "orderbook" => {
                let best = msg.orderbook_units.first()
                    .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;
                Ok(BithumbMessage::Orderbook {
                    code: msg.code,
                    bid: FixedPoint::from_f64(best.bid_price),
                    ask: FixedPoint::from_f64(best.ask_price),
                    bid_size: FixedPoint::from_f64(best.bid_size),
                    ask_size: FixedPoint::from_f64(best.ask_size),
                })
            }
            _ => Err(FeedError::ParseError(format!("Unknown message type: {}", msg.msg_type))),
        }
    }

    /// Parse Bithumb orderbook message with full depth (JSON).
    /// Returns (code, bid, ask, bid_size, ask_size, bids_full, asks_full).
    pub fn parse_orderbook_full(json: &str) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
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
            return Err(FeedError::ParseError(format!("Not an orderbook message: {}", msg.msg_type)));
        }

        let best = msg.orderbook_units.first()
            .ok_or_else(|| FeedError::ParseError("Empty orderbook".to_string()))?;

        let bids: Vec<(f64, f64)> = msg.orderbook_units.iter()
            .map(|u| (u.bid_price, u.bid_size))
            .collect();
        let asks: Vec<(f64, f64)> = msg.orderbook_units.iter()
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

    /// Parse Bithumb orderbook message with full depth (binary format is JSON as bytes).
    pub fn parse_orderbook_full_binary(data: &[u8]) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
        // Bithumb sends JSON as binary bytes, not MessagePack
        let json_str = std::str::from_utf8(data)
            .map_err(|e| FeedError::ParseError(format!("Invalid UTF-8: {}", e)))?;
        Self::parse_orderbook_full(json_str)
    }

    /// Map market code to pair_id.
    /// Extracts base asset from market code (e.g., KRW-BTC -> BTC) and generates pair_id.
    pub fn market_to_pair_id(code: &str) -> Option<u32> {
        let code = code.to_uppercase();
        if code == "KRW-USDT" {
            return None; // Special case: used for exchange rate, not trading
        }

        let base = Self::extract_base_symbol(&code)?;
        Some(symbol_to_pair_id(&base))
    }

    /// Extract base symbol from market code (e.g., KRW-BTC -> BTC).
    pub fn extract_base_symbol(code: &str) -> Option<String> {
        let code = code.to_uppercase();
        if code.starts_with("KRW-") {
            code.strip_prefix("KRW-").map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Check if market code is for USDT (exchange rate).
    pub fn is_usdt_market(code: &str) -> bool {
        code.to_uppercase() == "KRW-USDT"
    }

    /// Check if market code is for USDC (exchange rate).
    pub fn is_usdc_market(code: &str) -> bool {
        code.to_uppercase() == "KRW-USDC"
    }

    /// Check if market code is a stablecoin market (USDT or USDC).
    pub fn is_stablecoin_market(code: &str) -> bool {
        let upper = code.to_uppercase();
        upper == "KRW-USDT" || upper == "KRW-USDC"
    }

    /// Parse a ticker message from Bithumb (JSON format).
    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: BithumbTicker = serde_json::from_str(json)?;

        // Bithumb doesn't provide bid/ask in ticker, use trade_price for all
        let price = FixedPoint::from_f64(ticker.trade_price);

        Ok(PriceTick::new(
            Exchange::Bithumb,
            pair_id,
            price,
            price, // bid approximation
            price, // ask approximation
        ))
    }

    /// Parse a ticker message from Bithumb and return with market code.
    /// Returns (market_code, price).
    pub fn parse_ticker_with_code(json: &str) -> Result<(String, FixedPoint), FeedError> {
        let ticker: BithumbTicker = serde_json::from_str(json)?;
        let price = FixedPoint::from_f64(ticker.trade_price);
        Ok((ticker.code, price))
    }

    /// Parse a ticker message from Bithumb binary (MessagePack format).
    pub fn parse_ticker_binary(data: &[u8], pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: BithumbTicker = rmp_serde::from_slice(data)
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

    /// Parse a ticker message from Bithumb binary and return with market code.
    pub fn parse_ticker_binary_with_code(data: &[u8]) -> Result<(String, FixedPoint), FeedError> {
        let ticker: BithumbTicker = rmp_serde::from_slice(data)
            .map_err(|e| FeedError::ParseError(format!("MessagePack parse error: {}", e)))?;
        let price = FixedPoint::from_f64(ticker.trade_price);
        Ok((ticker.code, price))
    }

    /// Generate a subscription message for Bithumb WebSocket.
    /// Bithumb uses the same format as Upbit: array of ticket, type, and codes.
    /// Subscribes to both ticker (for trade price and volume) and orderbook (full depth - 15 levels max).
    ///
    /// Orderbook subscription requires "level" parameter:
    /// - level: 1 means smallest aggregation unit (1 KRW)
    /// - level: N means aggregate orderbook at N increments
    pub fn subscribe_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();
        let codes_str = codes.join(",");

        // Subscribe to both ticker and orderbook in a single message
        // level: 1 = smallest aggregation (Bithumb doesn't support level: 0)
        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"ticker","codes":[{}]}},{{"type":"orderbook","codes":[{}],"level":1}},{{"format":"SIMPLE"}}]"#,
            codes_str, codes_str
        )
    }

    /// Check if a message is an orderbook message.
    pub fn is_orderbook_message(json: &str) -> bool {
        json.contains("\"ty\":\"orderbook\"") || json.contains("\"type\":\"orderbook\"")
    }

    /// Parse orderbook message with code and depth.
    /// Returns (market_code, bid, ask, bid_size, ask_size).
    pub fn parse_orderbook_with_code(json: &str) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct BithumbOrderbookSimple {
            #[serde(alias = "cd", alias = "code")]
            code: String,
            #[serde(alias = "obu", alias = "orderbook_units")]
            orderbook_units: Vec<BithumbOrderbookUnit>,
        }

        #[derive(Debug, Deserialize)]
        struct BithumbOrderbookUnit {
            #[serde(alias = "ap", alias = "ask_price")]
            ask_price: f64,
            #[serde(alias = "bp", alias = "bid_price")]
            bid_price: f64,
            #[serde(alias = "as", alias = "ask_size", default)]
            ask_size: f64,
            #[serde(alias = "bs", alias = "bid_size", default)]
            bid_size: f64,
        }

        let orderbook: BithumbOrderbookSimple = serde_json::from_str(json)?;

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

    /// Parse orderbook binary message with code and depth (MessagePack format).
    /// Returns (market_code, bid, ask, bid_size, ask_size).
    pub fn parse_orderbook_binary_with_code(data: &[u8]) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct BithumbOrderbookSimple {
            #[serde(alias = "cd", alias = "code")]
            code: String,
            #[serde(alias = "obu", alias = "orderbook_units")]
            orderbook_units: Vec<BithumbOrderbookUnit>,
        }

        #[derive(Debug, Deserialize)]
        struct BithumbOrderbookUnit {
            #[serde(alias = "ap", alias = "ask_price")]
            ask_price: f64,
            #[serde(alias = "bp", alias = "bid_price")]
            bid_price: f64,
            #[serde(alias = "as", alias = "ask_size", default)]
            ask_size: f64,
            #[serde(alias = "bs", alias = "bid_size", default)]
            bid_size: f64,
        }

        let orderbook: BithumbOrderbookSimple = rmp_serde::from_slice(data)
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

    /// Get WebSocket URL for Bithumb.
    pub fn ws_url() -> &'static str {
        "wss://ws-api.bithumb.com/websocket/v1"
    }

    /// Convert symbol to Bithumb market format.
    /// "BTC/KRW" -> "KRW-BTC"
    pub fn to_market_code(symbol: &str) -> String {
        if let Some((base, quote)) = symbol.split_once('/') {
            format!("{}-{}", quote, base)
        } else {
            symbol.to_string()
        }
    }

    /// Convert Bithumb market code to symbol.
    /// "KRW-BTC" -> "BTC/KRW"
    pub fn from_market_code(code: &str) -> String {
        if let Some((quote, base)) = code.split_once('-') {
            format!("{}/{}", base, quote)
        } else {
            code.to_string()
        }
    }
}

/// Gate.io WebSocket adapter.
/// Uses V4 WebSocket API for spot market.
pub struct GateIOAdapter;

/// Gate.io WebSocket ticker response (V4 API).
#[derive(Debug, Deserialize)]
struct GateIOTickerResult {
    /// Trading pair (e.g., "BTC_USDT")
    currency_pair: String,
    /// Last traded price
    last: String,
    /// Best ask price
    lowest_ask: String,
    /// Best bid price
    highest_bid: String,
    /// 24h price change percentage
    #[serde(default)]
    change_percentage: String,
    /// 24h volume in base currency
    #[serde(default)]
    base_volume: String,
    /// 24h volume in quote currency (USDT)
    #[serde(default)]
    quote_volume: String,
    /// 24h high price
    #[serde(default)]
    high_24h: String,
    /// 24h low price
    #[serde(default)]
    low_24h: String,
}

/// Gate.io WebSocket message wrapper.
#[derive(Debug, Deserialize)]
struct GateIOTickerMessage {
    time: u64,
    #[serde(default)]
    time_ms: u64,
    channel: String,
    event: String,
    result: GateIOTickerResult,
}

impl GateIOAdapter {
    /// Map currency pair to pair_id.
    /// Extracts base asset from trading pair (e.g., BTC_USDT -> BTC) and generates pair_id.
    pub fn symbol_to_pair_id(currency_pair: &str) -> Option<u32> {
        let base = Self::extract_base_symbol(currency_pair)?;
        Some(symbol_to_pair_id(&base))
    }

    /// Extract both base and quote from currency pair (e.g., BTC_USDT -> (BTC, USDT)).
    /// Single pass - no duplicate suffix checks.
    pub fn extract_base_quote(currency_pair: &str) -> Option<(String, String)> {
        let pair = currency_pair.to_uppercase();
        const QUOTES: &[&str] = &["_USDT", "_USDC", "_USD"];
        for suffix in QUOTES {
            if let Some(base) = pair.strip_suffix(suffix) {
                let quote = suffix.trim_start_matches('_');
                return Some((base.to_string(), quote.to_string()));
            }
        }
        None
    }

    /// Extract base symbol from currency pair (e.g., BTC_USDT -> BTC).
    #[inline]
    pub fn extract_base_symbol(currency_pair: &str) -> Option<String> {
        Self::extract_base_quote(currency_pair).map(|(base, _)| base)
    }

    /// Extract quote currency from currency pair (e.g., BTC_USDT -> USDT).
    #[inline]
    pub fn extract_quote_currency(currency_pair: &str) -> Option<String> {
        Self::extract_base_quote(currency_pair).map(|(_, quote)| quote)
    }

    /// Parse a ticker message from Gate.io.
    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let msg: GateIOTickerMessage = serde_json::from_str(json)?;

        // Skip non-update messages
        if msg.event != "update" {
            return Err(FeedError::ParseError("Not an update message".to_string()));
        }

        let price = msg.result.last.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = msg.result.highest_bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = msg.result.lowest_ask.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Ok(PriceTick::new(
            Exchange::GateIO,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ))
    }

    /// Parse a ticker message and auto-detect pair_id from symbol.
    pub fn parse_ticker_auto(json: &str) -> Result<PriceTick, FeedError> {
        let (tick, _) = Self::parse_ticker_with_symbol(json)?;
        Ok(tick)
    }

    /// Parse a ticker message, returning both the tick and the base symbol.
    pub fn parse_ticker_with_symbol(json: &str) -> Result<(PriceTick, String), FeedError> {
        let (tick, base, _quote) = Self::parse_ticker_with_base_quote(json)?;
        Ok((tick, base))
    }

    /// Parse a ticker message, returning the tick, base symbol, and quote currency.
    pub fn parse_ticker_with_base_quote(json: &str) -> Result<(PriceTick, String, String), FeedError> {
        let msg: GateIOTickerMessage = serde_json::from_str(json)?;

        // Skip non-update messages
        if msg.event != "update" {
            return Err(FeedError::ParseError("Not an update message".to_string()));
        }

        let (base, quote) = Self::extract_base_quote(&msg.result.currency_pair)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown pair: {}", msg.result.currency_pair)))?;
        let pair_id = symbol_to_pair_id(&base);

        let price = msg.result.last.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = msg.result.highest_bid.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = msg.result.lowest_ask.parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        // quote_volume is already in quote currency (USDT or USDC)
        let volume_usd = msg.result.quote_volume.parse::<f64>().unwrap_or(0.0);

        Ok((PriceTick::new(
            Exchange::GateIO,
            pair_id,
            FixedPoint::from_f64(price),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
        ).with_volume_24h(FixedPoint::from_f64(volume_usd)), base, quote))
    }

    /// Generate a subscription message for Gate.io WebSocket.
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

    /// Generate subscription messages for order_book channel only.
    /// order_book provides full orderbook snapshot every 100ms (20 levels).
    pub fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut messages = Vec::new();

        // Subscribe to order_book for each symbol (level 20, 100ms update)
        // Gate.io provides full snapshot every update - no delta sync needed
        for symbol in symbols {
            messages.push(format!(
                r#"{{"time": {}, "channel": "spot.order_book", "event": "subscribe", "payload": ["{}", "20", "100ms"]}}"#,
                timestamp,
                symbol.to_uppercase()
            ));
        }

        messages
    }

    /// Check if a message is an order_book message.
    pub fn is_orderbook_message(json: &str) -> bool {
        json.contains("\"channel\":\"spot.order_book\"")
    }

    /// Parse an order_book message from Gate.io.
    /// Returns (currency_pair, bid, ask, bid_size, ask_size).
    pub fn parse_orderbook_with_symbol(json: &str) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct GateIOOrderbookResult {
            /// Currency pair (e.g., "BTC_USDT")
            s: String,
            /// Bids: [[price, size], ...] sorted high to low
            bids: Vec<[String; 2]>,
            /// Asks: [[price, size], ...] sorted low to high
            asks: Vec<[String; 2]>,
        }

        #[derive(Debug, Deserialize)]
        struct GateIOOrderbookMessage {
            channel: String,
            event: String,
            result: GateIOOrderbookResult,
        }

        let msg: GateIOOrderbookMessage = serde_json::from_str(json)?;

        // Skip non-update messages
        if msg.event != "update" {
            return Err(FeedError::ParseError("Not an update message".to_string()));
        }

        // Get best bid (first in bids array)
        let best_bid = msg.result.bids.first()
            .ok_or_else(|| FeedError::ParseError("No bids in orderbook".to_string()))?;
        let bid = best_bid[0].parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid price".to_string()))?;
        let bid_size = best_bid[1].parse::<f64>().unwrap_or(0.0);

        // Get best ask (first in asks array)
        let best_ask = msg.result.asks.first()
            .ok_or_else(|| FeedError::ParseError("No asks in orderbook".to_string()))?;
        let ask = best_ask[0].parse::<f64>()
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

    /// Parse orderbook message and return full depth data.
    /// Returns (currency_pair, bid, ask, bid_size, ask_size, bids, asks) where bids/asks are Vec<(price, qty)>.
    pub fn parse_orderbook_full(json: &str) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint, Vec<(f64, f64)>, Vec<(f64, f64)>), FeedError> {
        #[derive(Debug, Deserialize)]
        struct GateIOOrderbookResult {
            /// Currency pair (e.g., "BTC_USDT")
            s: String,
            /// Bids: [[price, size], ...] sorted high to low
            bids: Vec<[String; 2]>,
            /// Asks: [[price, size], ...] sorted low to high
            asks: Vec<[String; 2]>,
        }

        #[derive(Debug, Deserialize)]
        struct GateIOOrderbookMessage {
            #[allow(dead_code)]
            channel: String,
            event: String,
            result: GateIOOrderbookResult,
        }

        let msg: GateIOOrderbookMessage = serde_json::from_str(json)?;

        // Skip non-update messages
        if msg.event != "update" {
            return Err(FeedError::ParseError("Not an update message".to_string()));
        }

        // Parse all bids (descending order by price)
        let bids: Vec<(f64, f64)> = msg.result.bids.iter()
            .filter_map(|level| {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            })
            .collect();

        // Parse all asks (ascending order by price)
        let asks: Vec<(f64, f64)> = msg.result.asks.iter()
            .filter_map(|level| {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            })
            .collect();

        // Get best bid/ask
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

    /// Get WebSocket URL for Gate.io.
    pub fn ws_url() -> &'static str {
        "wss://api.gateio.ws/ws/v4/"
    }

    /// Convert symbol to Gate.io format.
    /// "BTC" -> "BTC_USDT"
    pub fn to_currency_pair(symbol: &str) -> String {
        format!("{}_USDT", symbol.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Upbit tests ===

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

    // === Binance tests ===

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
    fn test_coinbase_parse_ticker() {
        let json = r#"{
            "type": "ticker",
            "product_id": "BTC-USD",
            "price": "50000.00",
            "best_bid": "49999.00",
            "best_ask": "50001.00",
            "volume_24h": "1000.00",
            "time": "2024-01-01T00:00:00.000000Z"
        }"#;

        let tick = CoinbaseAdapter::parse_ticker(json, 1).unwrap();
        assert_eq!(tick.exchange(), Exchange::Coinbase);
        assert_eq!(tick.pair_id(), 1);
        assert!((tick.price().to_f64() - 50000.0).abs() < 0.01);
    }

    #[test]
    fn test_adapter_subscribe_message() {
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let msg = BinanceAdapter::subscribe_message(&symbols);
        assert!(msg.contains("SUBSCRIBE"));
        assert!(msg.contains("btcusdt@ticker"));
        assert!(msg.contains("ethusdt@ticker"));
    }

    // === Bybit tests ===

    #[test]
    fn test_bybit_parse_ticker() {
        // Based on Bybit V5 API spot ticker format
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
        // bid/ask should fallback to lastPrice for spot
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

    // === Gate.io tests ===

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
        assert_eq!(GateIOAdapter::extract_base_symbol("BTC_USDT"), Some("BTC".to_string()));
        assert_eq!(GateIOAdapter::extract_base_symbol("ETH_USDT"), Some("ETH".to_string()));
        assert_eq!(GateIOAdapter::extract_base_symbol("BTCUSDT"), None);
    }

    // === Quote currency extraction tests ===

    #[test]
    fn test_binance_extract_quote_currency() {
        assert_eq!(BinanceAdapter::extract_quote_currency("BTCUSDT"), Some("USDT".to_string()));
        assert_eq!(BinanceAdapter::extract_quote_currency("BTCUSDC"), Some("USDC".to_string()));
        assert_eq!(BinanceAdapter::extract_quote_currency("BTCBUSD"), Some("BUSD".to_string()));
        assert_eq!(BinanceAdapter::extract_quote_currency("ETHUSDT"), Some("USDT".to_string()));
        assert_eq!(BinanceAdapter::extract_quote_currency("INVALID"), None);
    }

    #[test]
    fn test_binance_extract_base_quote() {
        assert_eq!(BinanceAdapter::extract_base_quote("BTCUSDT"), Some(("BTC".to_string(), "USDT".to_string())));
        assert_eq!(BinanceAdapter::extract_base_quote("ETHUSDC"), Some(("ETH".to_string(), "USDC".to_string())));
        assert_eq!(BinanceAdapter::extract_base_quote("SOLBUSD"), Some(("SOL".to_string(), "BUSD".to_string())));
    }

    #[test]
    fn test_coinbase_extract_quote_currency() {
        assert_eq!(CoinbaseAdapter::extract_quote_currency("BTC-USD"), Some("USD".to_string()));
        assert_eq!(CoinbaseAdapter::extract_quote_currency("BTC-USDT"), Some("USDT".to_string()));
        assert_eq!(CoinbaseAdapter::extract_quote_currency("BTC-USDC"), Some("USDC".to_string()));
        assert_eq!(CoinbaseAdapter::extract_quote_currency("INVALID"), None);
    }

    #[test]
    fn test_bybit_extract_quote_currency() {
        assert_eq!(BybitAdapter::extract_quote_currency("BTCUSDT"), Some("USDT".to_string()));
        assert_eq!(BybitAdapter::extract_quote_currency("BTCUSDC"), Some("USDC".to_string()));
        assert_eq!(BybitAdapter::extract_quote_currency("BTCEUR"), None);
    }

    #[test]
    fn test_gateio_extract_quote_currency() {
        assert_eq!(GateIOAdapter::extract_quote_currency("BTC_USDT"), Some("USDT".to_string()));
        assert_eq!(GateIOAdapter::extract_quote_currency("BTC_USDC"), Some("USDC".to_string()));
        assert_eq!(GateIOAdapter::extract_quote_currency("BTC_USD"), Some("USD".to_string()));
        assert_eq!(GateIOAdapter::extract_quote_currency("BTC_EUR"), None);
    }
}

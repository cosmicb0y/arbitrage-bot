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
        // Extract base asset from USDT/BUSD pairs
        let base = if symbol.ends_with("USDT") {
            symbol.strip_suffix("USDT")
        } else if symbol.ends_with("BUSD") {
            symbol.strip_suffix("BUSD")
        } else {
            None
        }?;

        Some(symbol_to_pair_id(base))
    }

    /// Extract base symbol from trading pair (e.g., BTCUSDT -> BTC).
    pub fn extract_base_symbol(symbol: &str) -> Option<String> {
        let symbol = symbol.to_uppercase();
        if symbol.ends_with("USDT") {
            symbol.strip_suffix("USDT").map(|s| s.to_string())
        } else if symbol.ends_with("BUSD") {
            symbol.strip_suffix("BUSD").map(|s| s.to_string())
        } else {
            None
        }
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
        let ticker: BinanceTicker = serde_json::from_str(json)?;

        let symbol = Self::extract_base_symbol(&ticker.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", ticker.symbol)))?;
        let pair_id = symbol_to_pair_id(&symbol);

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
        ).with_volume_24h(FixedPoint::from_f64(volume_usd)), symbol))
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

    /// Get WebSocket URL for Binance.
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

impl UpbitAdapter {
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
    pub fn subscribe_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"ticker","codes":[{}]}},{{"format":"SIMPLE"}}]"#,
            codes.join(",")
        )
    }

    /// Generate a subscription message for orderbook.
    pub fn subscribe_orderbook_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"orderbook","codes":[{}]}},{{"format":"SIMPLE"}}]"#,
            codes.join(",")
        )
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
        } else {
            None
        }
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
        let ticker: CoinbaseTicker = serde_json::from_str(json)?;

        // Skip non-ticker messages
        if ticker.msg_type != "ticker" {
            return Err(FeedError::ParseError("Not a ticker message".to_string()));
        }

        let symbol = Self::extract_base_symbol(&ticker.product_id)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown product: {}", ticker.product_id)))?;
        let pair_id = symbol_to_pair_id(&symbol);

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
        ).with_volume_24h(FixedPoint::from_f64(volume_usd)), symbol))
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

    /// Get WebSocket URL for Coinbase.
    pub fn ws_url() -> &'static str {
        "wss://ws-feed.exchange.coinbase.com"
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

impl BybitAdapter {
    /// Map symbol to pair_id.
    /// Extracts base asset from trading pair (e.g., BTCUSDT -> BTC) and generates pair_id.
    pub fn symbol_to_pair_id(symbol: &str) -> Option<u32> {
        let base = Self::extract_base_symbol(symbol)?;
        Some(symbol_to_pair_id(&base))
    }

    /// Extract base symbol from trading pair (e.g., BTCUSDT -> BTC).
    pub fn extract_base_symbol(symbol: &str) -> Option<String> {
        let symbol = symbol.to_uppercase();
        if symbol.ends_with("USDT") {
            symbol.strip_suffix("USDT").map(|s| s.to_string())
        } else if symbol.ends_with("USDC") {
            symbol.strip_suffix("USDC").map(|s| s.to_string())
        } else {
            None
        }
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
        let msg: BybitTickerMessage = serde_json::from_str(json)?;

        let symbol = Self::extract_base_symbol(&msg.data.symbol)
            .ok_or_else(|| FeedError::ParseError(format!("Unknown symbol: {}", msg.data.symbol)))?;
        let pair_id = symbol_to_pair_id(&symbol);

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
        ).with_volume_24h(FixedPoint::from_f64(volume_usd)), symbol))
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
    pub fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        symbols
            .chunks(10)
            .map(|chunk| {
                let topics: Vec<String> = chunk
                    .iter()
                    .map(|s| format!("\"tickers.{}\"", s.to_uppercase()))
                    .collect();
                format!(
                    r#"{{"op": "subscribe", "args": [{}]}}"#,
                    topics.join(", ")
                )
            })
            .collect()
    }

    /// Get WebSocket URL for Bybit.
    pub fn ws_url() -> &'static str {
        "wss://stream.bybit.com/v5/public/spot"
    }
}

/// Bithumb WebSocket adapter.
/// Uses the same format as Upbit (Korean exchange).
pub struct BithumbAdapter;

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
    pub fn subscribe_message(markets: &[String]) -> String {
        let codes: Vec<String> = markets.iter().map(|m| format!("\"{}\"", m)).collect();

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"ticker","codes":[{}]}},{{"format":"SIMPLE"}}]"#,
            codes.join(",")
        )
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
}

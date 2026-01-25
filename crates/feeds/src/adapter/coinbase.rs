use arbitrage_core::{symbol_to_pair_id, Exchange, FixedPoint, PriceTick};
use serde::Deserialize;

use super::ExchangeAdapter;
use crate::FeedError;

/// Maximum L2 streams per WebSocket connection for Coinbase.
/// Coinbase returns "too many L2 streams requested in a single session" when exceeded.
/// Using 30 streams per connection to stay within limits.
pub const COINBASE_MAX_L2_STREAMS_PER_CONNECTION: usize = 30;

pub struct CoinbaseAdapter;

#[derive(Debug, Clone)]
pub enum CoinbaseL2Event {
    Snapshot {
        product_id: String,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
    },
    Update {
        product_id: String,
        changes: Vec<(String, f64, f64)>,
    },
}

#[derive(Debug, Clone)]
pub struct CoinbaseCredentials {
    pub key_name: String,
    pub secret_key: String,
}

impl CoinbaseCredentials {
    pub fn new(key_name: String, secret_key: String) -> Self {
        Self {
            key_name,
            secret_key,
        }
    }

    pub fn from_env() -> Option<Self> {
        let key_name = std::env::var("COINBASE_API_KEY_ID").ok()?;
        let secret_key = std::env::var("COINBASE_SECRET_KEY").ok()?;

        if key_name.is_empty() || secret_key.is_empty() {
            return None;
        }

        Some(Self {
            key_name,
            secret_key,
        })
    }

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

impl ExchangeAdapter for CoinbaseAdapter {
    fn exchange() -> Exchange {
        Exchange::Coinbase
    }

    fn ws_url() -> &'static str {
        "wss://ws-feed.exchange.coinbase.com"
    }

    fn extract_base_quote(symbol: &str) -> Option<(String, String)> {
        let product_id = symbol.to_uppercase();
        if product_id.ends_with("-USDT") {
            product_id
                .strip_suffix("-USDT")
                .map(|base| (base.to_string(), "USDT".to_string()))
        } else if product_id.ends_with("-USDC") {
            product_id
                .strip_suffix("-USDC")
                .map(|base| (base.to_string(), "USDC".to_string()))
        } else if product_id.ends_with("-USD") {
            product_id
                .strip_suffix("-USD")
                .map(|base| (base.to_string(), "USD".to_string()))
        } else {
            None
        }
    }

    fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        let products: Vec<String> = symbols.iter().map(|s| format!("\"{}\"", s)).collect();
        let products_str = products.join(", ");

        vec![format!(
            r#"{{"type": "subscribe", "product_ids": [{}], "channels": ["level2", "heartbeats"]}}"#,
            products_str
        )]
    }
}

impl CoinbaseAdapter {
    pub fn product_to_pair_id(product_id: &str) -> Option<u32> {
        let base = Self::extract_base_symbol(product_id)?;
        Some(symbol_to_pair_id(&base))
    }

    pub fn ws_url_advanced_trade() -> &'static str {
        "wss://advanced-trade-ws.coinbase.com"
    }

    pub fn parse_ticker(json: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        let ticker: CoinbaseTicker = serde_json::from_str(json)?;

        let price = ticker
            .price
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = ticker
            .best_bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker
            .best_ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Ok(PriceTick::new(
            Exchange::Coinbase,
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
        let ticker: CoinbaseTicker = serde_json::from_str(json)?;

        if ticker.msg_type != "ticker" {
            return Err(FeedError::ParseError("Not a ticker message".to_string()));
        }

        let (base, quote) = Self::extract_base_quote(&ticker.product_id).ok_or_else(|| {
            FeedError::ParseError(format!("Unknown product: {}", ticker.product_id))
        })?;
        let pair_id = symbol_to_pair_id(&base);

        let price = ticker
            .price
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let bid = ticker
            .best_bid
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let ask = ticker
            .best_ask
            .parse::<f64>()
            .map_err(|e| FeedError::ParseError(e.to_string()))?;
        let volume = ticker.volume_24h.parse::<f64>().unwrap_or(0.0);
        let volume_usd = volume * price;

        Ok((
            PriceTick::new(
                Exchange::Coinbase,
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

    pub fn subscribe_message(product_ids: &[String]) -> String {
        let products: Vec<String> = product_ids.iter().map(|s| format!("\"{}\"", s)).collect();

        format!(
            r#"{{"type": "subscribe", "product_ids": [{}], "channels": ["ticker"]}}"#,
            products.join(", ")
        )
    }

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
            return Err(FeedError::ParseError(
                "Not an l2_data channel message".to_string(),
            ));
        }

        let event = msg
            .events
            .first()
            .ok_or_else(|| FeedError::ParseError("No events in l2_data message".to_string()))?;

        match event.event_type.as_str() {
            "snapshot" => {
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

                bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                if bids.is_empty() || asks.is_empty() {
                    return Err(FeedError::ParseError(
                        "No valid bid/ask in snapshot".to_string(),
                    ));
                }

                Ok(CoinbaseL2Event::Snapshot {
                    product_id: event.product_id.clone(),
                    bids,
                    asks,
                })
            }
            "update" => {
                let changes: Vec<(String, f64, f64)> = event
                    .updates
                    .iter()
                    .filter_map(|update| {
                        let price = update.price_level.parse::<f64>().ok()?;
                        let size = update.new_quantity.parse::<f64>().ok()?;
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
            _ => Err(FeedError::ParseError(format!(
                "Unknown event type: {}",
                event.event_type
            ))),
        }
    }

    pub fn is_level2_message(json: &str) -> bool {
        json.contains("\"type\":\"l2update\"")
            || json.contains("\"type\":\"snapshot\"")
            || json.contains("\"type\":\"update\"")
            || json.contains("\"channel\":\"l2_data\"")
    }

    pub fn parse_level2_snapshot(
        json: &str,
    ) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        if let Ok(result) = Self::parse_advanced_trade_l2(json) {
            return Ok(result);
        }

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

        let best_bid = snapshot
            .bids
            .first()
            .ok_or_else(|| FeedError::ParseError("No bids in snapshot".to_string()))?;
        let bid = best_bid[0]
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid price".to_string()))?;
        let bid_size = best_bid[1].parse::<f64>().unwrap_or(0.0);

        let best_ask = snapshot
            .asks
            .first()
            .ok_or_else(|| FeedError::ParseError("No asks in snapshot".to_string()))?;
        let ask = best_ask[0]
            .parse::<f64>()
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

    fn parse_advanced_trade_l2(
        json: &str,
    ) -> Result<(String, FixedPoint, FixedPoint, FixedPoint, FixedPoint), FeedError> {
        #[derive(Debug, Deserialize)]
        struct L2DataMessage {
            channel: String,
            events: Vec<L2DataEvent>,
        }

        #[derive(Debug, Deserialize)]
        struct L2DataEvent {
            #[serde(rename = "type")]
            _event_type: String,
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
            return Err(FeedError::ParseError(
                "Not an l2_data channel message".to_string(),
            ));
        }

        let event = msg
            .events
            .first()
            .ok_or_else(|| FeedError::ParseError("No events in l2_data message".to_string()))?;

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

        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let (bid, bid_size) = bids.first().copied().unwrap_or((0.0, 0.0));
        let (ask, ask_size) = asks.first().copied().unwrap_or((0.0, 0.0));

        if bid == 0.0 || ask == 0.0 {
            return Err(FeedError::ParseError(
                "No valid bid/ask in l2_data".to_string(),
            ));
        }

        Ok((
            event.product_id.clone(),
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
            FixedPoint::from_f64(bid_size),
            FixedPoint::from_f64(ask_size),
        ))
    }

    pub fn parse_level2_update(json: &str) -> Result<(String, Vec<(String, f64, f64)>), FeedError> {
        if let Ok(result) = Self::parse_advanced_trade_l2_update(json) {
            return Ok(result);
        }

        #[derive(Debug, Deserialize)]
        struct Level2BatchUpdate {
            #[serde(rename = "type")]
            msg_type: String,
            product_id: String,
            #[serde(default)]
            changes: Vec<[String; 3]>,
        }

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

        if let Ok(update) = serde_json::from_str::<Level2BatchUpdate>(json) {
            if update.msg_type == "l2update" && !update.changes.is_empty() {
                let changes: Vec<(String, f64, f64)> = update
                    .changes
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

        if let Ok(update) = serde_json::from_str::<Level2Update>(json) {
            if update.msg_type == "update" && !update.updates.is_empty() {
                let changes: Vec<(String, f64, f64)> = update
                    .updates
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

        Err(FeedError::ParseError(
            "Not a valid level2 update message".to_string(),
        ))
    }

    fn parse_advanced_trade_l2_update(
        json: &str,
    ) -> Result<(String, Vec<(String, f64, f64)>), FeedError> {
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
            return Err(FeedError::ParseError(
                "Not an l2_data channel message".to_string(),
            ));
        }

        let event = msg
            .events
            .first()
            .ok_or_else(|| FeedError::ParseError("No events in l2_data message".to_string()))?;

        if event.event_type != "update" {
            return Err(FeedError::ParseError("Not an update event".to_string()));
        }

        let changes: Vec<(String, f64, f64)> = event
            .updates
            .iter()
            .filter_map(|update| {
                let price = update.price_level.parse::<f64>().ok()?;
                let size = update.new_quantity.parse::<f64>().ok()?;
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

    pub fn generate_ws_jwt(credentials: &CoinbaseCredentials) -> Result<String, crate::FeedError> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use p256::ecdsa::{signature::Signer, Signature, SigningKey};
        use p256::pkcs8::DecodePrivateKey;
        use p256::SecretKey;
        use std::time::{SystemTime, UNIX_EPOCH};

        let secret_key_pem = &credentials.secret_key;

        let signing_key = if secret_key_pem.contains("EC PRIVATE KEY") {
            SecretKey::from_sec1_pem(secret_key_pem)
                .map(|sk| SigningKey::from(&sk))
                .map_err(|e| {
                    crate::FeedError::ParseError(format!(
                        "Failed to parse SEC1 EC private key: {}",
                        e
                    ))
                })?
        } else if secret_key_pem.contains("PRIVATE KEY") {
            SigningKey::from_pkcs8_pem(secret_key_pem).map_err(|e| {
                crate::FeedError::ParseError(format!("Failed to parse PKCS#8 private key: {}", e))
            })?
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

        let nonce = format!(
            "{:016x}{:016x}",
            rand::random::<u64>(),
            rand::random::<u64>()
        );

        let header = serde_json::json!({
            "alg": "ES256",
            "typ": "JWT",
            "kid": credentials.key_name,
            "nonce": nonce
        });

        let payload = serde_json::json!({
            "iss": "cdp",
            "sub": credentials.key_name,
            "nbf": now,
            "exp": now + 120
        });

        let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
        let message = format!("{}.{}", header_b64, payload_b64);

        let signature: Signature = signing_key.sign(message.as_bytes());
        let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

        Ok(format!("{}.{}.{}", header_b64, payload_b64, signature_b64))
    }

    pub fn subscribe_messages_with_auth(
        product_ids: &[String],
        credentials: &CoinbaseCredentials,
    ) -> Result<Vec<String>, crate::FeedError> {
        let jwt = Self::generate_ws_jwt(credentials)?;

        let products: Vec<String> = product_ids.iter().map(|s| format!("\"{}\"", s)).collect();
        let products_str = products.join(", ");

        Ok(vec![
            format!(
                r#"{{"type": "subscribe", "product_ids": [{}], "channel": "level2", "jwt": "{}"}}"#,
                products_str, jwt
            ),
            format!(
                r#"{{"type": "subscribe", "product_ids": [{}], "channel": "heartbeats", "jwt": "{}"}}"#,
                products_str, jwt
            ),
        ])
    }

    /// Distribute symbols across multiple connection groups.
    ///
    /// Returns a vector of symbol groups, where each group has at most
    /// `COINBASE_MAX_L2_STREAMS_PER_CONNECTION` symbols (30).
    pub fn distribute_symbols(symbols: &[String]) -> Vec<Vec<String>> {
        if symbols.is_empty() {
            return vec![];
        }

        symbols
            .chunks(COINBASE_MAX_L2_STREAMS_PER_CONNECTION)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Calculate the number of connections needed for the given symbol count.
    pub fn connections_needed(symbol_count: usize) -> usize {
        if symbol_count == 0 {
            0
        } else {
            (symbol_count + COINBASE_MAX_L2_STREAMS_PER_CONNECTION - 1)
                / COINBASE_MAX_L2_STREAMS_PER_CONNECTION
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_coinbase_extract_quote_currency() {
        assert_eq!(
            CoinbaseAdapter::extract_quote_currency("BTC-USD"),
            Some("USD".to_string())
        );
        assert_eq!(
            CoinbaseAdapter::extract_quote_currency("BTC-USDT"),
            Some("USDT".to_string())
        );
        assert_eq!(
            CoinbaseAdapter::extract_quote_currency("BTC-USDC"),
            Some("USDC".to_string())
        );
        assert_eq!(CoinbaseAdapter::extract_quote_currency("INVALID"), None);
    }
}

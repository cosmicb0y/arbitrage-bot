//! REST API orderbook fetchers for initial data loading.
//!
//! Fetches orderbook data from exchanges via REST API on startup
//! to populate depth cache before WebSocket data arrives.

use crate::error::FeedError;
use arbitrage_core::FixedPoint;
use futures_util::future::join_all;
use std::collections::HashMap;
use tracing::debug;

/// Orderbook entry: (bid, ask, bid_size, ask_size)
pub type OrderbookEntry = (FixedPoint, FixedPoint, FixedPoint, FixedPoint);

/// Result type for orderbook fetch: symbol -> OrderbookEntry
pub type OrderbookResult = HashMap<String, OrderbookEntry>;

/// Binance REST API orderbook fetcher.
pub struct BinanceRestFetcher;

impl BinanceRestFetcher {
    const BASE_URL: &'static str = "https://api.binance.com";

    /// Fetch all book tickers in a single API call.
    /// Returns best bid/ask for ALL symbols on Binance.
    /// This is much more efficient than individual orderbook calls.
    async fn fetch_all_book_tickers() -> OrderbookResult {
        let url = format!("{}/api/v3/ticker/bookTicker", Self::BASE_URL);

        let response = match reqwest::get(&url).await {
            Ok(r) => r,
            Err(e) => {
                debug!("Binance: Failed to fetch book tickers: {}", e);
                return HashMap::new();
            }
        };

        if !response.status().is_success() {
            debug!("Binance: Book ticker HTTP {}", response.status());
            return HashMap::new();
        }

        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(e) => {
                debug!("Binance: Failed to parse book tickers: {}", e);
                return HashMap::new();
            }
        };

        let mut result = HashMap::new();

        // Response is array: [{"symbol":"BTCUSDT","bidPrice":"...","bidQty":"...","askPrice":"...","askQty":"..."}, ...]
        if let Some(tickers) = json.as_array() {
            for ticker in tickers {
                let symbol = match ticker["symbol"].as_str() {
                    Some(s) => s.to_lowercase(),
                    None => continue,
                };

                let bid = ticker["bidPrice"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let ask = ticker["askPrice"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let bid_size = ticker["bidQty"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let ask_size = ticker["askQty"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                if bid > 0.0 && ask > 0.0 {
                    result.insert(symbol, (
                        FixedPoint::from_f64(bid),
                        FixedPoint::from_f64(ask),
                        FixedPoint::from_f64(bid_size),
                        FixedPoint::from_f64(ask_size),
                    ));
                }
            }
        }

        result
    }

    /// Fetch orderbooks for specified symbols using bulk book ticker API.
    /// Fetches ALL tickers in one call, then filters to requested symbols.
    pub async fn fetch_orderbooks(symbols: &[String]) -> OrderbookResult {
        if symbols.is_empty() {
            debug!("Binance: No symbols to fetch");
            return HashMap::new();
        }
        debug!("Binance: Fetching {} orderbooks via book ticker API", symbols.len());

        // Fetch all book tickers in one call
        let all_tickers = Self::fetch_all_book_tickers().await;

        // Filter to only requested symbols
        let symbols_set: std::collections::HashSet<String> = symbols.iter()
            .map(|s| s.to_lowercase())
            .collect();

        let result: OrderbookResult = all_tickers.into_iter()
            .filter(|(symbol, _)| symbols_set.contains(symbol))
            .collect();

        debug!("Binance: Successfully fetched {} orderbooks", result.len());
        result
    }
}

/// Coinbase REST API orderbook fetcher.
pub struct CoinbaseRestFetcher;

impl CoinbaseRestFetcher {
    const BASE_URL: &'static str = "https://api.exchange.coinbase.com";

    /// Fetch orderbook for a single product.
    /// Returns (bid, ask, bid_size, ask_size).
    pub async fn fetch_orderbook(product_id: &str) -> Result<OrderbookEntry, FeedError> {
        let url = format!("{}/products/{}/book?level=1", Self::BASE_URL, product_id);

        let response = reqwest::get(&url).await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(FeedError::ParseError(format!("HTTP {}", response.status())));
        }

        let json: serde_json::Value = response.json::<serde_json::Value>().await
            .map_err(|e: reqwest::Error| FeedError::ParseError(e.to_string()))?;

        // Parse bids: [[price, size, num_orders], ...]
        let bids = json["bids"].as_array()
            .ok_or_else(|| FeedError::ParseError("No bids array".to_string()))?;
        let best_bid = bids.first()
            .ok_or_else(|| FeedError::ParseError("No bids".to_string()))?;
        let bid = best_bid[0].as_str()
            .ok_or_else(|| FeedError::ParseError("Invalid bid price".to_string()))?
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid price".to_string()))?;
        let bid_size = best_bid[1].as_str()
            .ok_or_else(|| FeedError::ParseError("Invalid bid size".to_string()))?
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid size".to_string()))?;

        // Parse asks: [[price, size, num_orders], ...]
        let asks = json["asks"].as_array()
            .ok_or_else(|| FeedError::ParseError("No asks array".to_string()))?;
        let best_ask = asks.first()
            .ok_or_else(|| FeedError::ParseError("No asks".to_string()))?;
        let ask = best_ask[0].as_str()
            .ok_or_else(|| FeedError::ParseError("Invalid ask price".to_string()))?
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid ask price".to_string()))?;
        let ask_size = best_ask[1].as_str()
            .ok_or_else(|| FeedError::ParseError("Invalid ask size".to_string()))?
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid ask size".to_string()))?;

        Ok((
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
            FixedPoint::from_f64(bid_size),
            FixedPoint::from_f64(ask_size),
        ))
    }

    /// Fetch orderbooks for multiple products in parallel.
    pub async fn fetch_orderbooks(product_ids: &[String]) -> OrderbookResult {
        if product_ids.is_empty() {
            debug!("Coinbase: No products to fetch");
            return HashMap::new();
        }
        debug!("Coinbase: Fetching {} orderbooks", product_ids.len());

        let futures: Vec<_> = product_ids.iter().map(|product_id| {
            let product_id = product_id.clone();
            async move {
                match Self::fetch_orderbook(&product_id).await {
                    Ok(entry) => Some((product_id, entry)),
                    Err(e) => {
                        debug!("Coinbase: Failed to fetch orderbook for {}: {}", product_id, e);
                        None
                    }
                }
            }
        }).collect();

        let results: Vec<Option<(String, OrderbookEntry)>> = join_all(futures).await;
        let result: OrderbookResult = results.into_iter().flatten().collect();
        debug!("Coinbase: Successfully fetched {} orderbooks", result.len());
        result
    }
}

/// Bybit REST API orderbook fetcher.
pub struct BybitRestFetcher;

impl BybitRestFetcher {
    const BASE_URL: &'static str = "https://api.bybit.com";

    /// Fetch all spot tickers in a single API call.
    /// Returns best bid/ask for ALL spot symbols on Bybit.
    async fn fetch_all_tickers() -> OrderbookResult {
        let url = format!("{}/v5/market/tickers?category=spot", Self::BASE_URL);

        let response = match reqwest::get(&url).await {
            Ok(r) => r,
            Err(e) => {
                debug!("Bybit: Failed to fetch tickers: {}", e);
                return HashMap::new();
            }
        };

        if !response.status().is_success() {
            debug!("Bybit: Tickers HTTP {}", response.status());
            return HashMap::new();
        }

        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(e) => {
                debug!("Bybit: Failed to parse tickers: {}", e);
                return HashMap::new();
            }
        };

        if json["retCode"].as_i64() != Some(0) {
            debug!("Bybit: Tickers API error: {:?}", json["retMsg"]);
            return HashMap::new();
        }

        let mut result = HashMap::new();

        // Response: {"result":{"list":[{"symbol":"BTCUSDT","bid1Price":"...","bid1Size":"...","ask1Price":"...","ask1Size":"..."}, ...]}}
        if let Some(list) = json["result"]["list"].as_array() {
            for ticker in list {
                let symbol = match ticker["symbol"].as_str() {
                    Some(s) => s.to_string(),
                    None => continue,
                };

                let bid = ticker["bid1Price"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let ask = ticker["ask1Price"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let bid_size = ticker["bid1Size"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let ask_size = ticker["ask1Size"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                if bid > 0.0 && ask > 0.0 {
                    result.insert(symbol, (
                        FixedPoint::from_f64(bid),
                        FixedPoint::from_f64(ask),
                        FixedPoint::from_f64(bid_size),
                        FixedPoint::from_f64(ask_size),
                    ));
                }
            }
        }

        result
    }

    /// Fetch orderbooks for specified symbols using bulk tickers API.
    pub async fn fetch_orderbooks(symbols: &[String]) -> OrderbookResult {
        if symbols.is_empty() {
            debug!("Bybit: No symbols to fetch");
            return HashMap::new();
        }
        debug!("Bybit: Fetching {} orderbooks via tickers API", symbols.len());

        // Fetch all tickers in one call
        let all_tickers = Self::fetch_all_tickers().await;

        // Filter to only requested symbols
        let symbols_set: std::collections::HashSet<String> = symbols.iter()
            .map(|s| s.to_uppercase())
            .collect();

        let result: OrderbookResult = all_tickers.into_iter()
            .filter(|(symbol, _)| symbols_set.contains(symbol))
            .collect();

        debug!("Bybit: Successfully fetched {} orderbooks", result.len());
        result
    }
}

/// Gate.io REST API orderbook fetcher.
pub struct GateIORestFetcher;

impl GateIORestFetcher {
    const BASE_URL: &'static str = "https://api.gateio.ws";
    /// Max concurrent individual ticker requests to avoid rate limiting
    const MAX_CONCURRENT_DEPTH_FETCHES: usize = 20;

    /// Fetch a single ticker with depth info.
    /// Individual ticker API returns size info, unlike the bulk tickers API.
    async fn fetch_single_ticker(currency_pair: &str) -> Option<(String, OrderbookEntry)> {
        let url = format!("{}/api/v4/spot/tickers?currency_pair={}", Self::BASE_URL, currency_pair);

        let response = match reqwest::get(&url).await {
            Ok(r) => r,
            Err(_) => return None,
        };
        if !response.status().is_success() {
            return None;
        }

        let tickers: Vec<serde_json::Value> = match response.json().await {
            Ok(t) => t,
            Err(_) => return None,
        };
        let ticker = tickers.first()?;

        let bid = ticker["highest_bid"].as_str()?.parse::<f64>().ok()?;
        let ask = ticker["lowest_ask"].as_str()?.parse::<f64>().ok()?;
        let bid_size = ticker["highest_size"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let ask_size = ticker["lowest_size"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        if bid > 0.0 && ask > 0.0 {
            Some((
                currency_pair.to_uppercase(),
                (
                    FixedPoint::from_f64(bid),
                    FixedPoint::from_f64(ask),
                    FixedPoint::from_f64(bid_size),
                    FixedPoint::from_f64(ask_size),
                ),
            ))
        } else {
            None
        }
    }

    /// Fetch orderbooks for specified currency pairs using tickers API.
    /// Uses single API call to get all tickers, then fetches individual tickers
    /// for those missing depth info (size = null in bulk API).
    pub async fn fetch_orderbooks(currency_pairs: &[String]) -> OrderbookResult {
        if currency_pairs.is_empty() {
            debug!("GateIO: No currency pairs to fetch");
            return HashMap::new();
        }
        debug!("GateIO: Fetching {} orderbooks via tickers API (single call)", currency_pairs.len());

        // Build a set of requested pairs for O(1) lookup
        let requested: std::collections::HashSet<String> = currency_pairs.iter()
            .map(|p| p.to_uppercase())
            .collect();

        let url = format!("{}/api/v4/spot/tickers", Self::BASE_URL);

        let response = match reqwest::get(&url).await {
            Ok(r) => r,
            Err(e) => {
                debug!("GateIO: Failed to fetch tickers: {}", e);
                return HashMap::new();
            }
        };

        if !response.status().is_success() {
            debug!("GateIO: Tickers API returned status {}", response.status());
            return HashMap::new();
        }

        let tickers: Vec<serde_json::Value> = match response.json().await {
            Ok(j) => j,
            Err(e) => {
                debug!("GateIO: Failed to parse tickers response: {}", e);
                return HashMap::new();
            }
        };

        let mut result = HashMap::new();
        let mut pairs_needing_depth: Vec<String> = Vec::new();

        // Response: [{"currency_pair":"BTC_USDT","highest_bid":"95000","highest_size":null,"lowest_ask":"95001","lowest_size":null,...}]
        // Note: Bulk API returns null for size fields, individual API returns actual values
        for ticker in tickers {
            let pair = match ticker["currency_pair"].as_str() {
                Some(p) => p.to_uppercase(),
                None => continue,
            };

            // Only include requested pairs
            if !requested.contains(&pair) {
                continue;
            }

            let bid = ticker["highest_bid"].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let ask = ticker["lowest_ask"].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            // Check if size is null (bulk API doesn't return size)
            let has_size = ticker["highest_size"].as_str().is_some()
                || ticker["lowest_size"].as_str().is_some();

            let bid_size = ticker["highest_size"].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let ask_size = ticker["lowest_size"].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            if bid > 0.0 && ask > 0.0 {
                result.insert(
                    pair.clone(),
                    (
                        FixedPoint::from_f64(bid),
                        FixedPoint::from_f64(ask),
                        FixedPoint::from_f64(bid_size),
                        FixedPoint::from_f64(ask_size),
                    ),
                );

                // Track pairs that need depth fetching (null sizes in bulk API)
                if !has_size {
                    pairs_needing_depth.push(pair);
                }
            }
        }

        // Only fetch individual tickers for stablecoins (for accurate depth)
        // Regular coins will get depth from WebSocket orderbook subscription
        let stablecoin_pairs: Vec<String> = pairs_needing_depth.iter()
            .filter(|p| {
                let upper = p.to_uppercase();
                upper.starts_with("USDT_") || upper.starts_with("USDC_") ||
                upper.starts_with("DAI_") || upper.starts_with("TUSD_") ||
                upper.starts_with("BUSD_") || upper.starts_with("FDUSD_")
            })
            .cloned()
            .collect();

        debug!("GateIO: Got {} prices, {} stablecoins need depth fetch (others via WebSocket)",
              result.len(), stablecoin_pairs.len());

        // Fetch depth only for stablecoins (small number, no batching needed)
        if !stablecoin_pairs.is_empty() {
            let futures: Vec<_> = stablecoin_pairs.iter()
                .map(|pair| Self::fetch_single_ticker(pair))
                .collect();

            let results = join_all(futures).await;
            let mut success_count = 0;
            for opt in results {
                if let Some((pair, entry)) = opt {
                    result.insert(pair, entry);
                    success_count += 1;
                }
            }
            debug!("GateIO: Fetched depth for {}/{} stablecoins", success_count, stablecoin_pairs.len());
        }

        debug!("GateIO: Successfully fetched {} orderbooks", result.len());
        result
    }
}

/// Upbit REST API orderbook fetcher.
pub struct UpbitRestFetcher;

impl UpbitRestFetcher {
    const BASE_URL: &'static str = "https://api.upbit.com";
    /// Max markets per batch request (Upbit allows multiple in one call)
    const BATCH_SIZE: usize = 100;

    /// Fetch orderbooks for multiple markets in a single API call.
    /// Upbit supports comma-separated markets: ?markets=KRW-BTC,KRW-ETH,...
    async fn fetch_batch(markets: &[String]) -> OrderbookResult {
        if markets.is_empty() {
            return HashMap::new();
        }

        let markets_param = markets.iter()
            .map(|m| m.to_uppercase())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!("{}/v1/orderbook?markets={}", Self::BASE_URL, markets_param);

        let response = match reqwest::get(&url).await {
            Ok(r) => r,
            Err(e) => {
                debug!("Upbit: Batch request failed: {}", e);
                return HashMap::new();
            }
        };

        if !response.status().is_success() {
            debug!("Upbit: Batch request HTTP {}", response.status());
            return HashMap::new();
        }

        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(e) => {
                debug!("Upbit: Failed to parse batch response: {}", e);
                return HashMap::new();
            }
        };

        let mut result = HashMap::new();

        // Response is an array: [{"market": "KRW-BTC", "orderbook_units": [...]}, ...]
        if let Some(orderbooks) = json.as_array() {
            for orderbook in orderbooks {
                let market = match orderbook["market"].as_str() {
                    Some(m) => m.to_string(),
                    None => continue,
                };

                let units = match orderbook["orderbook_units"].as_array() {
                    Some(u) => u,
                    None => continue,
                };

                let best = match units.first() {
                    Some(b) => b,
                    None => continue,
                };

                let ask = best["ask_price"].as_f64().unwrap_or(0.0);
                let bid = best["bid_price"].as_f64().unwrap_or(0.0);
                let ask_size = best["ask_size"].as_f64().unwrap_or(0.0);
                let bid_size = best["bid_size"].as_f64().unwrap_or(0.0);

                if bid > 0.0 && ask > 0.0 {
                    result.insert(market, (
                        FixedPoint::from_f64(bid),
                        FixedPoint::from_f64(ask),
                        FixedPoint::from_f64(bid_size),
                        FixedPoint::from_f64(ask_size),
                    ));
                }
            }
        }

        result
    }

    /// Fetch orderbooks for multiple markets using batch requests.
    pub async fn fetch_orderbooks(markets: &[String]) -> OrderbookResult {
        if markets.is_empty() {
            debug!("Upbit: No markets to fetch");
            return HashMap::new();
        }
        debug!("Upbit: Fetching {} orderbooks in {} batch(es)",
            markets.len(),
            (markets.len() + Self::BATCH_SIZE - 1) / Self::BATCH_SIZE
        );

        // Split into batches and fetch in parallel
        let batches: Vec<_> = markets.chunks(Self::BATCH_SIZE)
            .map(|chunk| chunk.to_vec())
            .collect();

        let futures: Vec<_> = batches.iter()
            .map(|batch| Self::fetch_batch(batch))
            .collect();

        let batch_results = join_all(futures).await;

        let mut result = HashMap::new();
        for batch_result in batch_results {
            result.extend(batch_result);
        }

        debug!("Upbit: Successfully fetched {} orderbooks", result.len());
        result
    }
}

/// Bithumb REST API orderbook fetcher.
pub struct BithumbRestFetcher;

impl BithumbRestFetcher {
    const BASE_URL: &'static str = "https://api.bithumb.com";

    /// Fetch orderbook for a single symbol.
    /// Symbol format: "BTC" (base currency, quote is always KRW)
    /// Returns (bid, ask, bid_size, ask_size) in KRW.
    pub async fn fetch_orderbook(symbol: &str) -> Result<OrderbookEntry, FeedError> {
        // Bithumb API: /public/orderbook/{symbol}_KRW
        let url = format!("{}/public/orderbook/{}_KRW", Self::BASE_URL, symbol.to_uppercase());

        let response = reqwest::get(&url).await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(FeedError::ParseError(format!("HTTP {}", response.status())));
        }

        let json: serde_json::Value = response.json::<serde_json::Value>().await
            .map_err(|e: reqwest::Error| FeedError::ParseError(e.to_string()))?;

        // Check status
        if json["status"].as_str() != Some("0000") {
            return Err(FeedError::ParseError(format!("Bithumb API error: {:?}", json["message"])));
        }

        let data = &json["data"];

        // Parse bids: [{"price": "...", "quantity": "..."}, ...]
        let bids = data["bids"].as_array()
            .ok_or_else(|| FeedError::ParseError("No bids array".to_string()))?;
        let best_bid = bids.first()
            .ok_or_else(|| FeedError::ParseError("No bids".to_string()))?;
        let bid = best_bid["price"].as_str()
            .ok_or_else(|| FeedError::ParseError("Invalid bid price".to_string()))?
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid price".to_string()))?;
        let bid_size = best_bid["quantity"].as_str()
            .ok_or_else(|| FeedError::ParseError("Invalid bid size".to_string()))?
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid bid size".to_string()))?;

        // Parse asks: [{"price": "...", "quantity": "..."}, ...]
        let asks = data["asks"].as_array()
            .ok_or_else(|| FeedError::ParseError("No asks array".to_string()))?;
        let best_ask = asks.first()
            .ok_or_else(|| FeedError::ParseError("No asks".to_string()))?;
        let ask = best_ask["price"].as_str()
            .ok_or_else(|| FeedError::ParseError("Invalid ask price".to_string()))?
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid ask price".to_string()))?;
        let ask_size = best_ask["quantity"].as_str()
            .ok_or_else(|| FeedError::ParseError("Invalid ask size".to_string()))?
            .parse::<f64>()
            .map_err(|_| FeedError::ParseError("Invalid ask size".to_string()))?;

        Ok((
            FixedPoint::from_f64(bid),
            FixedPoint::from_f64(ask),
            FixedPoint::from_f64(bid_size),
            FixedPoint::from_f64(ask_size),
        ))
    }

    /// Fetch orderbooks for multiple symbols in parallel.
    /// Symbols are base currencies (e.g., "BTC", "ETH")
    pub async fn fetch_orderbooks(symbols: &[String]) -> OrderbookResult {
        if symbols.is_empty() {
            debug!("Bithumb: No symbols to fetch");
            return HashMap::new();
        }
        debug!("Bithumb: Fetching {} orderbooks", symbols.len());

        let futures: Vec<_> = symbols.iter().map(|symbol| {
            let symbol = symbol.clone();
            async move {
                match Self::fetch_orderbook(&symbol).await {
                    Ok(entry) => Some((symbol, entry)),
                    Err(e) => {
                        debug!("Bithumb: Failed to fetch orderbook for {}: {}", symbol, e);
                        None
                    }
                }
            }
        }).collect();

        let results: Vec<Option<(String, OrderbookEntry)>> = join_all(futures).await;
        let result: OrderbookResult = results.into_iter().flatten().collect();
        debug!("Bithumb: Successfully fetched {} orderbooks", result.len());
        result
    }
}

/// Fetch initial orderbooks from all exchanges.
/// Returns a map of (exchange, symbol) -> OrderbookEntry.
pub async fn fetch_all_initial_orderbooks(
    binance_symbols: &[String],
    coinbase_products: &[String],
    bybit_symbols: &[String],
    gateio_pairs: &[String],
) -> HashMap<(String, String), OrderbookEntry> {
    debug!("Fetching initial orderbooks from all exchanges...");

    // Fetch from all exchanges in parallel
    let (binance_result, coinbase_result, bybit_result, gateio_result) = tokio::join!(
        BinanceRestFetcher::fetch_orderbooks(binance_symbols),
        CoinbaseRestFetcher::fetch_orderbooks(coinbase_products),
        BybitRestFetcher::fetch_orderbooks(bybit_symbols),
        GateIORestFetcher::fetch_orderbooks(gateio_pairs),
    );

    let mut all_orderbooks = HashMap::new();

    // Add Binance results
    for (symbol, entry) in binance_result {
        all_orderbooks.insert(("Binance".to_string(), symbol), entry);
    }
    debug!("Binance: Fetched {} orderbooks", all_orderbooks.iter().filter(|((ex, _), _)| ex == "Binance").count());

    // Add Coinbase results
    for (product_id, entry) in coinbase_result {
        all_orderbooks.insert(("Coinbase".to_string(), product_id), entry);
    }
    debug!("Coinbase: Fetched {} orderbooks", all_orderbooks.iter().filter(|((ex, _), _)| ex == "Coinbase").count());

    // Add Bybit results
    for (symbol, entry) in bybit_result {
        all_orderbooks.insert(("Bybit".to_string(), symbol), entry);
    }
    debug!("Bybit: Fetched {} orderbooks", all_orderbooks.iter().filter(|((ex, _), _)| ex == "Bybit").count());

    // Add GateIO results
    for (pair, entry) in gateio_result {
        all_orderbooks.insert(("GateIO".to_string(), pair), entry);
    }
    debug!("GateIO: Fetched {} orderbooks", all_orderbooks.iter().filter(|((ex, _), _)| ex == "GateIO").count());

    debug!("Total: Fetched {} orderbooks from all exchanges", all_orderbooks.len());
    all_orderbooks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_binance_fetch_orderbooks() {
        // This is an integration test - requires network
        let symbols = vec!["BTCUSDT".to_string()];
        let result = BinanceRestFetcher::fetch_orderbooks(&symbols).await;
        if let Some((bid, ask, bid_size, ask_size)) = result.get("BTCUSDT") {
            assert!(bid.to_f64() > 0.0);
            assert!(ask.to_f64() > 0.0);
            assert!(bid.to_f64() < ask.to_f64()); // bid < ask
            assert!(bid_size.to_f64() > 0.0);
            assert!(ask_size.to_f64() > 0.0);
        }
        // Don't fail if network is unavailable
    }
}

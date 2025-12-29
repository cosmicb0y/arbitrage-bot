//! Market discovery module.
//!
//! Fetches available markets from exchanges via REST APIs
//! and finds common trading pairs across exchanges.

use crate::FeedError;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// Normalized market info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarketInfo {
    /// Base asset (e.g., "BTC")
    pub base: String,
    /// Quote asset (e.g., "USDT", "USD", "KRW")
    pub quote: String,
    /// Original symbol from exchange
    pub symbol: String,
    /// Whether trading is enabled
    pub trading_enabled: bool,
}

impl MarketInfo {
    /// Get normalized pair key (e.g., "BTC/USD")
    pub fn pair_key(&self) -> String {
        format!("{}/{}", self.base, self.normalized_quote())
    }

    /// Normalize quote currency (USD, USDT, USDC -> USD)
    pub fn normalized_quote(&self) -> &str {
        match self.quote.as_str() {
            "USDT" | "USDC" | "BUSD" => "USD",
            other => other,
        }
    }
}

/// Exchange market data.
#[derive(Debug, Clone, Default)]
pub struct ExchangeMarkets {
    /// All available markets
    pub markets: Vec<MarketInfo>,
    /// Last update timestamp (ms)
    pub updated_at: u64,
}

/// Common markets across exchanges.
#[derive(Debug, Clone)]
pub struct CommonMarkets {
    /// Base assets that are available on 2+ exchanges
    /// Maps base asset -> list of (exchange, market_info)
    pub common: HashMap<String, Vec<(String, MarketInfo)>>,
    /// Exchanges that were compared
    pub exchanges: Vec<String>,
    /// Minimum number of exchanges required (for filtering)
    pub min_exchanges: usize,
}

impl CommonMarkets {
    /// Get list of common base assets (e.g., ["BTC", "ETH", "SOL"])
    pub fn common_bases(&self) -> Vec<String> {
        self.common.keys().cloned().collect()
    }

    /// Get market info for a specific base on an exchange
    pub fn get_market(&self, base: &str, exchange: &str) -> Option<&MarketInfo> {
        self.common.get(base).and_then(|markets| {
            markets
                .iter()
                .find(|(ex, _)| ex == exchange)
                .map(|(_, info)| info)
        })
    }

    /// Get the number of exchanges a market is available on
    pub fn exchange_count(&self, base: &str) -> usize {
        self.common.get(base).map(|m| m.len()).unwrap_or(0)
    }
}

/// Market discovery client.
pub struct MarketDiscovery {
    client: reqwest::Client,
}

impl Default for MarketDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketDiscovery {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Fetch markets from Binance.
    pub async fn fetch_binance(&self) -> Result<ExchangeMarkets, FeedError> {
        #[derive(Debug, Deserialize)]
        struct BinanceExchangeInfo {
            symbols: Vec<BinanceSymbol>,
        }

        #[derive(Debug, Deserialize)]
        struct BinanceSymbol {
            symbol: String,
            #[serde(rename = "baseAsset")]
            base_asset: String,
            #[serde(rename = "quoteAsset")]
            quote_asset: String,
            status: String,
        }

        let url = "https://api.binance.com/api/v3/exchangeInfo";
        let resp: BinanceExchangeInfo = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?
            .json()
            .await
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let markets: Vec<MarketInfo> = resp
            .symbols
            .into_iter()
            .filter(|s| s.quote_asset == "USDT" || s.quote_asset == "BUSD")
            .map(|s| MarketInfo {
                base: s.base_asset,
                quote: s.quote_asset,
                symbol: s.symbol,
                trading_enabled: s.status == "TRADING",
            })
            .collect();

        debug!("Binance: fetched {} USDT/BUSD markets", markets.len());

        Ok(ExchangeMarkets {
            markets,
            updated_at: now_ms(),
        })
    }

    /// Fetch markets from Coinbase.
    pub async fn fetch_coinbase(&self) -> Result<ExchangeMarkets, FeedError> {
        #[derive(Debug, Deserialize)]
        struct CoinbaseProduct {
            id: String,
            base_currency: String,
            quote_currency: String,
            status: String,
        }

        let url = "https://api.exchange.coinbase.com/products";
        let resp: Vec<CoinbaseProduct> = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?
            .json()
            .await
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let markets: Vec<MarketInfo> = resp
            .into_iter()
            .filter(|p| p.quote_currency == "USD" || p.quote_currency == "USDT")
            .map(|p| MarketInfo {
                base: p.base_currency,
                quote: p.quote_currency,
                symbol: p.id,
                trading_enabled: p.status == "online",
            })
            .collect();

        debug!("Coinbase: fetched {} USD/USDT markets", markets.len());

        Ok(ExchangeMarkets {
            markets,
            updated_at: now_ms(),
        })
    }

    /// Fetch markets from Upbit.
    pub async fn fetch_upbit(&self) -> Result<ExchangeMarkets, FeedError> {
        #[derive(Debug, Deserialize)]
        struct UpbitMarketEvent {
            warning: bool,
        }

        #[derive(Debug, Deserialize)]
        struct UpbitMarket {
            market: String,
            #[serde(rename = "korean_name")]
            _korean_name: String,
            #[serde(rename = "english_name")]
            _english_name: String,
            #[serde(default)]
            market_event: Option<UpbitMarketEvent>,
        }

        let url = "https://api.upbit.com/v1/market/all?isDetails=true";
        let resp: Vec<UpbitMarket> = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?
            .json()
            .await
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let markets: Vec<MarketInfo> = resp
            .into_iter()
            .filter(|m| m.market.starts_with("KRW-"))
            .map(|m| {
                let base = m.market.strip_prefix("KRW-").unwrap_or(&m.market);
                let trading_enabled = m.market_event.map(|e| !e.warning).unwrap_or(true);
                MarketInfo {
                    base: base.to_string(),
                    quote: "KRW".to_string(),
                    symbol: m.market,
                    trading_enabled,
                }
            })
            .collect();

        debug!("Upbit: fetched {} KRW markets", markets.len());

        Ok(ExchangeMarkets {
            markets,
            updated_at: now_ms(),
        })
    }

    /// Fetch markets from Bithumb.
    pub async fn fetch_bithumb(&self) -> Result<ExchangeMarkets, FeedError> {
        #[derive(Debug, Deserialize)]
        struct BithumbResponse {
            status: String,
            data: HashMap<String, serde_json::Value>,
        }

        let url = "https://api.bithumb.com/public/ticker/ALL_KRW";
        let resp: BithumbResponse = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?
            .json()
            .await
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        if resp.status != "0000" {
            return Err(FeedError::ParseError(format!(
                "Bithumb API error: status {}",
                resp.status
            )));
        }

        let markets: Vec<MarketInfo> = resp
            .data
            .keys()
            .filter(|k| *k != "date") // Bithumb includes a "date" field in the response
            .map(|base| MarketInfo {
                base: base.clone(),
                quote: "KRW".to_string(),
                symbol: format!("KRW-{}", base),
                trading_enabled: true,
            })
            .collect();

        debug!("Bithumb: fetched {} KRW markets", markets.len());

        Ok(ExchangeMarkets {
            markets,
            updated_at: now_ms(),
        })
    }

    /// Fetch markets from Bybit.
    pub async fn fetch_bybit(&self) -> Result<ExchangeMarkets, FeedError> {
        #[derive(Debug, Deserialize)]
        struct BybitResponse {
            result: BybitResult,
        }

        #[derive(Debug, Deserialize)]
        struct BybitResult {
            list: Vec<BybitSymbol>,
        }

        #[derive(Debug, Deserialize)]
        struct BybitSymbol {
            symbol: String,
            #[serde(rename = "baseCoin")]
            base_coin: String,
            #[serde(rename = "quoteCoin")]
            quote_coin: String,
            status: String,
        }

        let url = "https://api.bybit.com/v5/market/instruments-info?category=spot";
        let resp: BybitResponse = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?
            .json()
            .await
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let markets: Vec<MarketInfo> = resp
            .result
            .list
            .into_iter()
            .filter(|s| s.quote_coin == "USDT" || s.quote_coin == "USDC")
            .map(|s| MarketInfo {
                base: s.base_coin,
                quote: s.quote_coin,
                symbol: s.symbol,
                trading_enabled: s.status == "Trading",
            })
            .collect();

        debug!("Bybit: fetched {} USDT/USDC markets", markets.len());

        Ok(ExchangeMarkets {
            markets,
            updated_at: now_ms(),
        })
    }

    /// Fetch markets from OKX.
    pub async fn fetch_okx(&self) -> Result<ExchangeMarkets, FeedError> {
        #[derive(Debug, Deserialize)]
        struct OkxResponse {
            data: Vec<OkxInstrument>,
        }

        #[derive(Debug, Deserialize)]
        struct OkxInstrument {
            #[serde(rename = "instId")]
            inst_id: String,
            #[serde(rename = "baseCcy")]
            base_ccy: String,
            #[serde(rename = "quoteCcy")]
            quote_ccy: String,
            state: String,
        }

        let url = "https://www.okx.com/api/v5/public/instruments?instType=SPOT";
        let resp: OkxResponse = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?
            .json()
            .await
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let markets: Vec<MarketInfo> = resp
            .data
            .into_iter()
            .filter(|s| s.quote_ccy == "USDT" || s.quote_ccy == "USDC")
            .map(|s| MarketInfo {
                base: s.base_ccy,
                quote: s.quote_ccy,
                symbol: s.inst_id,
                trading_enabled: s.state == "live",
            })
            .collect();

        debug!("OKX: fetched {} USDT/USDC markets", markets.len());

        Ok(ExchangeMarkets {
            markets,
            updated_at: now_ms(),
        })
    }

    /// Fetch markets from Kraken.
    pub async fn fetch_kraken(&self) -> Result<ExchangeMarkets, FeedError> {
        #[derive(Debug, Deserialize)]
        struct KrakenResponse {
            result: HashMap<String, KrakenPair>,
        }

        #[derive(Debug, Deserialize)]
        struct KrakenPair {
            wsname: Option<String>,
            base: String,
            quote: String,
            status: String,
        }

        let url = "https://api.kraken.com/0/public/AssetPairs";
        let resp: KrakenResponse = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FeedError::ConnectionFailed(e.to_string()))?
            .json()
            .await
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let markets: Vec<MarketInfo> = resp
            .result
            .into_iter()
            .filter(|(_, p)| p.quote == "USD" || p.quote == "ZUSD" || p.quote == "USDT")
            .map(|(symbol, p)| {
                // Normalize Kraken's weird asset names (XXBT -> BTC, XETH -> ETH)
                let base = normalize_kraken_asset(&p.base);
                let quote = if p.quote == "ZUSD" { "USD" } else { &p.quote };
                MarketInfo {
                    base,
                    quote: quote.to_string(),
                    symbol,
                    trading_enabled: p.status == "online",
                }
            })
            .collect();

        debug!("Kraken: fetched {} USD/USDT markets", markets.len());

        Ok(ExchangeMarkets {
            markets,
            updated_at: now_ms(),
        })
    }

    /// Fetch all markets from all exchanges.
    pub async fn fetch_all(&self) -> HashMap<String, ExchangeMarkets> {
        let mut results = HashMap::new();

        // Fetch in parallel
        let (binance, coinbase, upbit, bithumb, bybit, okx, kraken) = tokio::join!(
            self.fetch_binance(),
            self.fetch_coinbase(),
            self.fetch_upbit(),
            self.fetch_bithumb(),
            self.fetch_bybit(),
            self.fetch_okx(),
            self.fetch_kraken(),
        );

        if let Ok(m) = binance {
            info!("Binance: {} markets", m.markets.len());
            results.insert("Binance".to_string(), m);
        } else if let Err(e) = binance {
            warn!("Failed to fetch Binance markets: {}", e);
        }

        if let Ok(m) = coinbase {
            info!("Coinbase: {} markets", m.markets.len());
            results.insert("Coinbase".to_string(), m);
        } else if let Err(e) = coinbase {
            warn!("Failed to fetch Coinbase markets: {}", e);
        }

        if let Ok(m) = upbit {
            info!("Upbit: {} markets", m.markets.len());
            results.insert("Upbit".to_string(), m);
        } else if let Err(e) = upbit {
            warn!("Failed to fetch Upbit markets: {}", e);
        }

        if let Ok(m) = bithumb {
            info!("Bithumb: {} markets", m.markets.len());
            results.insert("Bithumb".to_string(), m);
        } else if let Err(e) = bithumb {
            warn!("Failed to fetch Bithumb markets: {}", e);
        }

        if let Ok(m) = bybit {
            info!("Bybit: {} markets", m.markets.len());
            results.insert("Bybit".to_string(), m);
        } else if let Err(e) = bybit {
            warn!("Failed to fetch Bybit markets: {}", e);
        }

        if let Ok(m) = okx {
            info!("OKX: {} markets", m.markets.len());
            results.insert("Okx".to_string(), m);
        } else if let Err(e) = okx {
            warn!("Failed to fetch OKX markets: {}", e);
        }

        if let Ok(m) = kraken {
            info!("Kraken: {} markets", m.markets.len());
            results.insert("Kraken".to_string(), m);
        } else if let Err(e) = kraken {
            warn!("Failed to fetch Kraken markets: {}", e);
        }

        results
    }

    /// Find common markets across specified exchanges.
    /// Returns markets that are available on ALL specified exchanges.
    pub fn find_common_markets(
        all_markets: &HashMap<String, ExchangeMarkets>,
        exchanges: &[&str],
    ) -> CommonMarkets {
        Self::find_markets_on_n_exchanges(all_markets, exchanges, exchanges.len())
    }

    /// Find markets available on at least `min_exchanges` exchanges.
    /// Returns markets sorted by exchange count (descending).
    pub fn find_markets_on_n_exchanges(
        all_markets: &HashMap<String, ExchangeMarkets>,
        exchanges: &[&str],
        min_exchanges: usize,
    ) -> CommonMarkets {
        // Collect all unique base assets across all exchanges
        let mut all_bases: HashSet<String> = HashSet::new();
        for ex in exchanges {
            if let Some(markets) = all_markets.get(*ex) {
                for m in &markets.markets {
                    if m.trading_enabled {
                        all_bases.insert(m.base.clone());
                    }
                }
            }
        }

        // Build result with market info for each base that appears on min_exchanges or more
        let mut common: HashMap<String, Vec<(String, MarketInfo)>> = HashMap::new();

        for base in &all_bases {
            let mut markets_for_base = Vec::new();

            for ex in exchanges {
                if let Some(ex_markets) = all_markets.get(*ex) {
                    if let Some(market) = ex_markets
                        .markets
                        .iter()
                        .find(|m| &m.base == base && m.trading_enabled)
                    {
                        markets_for_base.push((ex.to_string(), market.clone()));
                    }
                }
            }

            if markets_for_base.len() >= min_exchanges {
                common.insert(base.clone(), markets_for_base);
            }
        }

        // Count by exchange availability
        let all_count = common.values().filter(|v| v.len() == exchanges.len()).count();
        let partial_count = common.len() - all_count;

        info!(
            "Found {} markets on {}+ exchanges ({} on all {}, {} on 2+ but not all)",
            common.len(),
            min_exchanges,
            all_count,
            exchanges.len(),
            partial_count
        );

        CommonMarkets {
            common,
            exchanges: exchanges.iter().map(|s| s.to_string()).collect(),
            min_exchanges,
        }
    }
}

/// Normalize Kraken's weird asset names.
fn normalize_kraken_asset(asset: &str) -> String {
    match asset {
        "XXBT" | "XBT" => "BTC".to_string(),
        "XETH" => "ETH".to_string(),
        "XXRP" => "XRP".to_string(),
        "XXLM" => "XLM".to_string(),
        "XXMR" => "XMR".to_string(),
        "XLTC" => "LTC".to_string(),
        "XDOGE" => "DOGE".to_string(),
        s if s.starts_with('X') && s.len() == 4 => s[1..].to_string(),
        s => s.to_string(),
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_info_pair_key() {
        let market = MarketInfo {
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            symbol: "BTCUSDT".to_string(),
            trading_enabled: true,
        };
        assert_eq!(market.pair_key(), "BTC/USD");
        assert_eq!(market.normalized_quote(), "USD");
    }

    #[test]
    fn test_normalize_kraken_asset() {
        assert_eq!(normalize_kraken_asset("XXBT"), "BTC");
        assert_eq!(normalize_kraken_asset("XETH"), "ETH");
        assert_eq!(normalize_kraken_asset("SOL"), "SOL");
    }

    #[test]
    fn test_find_common_markets() {
        let mut all_markets = HashMap::new();

        all_markets.insert(
            "Binance".to_string(),
            ExchangeMarkets {
                markets: vec![
                    MarketInfo {
                        base: "BTC".to_string(),
                        quote: "USDT".to_string(),
                        symbol: "BTCUSDT".to_string(),
                        trading_enabled: true,
                    },
                    MarketInfo {
                        base: "ETH".to_string(),
                        quote: "USDT".to_string(),
                        symbol: "ETHUSDT".to_string(),
                        trading_enabled: true,
                    },
                ],
                updated_at: 0,
            },
        );

        all_markets.insert(
            "Coinbase".to_string(),
            ExchangeMarkets {
                markets: vec![
                    MarketInfo {
                        base: "BTC".to_string(),
                        quote: "USD".to_string(),
                        symbol: "BTC-USD".to_string(),
                        trading_enabled: true,
                    },
                    MarketInfo {
                        base: "SOL".to_string(),
                        quote: "USD".to_string(),
                        symbol: "SOL-USD".to_string(),
                        trading_enabled: true,
                    },
                ],
                updated_at: 0,
            },
        );

        // find_common_markets returns only markets on ALL exchanges
        let common =
            MarketDiscovery::find_common_markets(&all_markets, &["Binance", "Coinbase"]);

        assert_eq!(common.common.len(), 1);
        assert!(common.common.contains_key("BTC"));
        assert!(!common.common.contains_key("ETH"));
        assert!(!common.common.contains_key("SOL"));
        assert_eq!(common.min_exchanges, 2);
    }

    #[test]
    fn test_find_markets_on_n_exchanges() {
        let mut all_markets = HashMap::new();

        all_markets.insert(
            "Binance".to_string(),
            ExchangeMarkets {
                markets: vec![
                    MarketInfo {
                        base: "BTC".to_string(),
                        quote: "USDT".to_string(),
                        symbol: "BTCUSDT".to_string(),
                        trading_enabled: true,
                    },
                    MarketInfo {
                        base: "ETH".to_string(),
                        quote: "USDT".to_string(),
                        symbol: "ETHUSDT".to_string(),
                        trading_enabled: true,
                    },
                ],
                updated_at: 0,
            },
        );

        all_markets.insert(
            "Coinbase".to_string(),
            ExchangeMarkets {
                markets: vec![
                    MarketInfo {
                        base: "BTC".to_string(),
                        quote: "USD".to_string(),
                        symbol: "BTC-USD".to_string(),
                        trading_enabled: true,
                    },
                    MarketInfo {
                        base: "SOL".to_string(),
                        quote: "USD".to_string(),
                        symbol: "SOL-USD".to_string(),
                        trading_enabled: true,
                    },
                ],
                updated_at: 0,
            },
        );

        all_markets.insert(
            "Upbit".to_string(),
            ExchangeMarkets {
                markets: vec![
                    MarketInfo {
                        base: "BTC".to_string(),
                        quote: "KRW".to_string(),
                        symbol: "KRW-BTC".to_string(),
                        trading_enabled: true,
                    },
                    MarketInfo {
                        base: "ETH".to_string(),
                        quote: "KRW".to_string(),
                        symbol: "KRW-ETH".to_string(),
                        trading_enabled: true,
                    },
                ],
                updated_at: 0,
            },
        );

        // Find markets on 2+ exchanges
        let common = MarketDiscovery::find_markets_on_n_exchanges(
            &all_markets,
            &["Binance", "Coinbase", "Upbit"],
            2,
        );

        // BTC is on all 3, ETH is on 2 (Binance, Upbit), SOL is on 1 (Coinbase only)
        assert_eq!(common.common.len(), 2); // BTC and ETH
        assert!(common.common.contains_key("BTC"));
        assert!(common.common.contains_key("ETH"));
        assert!(!common.common.contains_key("SOL")); // Only on 1 exchange

        // Check exchange counts
        assert_eq!(common.exchange_count("BTC"), 3);
        assert_eq!(common.exchange_count("ETH"), 2);
        assert_eq!(common.exchange_count("SOL"), 0); // Not in result
    }
}

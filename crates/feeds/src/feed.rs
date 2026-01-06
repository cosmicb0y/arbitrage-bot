//! Exchange-specific feed implementations.

use crate::{
    BinanceAdapter, BithumbAdapter, CoinbaseAdapter, FeedConfig, FeedError, PriceAggregator,
    WsMessage,
};
use arbitrage_core::{Exchange, PriceTick};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Trait for exchange feed handlers.
pub trait FeedHandler: Send + Sync {
    /// Parse a WebSocket message into a price tick.
    fn parse_message(&self, msg: &str, pair_id: u32) -> Result<PriceTick, FeedError>;

    /// Generate subscription message.
    fn subscribe_message(&self, symbols: &[String]) -> String;

    /// Get the exchange.
    fn exchange(&self) -> Exchange;
}

/// Binance feed handler.
pub struct BinanceFeed;

impl FeedHandler for BinanceFeed {
    fn parse_message(&self, msg: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        // Try book ticker first, then regular ticker
        BinanceAdapter::parse_book_ticker(msg, pair_id)
            .or_else(|_| BinanceAdapter::parse_ticker(msg, pair_id))
    }

    fn subscribe_message(&self, symbols: &[String]) -> String {
        BinanceAdapter::subscribe_message(symbols)
    }

    fn exchange(&self) -> Exchange {
        Exchange::Binance
    }
}

/// Coinbase feed handler.
pub struct CoinbaseFeed;

impl FeedHandler for CoinbaseFeed {
    fn parse_message(&self, msg: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        CoinbaseAdapter::parse_ticker(msg, pair_id)
    }

    fn subscribe_message(&self, symbols: &[String]) -> String {
        CoinbaseAdapter::subscribe_message(symbols)
    }

    fn exchange(&self) -> Exchange {
        Exchange::Coinbase
    }
}

/// Bithumb feed handler.
pub struct BithumbFeed;

impl FeedHandler for BithumbFeed {
    fn parse_message(&self, msg: &str, pair_id: u32) -> Result<PriceTick, FeedError> {
        BithumbAdapter::parse_ticker(msg, pair_id)
    }

    fn subscribe_message(&self, symbols: &[String]) -> String {
        BithumbAdapter::subscribe_message(symbols)
    }

    fn exchange(&self) -> Exchange {
        Exchange::Bithumb
    }
}

/// Price feed that connects to an exchange and updates the aggregator.
pub struct PriceFeed {
    config: FeedConfig,
    handler: Box<dyn FeedHandler>,
    aggregator: Arc<PriceAggregator>,
    symbols: Vec<String>,
    pair_id: u32,
}

impl PriceFeed {
    /// Create a new price feed.
    pub fn new(
        config: FeedConfig,
        handler: Box<dyn FeedHandler>,
        aggregator: Arc<PriceAggregator>,
        symbols: Vec<String>,
        pair_id: u32,
    ) -> Self {
        Self {
            config,
            handler,
            aggregator,
            symbols,
            pair_id,
        }
    }

    /// Create a Binance feed.
    pub fn binance(aggregator: Arc<PriceAggregator>, symbols: Vec<String>, pair_id: u32) -> Self {
        Self::new(
            FeedConfig::for_exchange(Exchange::Binance),
            Box::new(BinanceFeed),
            aggregator,
            symbols,
            pair_id,
        )
    }

    /// Create a Coinbase feed.
    pub fn coinbase(aggregator: Arc<PriceAggregator>, symbols: Vec<String>, pair_id: u32) -> Self {
        Self::new(
            FeedConfig::for_exchange(Exchange::Coinbase),
            Box::new(CoinbaseFeed),
            aggregator,
            symbols,
            pair_id,
        )
    }

    /// Run the feed, processing messages from the WebSocket.
    pub async fn run(self, mut rx: mpsc::Receiver<WsMessage>) {
        let exchange = self.handler.exchange();
        info!("Starting price feed for {:?}", exchange);

        let mut message_count = 0u64;
        let mut error_count = 0u64;

        while let Some(msg) = rx.recv().await {
            match msg {
                WsMessage::Text(text) => {
                    match self.handler.parse_message(&text, self.pair_id) {
                        Ok(tick) => {
                            self.aggregator.update(tick);
                            message_count += 1;

                            if message_count % 1000 == 0 {
                                debug!(
                                    "{:?}: Processed {} messages ({} errors)",
                                    exchange, message_count, error_count
                                );
                            }
                        }
                        Err(e) => {
                            // Not all messages are price updates
                            if !text.contains("result") && !text.contains("subscribed") {
                                error_count += 1;
                                if error_count % 100 == 1 {
                                    debug!("{:?} parse error: {}", exchange, e);
                                }
                            }
                        }
                    }
                }
                WsMessage::Connected => {
                    info!("{:?}: Connected", exchange);
                }
                WsMessage::Reconnected => {
                    info!("{:?}: Reconnected", exchange);
                }
                WsMessage::Disconnected => {
                    warn!("{:?}: Disconnected", exchange);
                }
                WsMessage::Error(e) => {
                    error!("{:?}: Error - {}", exchange, e);
                }
                WsMessage::Binary(_) => {
                    // Some exchanges use binary format
                }
            }
        }

        info!(
            "{:?}: Feed stopped. Total messages: {}, errors: {}",
            exchange, message_count, error_count
        );
    }

    /// Get subscription message for this feed.
    pub fn subscribe_message(&self) -> String {
        self.handler.subscribe_message(&self.symbols)
    }

    /// Get the feed config.
    pub fn config(&self) -> &FeedConfig {
        &self.config
    }
}

/// Builder for creating multiple feeds.
pub struct FeedBuilder {
    aggregator: Arc<PriceAggregator>,
    feeds: Vec<(FeedConfig, Box<dyn FeedHandler>, Vec<String>, u32)>,
}

impl FeedBuilder {
    /// Create a new feed builder.
    pub fn new(aggregator: Arc<PriceAggregator>) -> Self {
        Self {
            aggregator,
            feeds: Vec::new(),
        }
    }

    /// Add a Binance feed.
    pub fn add_binance(mut self, symbols: Vec<String>, pair_id: u32) -> Self {
        self.feeds.push((
            FeedConfig::for_exchange(Exchange::Binance),
            Box::new(BinanceFeed),
            symbols,
            pair_id,
        ));
        self
    }

    /// Add a Coinbase feed.
    pub fn add_coinbase(mut self, symbols: Vec<String>, pair_id: u32) -> Self {
        self.feeds.push((
            FeedConfig::for_exchange(Exchange::Coinbase),
            Box::new(CoinbaseFeed),
            symbols,
            pair_id,
        ));
        self
    }

    /// Build all feeds.
    pub fn build(self) -> Vec<PriceFeed> {
        self.feeds
            .into_iter()
            .map(|(config, handler, symbols, pair_id)| {
                PriceFeed::new(config, handler, self.aggregator.clone(), symbols, pair_id)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_feed_handler() {
        let handler = BinanceFeed;
        assert_eq!(handler.exchange(), Exchange::Binance);

        let sub = handler.subscribe_message(&["btcusdt".to_string()]);
        assert!(sub.contains("btcusdt"));
    }

    #[test]
    fn test_coinbase_feed_handler() {
        let handler = CoinbaseFeed;
        assert_eq!(handler.exchange(), Exchange::Coinbase);

        let sub = handler.subscribe_message(&["BTC-USD".to_string()]);
        assert!(sub.contains("BTC-USD"));
    }

    #[test]
    fn test_price_feed_creation() {
        let aggregator = Arc::new(PriceAggregator::new());
        let feed = PriceFeed::binance(aggregator, vec!["btcusdt".to_string()], 1);
        assert!(!feed.config().ws_url.is_empty());
    }

    #[test]
    fn test_feed_builder() {
        let aggregator = Arc::new(PriceAggregator::new());
        let feeds = FeedBuilder::new(aggregator)
            .add_binance(vec!["btcusdt".to_string()], 1)
            .add_coinbase(vec!["BTC-USD".to_string()], 1)
            .build();

        assert_eq!(feeds.len(), 2);
    }
}

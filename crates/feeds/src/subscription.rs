//! Subscription management types for dynamic market subscription.
//!
//! This module provides data structures and managers for runtime subscription
//! changes to exchange WebSocket connections.
//!
//! ## Components
//!
//! - [`SubscriptionChange`] - Enum representing subscribe/unsubscribe requests
//! - [`SubscriptionManager`] - Central coordinator for managing subscriptions across exchanges
//! - [`SubscriptionError`] - Error types for subscription operations
//!
//! ## Usage
//!
//! ```rust
//! use arbitrage_feeds::SubscriptionChange;
//!
//! // Subscribe to new markets
//! let subscribe = SubscriptionChange::Subscribe(vec![
//!     "BTCUSDT".to_string(),
//!     "ETHUSDT".to_string(),
//! ]);
//!
//! // Unsubscribe from markets
//! let unsubscribe = SubscriptionChange::Unsubscribe(vec![
//!     "XRPUSDT".to_string(),
//! ]);
//! ```

use crate::websocket::SubscriptionBuilder;
use arbitrage_core::Exchange;
use dashmap::DashMap;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default channel buffer size for subscription changes.
/// Set to 1024 to match Binance's maximum stream limit and prevent
/// buffer overflow during batch subscription operations.
pub const SUBSCRIPTION_CHANNEL_BUFFER: usize = 1024;

/// Represents a subscription change request for WebSocket connections.
///
/// This enum is used to communicate subscription changes through channels
/// from the `SubscriptionManager` to individual `WsClient` instances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionChange {
    /// Subscribe to the specified market symbols.
    ///
    /// The vector contains exchange-specific symbol formats
    /// (e.g., "BTCUSDT" for Binance, "BTC-USD" for Coinbase).
    Subscribe(Vec<String>),

    /// Unsubscribe from the specified market symbols.
    ///
    /// The vector contains exchange-specific symbol formats
    /// matching those used in the original subscription.
    Unsubscribe(Vec<String>),
}

impl SubscriptionChange {
    /// Returns true if this is a Subscribe variant.
    pub fn is_subscribe(&self) -> bool {
        matches!(self, SubscriptionChange::Subscribe(_))
    }

    /// Returns true if this is an Unsubscribe variant.
    pub fn is_unsubscribe(&self) -> bool {
        matches!(self, SubscriptionChange::Unsubscribe(_))
    }

    /// Returns the symbols contained in this change request.
    pub fn symbols(&self) -> &[String] {
        match self {
            SubscriptionChange::Subscribe(symbols) => symbols,
            SubscriptionChange::Unsubscribe(symbols) => symbols,
        }
    }

    /// Returns true if the symbol list is empty.
    ///
    /// Note: An empty subscription change (e.g., `Subscribe(vec![])`) is valid
    /// but may be ignored by `SubscriptionManager` as a no-op.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.symbols().is_empty()
    }

    /// Returns the number of symbols in this change request.
    #[must_use]
    pub fn len(&self) -> usize {
        self.symbols().len()
    }
}

/// Manages subscription state and channels for all exchanges.
///
/// `SubscriptionManager` is the central coordinator for runtime dynamic
/// market subscriptions. It tracks current subscriptions per exchange
/// and distributes subscription change requests through dedicated channels.
///
/// ## Architecture
///
/// - Each exchange has a dedicated `mpsc::Sender<SubscriptionChange>` channel
/// - Current subscriptions are tracked in a lock-free `DashMap`
/// - The `update_subscriptions` method calculates diffs and sends only new markets
///
/// ## Example
///
/// ```rust,ignore
/// use arbitrage_feeds::{SubscriptionManager, SubscriptionChange};
/// use arbitrage_core::Exchange;
///
/// let mut manager = SubscriptionManager::new();
/// let (tx, mut rx) = SubscriptionManager::create_channel();
/// manager.register_exchange(Exchange::Binance, tx);
///
/// // Update subscriptions - only sends diff
/// manager.update_subscriptions(Exchange::Binance, &["BTCUSDT".into(), "ETHUSDT".into()]).await;
/// ```
#[derive(Debug)]
pub struct SubscriptionManager {
    /// Channel senders for each exchange's WsClient
    senders: HashMap<Exchange, mpsc::Sender<SubscriptionChange>>,
    /// Current subscription state per exchange (lock-free)
    current_subscriptions: Arc<DashMap<Exchange, HashSet<String>>>,
}

impl SubscriptionManager {
    /// Create a new empty SubscriptionManager.
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
            current_subscriptions: Arc::new(DashMap::new()),
        }
    }

    /// Create a channel pair for subscription changes.
    ///
    /// Returns a `(Sender, Receiver)` pair with the default buffer size.
    /// The sender should be registered with `register_exchange()`,
    /// and the receiver should be passed to the WsClient.
    pub fn create_channel() -> (
        mpsc::Sender<SubscriptionChange>,
        mpsc::Receiver<SubscriptionChange>,
    ) {
        mpsc::channel(SUBSCRIPTION_CHANNEL_BUFFER)
    }

    /// Register an exchange with its subscription channel sender.
    ///
    /// This associates the exchange with a channel that will receive
    /// `SubscriptionChange` messages when new markets need to be subscribed.
    pub fn register_exchange(
        &mut self,
        exchange: Exchange,
        sender: mpsc::Sender<SubscriptionChange>,
    ) {
        self.senders.insert(exchange, sender);
        // Initialize empty subscription set for the exchange
        self.current_subscriptions.insert(exchange, HashSet::new());
    }

    /// Update subscriptions for an exchange with new markets.
    ///
    /// This method:
    /// 1. Calculates the diff between current subscriptions and new markets
    /// 2. Sends only the new markets (not already subscribed) to the channel
    /// 3. Updates the internal subscription tracking state
    ///
    /// Returns `Ok(usize)` with the count of newly subscribed markets,
    /// or `Err` if the channel send fails.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// // First call subscribes both
    /// manager.update_subscriptions(Exchange::Binance, &["BTCUSDT".into(), "ETHUSDT".into()]).await?;
    ///
    /// // Second call only subscribes SOLUSDT (BTC/ETH already subscribed)
    /// manager.update_subscriptions(Exchange::Binance, &["BTCUSDT".into(), "ETHUSDT".into(), "SOLUSDT".into()]).await?;
    /// ```
    pub async fn update_subscriptions(
        &self,
        exchange: Exchange,
        new_markets: &[String],
    ) -> Result<usize, SubscriptionError> {
        let new_set: HashSet<String> = new_markets.iter().cloned().collect();

        // Get current subscriptions or empty set
        let current = self
            .current_subscriptions
            .get(&exchange)
            .map(|r| r.value().clone())
            .unwrap_or_default();

        // Calculate diff: new_markets - current = to_subscribe
        let to_subscribe: Vec<String> = new_set.difference(&current).cloned().collect();

        let subscribed_count = to_subscribe.len();

        if !to_subscribe.is_empty() {
            if let Some(sender) = self.senders.get(&exchange) {
                sender
                    .send(SubscriptionChange::Subscribe(to_subscribe))
                    .await
                    .map_err(|e| SubscriptionError::ChannelSendError(e.to_string()))?;
            } else {
                return Err(SubscriptionError::ExchangeNotRegistered(exchange));
            }
        }

        // Update current subscriptions to include all new markets
        self.current_subscriptions.insert(exchange, new_set);

        Ok(subscribed_count)
    }

    /// Get the current subscriptions for an exchange.
    ///
    /// Returns `None` if the exchange is not registered.
    pub fn get_current_subscriptions(&self, exchange: Exchange) -> Option<HashSet<String>> {
        self.current_subscriptions
            .get(&exchange)
            .map(|r| r.value().clone())
    }

    /// Get the count of current subscriptions for an exchange.
    ///
    /// Returns 0 if the exchange is not registered.
    #[must_use]
    pub fn subscription_count(&self, exchange: Exchange) -> usize {
        self.current_subscriptions
            .get(&exchange)
            .map(|r| r.value().len())
            .unwrap_or(0)
    }

    /// Check if an exchange is registered.
    #[must_use]
    pub fn is_registered(&self, exchange: Exchange) -> bool {
        self.senders.contains_key(&exchange)
    }

    /// Get the number of registered exchanges.
    #[must_use]
    pub fn registered_exchange_count(&self) -> usize {
        self.senders.len()
    }

    /// Get a shared reference to the current subscriptions DashMap.
    ///
    /// This is useful for read-only access from multiple tasks.
    pub fn subscriptions(&self) -> Arc<DashMap<Exchange, HashSet<String>>> {
        Arc::clone(&self.current_subscriptions)
    }

    /// Resubscribe all current subscriptions for an exchange.
    ///
    /// This is used after a WebSocket reconnection to restore all subscriptions.
    /// Per NFR8, this ensures automatic re-subscription after connection drops.
    ///
    /// ## Returns
    /// - `Ok(count)` - Number of symbols resubscribed
    /// - `Err` - If the exchange is not registered or channel send fails
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// // After receiving WsMessage::Reconnected
    /// match manager.resubscribe_all(Exchange::Binance).await {
    ///     Ok(count) => info!("Resubscribed {} symbols", count),
    ///     Err(e) => error!("Resubscription failed: {}", e),
    /// }
    /// ```
    pub async fn resubscribe_all(&self, exchange: Exchange) -> Result<usize, SubscriptionError> {
        let current = self
            .current_subscriptions
            .get(&exchange)
            .map(|r| r.value().clone())
            .unwrap_or_default();

        let count = current.len();

        if count == 0 {
            return Ok(0);
        }

        let symbols: Vec<String> = current.into_iter().collect();

        if let Some(sender) = self.senders.get(&exchange) {
            sender
                .send(SubscriptionChange::Subscribe(symbols))
                .await
                .map_err(|e| SubscriptionError::ChannelSendError(e.to_string()))?;
        } else {
            return Err(SubscriptionError::ExchangeNotRegistered(exchange));
        }

        Ok(count)
    }

    /// Resubscribe all current subscriptions for all registered exchanges.
    ///
    /// This is useful for a global reconnection event.
    /// Returns a map of exchange to result (count or error).
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let results = manager.resubscribe_all_exchanges().await;
    /// for (exchange, result) in results {
    ///     match result {
    ///         Ok(count) => info!("{:?}: Resubscribed {} symbols", exchange, count),
    ///         Err(e) => error!("{:?}: Resubscription failed: {}", exchange, e),
    ///     }
    /// }
    /// ```
    pub async fn resubscribe_all_exchanges(
        &self,
    ) -> HashMap<Exchange, Result<usize, SubscriptionError>> {
        let mut results = HashMap::new();

        let exchanges: Vec<Exchange> = self.senders.keys().cloned().collect();

        for exchange in exchanges {
            let result = self.resubscribe_all(exchange).await;
            results.insert(exchange, result);
        }

        results
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for batch subscription processing.
///
/// When multiple markets are added simultaneously (e.g., new listing event),
/// this configuration controls how subscriptions are batched and rate-limited.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::BatchSubscriptionConfig;
///
/// // Default: 10 symbols per batch, 100ms delay
/// let config = BatchSubscriptionConfig::default();
///
/// // Custom: 5 symbols per batch, 200ms delay
/// let config = BatchSubscriptionConfig::new(5, 200);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BatchSubscriptionConfig {
    /// Maximum symbols per batch
    pub batch_size: usize,
    /// Delay between batches in milliseconds
    pub batch_delay_ms: u64,
}

impl BatchSubscriptionConfig {
    /// Create a new batch configuration.
    pub const fn new(batch_size: usize, batch_delay_ms: u64) -> Self {
        Self {
            batch_size,
            batch_delay_ms,
        }
    }

    /// Get the delay duration between batches.
    pub fn batch_delay(&self) -> Duration {
        Duration::from_millis(self.batch_delay_ms)
    }

    /// Calculate the number of batches needed for a given symbol count.
    pub fn batch_count(&self, symbol_count: usize) -> usize {
        if symbol_count == 0 {
            0
        } else {
            (symbol_count + self.batch_size - 1) / self.batch_size
        }
    }

    /// Estimate the total time to process all batches.
    pub fn estimated_duration(&self, symbol_count: usize) -> Duration {
        let batches = self.batch_count(symbol_count);
        if batches <= 1 {
            Duration::ZERO
        } else {
            Duration::from_millis(self.batch_delay_ms * (batches as u64 - 1))
        }
    }
}

impl Default for BatchSubscriptionConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            batch_delay_ms: 100,
        }
    }
}

/// Result of a batch subscription operation.
#[derive(Debug, Clone)]
pub struct BatchSubscriptionResult {
    /// Total symbols requested
    pub total_requested: usize,
    /// Successfully subscribed symbols
    pub subscribed: usize,
    /// Number of batches processed
    pub batches_processed: usize,
    /// Symbols that failed to subscribe (if any)
    pub failed: Vec<String>,
}

impl BatchSubscriptionResult {
    /// Create a successful result with all symbols subscribed.
    pub fn success(total: usize, batches: usize) -> Self {
        Self {
            total_requested: total,
            subscribed: total,
            batches_processed: batches,
            failed: Vec::new(),
        }
    }

    /// Check if all symbols were successfully subscribed.
    pub fn is_complete(&self) -> bool {
        self.subscribed == self.total_requested && self.failed.is_empty()
    }

    /// Get the success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.total_requested == 0 {
            100.0
        } else {
            (self.subscribed as f64 / self.total_requested as f64) * 100.0
        }
    }
}

impl SubscriptionManager {
    /// Subscribe to multiple symbols in batches with rate limiting.
    ///
    /// This method is optimized for bulk subscription requests, such as when
    /// multiple markets are listed simultaneously. It processes symbols in
    /// configurable batches with delays between each batch to respect rate limits.
    ///
    /// ## Arguments
    /// * `exchange` - The exchange to subscribe on
    /// * `symbols` - Symbols to subscribe
    /// * `config` - Batch processing configuration
    ///
    /// ## Returns
    /// * `Ok(BatchSubscriptionResult)` - Details of the batch operation
    /// * `Err(SubscriptionError)` - If the exchange is not registered
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let config = BatchSubscriptionConfig::new(5, 200);
    /// let symbols = vec!["BTCUSDT", "ETHUSDT", "SOLUSDT", /* ... 50 more */];
    ///
    /// let result = manager.subscribe_batch(Exchange::Binance, &symbols, config).await?;
    /// info!("Subscribed {}/{} in {} batches", result.subscribed, result.total_requested, result.batches_processed);
    /// ```
    pub async fn subscribe_batch(
        &mut self,
        exchange: Exchange,
        symbols: &[String],
        config: BatchSubscriptionConfig,
    ) -> Result<BatchSubscriptionResult, SubscriptionError> {
        if !self.is_registered(exchange) {
            return Err(SubscriptionError::ExchangeNotRegistered(exchange));
        }

        if symbols.is_empty() {
            return Ok(BatchSubscriptionResult::success(0, 0));
        }

        // Filter out already subscribed symbols
        let current = self
            .current_subscriptions
            .get(&exchange)
            .map(|r| r.value().clone())
            .unwrap_or_default();

        let to_subscribe: Vec<String> = symbols
            .iter()
            .filter(|s| !current.contains(*s))
            .cloned()
            .collect();

        if to_subscribe.is_empty() {
            return Ok(BatchSubscriptionResult::success(0, 0));
        }

        let total = to_subscribe.len();
        let mut subscribed = 0;
        let mut batches_processed = 0;

        // Process in batches
        for batch in to_subscribe.chunks(config.batch_size) {
            let batch_vec: Vec<String> = batch.to_vec();

            if let Some(sender) = self.senders.get(&exchange) {
                sender
                    .send(SubscriptionChange::Subscribe(batch_vec.clone()))
                    .await
                    .map_err(|e| SubscriptionError::ChannelSendError(e.to_string()))?;

                subscribed += batch_vec.len();
                batches_processed += 1;

                // Update current subscriptions
                self.current_subscriptions
                    .entry(exchange)
                    .or_default()
                    .extend(batch_vec);

                // Delay before next batch (except for last batch)
                if batches_processed < config.batch_count(total) {
                    tokio::time::sleep(config.batch_delay()).await;
                }
            }
        }

        Ok(BatchSubscriptionResult::success(
            subscribed,
            batches_processed,
        ))
    }

    /// Subscribe to symbols across multiple exchanges in batches.
    ///
    /// Processes each exchange sequentially, with batched subscriptions within each.
    /// This is useful when the same symbols need to be subscribed on multiple exchanges.
    ///
    /// ## Returns
    /// A map of exchange to batch subscription result.
    pub async fn subscribe_batch_multi(
        &mut self,
        requests: &[(Exchange, Vec<String>)],
        config: BatchSubscriptionConfig,
    ) -> HashMap<Exchange, Result<BatchSubscriptionResult, SubscriptionError>> {
        let mut results = HashMap::new();

        for (exchange, symbols) in requests {
            let result = self.subscribe_batch(*exchange, symbols, config).await;
            results.insert(*exchange, result);
        }

        results
    }
}

// ============================================================================
// Subscription Logging (Epic 4: 로깅 및 운영 가시성)
// ============================================================================

/// Subscription event types for structured logging.
///
/// This enum represents different subscription events that should be logged
/// for operational visibility per Epic 4 requirements.
#[derive(Debug, Clone)]
pub enum SubscriptionEventType {
    /// New market subscribed successfully (Story 4.1)
    Subscribed,
    /// Subscription failed (Story 4.2)
    Failed,
    /// Retry scheduled (Story 4.3)
    RetryScheduled,
    /// Max retries exceeded (Story 4.4)
    MaxRetriesExceeded,
    /// Unsubscribed from market
    Unsubscribed,
    /// Batch subscription completed
    BatchCompleted,
    /// Resubscription after reconnection
    Resubscribed,
}

/// Structured subscription event for logging.
///
/// Contains all relevant information for a subscription event,
/// designed to work with the tracing infrastructure (NFR12).
#[derive(Debug, Clone)]
pub struct SubscriptionEvent {
    /// Type of the event
    pub event_type: SubscriptionEventType,
    /// Exchange involved
    pub exchange: Exchange,
    /// Symbol(s) involved
    pub symbols: Vec<String>,
    /// Error reason (for failures)
    pub error: Option<String>,
    /// Retry attempt number (for retries)
    pub retry_attempt: Option<u32>,
    /// Delay until next retry (for retries)
    pub retry_delay_ms: Option<u64>,
    /// Batch information (for batch operations)
    pub batch_info: Option<(usize, usize)>, // (batch_num, total_batches)
}

impl SubscriptionEvent {
    /// Create a subscription success event.
    pub fn subscribed(exchange: Exchange, symbols: Vec<String>) -> Self {
        Self {
            event_type: SubscriptionEventType::Subscribed,
            exchange,
            symbols,
            error: None,
            retry_attempt: None,
            retry_delay_ms: None,
            batch_info: None,
        }
    }

    /// Create a subscription failure event.
    pub fn failed(exchange: Exchange, symbol: String, error: String) -> Self {
        Self {
            event_type: SubscriptionEventType::Failed,
            exchange,
            symbols: vec![symbol],
            error: Some(error),
            retry_attempt: None,
            retry_delay_ms: None,
            batch_info: None,
        }
    }

    /// Create a retry scheduled event.
    pub fn retry_scheduled(
        exchange: Exchange,
        symbol: String,
        attempt: u32,
        delay_ms: u64,
    ) -> Self {
        Self {
            event_type: SubscriptionEventType::RetryScheduled,
            exchange,
            symbols: vec![symbol],
            error: None,
            retry_attempt: Some(attempt),
            retry_delay_ms: Some(delay_ms),
            batch_info: None,
        }
    }

    /// Create a max retries exceeded event.
    pub fn max_retries_exceeded(exchange: Exchange, symbol: String, attempts: u32) -> Self {
        Self {
            event_type: SubscriptionEventType::MaxRetriesExceeded,
            exchange,
            symbols: vec![symbol],
            error: Some(format!("Max retries ({}) exceeded", attempts)),
            retry_attempt: Some(attempts),
            retry_delay_ms: None,
            batch_info: None,
        }
    }

    /// Create a batch completed event.
    pub fn batch_completed(
        exchange: Exchange,
        symbols: Vec<String>,
        batch_num: usize,
        total_batches: usize,
    ) -> Self {
        Self {
            event_type: SubscriptionEventType::BatchCompleted,
            exchange,
            symbols,
            error: None,
            retry_attempt: None,
            retry_delay_ms: None,
            batch_info: Some((batch_num, total_batches)),
        }
    }

    /// Create a resubscription event.
    pub fn resubscribed(exchange: Exchange, symbols: Vec<String>) -> Self {
        Self {
            event_type: SubscriptionEventType::Resubscribed,
            exchange,
            symbols,
            error: None,
            retry_attempt: None,
            retry_delay_ms: None,
            batch_info: None,
        }
    }

    /// Log this event using the tracing infrastructure.
    ///
    /// Implements the logging format requirements from Stories 4.1-4.4:
    /// - Story 4.1: `[INFO] New market subscribed: {symbol} on [{exchanges}]`
    /// - Story 4.2: `[WARN] Subscription failed for {symbol}: {error_reason}`
    /// - Story 4.3: `[INFO] Retry #{n} for {symbol} in {delay}s`
    /// - Story 4.4: `[ERROR] Max retries exceeded for {symbol} - manual intervention required`
    pub fn log(&self) {
        let symbols_str = if self.symbols.len() <= 3 {
            self.symbols.join(", ")
        } else {
            format!(
                "{}, ... (+{} more)",
                self.symbols[..3].join(", "),
                self.symbols.len() - 3
            )
        };

        match &self.event_type {
            SubscriptionEventType::Subscribed => {
                info!(
                    exchange = ?self.exchange,
                    symbols = %symbols_str,
                    count = self.symbols.len(),
                    "New market subscribed: {} on [{:?}]",
                    symbols_str,
                    self.exchange
                );
            }
            SubscriptionEventType::Failed => {
                warn!(
                    exchange = ?self.exchange,
                    symbol = %symbols_str,
                    error = ?self.error,
                    "Subscription failed for {}: {}",
                    symbols_str,
                    self.error.as_deref().unwrap_or("unknown error")
                );
            }
            SubscriptionEventType::RetryScheduled => {
                let delay_secs = self.retry_delay_ms.unwrap_or(0) as f64 / 1000.0;
                info!(
                    exchange = ?self.exchange,
                    symbol = %symbols_str,
                    attempt = self.retry_attempt.unwrap_or(0),
                    delay_secs = delay_secs,
                    "Retry #{} for {} in {:.1}s",
                    self.retry_attempt.unwrap_or(0),
                    symbols_str,
                    delay_secs
                );
            }
            SubscriptionEventType::MaxRetriesExceeded => {
                error!(
                    exchange = ?self.exchange,
                    symbol = %symbols_str,
                    attempts = self.retry_attempt.unwrap_or(0),
                    "Max retries exceeded for {} - manual intervention required",
                    symbols_str
                );
            }
            SubscriptionEventType::Unsubscribed => {
                info!(
                    exchange = ?self.exchange,
                    symbols = %symbols_str,
                    "Market unsubscribed: {} from [{:?}]",
                    symbols_str,
                    self.exchange
                );
            }
            SubscriptionEventType::BatchCompleted => {
                if let Some((batch_num, total)) = self.batch_info {
                    debug!(
                        exchange = ?self.exchange,
                        batch = batch_num,
                        total_batches = total,
                        symbols_count = self.symbols.len(),
                        "Batch {}/{} completed for {:?}",
                        batch_num,
                        total,
                        self.exchange
                    );
                }
            }
            SubscriptionEventType::Resubscribed => {
                info!(
                    exchange = ?self.exchange,
                    symbols = %symbols_str,
                    count = self.symbols.len(),
                    "Resubscribed {} markets on [{:?}] after reconnection",
                    self.symbols.len(),
                    self.exchange
                );
            }
        }
    }
}

/// Callback type for new market subscription events.
///
/// Called when a new market is successfully subscribed,
/// allowing integration with opportunity detection systems.
pub type NewMarketCallback = Box<dyn Fn(&str) + Send + Sync>;

/// Handler for new market subscription events.
///
/// This handler enables integration between the subscription system and
/// the opportunity detection pipeline (Story 5.1 & 5.2).
///
/// When a new market is subscribed, the callback is invoked to:
/// 1. Register the symbol with OpportunityDetector
/// 2. Verify price data reception (NFR3: < 10 seconds)
/// 3. Enable arbitrage opportunity detection
///
/// ## Example
///
/// ```rust,ignore
/// use arbitrage_feeds::NewMarketSubscriptionHandler;
/// use arbitrage_engine::OpportunityDetector;
/// use std::sync::Arc;
///
/// let detector = Arc::new(OpportunityDetector::new(config));
/// let detector_clone = Arc::clone(&detector);
///
/// let handler = NewMarketSubscriptionHandler::new(move |symbol| {
///     detector_clone.register_symbol(symbol);
///     info!("New market registered for detection: {}", symbol);
/// });
///
/// // Called by SubscriptionManager when new market subscribed
/// handler.on_market_subscribed("BTCUSDT");
/// ```
#[derive(Clone)]
pub struct NewMarketSubscriptionHandler {
    callback: Arc<NewMarketCallback>,
}

impl NewMarketSubscriptionHandler {
    /// Create a new handler with the given callback.
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(Box::new(callback)),
        }
    }

    /// Called when a new market is subscribed.
    pub fn on_market_subscribed(&self, symbol: &str) {
        (self.callback)(symbol);
    }

    /// Called when multiple markets are subscribed.
    pub fn on_markets_subscribed(&self, symbols: &[String]) {
        for symbol in symbols {
            (self.callback)(symbol);
        }
    }
}

impl std::fmt::Debug for NewMarketSubscriptionHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewMarketSubscriptionHandler")
            .field("callback", &"<callback>")
            .finish()
    }
}

/// Subscription logger for convenient logging operations.
///
/// Provides static methods for logging subscription events using
/// the tracing infrastructure.
pub struct SubscriptionLogger;

impl SubscriptionLogger {
    /// Log a successful subscription (Story 4.1).
    ///
    /// Format: `[INFO] New market subscribed: {symbol} on [{exchanges}]`
    pub fn log_subscribed(exchange: Exchange, symbols: &[String]) {
        SubscriptionEvent::subscribed(exchange, symbols.to_vec()).log();
    }

    /// Log a subscription failure (Story 4.2).
    ///
    /// Format: `[WARN] Subscription failed for {symbol}: {error_reason}`
    pub fn log_failed(exchange: Exchange, symbol: &str, error: &str) {
        SubscriptionEvent::failed(exchange, symbol.to_string(), error.to_string()).log();
    }

    /// Log a retry attempt (Story 4.3).
    ///
    /// Format: `[INFO] Retry #{n} for {symbol} in {delay}s`
    pub fn log_retry(exchange: Exchange, symbol: &str, attempt: u32, delay_ms: u64) {
        SubscriptionEvent::retry_scheduled(exchange, symbol.to_string(), attempt, delay_ms).log();
    }

    /// Log max retries exceeded (Story 4.4).
    ///
    /// Format: `[ERROR] Max retries exceeded for {symbol} - manual intervention required`
    pub fn log_max_retries_exceeded(exchange: Exchange, symbol: &str, attempts: u32) {
        SubscriptionEvent::max_retries_exceeded(exchange, symbol.to_string(), attempts).log();
    }

    /// Log a batch completion.
    pub fn log_batch_completed(
        exchange: Exchange,
        symbols: &[String],
        batch_num: usize,
        total_batches: usize,
    ) {
        SubscriptionEvent::batch_completed(exchange, symbols.to_vec(), batch_num, total_batches)
            .log();
    }

    /// Log a resubscription after reconnection.
    pub fn log_resubscribed(exchange: Exchange, symbols: &[String]) {
        SubscriptionEvent::resubscribed(exchange, symbols.to_vec()).log();
    }
}

/// Errors that can occur during subscription operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionError {
    /// The exchange is not registered with the manager.
    ExchangeNotRegistered(Exchange),
    /// Failed to send subscription change through the channel.
    ChannelSendError(String),
    /// Maximum retry attempts exceeded for a symbol on an exchange.
    /// Contains (exchange, symbol, attempt_count).
    MaxRetriesExceeded {
        exchange: Exchange,
        symbol: String,
        attempts: u32,
    },
    /// Subscription timed out waiting for confirmation.
    SubscriptionTimeout { exchange: Exchange, symbol: String },
}

impl std::fmt::Display for SubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionError::ExchangeNotRegistered(ex) => {
                write!(
                    f,
                    "Exchange {:?} is not registered with SubscriptionManager",
                    ex
                )
            }
            SubscriptionError::ChannelSendError(msg) => {
                write!(f, "Failed to send subscription change: {}", msg)
            }
            SubscriptionError::MaxRetriesExceeded {
                exchange,
                symbol,
                attempts,
            } => {
                write!(
                    f,
                    "Max retries exceeded for {} on {:?} after {} attempts - manual intervention required",
                    symbol, exchange, attempts
                )
            }
            SubscriptionError::SubscriptionTimeout { exchange, symbol } => {
                write!(f, "Subscription timeout for {} on {:?}", symbol, exchange)
            }
        }
    }
}

impl std::error::Error for SubscriptionError {}

/// Status of a subscription for a specific symbol on an exchange.
///
/// Used to track the health of individual subscriptions and implement
/// graceful degradation when subscriptions fail (NFR6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionStatus {
    /// Subscription is active and receiving data.
    Active,
    /// Subscription is pending (waiting for confirmation).
    Pending,
    /// Subscription is being retried after failure.
    Retrying { attempt: u32, next_retry_ms: u64 },
    /// Subscription has failed after max retries.
    /// The subscription is marked as failed but other exchanges continue (NFR6).
    Failed { attempts: u32, last_error: String },
}

impl SubscriptionStatus {
    /// Check if the subscription is in a healthy state.
    pub fn is_healthy(&self) -> bool {
        matches!(self, SubscriptionStatus::Active)
    }

    /// Check if the subscription has permanently failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, SubscriptionStatus::Failed { .. })
    }

    /// Check if the subscription is being retried.
    pub fn is_retrying(&self) -> bool {
        matches!(self, SubscriptionStatus::Retrying { .. })
    }
}

impl Default for SubscriptionStatus {
    fn default() -> Self {
        SubscriptionStatus::Pending
    }
}

/// Tracks subscription status per exchange per symbol.
///
/// This enables graceful degradation where a failed subscription on one
/// exchange doesn't affect other exchanges (NFR6).
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{ExchangeSubscriptionTracker, SubscriptionStatus};
/// use arbitrage_core::Exchange;
///
/// let mut tracker = ExchangeSubscriptionTracker::new();
///
/// // Mark subscription as active
/// tracker.set_active(Exchange::Binance, "BTCUSDT");
/// assert!(tracker.is_healthy(Exchange::Binance, "BTCUSDT"));
///
/// // Mark another exchange as failed - doesn't affect Binance
/// tracker.set_failed(Exchange::Coinbase, "BTC-USD", 5, "Connection refused");
/// assert!(tracker.is_healthy(Exchange::Binance, "BTCUSDT")); // Still healthy
/// assert!(tracker.is_failed(Exchange::Coinbase, "BTC-USD")); // Failed
/// ```
#[derive(Debug, Default)]
pub struct ExchangeSubscriptionTracker {
    /// Status per exchange per symbol
    statuses: DashMap<Exchange, HashMap<String, SubscriptionStatus>>,
    /// Retry states per exchange per symbol
    retry_states: DashMap<Exchange, HashMap<String, SubscriptionRetryState>>,
}

impl ExchangeSubscriptionTracker {
    /// Create a new tracker.
    pub fn new() -> Self {
        Self {
            statuses: DashMap::new(),
            retry_states: DashMap::new(),
        }
    }

    /// Set a subscription as active.
    pub fn set_active(&self, exchange: Exchange, symbol: &str) {
        self.statuses
            .entry(exchange)
            .or_default()
            .insert(symbol.to_string(), SubscriptionStatus::Active);

        // Reset retry state on success
        if let Some(mut states) = self.retry_states.get_mut(&exchange) {
            if let Some(state) = states.get_mut(symbol) {
                state.record_success();
            }
        }
    }

    /// Set a subscription as pending.
    pub fn set_pending(&self, exchange: Exchange, symbol: &str) {
        self.statuses
            .entry(exchange)
            .or_default()
            .insert(symbol.to_string(), SubscriptionStatus::Pending);
    }

    /// Record a failure and update retry state.
    /// Returns the updated retry state for determining next action.
    pub fn record_failure(
        &self,
        exchange: Exchange,
        symbol: &str,
        policy: &SubscriptionRetryPolicy,
    ) -> (SubscriptionRetryState, bool) {
        // Update retry state
        let mut retry_state = self
            .retry_states
            .entry(exchange)
            .or_default()
            .get(symbol)
            .cloned()
            .unwrap_or_default();

        retry_state.record_failure();
        let is_exhausted = retry_state.is_exhausted(policy);

        // Store updated state
        self.retry_states
            .entry(exchange)
            .or_default()
            .insert(symbol.to_string(), retry_state.clone());

        // Update status
        if is_exhausted {
            self.statuses.entry(exchange).or_default().insert(
                symbol.to_string(),
                SubscriptionStatus::Failed {
                    attempts: retry_state.attempt_count(),
                    last_error: "Max retries exceeded".to_string(),
                },
            );
        } else {
            let next_delay = policy.calculate_delay(retry_state.attempt_count());
            self.statuses.entry(exchange).or_default().insert(
                symbol.to_string(),
                SubscriptionStatus::Retrying {
                    attempt: retry_state.attempt_count(),
                    next_retry_ms: next_delay,
                },
            );
        }

        (retry_state, is_exhausted)
    }

    /// Set a subscription as failed with error message.
    pub fn set_failed(&self, exchange: Exchange, symbol: &str, attempts: u32, error: &str) {
        self.statuses.entry(exchange).or_default().insert(
            symbol.to_string(),
            SubscriptionStatus::Failed {
                attempts,
                last_error: error.to_string(),
            },
        );
    }

    /// Check if a subscription is healthy.
    pub fn is_healthy(&self, exchange: Exchange, symbol: &str) -> bool {
        self.statuses
            .get(&exchange)
            .and_then(|m| m.get(symbol).map(|s| s.is_healthy()))
            .unwrap_or(false)
    }

    /// Check if a subscription has failed.
    pub fn is_failed(&self, exchange: Exchange, symbol: &str) -> bool {
        self.statuses
            .get(&exchange)
            .and_then(|m| m.get(symbol).map(|s| s.is_failed()))
            .unwrap_or(false)
    }

    /// Get the status of a specific subscription.
    pub fn get_status(&self, exchange: Exchange, symbol: &str) -> Option<SubscriptionStatus> {
        self.statuses
            .get(&exchange)
            .and_then(|m| m.get(symbol).cloned())
    }

    /// Get all failed subscriptions across all exchanges.
    pub fn get_all_failed(&self) -> Vec<(Exchange, String, u32)> {
        let mut failed = Vec::new();
        for entry in self.statuses.iter() {
            let exchange = *entry.key();
            for (symbol, status) in entry.value().iter() {
                if let SubscriptionStatus::Failed { attempts, .. } = status {
                    failed.push((exchange, symbol.clone(), *attempts));
                }
            }
        }
        failed
    }

    /// Get count of healthy subscriptions for an exchange.
    pub fn healthy_count(&self, exchange: Exchange) -> usize {
        self.statuses
            .get(&exchange)
            .map(|m| m.values().filter(|s| s.is_healthy()).count())
            .unwrap_or(0)
    }

    /// Get count of failed subscriptions for an exchange.
    pub fn failed_count(&self, exchange: Exchange) -> usize {
        self.statuses
            .get(&exchange)
            .map(|m| m.values().filter(|s| s.is_failed()).count())
            .unwrap_or(0)
    }

    /// Calculate failure rate for an exchange (for NFR7 monitoring).
    /// Returns None if no subscriptions exist.
    pub fn failure_rate(&self, exchange: Exchange) -> Option<f64> {
        self.statuses.get(&exchange).map(|m| {
            let total = m.len();
            if total == 0 {
                return 0.0;
            }
            let failed = m.values().filter(|s| s.is_failed()).count();
            failed as f64 / total as f64
        })
    }

    /// Check if failure rate exceeds threshold (NFR7: < 1%).
    pub fn exceeds_failure_threshold(&self, exchange: Exchange, threshold: f64) -> bool {
        self.failure_rate(exchange)
            .map(|rate| rate > threshold)
            .unwrap_or(false)
    }
}

/// Rate limit configuration for an exchange.
///
/// Defines the maximum number of messages allowed within a time window
/// per NFR10 requirements.
#[derive(Debug, Clone, Copy)]
pub struct ExchangeRateLimit {
    /// Maximum messages per window
    pub max_messages: u32,
    /// Time window in milliseconds
    pub window_ms: u64,
    /// Minimum delay between messages in milliseconds
    pub min_delay_ms: u64,
}

impl ExchangeRateLimit {
    /// Create a new rate limit configuration.
    pub const fn new(max_messages: u32, window_ms: u64, min_delay_ms: u64) -> Self {
        Self {
            max_messages,
            window_ms,
            min_delay_ms,
        }
    }

    /// Get the rate limit for a specific exchange per NFR10.
    ///
    /// ## Exchange-specific limits:
    /// - **Binance**: 5 msg/sec (WebSocket subscription rate limit)
    /// - **Bybit**: 500 connections/5min, 10 subscriptions/sec recommended
    /// - **Coinbase**: No strict limit, but 100 msg/sec recommended
    /// - **GateIO**: 50 req/sec per channel
    /// - **Upbit**: 15 req/sec (conservative estimate)
    /// - **Bithumb**: 15 req/sec (conservative estimate, similar to Upbit)
    pub fn for_exchange(exchange: Exchange) -> Self {
        match exchange {
            Exchange::Binance => Self::new(5, 1000, 200), // 5 msg/sec, 200ms min delay
            Exchange::Bybit => Self::new(10, 1000, 100),  // 10 msg/sec, 100ms min delay
            Exchange::Coinbase => Self::new(50, 1000, 20), // 50 msg/sec, 20ms min delay
            Exchange::GateIO => Self::new(50, 1000, 20),  // 50 msg/sec, 20ms min delay
            Exchange::Upbit => Self::new(15, 1000, 67),   // 15 msg/sec, ~67ms min delay
            Exchange::Bithumb => Self::new(15, 1000, 67), // 15 msg/sec, ~67ms min delay
            _ => Self::new(10, 1000, 100),                // Default: 10 msg/sec
        }
    }

    /// Calculate the minimum time to wait between messages.
    pub fn min_delay(&self) -> Duration {
        Duration::from_millis(self.min_delay_ms)
    }

    /// Calculate the time window duration.
    pub fn window(&self) -> Duration {
        Duration::from_millis(self.window_ms)
    }
}

/// Token bucket rate limiter for subscription requests.
///
/// Implements a simple token bucket algorithm to enforce rate limits.
/// Tokens are replenished over time and consumed when sending messages.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{SubscriptionRateLimiter, ExchangeRateLimit};
/// use arbitrage_core::Exchange;
///
/// let rate_limit = ExchangeRateLimit::for_exchange(Exchange::Binance);
/// let mut limiter = SubscriptionRateLimiter::new(rate_limit);
///
/// // Check if we can send
/// if limiter.try_acquire() {
///     // Send the message
///     println!("Sending message");
/// } else {
///     // Wait before sending
///     let wait = limiter.time_until_available();
///     println!("Must wait {:?}", wait);
/// }
/// ```
#[derive(Debug)]
pub struct SubscriptionRateLimiter {
    /// Rate limit configuration
    config: ExchangeRateLimit,
    /// Tokens available (messages that can be sent)
    tokens: f64,
    /// Last time tokens were updated
    last_update: Instant,
    /// Last time a message was sent
    last_send: Option<Instant>,
}

impl SubscriptionRateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: ExchangeRateLimit) -> Self {
        Self {
            tokens: config.max_messages as f64,
            last_update: Instant::now(),
            last_send: None,
            config,
        }
    }

    /// Create a rate limiter for a specific exchange.
    pub fn for_exchange(exchange: Exchange) -> Self {
        Self::new(ExchangeRateLimit::for_exchange(exchange))
    }

    /// Replenish tokens based on elapsed time.
    fn replenish(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);

        // Calculate tokens to add based on elapsed time
        let window_ms = self.config.window_ms as f64;
        let elapsed_ms = elapsed.as_millis() as f64;
        let tokens_to_add = (elapsed_ms / window_ms) * self.config.max_messages as f64;

        self.tokens = (self.tokens + tokens_to_add).min(self.config.max_messages as f64);
        self.last_update = now;
    }

    /// Try to acquire a token for sending a message.
    ///
    /// Returns `true` if a token was acquired, `false` if rate limited.
    pub fn try_acquire(&mut self) -> bool {
        self.replenish();

        // Also check minimum delay since last send
        if let Some(last) = self.last_send {
            if last.elapsed() < Duration::from_millis(self.config.min_delay_ms) {
                return false;
            }
        }

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            self.last_send = Some(Instant::now());
            true
        } else {
            false
        }
    }

    /// Get the time until a token becomes available.
    ///
    /// Returns `Duration::ZERO` if a token is available now.
    pub fn time_until_available(&mut self) -> Duration {
        self.replenish();

        // Check minimum delay first
        if let Some(last) = self.last_send {
            let since_last = last.elapsed();
            let min_delay = Duration::from_millis(self.config.min_delay_ms);
            if since_last < min_delay {
                return min_delay - since_last;
            }
        }

        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            // Calculate time needed to replenish 1 token
            let tokens_needed = 1.0 - self.tokens;
            let window_ms = self.config.window_ms as f64;
            let time_per_token = window_ms / self.config.max_messages as f64;
            let ms_needed = (tokens_needed * time_per_token) as u64;
            Duration::from_millis(ms_needed)
        }
    }

    /// Wait until a token is available (async).
    pub async fn acquire(&mut self) {
        loop {
            let wait = self.time_until_available();
            if wait.is_zero() {
                if self.try_acquire() {
                    return;
                }
            }
            tokio::time::sleep(wait.max(Duration::from_millis(1))).await;
        }
    }

    /// Get the current token count (for testing/monitoring).
    pub fn available_tokens(&mut self) -> f64 {
        self.replenish();
        self.tokens
    }

    /// Get the rate limit configuration.
    pub fn config(&self) -> &ExchangeRateLimit {
        &self.config
    }

    /// Reset the rate limiter to full capacity.
    pub fn reset(&mut self) {
        self.tokens = self.config.max_messages as f64;
        self.last_update = Instant::now();
        self.last_send = None;
    }
}

/// Configuration for subscription retry policy with exponential backoff.
///
/// This struct defines the retry behavior for subscription failures,
/// implementing exponential backoff with configurable parameters per NFR2.
///
/// ## Default Values
/// - Initial delay: 2 seconds (NFR2 requirement)
/// - Maximum delay: 5 minutes (300 seconds, NFR2 requirement)
/// - Maximum retry attempts: 5
/// - Jitter: 0-25% of base delay to prevent thundering herd
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::SubscriptionRetryPolicy;
///
/// let policy = SubscriptionRetryPolicy::default();
/// assert_eq!(policy.initial_delay_ms(), 2000);
/// assert_eq!(policy.max_delay_ms(), 300_000);
///
/// // Calculate delays for successive retries
/// let delay1 = policy.calculate_delay(1); // ~2000ms + jitter
/// let delay2 = policy.calculate_delay(2); // ~4000ms + jitter
/// let delay3 = policy.calculate_delay(3); // ~8000ms + jitter
/// ```
#[derive(Debug, Clone)]
pub struct SubscriptionRetryPolicy {
    /// Initial delay in milliseconds (default: 2000ms = 2 seconds)
    initial_delay_ms: u64,
    /// Maximum delay in milliseconds (default: 300_000ms = 5 minutes)
    max_delay_ms: u64,
    /// Maximum number of retry attempts (default: 5)
    max_retries: u32,
    /// Whether to add jitter to delay (default: true)
    jitter_enabled: bool,
}

impl SubscriptionRetryPolicy {
    /// Create a new retry policy with custom parameters.
    ///
    /// # Arguments
    /// * `initial_delay_ms` - Initial delay in milliseconds
    /// * `max_delay_ms` - Maximum delay cap in milliseconds
    /// * `max_retries` - Maximum number of retry attempts
    pub fn new(initial_delay_ms: u64, max_delay_ms: u64, max_retries: u32) -> Self {
        Self {
            initial_delay_ms,
            max_delay_ms,
            max_retries,
            jitter_enabled: true,
        }
    }

    /// Create a policy with jitter disabled (useful for testing).
    pub fn without_jitter(mut self) -> Self {
        self.jitter_enabled = false;
        self
    }

    /// Get the initial delay in milliseconds.
    pub fn initial_delay_ms(&self) -> u64 {
        self.initial_delay_ms
    }

    /// Get the maximum delay in milliseconds.
    pub fn max_delay_ms(&self) -> u64 {
        self.max_delay_ms
    }

    /// Get the maximum number of retry attempts.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Check if jitter is enabled.
    pub fn jitter_enabled(&self) -> bool {
        self.jitter_enabled
    }

    /// Calculate the delay for a given retry attempt using exponential backoff.
    ///
    /// The delay doubles with each attempt: 2s → 4s → 8s → 16s → ...
    /// A random jitter of 0-25% is added to prevent thundering herd.
    ///
    /// # Arguments
    /// * `attempt` - The retry attempt number (1-based)
    ///
    /// # Returns
    /// Delay in milliseconds, capped at `max_delay_ms`
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        // Cap the power to prevent overflow (2^8 = 256x is plenty)
        let backoff_power = attempt.saturating_sub(1).min(8);
        let exponential = self.initial_delay_ms.saturating_mul(1 << backoff_power);
        let capped = exponential.min(self.max_delay_ms);

        if self.jitter_enabled {
            // Add 0-25% random jitter
            let jitter = (capped as f64 * rand::thread_rng().gen::<f64>() * 0.25) as u64;
            capped + jitter
        } else {
            capped
        }
    }

    /// Calculate the delay as a Duration.
    pub fn calculate_delay_duration(&self, attempt: u32) -> Duration {
        Duration::from_millis(self.calculate_delay(attempt))
    }

    /// Check if retry should continue based on attempt count.
    ///
    /// Returns `true` if `attempt <= max_retries`.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt <= self.max_retries
    }
}

impl Default for SubscriptionRetryPolicy {
    /// Create default policy per NFR2 requirements:
    /// - Initial delay: 2 seconds
    /// - Maximum delay: 5 minutes
    /// - Maximum retries: 5
    fn default() -> Self {
        Self {
            initial_delay_ms: 2_000, // 2 seconds
            max_delay_ms: 300_000,   // 5 minutes
            max_retries: 5,
            jitter_enabled: true,
        }
    }
}

/// State tracker for subscription retry attempts.
///
/// This struct tracks the retry state for a specific symbol or exchange,
/// using the configured `SubscriptionRetryPolicy` to determine delays.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{SubscriptionRetryPolicy, SubscriptionRetryState};
///
/// let policy = SubscriptionRetryPolicy::default();
/// let mut state = SubscriptionRetryState::new();
///
/// // First failure
/// state.record_failure();
/// assert_eq!(state.attempt_count(), 1);
/// assert!(state.should_retry(&policy));
///
/// // Get delay for next retry
/// let delay = state.next_delay(&policy);
/// println!("Retry in {:?}", delay);
///
/// // After success, reset state
/// state.record_success();
/// assert_eq!(state.attempt_count(), 0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct SubscriptionRetryState {
    /// Current retry attempt count
    attempt_count: u32,
    /// Total failures recorded (for metrics)
    total_failures: u64,
}

impl SubscriptionRetryState {
    /// Create a new retry state with zero attempts.
    pub fn new() -> Self {
        Self {
            attempt_count: 0,
            total_failures: 0,
        }
    }

    /// Record a subscription failure, incrementing the attempt count.
    pub fn record_failure(&mut self) {
        self.attempt_count = self.attempt_count.saturating_add(1);
        self.total_failures = self.total_failures.saturating_add(1);
    }

    /// Record a subscription success, resetting the attempt count.
    pub fn record_success(&mut self) {
        self.attempt_count = 0;
    }

    /// Get the current attempt count.
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }

    /// Get the total number of failures recorded.
    pub fn total_failures(&self) -> u64 {
        self.total_failures
    }

    /// Check if retry should continue based on policy.
    pub fn should_retry(&self, policy: &SubscriptionRetryPolicy) -> bool {
        policy.should_retry(self.attempt_count)
    }

    /// Calculate the next retry delay based on policy and current attempt.
    pub fn next_delay(&self, policy: &SubscriptionRetryPolicy) -> Duration {
        policy.calculate_delay_duration(self.attempt_count)
    }

    /// Check if max retries have been exceeded.
    pub fn is_exhausted(&self, policy: &SubscriptionRetryPolicy) -> bool {
        self.attempt_count > policy.max_retries()
    }
}

/// Binance subscription message builder.
///
/// Builds WebSocket subscription messages for Binance depth20@100ms streams.
/// Each call to `build_subscribe_message` generates a unique message ID.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{BinanceSubscriptionBuilder, SubscriptionBuilder};
///
/// let builder = BinanceSubscriptionBuilder::new();
/// let msg = builder.build_subscribe_message(&["BTCUSDT".to_string(), "ETHUSDT".to_string()]);
/// // Produces: {"method":"SUBSCRIBE","params":["btcusdt@depth20@100ms","ethusdt@depth20@100ms"],"id":1}
/// ```
#[derive(Debug)]
pub struct BinanceSubscriptionBuilder {
    id_counter: AtomicU32,
}

impl BinanceSubscriptionBuilder {
    /// Create a new BinanceSubscriptionBuilder.
    pub fn new() -> Self {
        Self {
            id_counter: AtomicU32::new(1),
        }
    }
}

impl Default for BinanceSubscriptionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionBuilder for BinanceSubscriptionBuilder {
    fn build_subscribe_message(&self, symbols: &[String]) -> String {
        let id = self.id_counter.fetch_add(1, Ordering::Relaxed);
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("\"{}@depth20@100ms\"", s.to_lowercase()))
            .collect();
        format!(
            r#"{{"method":"SUBSCRIBE","params":[{}],"id":{}}}"#,
            streams.join(","),
            id
        )
    }
}

/// Coinbase subscription message builder.
///
/// Builds WebSocket subscription messages for Coinbase level2 and heartbeats channels.
/// Uses the standard Coinbase WebSocket protocol format.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{CoinbaseSubscriptionBuilder, SubscriptionBuilder};
///
/// let builder = CoinbaseSubscriptionBuilder::new();
/// let msg = builder.build_subscribe_message(&["BTC-USD".to_string(), "ETH-USD".to_string()]);
/// // Produces: {"type": "subscribe", "product_ids": ["BTC-USD", "ETH-USD"], "channels": ["level2", "heartbeats"]}
/// ```
#[derive(Debug)]
pub struct CoinbaseSubscriptionBuilder;

impl CoinbaseSubscriptionBuilder {
    /// Create a new CoinbaseSubscriptionBuilder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CoinbaseSubscriptionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionBuilder for CoinbaseSubscriptionBuilder {
    fn build_subscribe_message(&self, symbols: &[String]) -> String {
        let products: Vec<String> = symbols.iter().map(|s| format!("\"{}\"", s)).collect();
        format!(
            r#"{{"type": "subscribe", "product_ids": [{}], "channels": ["level2", "heartbeats"]}}"#,
            products.join(", ")
        )
    }
}

/// Bybit subscription message builder.
///
/// Builds WebSocket subscription messages for Bybit orderbook.50 streams.
/// Symbols are converted to uppercase per Bybit API requirements.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{BybitSubscriptionBuilder, SubscriptionBuilder};
///
/// let builder = BybitSubscriptionBuilder::new();
/// let msg = builder.build_subscribe_message(&["BTCUSDT".to_string(), "ETHUSDT".to_string()]);
/// // Produces: {"op": "subscribe", "args": ["orderbook.50.BTCUSDT", "orderbook.50.ETHUSDT"]}
/// ```
#[derive(Debug)]
pub struct BybitSubscriptionBuilder;

impl BybitSubscriptionBuilder {
    /// Create a new BybitSubscriptionBuilder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for BybitSubscriptionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionBuilder for BybitSubscriptionBuilder {
    fn build_subscribe_message(&self, symbols: &[String]) -> String {
        let topics: Vec<String> = symbols
            .iter()
            .map(|s| format!("\"orderbook.50.{}\"", s.to_uppercase()))
            .collect();
        format!(r#"{{"op": "subscribe", "args": [{}]}}"#, topics.join(", "))
    }
}

/// GateIO subscription message builder.
///
/// Builds WebSocket subscription messages for GateIO spot.obu (orderbook update) channel.
/// Uses the standard Gate.io WebSocket API v4 format with timestamp.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{GateIOSubscriptionBuilder, SubscriptionBuilder};
///
/// let builder = GateIOSubscriptionBuilder::new();
/// let msg = builder.build_subscribe_message(&["BTC_USDT".to_string(), "ETH_USDT".to_string()]);
/// // Produces: {"time": <timestamp>, "channel": "spot.obu", "event": "subscribe", "payload": ["ob.BTC_USDT.50", "ob.ETH_USDT.50"]}
/// ```
#[derive(Debug)]
pub struct GateIOSubscriptionBuilder;

impl GateIOSubscriptionBuilder {
    /// Create a new GateIOSubscriptionBuilder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for GateIOSubscriptionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionBuilder for GateIOSubscriptionBuilder {
    fn build_subscribe_message(&self, symbols: &[String]) -> String {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let payloads: Vec<String> = symbols
            .iter()
            .map(|s| format!("\"ob.{}.50\"", s.to_uppercase()))
            .collect();
        format!(
            r#"{{"time": {}, "channel": "spot.obu", "event": "subscribe", "payload": [{}]}}"#,
            timestamp,
            payloads.join(", ")
        )
    }
}

/// Upbit subscription message builder.
///
/// Builds WebSocket subscription messages for Upbit ticker and orderbook channels.
/// **Note:** Upbit requires full subscription list on each message (no delta subscriptions).
/// When new symbols are added, the entire current subscription list must be sent.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{UpbitSubscriptionBuilder, SubscriptionBuilder};
///
/// let builder = UpbitSubscriptionBuilder::new();
/// let msg = builder.build_subscribe_message(&["KRW-BTC".to_string(), "KRW-ETH".to_string()]);
/// // Produces: [{"ticket":"arbitrage-bot"},{"type":"ticker","codes":["KRW-BTC","KRW-ETH"]},{"type":"orderbook","codes":["KRW-BTC","KRW-ETH"],"level":0},{"format":"SIMPLE"}]
/// ```
#[derive(Debug)]
pub struct UpbitSubscriptionBuilder;

impl UpbitSubscriptionBuilder {
    /// Create a new UpbitSubscriptionBuilder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for UpbitSubscriptionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionBuilder for UpbitSubscriptionBuilder {
    fn build_subscribe_message(&self, symbols: &[String]) -> String {
        let codes: Vec<String> = symbols.iter().map(|m| format!("\"{}\"", m)).collect();
        let codes_str = codes.join(",");

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"ticker","codes":[{}]}},{{"type":"orderbook","codes":[{}],"level":0}},{{"format":"SIMPLE"}}]"#,
            codes_str, codes_str
        )
    }
}

/// Bithumb subscription message builder.
///
/// Builds WebSocket subscription messages for Bithumb ticker and orderbook channels.
/// **Note:** Bithumb requires full subscription list on each message (no delta subscriptions).
/// Similar to Upbit but uses orderbook level=1 instead of level=0.
///
/// ## Example
///
/// ```rust
/// use arbitrage_feeds::{BithumbSubscriptionBuilder, SubscriptionBuilder};
///
/// let builder = BithumbSubscriptionBuilder::new();
/// let msg = builder.build_subscribe_message(&["KRW-BTC".to_string(), "KRW-ETH".to_string()]);
/// // Produces: [{"ticket":"arbitrage-bot"},{"type":"ticker","codes":["KRW-BTC","KRW-ETH"]},{"type":"orderbook","codes":["KRW-BTC","KRW-ETH"],"level":1},{"format":"SIMPLE"}]
/// ```
#[derive(Debug)]
pub struct BithumbSubscriptionBuilder;

impl BithumbSubscriptionBuilder {
    /// Create a new BithumbSubscriptionBuilder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for BithumbSubscriptionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionBuilder for BithumbSubscriptionBuilder {
    fn build_subscribe_message(&self, symbols: &[String]) -> String {
        let codes: Vec<String> = symbols.iter().map(|m| format!("\"{}\"", m)).collect();
        let codes_str = codes.join(",");

        format!(
            r#"[{{"ticket":"arbitrage-bot"}},{{"type":"ticker","codes":[{}]}},{{"type":"orderbook","codes":[{}],"level":1}},{{"format":"SIMPLE"}}]"#,
            codes_str, codes_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_change_subscribe() {
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let change = SubscriptionChange::Subscribe(symbols.clone());

        assert!(change.is_subscribe());
        assert!(!change.is_unsubscribe());
        assert_eq!(change.symbols(), &symbols);
        assert_eq!(change.len(), 2);
        assert!(!change.is_empty());

        // Pattern matching test
        if let SubscriptionChange::Subscribe(s) = change {
            assert_eq!(s, symbols);
        } else {
            panic!("Expected Subscribe variant");
        }
    }

    #[test]
    fn test_subscription_change_unsubscribe() {
        let symbols = vec!["XRPUSDT".to_string()];
        let change = SubscriptionChange::Unsubscribe(symbols.clone());

        assert!(!change.is_subscribe());
        assert!(change.is_unsubscribe());
        assert_eq!(change.symbols(), &symbols);
        assert_eq!(change.len(), 1);
        assert!(!change.is_empty());

        // Pattern matching test
        if let SubscriptionChange::Unsubscribe(s) = change {
            assert_eq!(s, symbols);
        } else {
            panic!("Expected Unsubscribe variant");
        }
    }

    #[test]
    fn test_subscription_change_clone() {
        let original = SubscriptionChange::Subscribe(vec!["BTCUSDT".to_string()]);
        let cloned = original.clone();

        assert!(cloned.is_subscribe());
        assert_eq!(cloned.symbols(), original.symbols());
        // PartialEq allows direct comparison
        assert_eq!(cloned, original);
    }

    #[test]
    fn test_subscription_change_equality() {
        let a = SubscriptionChange::Subscribe(vec!["BTCUSDT".to_string()]);
        let b = SubscriptionChange::Subscribe(vec!["BTCUSDT".to_string()]);
        let c = SubscriptionChange::Subscribe(vec!["ETHUSDT".to_string()]);
        let d = SubscriptionChange::Unsubscribe(vec!["BTCUSDT".to_string()]);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn test_subscription_change_empty_vector() {
        let empty_subscribe = SubscriptionChange::Subscribe(vec![]);
        assert!(empty_subscribe.is_empty());
        assert_eq!(empty_subscribe.len(), 0);

        let empty_unsubscribe = SubscriptionChange::Unsubscribe(vec![]);
        assert!(empty_unsubscribe.is_empty());
        assert_eq!(empty_unsubscribe.len(), 0);
    }

    #[test]
    fn test_subscription_change_debug() {
        let change = SubscriptionChange::Subscribe(vec!["BTCUSDT".to_string()]);
        let debug_str = format!("{:?}", change);
        assert!(debug_str.contains("Subscribe"));
        assert!(debug_str.contains("BTCUSDT"));
    }

    // ========== SubscriptionManager Tests ==========

    #[test]
    fn test_subscription_manager_new() {
        let manager = SubscriptionManager::new();
        assert_eq!(manager.registered_exchange_count(), 0);
    }

    #[test]
    fn test_subscription_manager_default() {
        let manager = SubscriptionManager::default();
        assert_eq!(manager.registered_exchange_count(), 0);
    }

    #[test]
    fn test_subscription_manager_register_exchange() {
        let mut manager = SubscriptionManager::new();
        let (tx, _rx) = SubscriptionManager::create_channel();

        manager.register_exchange(Exchange::Binance, tx);

        assert!(manager.is_registered(Exchange::Binance));
        assert!(!manager.is_registered(Exchange::Coinbase));
        assert_eq!(manager.registered_exchange_count(), 1);
        assert_eq!(manager.subscription_count(Exchange::Binance), 0);
    }

    #[test]
    fn test_subscription_manager_register_multiple_exchanges() {
        let mut manager = SubscriptionManager::new();

        let (tx1, _rx1) = SubscriptionManager::create_channel();
        let (tx2, _rx2) = SubscriptionManager::create_channel();
        let (tx3, _rx3) = SubscriptionManager::create_channel();

        manager.register_exchange(Exchange::Binance, tx1);
        manager.register_exchange(Exchange::Coinbase, tx2);
        manager.register_exchange(Exchange::Upbit, tx3);

        assert!(manager.is_registered(Exchange::Binance));
        assert!(manager.is_registered(Exchange::Coinbase));
        assert!(manager.is_registered(Exchange::Upbit));
        assert!(!manager.is_registered(Exchange::Bybit));
        assert_eq!(manager.registered_exchange_count(), 3);
    }

    #[tokio::test]
    async fn test_subscription_manager_update_subscriptions_new_markets() {
        let mut manager = SubscriptionManager::new();
        let (tx, mut rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        let markets = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let result = manager
            .update_subscriptions(Exchange::Binance, &markets)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // 2 new markets subscribed

        // Verify channel received the subscription change
        let received = rx.try_recv();
        assert!(received.is_ok());

        let change = received.unwrap();
        assert!(change.is_subscribe());
        assert_eq!(change.len(), 2);

        // Verify current subscriptions updated
        let current = manager
            .get_current_subscriptions(Exchange::Binance)
            .unwrap();
        assert!(current.contains("BTCUSDT"));
        assert!(current.contains("ETHUSDT"));
        assert_eq!(manager.subscription_count(Exchange::Binance), 2);
    }

    #[tokio::test]
    async fn test_subscription_manager_update_subscriptions_diff_only() {
        let mut manager = SubscriptionManager::new();
        let (tx, mut rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        // First update: subscribe to BTC and ETH
        let markets1 = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let result1 = manager
            .update_subscriptions(Exchange::Binance, &markets1)
            .await;
        assert_eq!(result1.unwrap(), 2);
        let _ = rx.try_recv(); // Consume first message

        // Second update: add SOL (BTC and ETH already subscribed)
        let markets2 = vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "SOLUSDT".to_string(),
        ];
        let result2 = manager
            .update_subscriptions(Exchange::Binance, &markets2)
            .await;
        assert_eq!(result2.unwrap(), 1); // Only SOL is new

        // Verify only SOL was sent
        let received = rx.try_recv().unwrap();
        assert!(received.is_subscribe());
        assert_eq!(received.len(), 1);
        assert!(received.symbols().contains(&"SOLUSDT".to_string()));

        // Verify current subscriptions include all 3
        assert_eq!(manager.subscription_count(Exchange::Binance), 3);
    }

    #[tokio::test]
    async fn test_subscription_manager_update_subscriptions_no_new_markets() {
        let mut manager = SubscriptionManager::new();
        let (tx, mut rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        // First update
        let markets = vec!["BTCUSDT".to_string()];
        let _ = manager
            .update_subscriptions(Exchange::Binance, &markets)
            .await;
        let _ = rx.try_recv(); // Consume first message

        // Second update with same markets - no new subscriptions
        let result = manager
            .update_subscriptions(Exchange::Binance, &markets)
            .await;
        assert_eq!(result.unwrap(), 0);

        // No message should be sent (nothing new to subscribe)
        let received = rx.try_recv();
        assert!(received.is_err()); // Channel should be empty
    }

    #[tokio::test]
    async fn test_subscription_manager_update_unregistered_exchange() {
        let manager = SubscriptionManager::new();
        // Don't register any exchange

        let markets = vec!["BTCUSDT".to_string()];
        let result = manager
            .update_subscriptions(Exchange::Binance, &markets)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SubscriptionError::ExchangeNotRegistered(ex) => {
                assert_eq!(ex, Exchange::Binance);
            }
            _ => panic!("Expected ExchangeNotRegistered error"),
        }
    }

    #[tokio::test]
    async fn test_subscription_manager_multi_exchange_independence() {
        let mut manager = SubscriptionManager::new();
        let (tx1, mut rx1) = SubscriptionManager::create_channel();
        let (tx2, mut rx2) = SubscriptionManager::create_channel();

        manager.register_exchange(Exchange::Binance, tx1);
        manager.register_exchange(Exchange::Upbit, tx2);

        // Subscribe different markets to different exchanges
        let binance_markets = vec!["BTCUSDT".to_string()];
        let upbit_markets = vec!["KRW-BTC".to_string(), "KRW-ETH".to_string()];

        let _ = manager
            .update_subscriptions(Exchange::Binance, &binance_markets)
            .await;
        let _ = manager
            .update_subscriptions(Exchange::Upbit, &upbit_markets)
            .await;

        // Verify each exchange received its own subscriptions
        let binance_msg = rx1.try_recv().unwrap();
        assert_eq!(binance_msg.len(), 1);
        assert!(binance_msg.symbols().contains(&"BTCUSDT".to_string()));

        let upbit_msg = rx2.try_recv().unwrap();
        assert_eq!(upbit_msg.len(), 2);
        assert!(upbit_msg.symbols().contains(&"KRW-BTC".to_string()));

        // Verify subscription counts are independent
        assert_eq!(manager.subscription_count(Exchange::Binance), 1);
        assert_eq!(manager.subscription_count(Exchange::Upbit), 2);
    }

    #[test]
    fn test_subscription_manager_get_current_subscriptions_unregistered() {
        let manager = SubscriptionManager::new();
        let result = manager.get_current_subscriptions(Exchange::Binance);
        assert!(result.is_none());
    }

    #[test]
    fn test_subscription_manager_subscriptions_arc() {
        let mut manager = SubscriptionManager::new();
        let (tx, _rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        let subs1 = manager.subscriptions();
        let subs2 = manager.subscriptions();

        // Both should point to the same underlying DashMap
        assert!(Arc::ptr_eq(&subs1, &subs2));
    }

    #[tokio::test]
    async fn test_subscription_manager_resubscribe_all_nfr8() {
        // NFR8: Automatic re-subscription after connection drop
        let mut manager = SubscriptionManager::new();
        let (tx, mut rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        // First, establish some subscriptions
        let markets = vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "SOLUSDT".to_string(),
        ];
        manager
            .update_subscriptions(Exchange::Binance, &markets)
            .await
            .unwrap();
        let _ = rx.try_recv(); // Consume initial subscription

        // Simulate reconnection - resubscribe all
        let result = manager.resubscribe_all(Exchange::Binance).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3); // 3 symbols resubscribed

        // Verify resubscription message was sent
        let received = rx.try_recv().unwrap();
        assert!(received.is_subscribe());
        assert_eq!(received.len(), 3);
    }

    #[tokio::test]
    async fn test_subscription_manager_resubscribe_all_empty() {
        let mut manager = SubscriptionManager::new();
        let (tx, mut rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        // No subscriptions yet
        let result = manager.resubscribe_all(Exchange::Binance).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // No message should be sent
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_subscription_manager_resubscribe_all_unregistered() {
        let manager = SubscriptionManager::new();

        let result = manager.resubscribe_all(Exchange::Binance).await;

        assert!(result.is_ok()); // Returns Ok(0) for unregistered exchange with no subscriptions
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_subscription_manager_resubscribe_all_exchanges() {
        let mut manager = SubscriptionManager::new();
        let (tx1, mut rx1) = SubscriptionManager::create_channel();
        let (tx2, mut rx2) = SubscriptionManager::create_channel();

        manager.register_exchange(Exchange::Binance, tx1);
        manager.register_exchange(Exchange::Coinbase, tx2);

        // Subscribe to both exchanges
        manager
            .update_subscriptions(Exchange::Binance, &["BTCUSDT".to_string()])
            .await
            .unwrap();
        manager
            .update_subscriptions(
                Exchange::Coinbase,
                &["BTC-USD".to_string(), "ETH-USD".to_string()],
            )
            .await
            .unwrap();

        let _ = rx1.try_recv(); // Consume initial
        let _ = rx2.try_recv(); // Consume initial

        // Resubscribe all exchanges
        let results = manager.resubscribe_all_exchanges().await;

        assert_eq!(results.len(), 2);
        assert_eq!(
            *results.get(&Exchange::Binance).unwrap().as_ref().unwrap(),
            1
        );
        assert_eq!(
            *results.get(&Exchange::Coinbase).unwrap().as_ref().unwrap(),
            2
        );

        // Verify both exchanges received resubscription
        let binance_msg = rx1.try_recv().unwrap();
        assert_eq!(binance_msg.len(), 1);

        let coinbase_msg = rx2.try_recv().unwrap();
        assert_eq!(coinbase_msg.len(), 2);
    }

    #[test]
    fn test_create_channel_buffer_size() {
        // Verify channel is created with correct buffer size
        let (tx, _rx) = SubscriptionManager::create_channel();

        // The channel should be able to hold SUBSCRIPTION_CHANNEL_BUFFER items
        // without blocking (we can't test exact capacity, but we can verify it works)
        assert_eq!(SUBSCRIPTION_CHANNEL_BUFFER, 1024);

        // Sender should not be closed
        assert!(!tx.is_closed());
    }

    // ========== SubscriptionError Tests ==========

    #[test]
    fn test_subscription_error_display() {
        let err1 = SubscriptionError::ExchangeNotRegistered(Exchange::Binance);
        let display1 = format!("{}", err1);
        assert!(display1.contains("Binance"));
        assert!(display1.contains("not registered"));

        let err2 = SubscriptionError::ChannelSendError("channel closed".to_string());
        let display2 = format!("{}", err2);
        assert!(display2.contains("channel closed"));
    }

    #[test]
    fn test_subscription_error_equality() {
        let err1 = SubscriptionError::ExchangeNotRegistered(Exchange::Binance);
        let err2 = SubscriptionError::ExchangeNotRegistered(Exchange::Binance);
        let err3 = SubscriptionError::ExchangeNotRegistered(Exchange::Coinbase);

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    // ========== BinanceSubscriptionBuilder Tests ==========

    #[test]
    fn test_binance_subscription_builder_new() {
        let builder = BinanceSubscriptionBuilder::new();
        // Should start with ID counter at 1
        assert!(format!("{:?}", builder).contains("id_counter"));
    }

    #[test]
    fn test_binance_subscription_builder_default() {
        let builder = BinanceSubscriptionBuilder::default();
        // Default should be same as new
        assert!(format!("{:?}", builder).contains("id_counter"));
    }

    #[test]
    fn test_binance_subscription_builder_single_symbol() {
        let builder = BinanceSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["BTCUSDT".to_string()]);

        assert!(msg.contains(r#""method":"SUBSCRIBE""#));
        assert!(msg.contains(r#""btcusdt@depth20@100ms""#));
        assert!(msg.contains(r#""id":1"#));
    }

    #[test]
    fn test_binance_subscription_builder_multiple_symbols() {
        let builder = BinanceSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "SOLUSDT".to_string(),
        ]);

        assert!(msg.contains(r#""method":"SUBSCRIBE""#));
        assert!(msg.contains(r#""btcusdt@depth20@100ms""#));
        assert!(msg.contains(r#""ethusdt@depth20@100ms""#));
        assert!(msg.contains(r#""solusdt@depth20@100ms""#));
        assert!(msg.contains(r#""id":1"#));
    }

    #[test]
    fn test_binance_subscription_builder_lowercase_conversion() {
        let builder = BinanceSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["BTCUSDT".to_string()]);

        // Should convert to lowercase
        assert!(msg.contains("btcusdt@depth20@100ms"));
        assert!(!msg.contains("BTCUSDT@depth20@100ms"));
    }

    #[test]
    fn test_binance_subscription_builder_increments_id() {
        let builder = BinanceSubscriptionBuilder::new();

        let msg1 = builder.build_subscribe_message(&["BTCUSDT".to_string()]);
        let msg2 = builder.build_subscribe_message(&["ETHUSDT".to_string()]);
        let msg3 = builder.build_subscribe_message(&["SOLUSDT".to_string()]);

        assert!(msg1.contains(r#""id":1"#));
        assert!(msg2.contains(r#""id":2"#));
        assert!(msg3.contains(r#""id":3"#));
    }

    #[test]
    fn test_binance_subscription_builder_empty_symbols() {
        let builder = BinanceSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[]);

        // Should produce valid JSON with empty params
        assert!(msg.contains(r#""method":"SUBSCRIBE""#));
        assert!(msg.contains(r#""params":[]"#));
    }

    #[test]
    fn test_binance_subscription_builder_message_format() {
        let builder = BinanceSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["BTCUSDT".to_string(), "ETHUSDT".to_string()]);

        // Verify exact JSON structure
        let expected = r#"{"method":"SUBSCRIBE","params":["btcusdt@depth20@100ms","ethusdt@depth20@100ms"],"id":1}"#;
        assert_eq!(msg, expected);
    }

    #[test]
    fn test_binance_subscription_builder_is_send_sync() {
        // Verify BinanceSubscriptionBuilder implements Send + Sync (required by SubscriptionBuilder)
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BinanceSubscriptionBuilder>();
    }

    // ========== CoinbaseSubscriptionBuilder Tests ==========

    #[test]
    fn test_coinbase_subscription_builder_new() {
        let builder = CoinbaseSubscriptionBuilder::new();
        assert!(format!("{:?}", builder).contains("CoinbaseSubscriptionBuilder"));
    }

    #[test]
    fn test_coinbase_subscription_builder_default() {
        let builder = CoinbaseSubscriptionBuilder::default();
        assert!(format!("{:?}", builder).contains("CoinbaseSubscriptionBuilder"));
    }

    #[test]
    fn test_coinbase_subscription_builder_single_symbol() {
        let builder = CoinbaseSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["BTC-USD".to_string()]);

        assert!(msg.contains(r#""type": "subscribe""#));
        assert!(msg.contains(r#""BTC-USD""#));
        assert!(msg.contains(r#""channels": ["level2", "heartbeats"]"#));
    }

    #[test]
    fn test_coinbase_subscription_builder_multiple_symbols() {
        let builder = CoinbaseSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[
            "BTC-USD".to_string(),
            "ETH-USD".to_string(),
            "SOL-USD".to_string(),
        ]);

        assert!(msg.contains(r#""type": "subscribe""#));
        assert!(msg.contains(r#""BTC-USD""#));
        assert!(msg.contains(r#""ETH-USD""#));
        assert!(msg.contains(r#""SOL-USD""#));
    }

    #[test]
    fn test_coinbase_subscription_builder_empty_symbols() {
        let builder = CoinbaseSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[]);

        assert!(msg.contains(r#""type": "subscribe""#));
        assert!(msg.contains(r#""product_ids": []"#));
    }

    #[test]
    fn test_coinbase_subscription_builder_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CoinbaseSubscriptionBuilder>();
    }

    // ========== BybitSubscriptionBuilder Tests ==========

    #[test]
    fn test_bybit_subscription_builder_new() {
        let builder = BybitSubscriptionBuilder::new();
        assert!(format!("{:?}", builder).contains("BybitSubscriptionBuilder"));
    }

    #[test]
    fn test_bybit_subscription_builder_default() {
        let builder = BybitSubscriptionBuilder::default();
        assert!(format!("{:?}", builder).contains("BybitSubscriptionBuilder"));
    }

    #[test]
    fn test_bybit_subscription_builder_single_symbol() {
        let builder = BybitSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["BTCUSDT".to_string()]);

        assert!(msg.contains(r#""op": "subscribe""#));
        assert!(msg.contains(r#""orderbook.50.BTCUSDT""#));
    }

    #[test]
    fn test_bybit_subscription_builder_multiple_symbols() {
        let builder = BybitSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "SOLUSDT".to_string(),
        ]);

        assert!(msg.contains(r#""op": "subscribe""#));
        assert!(msg.contains(r#""orderbook.50.BTCUSDT""#));
        assert!(msg.contains(r#""orderbook.50.ETHUSDT""#));
        assert!(msg.contains(r#""orderbook.50.SOLUSDT""#));
    }

    #[test]
    fn test_bybit_subscription_builder_uppercase_conversion() {
        let builder = BybitSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["btcusdt".to_string()]);

        // Should convert to uppercase
        assert!(msg.contains("orderbook.50.BTCUSDT"));
        assert!(!msg.contains("orderbook.50.btcusdt"));
    }

    #[test]
    fn test_bybit_subscription_builder_empty_symbols() {
        let builder = BybitSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[]);

        assert!(msg.contains(r#""op": "subscribe""#));
        assert!(msg.contains(r#""args": []"#));
    }

    #[test]
    fn test_bybit_subscription_builder_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BybitSubscriptionBuilder>();
    }

    // ========== GateIOSubscriptionBuilder Tests ==========

    #[test]
    fn test_gateio_subscription_builder_new() {
        let builder = GateIOSubscriptionBuilder::new();
        assert!(format!("{:?}", builder).contains("GateIOSubscriptionBuilder"));
    }

    #[test]
    fn test_gateio_subscription_builder_default() {
        let builder = GateIOSubscriptionBuilder::default();
        assert!(format!("{:?}", builder).contains("GateIOSubscriptionBuilder"));
    }

    #[test]
    fn test_gateio_subscription_builder_single_symbol() {
        let builder = GateIOSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["BTC_USDT".to_string()]);

        assert!(msg.contains(r#""channel": "spot.obu""#));
        assert!(msg.contains(r#""event": "subscribe""#));
        assert!(msg.contains(r#""ob.BTC_USDT.50""#));
        assert!(msg.contains(r#""time":"#));
    }

    #[test]
    fn test_gateio_subscription_builder_multiple_symbols() {
        let builder = GateIOSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[
            "BTC_USDT".to_string(),
            "ETH_USDT".to_string(),
            "SOL_USDT".to_string(),
        ]);

        assert!(msg.contains(r#""channel": "spot.obu""#));
        assert!(msg.contains(r#""ob.BTC_USDT.50""#));
        assert!(msg.contains(r#""ob.ETH_USDT.50""#));
        assert!(msg.contains(r#""ob.SOL_USDT.50""#));
    }

    #[test]
    fn test_gateio_subscription_builder_uppercase_conversion() {
        let builder = GateIOSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["btc_usdt".to_string()]);

        // Should convert to uppercase
        assert!(msg.contains("ob.BTC_USDT.50"));
        assert!(!msg.contains("ob.btc_usdt.50"));
    }

    #[test]
    fn test_gateio_subscription_builder_empty_symbols() {
        let builder = GateIOSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[]);

        assert!(msg.contains(r#""channel": "spot.obu""#));
        assert!(msg.contains(r#""payload": []"#));
    }

    #[test]
    fn test_gateio_subscription_builder_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GateIOSubscriptionBuilder>();
    }

    // ========== UpbitSubscriptionBuilder Tests ==========

    #[test]
    fn test_upbit_subscription_builder_new() {
        let builder = UpbitSubscriptionBuilder::new();
        assert!(format!("{:?}", builder).contains("UpbitSubscriptionBuilder"));
    }

    #[test]
    fn test_upbit_subscription_builder_default() {
        let builder = UpbitSubscriptionBuilder::default();
        assert!(format!("{:?}", builder).contains("UpbitSubscriptionBuilder"));
    }

    #[test]
    fn test_upbit_subscription_builder_single_symbol() {
        let builder = UpbitSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["KRW-BTC".to_string()]);

        assert!(msg.contains(r#""ticket":"arbitrage-bot""#));
        assert!(msg.contains(r#""type":"ticker""#));
        assert!(msg.contains(r#""type":"orderbook""#));
        assert!(msg.contains(r#""KRW-BTC""#));
        assert!(msg.contains(r#""format":"SIMPLE""#));
    }

    #[test]
    fn test_upbit_subscription_builder_multiple_symbols() {
        let builder = UpbitSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[
            "KRW-BTC".to_string(),
            "KRW-ETH".to_string(),
            "KRW-SOL".to_string(),
        ]);

        assert!(msg.contains(r#""KRW-BTC""#));
        assert!(msg.contains(r#""KRW-ETH""#));
        assert!(msg.contains(r#""KRW-SOL""#));
        // Verify both ticker and orderbook sections contain all codes
        assert!(msg.contains(r#""type":"ticker","codes":["KRW-BTC","KRW-ETH","KRW-SOL"]"#));
        assert!(msg.contains(r#""type":"orderbook","codes":["KRW-BTC","KRW-ETH","KRW-SOL"]"#));
    }

    #[test]
    fn test_upbit_subscription_builder_empty_symbols() {
        let builder = UpbitSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[]);

        assert!(msg.contains(r#""ticket":"arbitrage-bot""#));
        assert!(msg.contains(r#""codes":[]"#));
    }

    #[test]
    fn test_upbit_subscription_builder_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<UpbitSubscriptionBuilder>();
    }

    // ========== BithumbSubscriptionBuilder Tests ==========

    #[test]
    fn test_bithumb_subscription_builder_new() {
        let builder = BithumbSubscriptionBuilder::new();
        assert!(format!("{:?}", builder).contains("BithumbSubscriptionBuilder"));
    }

    #[test]
    fn test_bithumb_subscription_builder_default() {
        let builder = BithumbSubscriptionBuilder::default();
        assert!(format!("{:?}", builder).contains("BithumbSubscriptionBuilder"));
    }

    #[test]
    fn test_bithumb_subscription_builder_single_symbol() {
        let builder = BithumbSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&["KRW-BTC".to_string()]);

        assert!(msg.contains(r#""ticket":"arbitrage-bot""#));
        assert!(msg.contains(r#""type":"ticker""#));
        assert!(msg.contains(r#""type":"orderbook""#));
        assert!(msg.contains(r#""KRW-BTC""#));
        assert!(msg.contains(r#""level":1"#)); // Bithumb uses level=1
        assert!(msg.contains(r#""format":"SIMPLE""#));
    }

    #[test]
    fn test_bithumb_subscription_builder_multiple_symbols() {
        let builder = BithumbSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[
            "KRW-BTC".to_string(),
            "KRW-ETH".to_string(),
            "KRW-SOL".to_string(),
        ]);

        assert!(msg.contains(r#""KRW-BTC""#));
        assert!(msg.contains(r#""KRW-ETH""#));
        assert!(msg.contains(r#""KRW-SOL""#));
        // Verify both ticker and orderbook sections contain all codes
        assert!(msg.contains(r#""type":"ticker","codes":["KRW-BTC","KRW-ETH","KRW-SOL"]"#));
        assert!(msg.contains(r#""type":"orderbook","codes":["KRW-BTC","KRW-ETH","KRW-SOL"]"#));
    }

    #[test]
    fn test_bithumb_subscription_builder_empty_symbols() {
        let builder = BithumbSubscriptionBuilder::new();
        let msg = builder.build_subscribe_message(&[]);

        assert!(msg.contains(r#""ticket":"arbitrage-bot""#));
        assert!(msg.contains(r#""codes":[]"#));
        assert!(msg.contains(r#""level":1"#));
    }

    #[test]
    fn test_bithumb_subscription_builder_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BithumbSubscriptionBuilder>();
    }

    #[test]
    fn test_bithumb_vs_upbit_level_difference() {
        // Verify Bithumb uses level=1 while Upbit uses level=0
        let bithumb_msg =
            BithumbSubscriptionBuilder::new().build_subscribe_message(&["KRW-BTC".to_string()]);
        let upbit_msg =
            UpbitSubscriptionBuilder::new().build_subscribe_message(&["KRW-BTC".to_string()]);

        assert!(bithumb_msg.contains(r#""level":1"#));
        assert!(upbit_msg.contains(r#""level":0"#));
    }

    // ========== SubscriptionRetryPolicy Tests ==========

    #[test]
    fn test_subscription_retry_policy_default() {
        let policy = SubscriptionRetryPolicy::default();

        // NFR2: Initial delay 2 seconds
        assert_eq!(policy.initial_delay_ms(), 2_000);
        // NFR2: Max delay 5 minutes
        assert_eq!(policy.max_delay_ms(), 300_000);
        // Default max retries
        assert_eq!(policy.max_retries(), 5);
        // Jitter enabled by default
        assert!(policy.jitter_enabled());
    }

    #[test]
    fn test_subscription_retry_policy_new() {
        let policy = SubscriptionRetryPolicy::new(1000, 60_000, 3);

        assert_eq!(policy.initial_delay_ms(), 1000);
        assert_eq!(policy.max_delay_ms(), 60_000);
        assert_eq!(policy.max_retries(), 3);
        assert!(policy.jitter_enabled());
    }

    #[test]
    fn test_subscription_retry_policy_without_jitter() {
        let policy = SubscriptionRetryPolicy::default().without_jitter();

        assert!(!policy.jitter_enabled());
        // Without jitter, delay should be exact
        assert_eq!(policy.calculate_delay(1), 2_000);
        assert_eq!(policy.calculate_delay(2), 4_000);
    }

    #[test]
    fn test_subscription_retry_policy_exponential_backoff() {
        let policy = SubscriptionRetryPolicy::default().without_jitter();

        // NFR2: 2s → 4s → 8s → 16s → 32s (then capped at 5 min)
        assert_eq!(policy.calculate_delay(1), 2_000); // 2^0 * 2000 = 2000
        assert_eq!(policy.calculate_delay(2), 4_000); // 2^1 * 2000 = 4000
        assert_eq!(policy.calculate_delay(3), 8_000); // 2^2 * 2000 = 8000
        assert_eq!(policy.calculate_delay(4), 16_000); // 2^3 * 2000 = 16000
        assert_eq!(policy.calculate_delay(5), 32_000); // 2^4 * 2000 = 32000
        assert_eq!(policy.calculate_delay(6), 64_000); // 2^5 * 2000 = 64000
        assert_eq!(policy.calculate_delay(7), 128_000); // 2^6 * 2000 = 128000
        assert_eq!(policy.calculate_delay(8), 256_000); // 2^7 * 2000 = 256000
        assert_eq!(policy.calculate_delay(9), 300_000); // Capped at 5 min
        assert_eq!(policy.calculate_delay(10), 300_000); // Still capped
    }

    #[test]
    fn test_subscription_retry_policy_max_delay_cap() {
        let policy = SubscriptionRetryPolicy::new(2_000, 10_000, 10).without_jitter();

        // Should cap at 10 seconds
        assert_eq!(policy.calculate_delay(1), 2_000);
        assert_eq!(policy.calculate_delay(2), 4_000);
        assert_eq!(policy.calculate_delay(3), 8_000);
        assert_eq!(policy.calculate_delay(4), 10_000); // Capped
        assert_eq!(policy.calculate_delay(5), 10_000); // Still capped
    }

    #[test]
    fn test_subscription_retry_policy_jitter_range() {
        let policy = SubscriptionRetryPolicy::default();

        // With jitter enabled, delays should be between base and base * 1.25
        for _ in 0..100 {
            let delay = policy.calculate_delay(1);
            assert!(delay >= 2_000, "Delay {} should be >= 2000", delay);
            assert!(
                delay <= 2_500,
                "Delay {} should be <= 2500 (25% jitter)",
                delay
            );
        }
    }

    #[test]
    fn test_subscription_retry_policy_should_retry() {
        let policy = SubscriptionRetryPolicy::new(1000, 60_000, 3);

        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(policy.should_retry(3));
        assert!(!policy.should_retry(4));
        assert!(!policy.should_retry(5));
    }

    #[test]
    fn test_subscription_retry_policy_calculate_delay_duration() {
        let policy = SubscriptionRetryPolicy::default().without_jitter();

        let duration = policy.calculate_delay_duration(1);
        assert_eq!(duration, std::time::Duration::from_millis(2_000));

        let duration = policy.calculate_delay_duration(3);
        assert_eq!(duration, std::time::Duration::from_millis(8_000));
    }

    #[test]
    fn test_subscription_retry_policy_zero_attempt() {
        let policy = SubscriptionRetryPolicy::default().without_jitter();

        // Attempt 0 should behave like attempt 1 (saturating_sub prevents underflow)
        assert_eq!(policy.calculate_delay(0), 2_000);
    }

    // ========== SubscriptionRetryState Tests ==========

    #[test]
    fn test_subscription_retry_state_new() {
        let state = SubscriptionRetryState::new();

        assert_eq!(state.attempt_count(), 0);
        assert_eq!(state.total_failures(), 0);
    }

    #[test]
    fn test_subscription_retry_state_default() {
        let state = SubscriptionRetryState::default();

        assert_eq!(state.attempt_count(), 0);
        assert_eq!(state.total_failures(), 0);
    }

    #[test]
    fn test_subscription_retry_state_record_failure() {
        let mut state = SubscriptionRetryState::new();

        state.record_failure();
        assert_eq!(state.attempt_count(), 1);
        assert_eq!(state.total_failures(), 1);

        state.record_failure();
        assert_eq!(state.attempt_count(), 2);
        assert_eq!(state.total_failures(), 2);

        state.record_failure();
        assert_eq!(state.attempt_count(), 3);
        assert_eq!(state.total_failures(), 3);
    }

    #[test]
    fn test_subscription_retry_state_record_success() {
        let mut state = SubscriptionRetryState::new();

        state.record_failure();
        state.record_failure();
        assert_eq!(state.attempt_count(), 2);

        state.record_success();
        assert_eq!(state.attempt_count(), 0); // Reset
        assert_eq!(state.total_failures(), 2); // Total preserved
    }

    #[test]
    fn test_subscription_retry_state_should_retry() {
        let policy = SubscriptionRetryPolicy::new(1000, 60_000, 3);
        let mut state = SubscriptionRetryState::new();

        // Initial state - should retry (attempt 0)
        assert!(state.should_retry(&policy));

        state.record_failure(); // attempt 1
        assert!(state.should_retry(&policy));

        state.record_failure(); // attempt 2
        assert!(state.should_retry(&policy));

        state.record_failure(); // attempt 3
        assert!(state.should_retry(&policy));

        state.record_failure(); // attempt 4
        assert!(!state.should_retry(&policy)); // Exceeded max_retries=3
    }

    #[test]
    fn test_subscription_retry_state_next_delay() {
        let policy = SubscriptionRetryPolicy::default().without_jitter();
        let mut state = SubscriptionRetryState::new();

        // Initial delay (attempt 0)
        assert_eq!(
            state.next_delay(&policy),
            std::time::Duration::from_millis(2_000)
        );

        state.record_failure(); // attempt 1
        assert_eq!(
            state.next_delay(&policy),
            std::time::Duration::from_millis(2_000)
        );

        state.record_failure(); // attempt 2
        assert_eq!(
            state.next_delay(&policy),
            std::time::Duration::from_millis(4_000)
        );

        state.record_failure(); // attempt 3
        assert_eq!(
            state.next_delay(&policy),
            std::time::Duration::from_millis(8_000)
        );
    }

    #[test]
    fn test_subscription_retry_state_is_exhausted() {
        let policy = SubscriptionRetryPolicy::new(1000, 60_000, 2);
        let mut state = SubscriptionRetryState::new();

        assert!(!state.is_exhausted(&policy)); // 0 <= 2

        state.record_failure(); // 1
        assert!(!state.is_exhausted(&policy)); // 1 <= 2

        state.record_failure(); // 2
        assert!(!state.is_exhausted(&policy)); // 2 <= 2

        state.record_failure(); // 3
        assert!(state.is_exhausted(&policy)); // 3 > 2
    }

    #[test]
    fn test_subscription_retry_state_total_failures_preserved() {
        let mut state = SubscriptionRetryState::new();

        state.record_failure();
        state.record_failure();
        state.record_success(); // Reset attempt_count, preserve total

        state.record_failure();
        state.record_failure();
        state.record_failure();

        assert_eq!(state.attempt_count(), 3);
        assert_eq!(state.total_failures(), 5); // 2 + 3 = 5
    }

    #[test]
    fn test_subscription_retry_state_clone() {
        let mut state = SubscriptionRetryState::new();
        state.record_failure();
        state.record_failure();

        let cloned = state.clone();
        assert_eq!(cloned.attempt_count(), 2);
        assert_eq!(cloned.total_failures(), 2);
    }

    #[test]
    fn test_subscription_retry_policy_clone() {
        let policy = SubscriptionRetryPolicy::new(1000, 60_000, 3).without_jitter();
        let cloned = policy.clone();

        assert_eq!(cloned.initial_delay_ms(), 1000);
        assert_eq!(cloned.max_delay_ms(), 60_000);
        assert_eq!(cloned.max_retries(), 3);
        assert!(!cloned.jitter_enabled());
    }

    // ========== SubscriptionError Extended Tests ==========

    #[test]
    fn test_subscription_error_max_retries_exceeded() {
        let err = SubscriptionError::MaxRetriesExceeded {
            exchange: Exchange::Binance,
            symbol: "BTCUSDT".to_string(),
            attempts: 5,
        };

        let display = format!("{}", err);
        assert!(display.contains("Max retries exceeded"));
        assert!(display.contains("BTCUSDT"));
        assert!(display.contains("Binance"));
        assert!(display.contains("5 attempts"));
        assert!(display.contains("manual intervention"));
    }

    #[test]
    fn test_subscription_error_timeout() {
        let err = SubscriptionError::SubscriptionTimeout {
            exchange: Exchange::Coinbase,
            symbol: "BTC-USD".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("timeout"));
        assert!(display.contains("BTC-USD"));
        assert!(display.contains("Coinbase"));
    }

    // ========== SubscriptionStatus Tests ==========

    #[test]
    fn test_subscription_status_active() {
        let status = SubscriptionStatus::Active;

        assert!(status.is_healthy());
        assert!(!status.is_failed());
        assert!(!status.is_retrying());
    }

    #[test]
    fn test_subscription_status_pending() {
        let status = SubscriptionStatus::Pending;

        assert!(!status.is_healthy());
        assert!(!status.is_failed());
        assert!(!status.is_retrying());
    }

    #[test]
    fn test_subscription_status_retrying() {
        let status = SubscriptionStatus::Retrying {
            attempt: 2,
            next_retry_ms: 4000,
        };

        assert!(!status.is_healthy());
        assert!(!status.is_failed());
        assert!(status.is_retrying());
    }

    #[test]
    fn test_subscription_status_failed() {
        let status = SubscriptionStatus::Failed {
            attempts: 5,
            last_error: "Connection refused".to_string(),
        };

        assert!(!status.is_healthy());
        assert!(status.is_failed());
        assert!(!status.is_retrying());
    }

    #[test]
    fn test_subscription_status_default() {
        let status = SubscriptionStatus::default();
        assert!(matches!(status, SubscriptionStatus::Pending));
    }

    // ========== ExchangeSubscriptionTracker Tests ==========

    #[test]
    fn test_exchange_subscription_tracker_new() {
        let tracker = ExchangeSubscriptionTracker::new();
        assert_eq!(tracker.healthy_count(Exchange::Binance), 0);
        assert_eq!(tracker.failed_count(Exchange::Binance), 0);
    }

    #[test]
    fn test_exchange_subscription_tracker_set_active() {
        let tracker = ExchangeSubscriptionTracker::new();

        tracker.set_active(Exchange::Binance, "BTCUSDT");

        assert!(tracker.is_healthy(Exchange::Binance, "BTCUSDT"));
        assert!(!tracker.is_failed(Exchange::Binance, "BTCUSDT"));
        assert_eq!(tracker.healthy_count(Exchange::Binance), 1);
    }

    #[test]
    fn test_exchange_subscription_tracker_set_pending() {
        let tracker = ExchangeSubscriptionTracker::new();

        tracker.set_pending(Exchange::Binance, "BTCUSDT");

        assert!(!tracker.is_healthy(Exchange::Binance, "BTCUSDT"));
        assert!(!tracker.is_failed(Exchange::Binance, "BTCUSDT"));

        let status = tracker.get_status(Exchange::Binance, "BTCUSDT");
        assert!(matches!(status, Some(SubscriptionStatus::Pending)));
    }

    #[test]
    fn test_exchange_subscription_tracker_set_failed() {
        let tracker = ExchangeSubscriptionTracker::new();

        tracker.set_failed(Exchange::Coinbase, "BTC-USD", 5, "Connection refused");

        assert!(!tracker.is_healthy(Exchange::Coinbase, "BTC-USD"));
        assert!(tracker.is_failed(Exchange::Coinbase, "BTC-USD"));
        assert_eq!(tracker.failed_count(Exchange::Coinbase), 1);
    }

    #[test]
    fn test_exchange_subscription_tracker_exchange_isolation_nfr6() {
        // NFR6: Single exchange failure doesn't affect other exchanges
        let tracker = ExchangeSubscriptionTracker::new();

        // Binance is healthy
        tracker.set_active(Exchange::Binance, "BTCUSDT");

        // Coinbase fails
        tracker.set_failed(Exchange::Coinbase, "BTC-USD", 5, "Max retries");

        // Verify isolation: Binance still healthy despite Coinbase failure
        assert!(tracker.is_healthy(Exchange::Binance, "BTCUSDT"));
        assert!(tracker.is_failed(Exchange::Coinbase, "BTC-USD"));
    }

    #[test]
    fn test_exchange_subscription_tracker_record_failure() {
        let tracker = ExchangeSubscriptionTracker::new();
        let policy = SubscriptionRetryPolicy::new(1000, 60_000, 3);

        // First failure
        let (state, is_exhausted) = tracker.record_failure(Exchange::Binance, "BTCUSDT", &policy);
        assert_eq!(state.attempt_count(), 1);
        assert!(!is_exhausted);

        let status = tracker.get_status(Exchange::Binance, "BTCUSDT");
        assert!(matches!(
            status,
            Some(SubscriptionStatus::Retrying { attempt: 1, .. })
        ));

        // Second and third failures
        tracker.record_failure(Exchange::Binance, "BTCUSDT", &policy);
        tracker.record_failure(Exchange::Binance, "BTCUSDT", &policy);

        // Fourth failure - should be exhausted (max_retries = 3)
        let (state, is_exhausted) = tracker.record_failure(Exchange::Binance, "BTCUSDT", &policy);
        assert_eq!(state.attempt_count(), 4);
        assert!(is_exhausted);

        // Status should now be Failed
        assert!(tracker.is_failed(Exchange::Binance, "BTCUSDT"));
    }

    #[test]
    fn test_exchange_subscription_tracker_active_resets_retry() {
        let tracker = ExchangeSubscriptionTracker::new();
        let policy = SubscriptionRetryPolicy::new(1000, 60_000, 3);

        // Record some failures
        tracker.record_failure(Exchange::Binance, "BTCUSDT", &policy);
        tracker.record_failure(Exchange::Binance, "BTCUSDT", &policy);

        // Now succeed
        tracker.set_active(Exchange::Binance, "BTCUSDT");

        // New failure should start from attempt 1
        let (state, _) = tracker.record_failure(Exchange::Binance, "BTCUSDT", &policy);
        assert_eq!(state.attempt_count(), 1);
    }

    #[test]
    fn test_exchange_subscription_tracker_get_all_failed() {
        let tracker = ExchangeSubscriptionTracker::new();

        tracker.set_failed(Exchange::Binance, "BTCUSDT", 5, "Error1");
        tracker.set_failed(Exchange::Coinbase, "BTC-USD", 3, "Error2");
        tracker.set_active(Exchange::Bybit, "BTCUSDT"); // Not failed

        let failed = tracker.get_all_failed();
        assert_eq!(failed.len(), 2);

        // Check both failures are present
        let binance_failed = failed
            .iter()
            .any(|(ex, sym, _)| *ex == Exchange::Binance && sym == "BTCUSDT");
        let coinbase_failed = failed
            .iter()
            .any(|(ex, sym, _)| *ex == Exchange::Coinbase && sym == "BTC-USD");
        assert!(binance_failed);
        assert!(coinbase_failed);
    }

    #[test]
    fn test_exchange_subscription_tracker_failure_rate_nfr7() {
        // NFR7: Subscription failure rate < 1%
        let tracker = ExchangeSubscriptionTracker::new();

        // 99 healthy, 1 failed = 1% failure rate
        for i in 0..99 {
            tracker.set_active(Exchange::Binance, &format!("SYMBOL{}", i));
        }
        tracker.set_failed(Exchange::Binance, "FAILED", 5, "Error");

        let rate = tracker.failure_rate(Exchange::Binance).unwrap();
        assert!((rate - 0.01).abs() < 0.001); // ~1%

        // Threshold check
        assert!(!tracker.exceeds_failure_threshold(Exchange::Binance, 0.01)); // Equal, not exceeded
        assert!(tracker.exceeds_failure_threshold(Exchange::Binance, 0.009)); // 0.9% threshold exceeded
    }

    #[test]
    fn test_exchange_subscription_tracker_failure_rate_empty() {
        let tracker = ExchangeSubscriptionTracker::new();

        // No subscriptions - should return None or 0
        let rate = tracker.failure_rate(Exchange::Binance);
        assert!(rate.is_none());
    }

    #[test]
    fn test_exchange_subscription_tracker_multiple_symbols() {
        let tracker = ExchangeSubscriptionTracker::new();

        tracker.set_active(Exchange::Binance, "BTCUSDT");
        tracker.set_active(Exchange::Binance, "ETHUSDT");
        tracker.set_failed(Exchange::Binance, "XRPUSDT", 3, "Error");

        assert_eq!(tracker.healthy_count(Exchange::Binance), 2);
        assert_eq!(tracker.failed_count(Exchange::Binance), 1);
    }

    // ========== ExchangeRateLimit Tests (NFR10) ==========

    #[test]
    fn test_exchange_rate_limit_new() {
        let limit = ExchangeRateLimit::new(10, 1000, 100);

        assert_eq!(limit.max_messages, 10);
        assert_eq!(limit.window_ms, 1000);
        assert_eq!(limit.min_delay_ms, 100);
    }

    #[test]
    fn test_exchange_rate_limit_binance_nfr10() {
        // NFR10: Binance 5 msg/sec
        let limit = ExchangeRateLimit::for_exchange(Exchange::Binance);

        assert_eq!(limit.max_messages, 5);
        assert_eq!(limit.window_ms, 1000);
        assert_eq!(limit.min_delay_ms, 200);
    }

    #[test]
    fn test_exchange_rate_limit_bybit_nfr10() {
        // NFR10: Bybit 10 msg/sec
        let limit = ExchangeRateLimit::for_exchange(Exchange::Bybit);

        assert_eq!(limit.max_messages, 10);
        assert_eq!(limit.window_ms, 1000);
        assert_eq!(limit.min_delay_ms, 100);
    }

    #[test]
    fn test_exchange_rate_limit_coinbase_nfr10() {
        // NFR10: Coinbase 50 msg/sec
        let limit = ExchangeRateLimit::for_exchange(Exchange::Coinbase);

        assert_eq!(limit.max_messages, 50);
        assert_eq!(limit.window_ms, 1000);
        assert_eq!(limit.min_delay_ms, 20);
    }

    #[test]
    fn test_exchange_rate_limit_gateio_nfr10() {
        // NFR10: GateIO 50 msg/sec
        let limit = ExchangeRateLimit::for_exchange(Exchange::GateIO);

        assert_eq!(limit.max_messages, 50);
        assert_eq!(limit.window_ms, 1000);
        assert_eq!(limit.min_delay_ms, 20);
    }

    #[test]
    fn test_exchange_rate_limit_upbit_nfr10() {
        // NFR10: Upbit 15 msg/sec
        let limit = ExchangeRateLimit::for_exchange(Exchange::Upbit);

        assert_eq!(limit.max_messages, 15);
        assert_eq!(limit.window_ms, 1000);
        assert_eq!(limit.min_delay_ms, 67);
    }

    #[test]
    fn test_exchange_rate_limit_bithumb_nfr10() {
        // NFR10: Bithumb 15 msg/sec
        let limit = ExchangeRateLimit::for_exchange(Exchange::Bithumb);

        assert_eq!(limit.max_messages, 15);
        assert_eq!(limit.window_ms, 1000);
        assert_eq!(limit.min_delay_ms, 67);
    }

    #[test]
    fn test_exchange_rate_limit_min_delay() {
        let limit = ExchangeRateLimit::new(10, 1000, 100);

        assert_eq!(limit.min_delay(), std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_exchange_rate_limit_window() {
        let limit = ExchangeRateLimit::new(10, 2000, 100);

        assert_eq!(limit.window(), std::time::Duration::from_millis(2000));
    }

    #[test]
    fn test_exchange_rate_limit_clone() {
        let limit = ExchangeRateLimit::new(10, 1000, 100);
        let cloned = limit.clone();

        assert_eq!(cloned.max_messages, limit.max_messages);
        assert_eq!(cloned.window_ms, limit.window_ms);
        assert_eq!(cloned.min_delay_ms, limit.min_delay_ms);
    }

    #[test]
    fn test_exchange_rate_limit_copy() {
        let limit = ExchangeRateLimit::new(10, 1000, 100);
        let copied = limit; // Copy, not move

        assert_eq!(copied.max_messages, 10);
        assert_eq!(limit.max_messages, 10); // Original still accessible
    }

    // ========== SubscriptionRateLimiter Tests ==========

    #[test]
    fn test_subscription_rate_limiter_new() {
        let config = ExchangeRateLimit::new(10, 1000, 100);
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Should start with full tokens
        assert_eq!(limiter.available_tokens(), 10.0);
        assert_eq!(limiter.config().max_messages, 10);
    }

    #[test]
    fn test_subscription_rate_limiter_for_exchange() {
        let mut limiter = SubscriptionRateLimiter::for_exchange(Exchange::Binance);

        assert_eq!(limiter.config().max_messages, 5);
        assert_eq!(limiter.available_tokens(), 5.0);
    }

    #[test]
    fn test_subscription_rate_limiter_try_acquire_success() {
        let config = ExchangeRateLimit::new(10, 1000, 0); // No min delay for testing
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Should succeed with available tokens
        assert!(limiter.try_acquire());
        assert_eq!(limiter.available_tokens().floor() as u32, 9);
    }

    #[test]
    fn test_subscription_rate_limiter_try_acquire_depletes_tokens() {
        let config = ExchangeRateLimit::new(3, 1000, 0); // 3 tokens, no min delay
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Consume all tokens
        assert!(limiter.try_acquire()); // 2 left
        assert!(limiter.try_acquire()); // 1 left
        assert!(limiter.try_acquire()); // 0 left

        // Should fail with no tokens
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn test_subscription_rate_limiter_min_delay_enforcement() {
        let config = ExchangeRateLimit::new(10, 1000, 100); // 100ms min delay
        let mut limiter = SubscriptionRateLimiter::new(config);

        // First acquire should succeed
        assert!(limiter.try_acquire());

        // Immediate second acquire should fail due to min_delay
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn test_subscription_rate_limiter_time_until_available_immediate() {
        let config = ExchangeRateLimit::new(10, 1000, 0);
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Should be immediately available
        assert_eq!(limiter.time_until_available(), std::time::Duration::ZERO);
    }

    #[test]
    fn test_subscription_rate_limiter_time_until_available_after_depletion() {
        let config = ExchangeRateLimit::new(1, 1000, 0); // 1 token per second
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Deplete tokens
        assert!(limiter.try_acquire());

        // Should need to wait for replenishment
        let wait = limiter.time_until_available();
        assert!(wait > std::time::Duration::ZERO);
        assert!(wait <= std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_subscription_rate_limiter_reset() {
        let config = ExchangeRateLimit::new(5, 1000, 0);
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Deplete some tokens
        limiter.try_acquire();
        limiter.try_acquire();
        assert!(limiter.available_tokens() < 5.0);

        // Reset should restore full capacity
        limiter.reset();
        assert_eq!(limiter.available_tokens(), 5.0);
    }

    #[test]
    fn test_subscription_rate_limiter_config_accessor() {
        let config = ExchangeRateLimit::new(25, 2000, 50);
        let limiter = SubscriptionRateLimiter::new(config);

        let retrieved_config = limiter.config();
        assert_eq!(retrieved_config.max_messages, 25);
        assert_eq!(retrieved_config.window_ms, 2000);
        assert_eq!(retrieved_config.min_delay_ms, 50);
    }

    #[tokio::test]
    async fn test_subscription_rate_limiter_replenish_over_time() {
        let config = ExchangeRateLimit::new(10, 100, 0); // 10 tokens per 100ms
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Deplete all tokens
        for _ in 0..10 {
            limiter.try_acquire();
        }
        assert!(limiter.available_tokens() < 1.0);

        // Wait for replenishment
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Should have some tokens back (~5 tokens for 50ms wait)
        let tokens = limiter.available_tokens();
        assert!(tokens >= 3.0, "Expected ~5 tokens, got {}", tokens);
        assert!(tokens <= 7.0, "Expected ~5 tokens, got {}", tokens);
    }

    #[tokio::test]
    async fn test_subscription_rate_limiter_acquire_async() {
        let config = ExchangeRateLimit::new(2, 100, 0); // 2 tokens per 100ms
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Deplete tokens
        limiter.try_acquire();
        limiter.try_acquire();

        // acquire() should wait and then succeed
        let start = std::time::Instant::now();
        limiter.acquire().await;
        let elapsed = start.elapsed();

        // Should have waited for replenishment
        assert!(
            elapsed >= std::time::Duration::from_millis(30),
            "Should have waited for token"
        );
    }

    #[test]
    fn test_subscription_rate_limiter_tokens_cap_at_max() {
        let config = ExchangeRateLimit::new(5, 1000, 0);
        let mut limiter = SubscriptionRateLimiter::new(config);

        // Force replenish multiple times - should still cap at max
        for _ in 0..10 {
            // Access tokens to trigger replenish
            let _ = limiter.available_tokens();
        }

        assert!(limiter.available_tokens() <= 5.0);
    }

    #[test]
    fn test_subscription_rate_limiter_debug() {
        let config = ExchangeRateLimit::new(10, 1000, 100);
        let limiter = SubscriptionRateLimiter::new(config);

        let debug_str = format!("{:?}", limiter);
        assert!(debug_str.contains("SubscriptionRateLimiter"));
        assert!(debug_str.contains("config"));
        assert!(debug_str.contains("tokens"));
    }

    // ========== BatchSubscriptionConfig Tests ==========

    #[test]
    fn test_batch_subscription_config_new() {
        let config = BatchSubscriptionConfig::new(5, 200);

        assert_eq!(config.batch_size, 5);
        assert_eq!(config.batch_delay_ms, 200);
    }

    #[test]
    fn test_batch_subscription_config_default() {
        let config = BatchSubscriptionConfig::default();

        assert_eq!(config.batch_size, 10);
        assert_eq!(config.batch_delay_ms, 100);
    }

    #[test]
    fn test_batch_subscription_config_batch_delay() {
        let config = BatchSubscriptionConfig::new(10, 150);

        assert_eq!(config.batch_delay(), std::time::Duration::from_millis(150));
    }

    #[test]
    fn test_batch_subscription_config_batch_count() {
        let config = BatchSubscriptionConfig::new(10, 100);

        assert_eq!(config.batch_count(0), 0);
        assert_eq!(config.batch_count(1), 1);
        assert_eq!(config.batch_count(10), 1);
        assert_eq!(config.batch_count(11), 2);
        assert_eq!(config.batch_count(20), 2);
        assert_eq!(config.batch_count(21), 3);
        assert_eq!(config.batch_count(100), 10);
    }

    #[test]
    fn test_batch_subscription_config_estimated_duration() {
        let config = BatchSubscriptionConfig::new(10, 100);

        // 0 symbols = 0 batches = no delay
        assert_eq!(config.estimated_duration(0), std::time::Duration::ZERO);

        // 10 symbols = 1 batch = no delay (no inter-batch delay needed)
        assert_eq!(config.estimated_duration(10), std::time::Duration::ZERO);

        // 11 symbols = 2 batches = 1 delay = 100ms
        assert_eq!(
            config.estimated_duration(11),
            std::time::Duration::from_millis(100)
        );

        // 100 symbols = 10 batches = 9 delays = 900ms
        assert_eq!(
            config.estimated_duration(100),
            std::time::Duration::from_millis(900)
        );
    }

    #[test]
    fn test_batch_subscription_config_clone() {
        let config = BatchSubscriptionConfig::new(5, 200);
        let cloned = config.clone();

        assert_eq!(cloned.batch_size, 5);
        assert_eq!(cloned.batch_delay_ms, 200);
    }

    #[test]
    fn test_batch_subscription_config_copy() {
        let config = BatchSubscriptionConfig::new(5, 200);
        let copied = config; // Copy

        assert_eq!(copied.batch_size, 5);
        assert_eq!(config.batch_size, 5); // Original still accessible
    }

    // ========== BatchSubscriptionResult Tests ==========

    #[test]
    fn test_batch_subscription_result_success() {
        let result = BatchSubscriptionResult::success(50, 5);

        assert_eq!(result.total_requested, 50);
        assert_eq!(result.subscribed, 50);
        assert_eq!(result.batches_processed, 5);
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_batch_subscription_result_is_complete() {
        let complete = BatchSubscriptionResult::success(10, 1);
        assert!(complete.is_complete());

        let incomplete = BatchSubscriptionResult {
            total_requested: 10,
            subscribed: 8,
            batches_processed: 1,
            failed: vec!["FAIL1".to_string(), "FAIL2".to_string()],
        };
        assert!(!incomplete.is_complete());
    }

    #[test]
    fn test_batch_subscription_result_success_rate() {
        let full = BatchSubscriptionResult::success(100, 10);
        assert!((full.success_rate() - 100.0).abs() < 0.001);

        let partial = BatchSubscriptionResult {
            total_requested: 100,
            subscribed: 80,
            batches_processed: 10,
            failed: vec!["F1".to_string()],
        };
        assert!((partial.success_rate() - 80.0).abs() < 0.001);

        let empty = BatchSubscriptionResult::success(0, 0);
        assert!((empty.success_rate() - 100.0).abs() < 0.001);
    }

    // ========== SubscriptionManager Batch Tests ==========

    #[tokio::test]
    async fn test_subscription_manager_subscribe_batch_empty() {
        let mut manager = SubscriptionManager::new();
        let (tx, _rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        let config = BatchSubscriptionConfig::default();
        let result = manager
            .subscribe_batch(Exchange::Binance, &[], config)
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.total_requested, 0);
        assert_eq!(result.subscribed, 0);
        assert_eq!(result.batches_processed, 0);
    }

    #[tokio::test]
    async fn test_subscription_manager_subscribe_batch_single_batch() {
        let mut manager = SubscriptionManager::new();
        let (tx, mut rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        let config = BatchSubscriptionConfig::new(10, 100);
        let symbols: Vec<String> = (0..5).map(|i| format!("SYM{}", i)).collect();

        let result = manager
            .subscribe_batch(Exchange::Binance, &symbols, config)
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.total_requested, 5);
        assert_eq!(result.subscribed, 5);
        assert_eq!(result.batches_processed, 1);
        assert!(result.is_complete());

        // Verify channel received the subscription
        let received = rx.try_recv().unwrap();
        assert!(received.is_subscribe());
        assert_eq!(received.len(), 5);
    }

    #[tokio::test]
    async fn test_subscription_manager_subscribe_batch_multiple_batches() {
        let mut manager = SubscriptionManager::new();
        let (tx, mut rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        let config = BatchSubscriptionConfig::new(3, 10); // 3 per batch, 10ms delay
        let symbols: Vec<String> = (0..10).map(|i| format!("SYM{}", i)).collect();

        let start = std::time::Instant::now();
        let result = manager
            .subscribe_batch(Exchange::Binance, &symbols, config)
            .await;
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.total_requested, 10);
        assert_eq!(result.subscribed, 10);
        assert_eq!(result.batches_processed, 4); // 3+3+3+1 = 4 batches

        // Should have had 3 delays (between 4 batches)
        assert!(
            elapsed >= std::time::Duration::from_millis(30),
            "Expected at least 30ms delay, got {:?}",
            elapsed
        );

        // Verify all batches were sent
        let mut total_symbols = 0;
        while let Ok(change) = rx.try_recv() {
            total_symbols += change.len();
        }
        assert_eq!(total_symbols, 10);
    }

    #[tokio::test]
    async fn test_subscription_manager_subscribe_batch_unregistered() {
        let mut manager = SubscriptionManager::new();
        // Don't register any exchange

        let config = BatchSubscriptionConfig::default();
        let symbols = vec!["BTCUSDT".to_string()];

        let result = manager
            .subscribe_batch(Exchange::Binance, &symbols, config)
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SubscriptionError::ExchangeNotRegistered(Exchange::Binance)
        ));
    }

    #[tokio::test]
    async fn test_subscription_manager_subscribe_batch_skips_existing() {
        let mut manager = SubscriptionManager::new();
        let (tx, mut rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        // First, subscribe to some symbols
        let initial = vec!["SYM0".to_string(), "SYM1".to_string()];
        manager
            .update_subscriptions(Exchange::Binance, &initial)
            .await
            .unwrap();
        let _ = rx.try_recv(); // Consume initial message

        // Now batch subscribe including existing symbols
        let config = BatchSubscriptionConfig::default();
        let symbols: Vec<String> = (0..5).map(|i| format!("SYM{}", i)).collect(); // SYM0-SYM4

        let result = manager
            .subscribe_batch(Exchange::Binance, &symbols, config)
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        // Only SYM2, SYM3, SYM4 should be new
        assert_eq!(result.subscribed, 3);
    }

    #[tokio::test]
    async fn test_subscription_manager_subscribe_batch_multi() {
        let mut manager = SubscriptionManager::new();
        let (tx1, _rx1) = SubscriptionManager::create_channel();
        let (tx2, _rx2) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx1);
        manager.register_exchange(Exchange::Coinbase, tx2);

        let config = BatchSubscriptionConfig::default();
        let requests = vec![
            (
                Exchange::Binance,
                vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
            ),
            (Exchange::Coinbase, vec!["BTC-USD".to_string()]),
        ];

        let results = manager.subscribe_batch_multi(&requests, config).await;

        assert_eq!(results.len(), 2);
        assert!(results.get(&Exchange::Binance).unwrap().is_ok());
        assert!(results.get(&Exchange::Coinbase).unwrap().is_ok());

        let binance_result = results.get(&Exchange::Binance).unwrap().as_ref().unwrap();
        assert_eq!(binance_result.subscribed, 2);

        let coinbase_result = results.get(&Exchange::Coinbase).unwrap().as_ref().unwrap();
        assert_eq!(coinbase_result.subscribed, 1);
    }

    #[tokio::test]
    async fn test_subscription_manager_subscribe_batch_updates_current() {
        let mut manager = SubscriptionManager::new();
        let (tx, _rx) = SubscriptionManager::create_channel();
        manager.register_exchange(Exchange::Binance, tx);

        let config = BatchSubscriptionConfig::new(2, 10);
        let symbols: Vec<String> = (0..5).map(|i| format!("SYM{}", i)).collect();

        manager
            .subscribe_batch(Exchange::Binance, &symbols, config)
            .await
            .unwrap();

        // Verify current subscriptions were updated
        assert_eq!(manager.subscription_count(Exchange::Binance), 5);
        let current = manager
            .get_current_subscriptions(Exchange::Binance)
            .unwrap();
        for i in 0..5 {
            assert!(current.contains(&format!("SYM{}", i)));
        }
    }

    // ========== SubscriptionEventType Tests ==========

    #[test]
    fn test_subscription_event_type_debug() {
        let types = vec![
            SubscriptionEventType::Subscribed,
            SubscriptionEventType::Failed,
            SubscriptionEventType::RetryScheduled,
            SubscriptionEventType::MaxRetriesExceeded,
            SubscriptionEventType::Unsubscribed,
            SubscriptionEventType::BatchCompleted,
            SubscriptionEventType::Resubscribed,
        ];

        for event_type in types {
            let debug_str = format!("{:?}", event_type);
            assert!(!debug_str.is_empty());
        }
    }

    // ========== SubscriptionEvent Tests ==========

    #[test]
    fn test_subscription_event_subscribed() {
        let event = SubscriptionEvent::subscribed(
            Exchange::Binance,
            vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
        );

        assert!(matches!(
            event.event_type,
            SubscriptionEventType::Subscribed
        ));
        assert_eq!(event.exchange, Exchange::Binance);
        assert_eq!(event.symbols.len(), 2);
        assert!(event.error.is_none());
        assert!(event.retry_attempt.is_none());
        assert!(event.batch_info.is_none());
    }

    #[test]
    fn test_subscription_event_failed() {
        let event = SubscriptionEvent::failed(
            Exchange::Coinbase,
            "BTC-USD".to_string(),
            "Connection refused".to_string(),
        );

        assert!(matches!(event.event_type, SubscriptionEventType::Failed));
        assert_eq!(event.exchange, Exchange::Coinbase);
        assert_eq!(event.symbols, vec!["BTC-USD"]);
        assert_eq!(event.error, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_subscription_event_retry_scheduled() {
        let event =
            SubscriptionEvent::retry_scheduled(Exchange::Bybit, "BTCUSDT".to_string(), 3, 8000);

        assert!(matches!(
            event.event_type,
            SubscriptionEventType::RetryScheduled
        ));
        assert_eq!(event.exchange, Exchange::Bybit);
        assert_eq!(event.retry_attempt, Some(3));
        assert_eq!(event.retry_delay_ms, Some(8000));
    }

    #[test]
    fn test_subscription_event_max_retries_exceeded() {
        let event =
            SubscriptionEvent::max_retries_exceeded(Exchange::Upbit, "KRW-BTC".to_string(), 5);

        assert!(matches!(
            event.event_type,
            SubscriptionEventType::MaxRetriesExceeded
        ));
        assert_eq!(event.exchange, Exchange::Upbit);
        assert_eq!(event.retry_attempt, Some(5));
        assert!(event.error.is_some());
        assert!(event.error.unwrap().contains("5"));
    }

    #[test]
    fn test_subscription_event_batch_completed() {
        let event = SubscriptionEvent::batch_completed(
            Exchange::GateIO,
            vec!["BTC_USDT".to_string(), "ETH_USDT".to_string()],
            2,
            5,
        );

        assert!(matches!(
            event.event_type,
            SubscriptionEventType::BatchCompleted
        ));
        assert_eq!(event.exchange, Exchange::GateIO);
        assert_eq!(event.batch_info, Some((2, 5)));
        assert_eq!(event.symbols.len(), 2);
    }

    #[test]
    fn test_subscription_event_resubscribed() {
        let event = SubscriptionEvent::resubscribed(
            Exchange::Bithumb,
            vec![
                "KRW-BTC".to_string(),
                "KRW-ETH".to_string(),
                "KRW-XRP".to_string(),
            ],
        );

        assert!(matches!(
            event.event_type,
            SubscriptionEventType::Resubscribed
        ));
        assert_eq!(event.exchange, Exchange::Bithumb);
        assert_eq!(event.symbols.len(), 3);
    }

    #[test]
    fn test_subscription_event_clone() {
        let event = SubscriptionEvent::failed(
            Exchange::Binance,
            "BTCUSDT".to_string(),
            "Timeout".to_string(),
        );
        let cloned = event.clone();

        assert_eq!(cloned.exchange, event.exchange);
        assert_eq!(cloned.symbols, event.symbols);
        assert_eq!(cloned.error, event.error);
    }

    // ========== SubscriptionLogger Tests ==========

    #[test]
    fn test_subscription_logger_log_subscribed() {
        // This test verifies the function doesn't panic
        // Actual log output is verified via tracing subscriber in integration tests
        SubscriptionLogger::log_subscribed(
            Exchange::Binance,
            &["BTCUSDT".to_string(), "ETHUSDT".to_string()],
        );
    }

    #[test]
    fn test_subscription_logger_log_failed() {
        SubscriptionLogger::log_failed(Exchange::Coinbase, "BTC-USD", "Connection timeout");
    }

    #[test]
    fn test_subscription_logger_log_retry() {
        SubscriptionLogger::log_retry(Exchange::Bybit, "BTCUSDT", 2, 4000);
    }

    #[test]
    fn test_subscription_logger_log_max_retries_exceeded() {
        SubscriptionLogger::log_max_retries_exceeded(Exchange::Upbit, "KRW-BTC", 5);
    }

    #[test]
    fn test_subscription_logger_log_batch_completed() {
        SubscriptionLogger::log_batch_completed(
            Exchange::GateIO,
            &["BTC_USDT".to_string(), "ETH_USDT".to_string()],
            1,
            3,
        );
    }

    #[test]
    fn test_subscription_logger_log_resubscribed() {
        SubscriptionLogger::log_resubscribed(
            Exchange::Bithumb,
            &["KRW-BTC".to_string(), "KRW-ETH".to_string()],
        );
    }

    #[test]
    fn test_subscription_event_log_many_symbols() {
        // Test truncation for many symbols (> 3)
        let event = SubscriptionEvent::subscribed(
            Exchange::Binance,
            (0..10).map(|i| format!("SYM{}", i)).collect(),
        );
        // Should not panic, and should truncate to "SYM0, SYM1, SYM2, ... (+7 more)"
        event.log();
    }

    #[test]
    fn test_subscription_event_log_all_types() {
        // Ensure all event types can be logged without panic
        let events = vec![
            SubscriptionEvent::subscribed(Exchange::Binance, vec!["BTC".to_string()]),
            SubscriptionEvent::failed(Exchange::Coinbase, "BTC".to_string(), "Error".to_string()),
            SubscriptionEvent::retry_scheduled(Exchange::Bybit, "BTC".to_string(), 1, 2000),
            SubscriptionEvent::max_retries_exceeded(Exchange::Upbit, "BTC".to_string(), 5),
            SubscriptionEvent::batch_completed(Exchange::GateIO, vec!["BTC".to_string()], 1, 1),
            SubscriptionEvent::resubscribed(Exchange::Bithumb, vec!["BTC".to_string()]),
        ];

        for event in events {
            event.log();
        }
    }

    // ========== NewMarketSubscriptionHandler Tests (Epic 5) ==========

    #[test]
    fn test_new_market_subscription_handler_new() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let handler = NewMarketSubscriptionHandler::new(move |_symbol| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        handler.on_market_subscribed("BTCUSDT");
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_new_market_subscription_handler_on_market_subscribed() {
        use std::sync::Mutex;

        let symbols = Arc::new(Mutex::new(Vec::<String>::new()));
        let symbols_clone = Arc::clone(&symbols);

        let handler = NewMarketSubscriptionHandler::new(move |symbol| {
            symbols_clone.lock().unwrap().push(symbol.to_string());
        });

        handler.on_market_subscribed("BTCUSDT");
        handler.on_market_subscribed("ETHUSDT");

        let captured = symbols.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert!(captured.contains(&"BTCUSDT".to_string()));
        assert!(captured.contains(&"ETHUSDT".to_string()));
    }

    #[test]
    fn test_new_market_subscription_handler_on_markets_subscribed() {
        use std::sync::Mutex;

        let symbols = Arc::new(Mutex::new(Vec::<String>::new()));
        let symbols_clone = Arc::clone(&symbols);

        let handler = NewMarketSubscriptionHandler::new(move |symbol| {
            symbols_clone.lock().unwrap().push(symbol.to_string());
        });

        let batch = vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "SOLUSDT".to_string(),
        ];
        handler.on_markets_subscribed(&batch);

        let captured = symbols.lock().unwrap();
        assert_eq!(captured.len(), 3);
    }

    #[test]
    fn test_new_market_subscription_handler_clone() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let handler = NewMarketSubscriptionHandler::new(move |_symbol| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        let cloned = handler.clone();

        handler.on_market_subscribed("BTC");
        cloned.on_market_subscribed("ETH");

        // Both handlers share the same callback
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_new_market_subscription_handler_debug() {
        let handler = NewMarketSubscriptionHandler::new(|_| {});
        let debug_str = format!("{:?}", handler);
        assert!(debug_str.contains("NewMarketSubscriptionHandler"));
        assert!(debug_str.contains("<callback>"));
    }

    #[test]
    fn test_new_market_subscription_handler_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NewMarketSubscriptionHandler>();
    }
}

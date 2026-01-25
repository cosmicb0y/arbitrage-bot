//! Connection pool for managing multiple WebSocket connections per exchange.
//!
//! This module provides connection pools that distribute symbols across
//! multiple WebSocket connections to respect exchange stream limits:
//! - `BinanceConnectionPool`: 1024 streams per connection
//! - `CoinbaseConnectionPool`: 30 L2 streams per connection

use crate::{
    adapter::{
        BinanceAdapter, CoinbaseAdapter, CoinbaseCredentials,
        COINBASE_MAX_L2_STREAMS_PER_CONNECTION, MAX_STREAMS_PER_CONNECTION,
    },
    BinanceSubscriptionBuilder, CoinbaseSubscriptionBuilder, FeedConfig, FeedMessage,
    SubscriptionChange, WsClient, WsMessage,
};
use arbitrage_core::Exchange;
use std::collections::HashSet;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Information about a single WebSocket connection in the pool.
#[derive(Debug)]
pub struct ConnectionInfo {
    /// Index of this connection in the pool (0-based).
    pub index: usize,
    /// Symbols subscribed on this connection.
    pub symbols: HashSet<String>,
    /// Sender for subscription changes to this connection.
    pub sub_tx: mpsc::Sender<SubscriptionChange>,
}

/// Pool of WebSocket connections for Binance.
///
/// Distributes symbols across multiple connections to respect the
/// 1024 stream limit per connection. Each connection handles up to
/// `MAX_STREAMS_PER_CONNECTION` (1000) symbols.
///
/// ## Example
///
/// ```rust,ignore
/// let pool = BinanceConnectionPool::new();
/// let handles = pool.connect_all(
///     &all_binance_symbols,
///     feed_tx.clone(),
///     &mut subscription_manager,
/// ).await;
/// ```
pub struct BinanceConnectionPool {
    /// Connection information for each WebSocket.
    connections: Vec<ConnectionInfo>,
    /// Maximum streams per connection.
    max_streams: usize,
}

impl BinanceConnectionPool {
    /// Create a new empty connection pool.
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            max_streams: MAX_STREAMS_PER_CONNECTION,
        }
    }

    /// Create a connection pool with custom max streams per connection.
    pub fn with_max_streams(max_streams: usize) -> Self {
        Self {
            connections: Vec::new(),
            max_streams,
        }
    }

    /// Get the number of connections in the pool.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Get total symbol count across all connections.
    pub fn total_symbol_count(&self) -> usize {
        self.connections.iter().map(|c| c.symbols.len()).sum()
    }

    /// Connect all necessary WebSocket connections for the given symbols.
    ///
    /// This method:
    /// 1. Distributes symbols across connection groups
    /// 2. Creates WebSocket connections for each group
    /// 3. Registers subscription channels with the subscription manager
    /// 4. Returns task handles for all connections
    ///
    /// ## Arguments
    ///
    /// * `symbols` - All symbols to subscribe to
    /// * `feed_tx` - Channel to send feed messages to
    /// * `sub_senders` - Output vector to receive subscription senders for each connection
    ///
    /// ## Returns
    ///
    /// Vector of `(JoinHandle, mpsc::Receiver<WsMessage>)` pairs for each connection.
    /// The receiver should be passed to a feed runner.
    pub async fn connect_all(
        &mut self,
        symbols: &[String],
        _feed_tx: mpsc::Sender<FeedMessage>,
        sub_senders: &mut Vec<mpsc::Sender<SubscriptionChange>>,
    ) -> Vec<(JoinHandle<()>, mpsc::Receiver<WsMessage>)> {
        let groups = BinanceAdapter::distribute_symbols(symbols);
        let num_connections = groups.len();

        if num_connections == 0 {
            info!("BinanceConnectionPool: No symbols to connect");
            return vec![];
        }

        info!(
            "BinanceConnectionPool: Creating {} connections for {} symbols",
            num_connections,
            symbols.len()
        );

        let mut handles_and_receivers = Vec::with_capacity(num_connections);

        for (index, symbol_group) in groups.into_iter().enumerate() {
            let (ws_tx, ws_rx) = mpsc::channel::<WsMessage>(5000);
            let (sub_tx, sub_rx) = mpsc::channel::<SubscriptionChange>(1024);

            // Build the combined stream URL for this group
            let combined_url = BinanceAdapter::ws_url_combined(&symbol_group);
            let mut config = FeedConfig::for_exchange(Exchange::Binance);
            config.ws_url = combined_url;

            info!(
                "BinanceConnectionPool: Connection {} subscribing to {} symbols",
                index,
                symbol_group.len()
            );
            debug!(
                "BinanceConnectionPool: Connection {} symbols: {:?}",
                index,
                symbol_group.iter().take(5).collect::<Vec<_>>()
            );

            // Create the WebSocket client with subscription support
            let client = WsClient::new(config, ws_tx)
                .with_subscription_channel(sub_rx, Box::new(BinanceSubscriptionBuilder::new()));

            // Spawn the WebSocket connection task
            let connection_index = index;
            let handle = tokio::spawn(async move {
                if let Err(e) = client.run(None).await {
                    warn!(
                        "BinanceConnectionPool: Connection {} error: {}",
                        connection_index, e
                    );
                }
            });

            // Store connection info
            let symbols_set: HashSet<String> = symbol_group.into_iter().collect();
            self.connections.push(ConnectionInfo {
                index,
                symbols: symbols_set,
                sub_tx: sub_tx.clone(),
            });

            // Add sender to the output vector for subscription manager
            sub_senders.push(sub_tx);

            handles_and_receivers.push((handle, ws_rx));
        }

        info!(
            "BinanceConnectionPool: All {} connections started",
            num_connections
        );

        handles_and_receivers
    }

    /// Find which connection a symbol belongs to.
    ///
    /// Returns the connection index, or None if the symbol is not found.
    pub fn find_connection_for_symbol(&self, symbol: &str) -> Option<usize> {
        for conn in &self.connections {
            if conn.symbols.contains(symbol) {
                return Some(conn.index);
            }
        }
        None
    }

    /// Get the subscription sender for a specific connection index.
    pub fn get_sender(&self, index: usize) -> Option<&mpsc::Sender<SubscriptionChange>> {
        self.connections.get(index).map(|c| &c.sub_tx)
    }

    /// Subscribe to new symbols, distributing them to appropriate connections.
    ///
    /// New symbols will be added to the connection with the most available capacity.
    /// If all connections are at capacity, this will return an error.
    pub async fn subscribe(
        &mut self,
        symbols: &[String],
    ) -> Result<usize, String> {
        if symbols.is_empty() {
            return Ok(0);
        }

        // Find connection with most available capacity
        let mut best_conn_idx = None;
        let mut best_capacity = 0;

        for (idx, conn) in self.connections.iter().enumerate() {
            let available = self.max_streams.saturating_sub(conn.symbols.len());
            if available > best_capacity {
                best_capacity = available;
                best_conn_idx = Some(idx);
            }
        }

        let conn_idx = best_conn_idx.ok_or_else(|| {
            "No connections available (all at capacity)".to_string()
        })?;

        if symbols.len() > best_capacity {
            return Err(format!(
                "Not enough capacity: need {} slots, only {} available",
                symbols.len(),
                best_capacity
            ));
        }

        let conn = &mut self.connections[conn_idx];
        let new_symbols: Vec<String> = symbols
            .iter()
            .filter(|s| !conn.symbols.contains(*s))
            .cloned()
            .collect();

        if new_symbols.is_empty() {
            return Ok(0);
        }

        // Send subscription request
        conn.sub_tx
            .send(SubscriptionChange::Subscribe(new_symbols.clone()))
            .await
            .map_err(|e| format!("Failed to send subscription: {}", e))?;

        // Update local state
        for symbol in &new_symbols {
            conn.symbols.insert(symbol.clone());
        }

        debug!(
            "BinanceConnectionPool: Subscribed {} new symbols to connection {}",
            new_symbols.len(),
            conn_idx
        );

        Ok(new_symbols.len())
    }
}

impl Default for BinanceConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Pool of WebSocket connections for Coinbase.
///
/// Distributes symbols across multiple connections to respect Coinbase's
/// L2 stream limit per connection (30 streams per connection).
pub struct CoinbaseConnectionPool {
    /// Connection information for each WebSocket.
    connections: Vec<ConnectionInfo>,
    /// Maximum L2 streams per connection.
    max_streams: usize,
    /// Credentials for authentication.
    credentials: CoinbaseCredentials,
}

impl CoinbaseConnectionPool {
    /// Create a new connection pool with credentials.
    pub fn new(credentials: CoinbaseCredentials) -> Self {
        Self {
            connections: Vec::new(),
            max_streams: COINBASE_MAX_L2_STREAMS_PER_CONNECTION,
            credentials,
        }
    }

    /// Get the number of connections in the pool.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Get total symbol count across all connections.
    pub fn total_symbol_count(&self) -> usize {
        self.connections.iter().map(|c| c.symbols.len()).sum()
    }

    /// Connect all necessary WebSocket connections for the given symbols.
    ///
    /// This distributes symbols across multiple connections, each handling
    /// up to 30 L2 streams to stay within Coinbase's limits.
    pub async fn connect_all(
        &mut self,
        symbols: &[String],
        _feed_tx: mpsc::Sender<FeedMessage>,
        sub_senders: &mut Vec<mpsc::Sender<SubscriptionChange>>,
    ) -> Vec<(JoinHandle<()>, mpsc::Receiver<WsMessage>)> {
        let groups = CoinbaseAdapter::distribute_symbols(symbols);
        let num_connections = groups.len();

        if num_connections == 0 {
            info!("CoinbaseConnectionPool: No symbols to connect");
            return vec![];
        }

        info!(
            "CoinbaseConnectionPool: Creating {} connections for {} symbols ({} per connection)",
            num_connections,
            symbols.len(),
            self.max_streams
        );

        let mut handles_and_receivers = Vec::with_capacity(num_connections);

        for (index, symbol_group) in groups.into_iter().enumerate() {
            let (ws_tx, ws_rx) = mpsc::channel::<WsMessage>(5000);
            let (sub_tx, sub_rx) = mpsc::channel::<SubscriptionChange>(1024);

            // Generate subscription messages for this group
            let subscribe_msgs = match CoinbaseAdapter::subscribe_messages_with_auth(
                &symbol_group,
                &self.credentials,
            ) {
                Ok(msgs) => msgs,
                Err(e) => {
                    warn!(
                        "CoinbaseConnectionPool: Failed to generate subscription for connection {}: {}",
                        index, e
                    );
                    continue;
                }
            };

            info!(
                "CoinbaseConnectionPool: Connection {} subscribing to {} symbols",
                index,
                symbol_group.len()
            );
            debug!(
                "CoinbaseConnectionPool: Connection {} symbols: {:?}",
                index,
                symbol_group.iter().take(5).collect::<Vec<_>>()
            );

            // Create WebSocket config for Coinbase Advanced Trade
            let config = FeedConfig::for_exchange(Exchange::Coinbase);

            // Create the WebSocket client with subscription support
            let client = WsClient::new(config, ws_tx).with_subscription_channel(
                sub_rx,
                Box::new(CoinbaseSubscriptionBuilder::with_credentials(
                    self.credentials.clone(),
                )),
            );

            // Spawn the WebSocket connection task
            let connection_index = index;
            let handle = tokio::spawn(async move {
                if let Err(e) = client.run_with_messages(Some(subscribe_msgs)).await {
                    warn!(
                        "CoinbaseConnectionPool: Connection {} error: {}",
                        connection_index, e
                    );
                }
            });

            // Store connection info
            let symbols_set: HashSet<String> = symbol_group.into_iter().collect();
            self.connections.push(ConnectionInfo {
                index,
                symbols: symbols_set,
                sub_tx: sub_tx.clone(),
            });

            // Add sender to the output vector for subscription manager
            sub_senders.push(sub_tx);

            handles_and_receivers.push((handle, ws_rx));

            // Delay between connection creations to avoid rate limiting
            if index < num_connections - 1 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }

        info!(
            "CoinbaseConnectionPool: All {} connections started",
            self.connections.len()
        );

        handles_and_receivers
    }

    /// Find which connection a symbol belongs to.
    pub fn find_connection_for_symbol(&self, symbol: &str) -> Option<usize> {
        for conn in &self.connections {
            if conn.symbols.contains(symbol) {
                return Some(conn.index);
            }
        }
        None
    }

    /// Get the subscription sender for a specific connection index.
    pub fn get_sender(&self, index: usize) -> Option<&mpsc::Sender<SubscriptionChange>> {
        self.connections.get(index).map(|c| &c.sub_tx)
    }

    /// Get available capacity (remaining slots) for a connection.
    pub fn available_capacity(&self, index: usize) -> usize {
        self.connections
            .get(index)
            .map(|c| self.max_streams.saturating_sub(c.symbols.len()))
            .unwrap_or(0)
    }

    /// Get total available capacity across all connections.
    pub fn total_available_capacity(&self) -> usize {
        self.connections
            .iter()
            .map(|c| self.max_streams.saturating_sub(c.symbols.len()))
            .sum()
    }

    /// Subscribe to new symbols, distributing them to connections with available capacity.
    ///
    /// Returns the number of symbols actually subscribed.
    /// Symbols that couldn't be subscribed (no capacity) are skipped with a warning.
    pub async fn subscribe_with_capacity(&mut self, symbols: &[String]) -> usize {
        if symbols.is_empty() {
            return 0;
        }

        // Filter out symbols already subscribed
        let new_symbols: Vec<String> = symbols
            .iter()
            .filter(|s| self.find_connection_for_symbol(s).is_none())
            .cloned()
            .collect();

        if new_symbols.is_empty() {
            debug!("CoinbaseConnectionPool: All {} symbols already subscribed", symbols.len());
            return 0;
        }

        let mut subscribed_count = 0;
        let mut symbols_to_subscribe = new_symbols.clone();

        // Find connections with available capacity and distribute symbols
        for conn in &mut self.connections {
            if symbols_to_subscribe.is_empty() {
                break;
            }

            let available = self.max_streams.saturating_sub(conn.symbols.len());
            if available == 0 {
                continue;
            }

            // Take as many symbols as this connection can handle
            let take_count = available.min(symbols_to_subscribe.len());
            let batch: Vec<String> = symbols_to_subscribe.drain(..take_count).collect();

            if !batch.is_empty() {
                debug!(
                    "CoinbaseConnectionPool: Connection {} subscribing to {} symbols (capacity: {}/{})",
                    conn.index,
                    batch.len(),
                    conn.symbols.len() + batch.len(),
                    self.max_streams
                );

                // Send subscription request
                if let Err(e) = conn
                    .sub_tx
                    .send(SubscriptionChange::Subscribe(batch.clone()))
                    .await
                {
                    warn!(
                        "CoinbaseConnectionPool: Failed to send subscription to connection {}: {}",
                        conn.index, e
                    );
                    continue;
                }

                // Update local tracking
                for symbol in batch {
                    conn.symbols.insert(symbol);
                    subscribed_count += 1;
                }
            }
        }

        if !symbols_to_subscribe.is_empty() {
            warn!(
                "CoinbaseConnectionPool: {} symbols skipped (no capacity). Consider increasing connections.",
                symbols_to_subscribe.len()
            );
        }

        info!(
            "CoinbaseConnectionPool: Subscribed {} of {} requested symbols",
            subscribed_count,
            new_symbols.len()
        );

        subscribed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_pool_new() {
        let pool = BinanceConnectionPool::new();
        assert_eq!(pool.connection_count(), 0);
        assert_eq!(pool.total_symbol_count(), 0);
        assert_eq!(pool.max_streams, MAX_STREAMS_PER_CONNECTION);
    }

    #[test]
    fn test_connection_pool_with_max_streams() {
        let pool = BinanceConnectionPool::with_max_streams(500);
        assert_eq!(pool.max_streams, 500);
    }

    #[test]
    fn test_find_connection_for_symbol_empty() {
        let pool = BinanceConnectionPool::new();
        assert_eq!(pool.find_connection_for_symbol("BTCUSDT"), None);
    }
}

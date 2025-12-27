//! WebSocket connection manager.
//!
//! Handles connection lifecycle, reconnection, and message routing.

use arbitrage_core::Exchange;

/// Connection state for a WebSocket feed.
#[derive(Debug, Clone)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
    Error(String),
}

impl ConnectionState {
    /// Transition to connecting state.
    pub fn connect(self) -> Self {
        ConnectionState::Connecting
    }

    /// Transition to connected state.
    pub fn connected(self) -> Self {
        ConnectionState::Connected
    }

    /// Transition to disconnected state.
    pub fn disconnect(self) -> Self {
        ConnectionState::Disconnected
    }

    /// Transition to error state.
    pub fn error(self, msg: &str) -> Self {
        ConnectionState::Error(msg.to_string())
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }
}

/// Configuration for a feed connection.
#[derive(Debug, Clone)]
pub struct FeedConfig {
    /// WebSocket URL
    pub ws_url: String,
    /// Exchange identifier
    pub exchange: Exchange,
    /// Delay before reconnecting (ms)
    pub reconnect_delay_ms: u64,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Ping interval to keep connection alive (ms)
    pub ping_interval_ms: u64,
    /// Connection timeout (ms)
    pub connect_timeout_ms: u64,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            ws_url: String::new(),
            exchange: Exchange::Binance,
            reconnect_delay_ms: 1000,
            max_reconnect_attempts: 10,
            ping_interval_ms: 30000,
            connect_timeout_ms: 10000,
        }
    }
}

impl FeedConfig {
    /// Create config for a specific exchange.
    pub fn for_exchange(exchange: Exchange) -> Self {
        let ws_url = match exchange {
            Exchange::Binance => "wss://stream.binance.com:9443/ws".to_string(),
            Exchange::Coinbase => "wss://ws-feed.exchange.coinbase.com".to_string(),
            Exchange::Kraken => "wss://ws.kraken.com".to_string(),
            Exchange::Okx => "wss://ws.okx.com:8443/ws/v5/public".to_string(),
            Exchange::Bybit => "wss://stream.bybit.com/v5/public/spot".to_string(),
            Exchange::Upbit => "wss://api.upbit.com/websocket/v1".to_string(),
            _ => String::new(),
        };

        Self {
            ws_url,
            exchange,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_transitions() {
        let mut state = ConnectionState::Disconnected;

        state = state.connect();
        assert!(matches!(state, ConnectionState::Connecting));

        state = state.connected();
        assert!(matches!(state, ConnectionState::Connected));

        state = state.disconnect();
        assert!(matches!(state, ConnectionState::Disconnected));
    }

    #[test]
    fn test_connection_state_error() {
        let state = ConnectionState::Connecting;
        let state = state.error("test error");
        assert!(matches!(state, ConnectionState::Error(_)));
    }

    #[test]
    fn test_feed_config_default() {
        let config = FeedConfig::default();
        assert!(config.reconnect_delay_ms > 0);
        assert!(config.max_reconnect_attempts > 0);
        assert!(config.ping_interval_ms > 0);
    }

    #[test]
    fn test_feed_config_for_exchange() {
        let binance = FeedConfig::for_exchange(Exchange::Binance);
        assert!(binance.ws_url.contains("binance"));

        let coinbase = FeedConfig::for_exchange(Exchange::Coinbase);
        assert!(coinbase.ws_url.contains("coinbase"));
    }
}

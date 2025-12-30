//! WebSocket client for exchange connections.

use crate::{FeedConfig, FeedError};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// Message received from WebSocket.
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// Text message (JSON).
    Text(String),
    /// Binary message.
    Binary(Vec<u8>),
    /// Connection established.
    Connected,
    /// Connection closed.
    Disconnected,
    /// Error occurred.
    Error(String),
}

/// WebSocket client for a single exchange connection.
pub struct WsClient {
    config: FeedConfig,
    tx: mpsc::Sender<WsMessage>,
}

impl WsClient {
    /// Create a new WebSocket client.
    pub fn new(config: FeedConfig, tx: mpsc::Sender<WsMessage>) -> Self {
        Self { config, tx }
    }

    /// Connect and run the WebSocket client.
    pub async fn run(self, subscribe_msg: Option<String>) -> Result<(), FeedError> {
        let msgs = subscribe_msg.map(|m| vec![m]);
        self.run_with_messages(msgs).await
    }

    /// Connect and run the WebSocket client with multiple subscribe messages.
    /// Useful for exchanges like Bybit that have limits on args per subscription.
    pub async fn run_with_messages(self, subscribe_msgs: Option<Vec<String>>) -> Result<(), FeedError> {
        let mut reconnect_attempts = 0;

        loop {
            match self.connect_and_handle(&subscribe_msgs).await {
                Ok(()) => {
                    info!("WebSocket connection closed normally");
                    break;
                }
                Err(e) => {
                    reconnect_attempts += 1;
                    if reconnect_attempts > self.config.max_reconnect_attempts {
                        error!(
                            "Max reconnection attempts ({}) reached for {:?}",
                            self.config.max_reconnect_attempts, self.config.exchange
                        );
                        return Err(e);
                    }

                    warn!(
                        "WebSocket error for {:?}: {}. Reconnecting ({}/{})",
                        self.config.exchange,
                        e,
                        reconnect_attempts,
                        self.config.max_reconnect_attempts
                    );

                    let _ = self.tx.send(WsMessage::Disconnected).await;

                    // Exponential backoff
                    let delay = self.config.reconnect_delay_ms * (1 << reconnect_attempts.min(5));
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }

        Ok(())
    }

    async fn connect_and_handle(&self, subscribe_msgs: &Option<Vec<String>>) -> Result<(), FeedError> {
        debug!("Connecting to {}", self.config.ws_url);

        let (ws_stream, _) = connect_async(&self.config.ws_url).await?;

        info!("Connected to {:?}", self.config.exchange);
        let _ = self.tx.send(WsMessage::Connected).await;

        let (mut write, mut read) = ws_stream.split();

        // Send subscription messages if provided
        if let Some(ref msgs) = subscribe_msgs {
            for (i, msg) in msgs.iter().enumerate() {
                debug!("Sending subscription {}/{}: {}", i + 1, msgs.len(), msg);
                write.send(Message::Text(msg.clone())).await?;
                // Small delay between messages to avoid rate limiting
                if msgs.len() > 1 && i < msgs.len() - 1 {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
            if msgs.len() > 1 {
                info!("Sent {} subscription messages to {:?}", msgs.len(), self.config.exchange);
            }
        }

        // Set up ping interval
        let ping_interval = Duration::from_millis(self.config.ping_interval_ms);
        let mut ping_timer = tokio::time::interval(ping_interval);

        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            let _ = self.tx.send(WsMessage::Text(text)).await;
                        }
                        Some(Ok(Message::Binary(data))) => {
                            let _ = self.tx.send(WsMessage::Binary(data)).await;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            write.send(Message::Pong(data)).await?;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // Pong received, connection is alive
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Received close frame from {:?}", self.config.exchange);
                            return Ok(());
                        }
                        Some(Err(e)) => {
                            return Err(FeedError::ConnectionFailed(e.to_string()));
                        }
                        None => {
                            return Err(FeedError::Disconnected("Stream ended".to_string()));
                        }
                        _ => {}
                    }
                }
                _ = ping_timer.tick() => {
                    write.send(Message::Ping(vec![])).await?;
                }
            }
        }
    }
}

/// Multi-exchange WebSocket manager.
pub struct WsManager {
    receivers: Vec<mpsc::Receiver<WsMessage>>,
}

impl WsManager {
    /// Create a new manager.
    pub fn new() -> Self {
        Self {
            receivers: Vec::new(),
        }
    }

    /// Spawn a WebSocket client for an exchange.
    pub fn spawn_client(
        &mut self,
        config: FeedConfig,
        subscribe_msg: Option<String>,
    ) -> mpsc::Receiver<WsMessage> {
        let (tx, rx) = mpsc::channel(1000);

        let client = WsClient::new(config.clone(), tx);

        tokio::spawn(async move {
            if let Err(e) = client.run(subscribe_msg).await {
                error!("WebSocket client error for {:?}: {}", config.exchange, e);
            }
        });

        rx
    }
}

impl Default for WsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrage_core::Exchange;

    #[test]
    fn test_ws_message_variants() {
        let msg = WsMessage::Text("test".to_string());
        assert!(matches!(msg, WsMessage::Text(_)));

        let msg = WsMessage::Connected;
        assert!(matches!(msg, WsMessage::Connected));
    }

    #[test]
    fn test_ws_manager_new() {
        let manager = WsManager::new();
        assert!(manager.receivers.is_empty());
    }

    #[tokio::test]
    async fn test_ws_client_creation() {
        let config = FeedConfig::for_exchange(Exchange::Binance);
        let (tx, _rx) = mpsc::channel(100);
        let _client = WsClient::new(config, tx);
    }
}

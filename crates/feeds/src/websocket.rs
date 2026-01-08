//! WebSocket client for exchange connections.

use crate::{FeedConfig, FeedError};
use arbitrage_core::Exchange;
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
    /// Connection established (first time).
    Connected,
    /// Connection closed.
    Disconnected,
    /// Reconnected after disconnection.
    /// Consumers should clear orderbook cache when receiving this,
    /// as delta updates are invalid without a fresh snapshot.
    Reconnected,
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

    /// Create a Gate.io ping message.
    /// Gate.io requires application-level ping: {"time": <unix_timestamp>, "channel": "spot.ping"}
    fn create_gateio_ping() -> String {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!(r#"{{"time": {}, "channel": "spot.ping"}}"#, timestamp)
    }

    /// Send Gate.io subscriptions while concurrently draining incoming messages.
    /// This is critical because Gate.io sends responses for each subscription,
    /// and if we don't read them, the TCP buffer fills up and connection breaks.
    async fn send_gateio_subscriptions<S, R>(
        &self,
        write: &mut S,
        read: &mut R,
        msgs: &[String],
    ) -> Result<(), FeedError>
    where
        S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
        R: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let total = msgs.len();
        let sent_count = Arc::new(AtomicUsize::new(0));
        let sent_count_clone = sent_count.clone();

        debug!("Gate.io: Starting subscription of {} symbols", total);

        // Spawn a task to drain incoming messages
        let (drain_tx, mut drain_rx) = mpsc::channel::<()>(1);
        let drain_handle = tokio::spawn(async move {
            // This task just signals when to stop
            drain_rx.recv().await;
        });

        let mut last_ping = std::time::Instant::now();
        let mut messages_drained = 0u64;
        let mut subscription_errors = 0u32;

        for (i, msg) in msgs.iter().enumerate() {
            // Send ping every 5 seconds
            if last_ping.elapsed().as_secs() >= 5 {
                // Send dual ping (WS + app-level)
                if let Err(e) = write.send(Message::Ping(vec![])).await {
                    warn!("Gate.io: Failed to send keep-alive WS PING: {}", e);
                }
                let ping_msg = Self::create_gateio_ping();
                if let Err(e) = write.send(Message::Text(ping_msg)).await {
                    warn!("Gate.io: Failed to send keep-alive app ping: {}", e);
                }
                debug!("Gate.io: Sent keep-alive ping at {}/{} (drained {} msgs)", i + 1, total, messages_drained);
                last_ping = std::time::Instant::now();
            }

            // Drain any pending messages (non-blocking)
            // Forward orderbook updates to the channel while draining subscription responses
            loop {
                match tokio::time::timeout(Duration::from_millis(1), read.next()).await {
                    Ok(Some(Ok(msg))) => {
                        messages_drained += 1;
                        match &msg {
                            Message::Ping(data) => {
                                let _ = write.send(Message::Pong(data.clone())).await;
                            }
                            Message::Text(text) => {
                                // Forward orderbook updates to the channel (non-blocking)
                                // Gate.io uses partial snapshots, so dropping is OK during subscription phase
                                if text.contains("\"channel\":\"spot.order_book\"") && text.contains("\"event\":\"update\"") {
                                    let _ = self.tx.try_send(WsMessage::Text(text.clone()));
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Some(Err(e))) => {
                        error!("Gate.io: Read error during subscription: {}", e);
                        return Err(FeedError::ConnectionFailed(format!("Read error: {}", e)));
                    }
                    Ok(None) => {
                        error!("Gate.io: Connection closed during subscription");
                        return Err(FeedError::Disconnected("Connection closed".to_string()));
                    }
                    Err(_) => break, // Timeout - no more messages to drain
                }
            }

            // Send subscription (only log progress every 200)
            if i % 200 == 0 {
                debug!("Gate.io: Sending subscription {}/{}", i + 1, total);
            }

            if let Err(e) = write.send(Message::Text(msg.clone())).await {
                subscription_errors += 1;
                if subscription_errors > 3 {
                    error!("Gate.io: Too many subscription errors, aborting");
                    return Err(FeedError::ConnectionFailed(format!("Subscription failed: {}", e)));
                }
                warn!("Gate.io: Subscription error ({}): {}", subscription_errors, e);
                // Wait a bit and retry
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            sent_count.fetch_add(1, Ordering::Relaxed);

            // Gate.io allows 50 req/s per channel
            // Using 10ms delay = 100 req/s, but with drain overhead it's effectively ~50 req/s
            if i < total - 1 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        // Final drain - forward orderbook updates (non-blocking)
        let mut final_drain = 0;
        loop {
            match tokio::time::timeout(Duration::from_millis(100), read.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    final_drain += 1;
                    if text.contains("\"channel\":\"spot.order_book\"") && text.contains("\"event\":\"update\"") {
                        let _ = self.tx.try_send(WsMessage::Text(text));
                    }
                }
                Ok(Some(Ok(_))) => final_drain += 1,
                _ => break,
            }
        }

        let _ = drain_tx.send(()).await;
        drop(drain_handle);

        debug!(
            "Gate.io: Subscription complete! Sent {}/{}, drained {} messages",
            sent_count.load(Ordering::Relaxed),
            total,
            messages_drained + final_drain as u64
        );

        Ok(())
    }

    /// Connect and run the WebSocket client with multiple subscribe messages.
    /// Useful for exchanges like Bybit that have limits on args per subscription.
    ///
    /// This method will retry indefinitely with exponential backoff (max 5 min delay).
    /// Reconnection counter resets after 5 minutes of stable connection.
    pub async fn run_with_messages(self, subscribe_msgs: Option<Vec<String>>) -> Result<(), FeedError> {
        let mut reconnect_attempts = 0u32;
        let mut connection_start: std::time::Instant;
        let mut has_connected_once = false;

        loop {
            connection_start = std::time::Instant::now();
            let is_reconnect = has_connected_once;

            match self.connect_and_handle(&subscribe_msgs, is_reconnect).await {
                Ok(()) => {
                    debug!("WebSocket connection closed normally for {:?}", self.config.exchange);
                    break;
                }
                Err(e) => {
                    let connection_duration = connection_start.elapsed();
                    has_connected_once = true;

                    // Reset reconnect counter if connection was stable (5+ minutes)
                    if connection_duration > Duration::from_secs(300) {
                        info!(
                            "{:?}: Connection was stable for {:?}, resetting reconnect counter",
                            self.config.exchange,
                            connection_duration
                        );
                        reconnect_attempts = 0;
                    }

                    reconnect_attempts = reconnect_attempts.saturating_add(1);

                    // Calculate delay with exponential backoff, capped at 5 minutes
                    let backoff_power = reconnect_attempts.min(8); // max 2^8 = 256x base delay
                    let delay_ms = (self.config.reconnect_delay_ms * (1 << backoff_power)).min(300_000);

                    warn!(
                        "{:?}: WebSocket error after {:?}: {}. Reconnecting in {:.1}s (attempt #{})",
                        self.config.exchange,
                        connection_duration,
                        e,
                        delay_ms as f64 / 1000.0,
                        reconnect_attempts
                    );

                    let _ = self.tx.send(WsMessage::Disconnected).await;

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        Ok(())
    }

    async fn connect_and_handle(&self, subscribe_msgs: &Option<Vec<String>>, is_reconnect: bool) -> Result<(), FeedError> {
        let is_gateio = self.config.exchange == Exchange::GateIO;
        debug!("Connecting to {:?}: {}", self.config.exchange, self.config.ws_url);

        let (ws_stream, response) = connect_async(&self.config.ws_url).await?;
        debug!("{:?}: Connected (status: {:?})", self.config.exchange, response.status());
        if is_reconnect {
            let _ = self.tx.send(WsMessage::Reconnected).await;
        } else {
            let _ = self.tx.send(WsMessage::Connected).await;
        }

        let (mut write, mut read) = ws_stream.split();

        // For Gate.io: use concurrent subscription with message draining
        // This prevents TCP buffer overflow by reading server responses while sending subscriptions
        if is_gateio {
            if let Some(ref msgs) = subscribe_msgs {
                self.send_gateio_subscriptions(&mut write, &mut read, msgs).await?;
            }
        } else {
            // Other exchanges: simple sequential subscription
            if let Some(ref msgs) = subscribe_msgs {
                debug!("{:?}: Sending {} subscription message(s)", self.config.exchange, msgs.len());
                for (i, msg) in msgs.iter().enumerate() {
                    debug!("{:?}: Sending subscription {}/{}", self.config.exchange, i + 1, msgs.len());
                    if let Err(e) = write.send(Message::Text(msg.clone())).await {
                        error!("{:?}: Failed to send subscription: {}", self.config.exchange, e);
                        return Err(FeedError::ConnectionFailed(format!("Subscription failed: {}", e)));
                    }
                    if msgs.len() > 1 && i < msgs.len() - 1 {
                        // Binance has stricter rate limits, use longer delay
                        let delay = if self.config.exchange == Exchange::Binance { 250 } else { 50 };
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    }
                }
                debug!("{:?}: Subscription complete", self.config.exchange);
            }
        }

        // Set up ping interval
        let ping_interval = Duration::from_millis(self.config.ping_interval_ms);
        let mut ping_timer = tokio::time::interval(ping_interval);

        // Stale connection detection: if no message received for 2 minutes, reconnect
        // This helps detect "silent disconnects" where the connection appears alive but is dead
        let stale_timeout = Duration::from_secs(120);
        let mut last_message_time = std::time::Instant::now();

        // Ping timeout detection: if no PONG received after sending PING, connection is dead
        // Upbit and other exchanges may silently drop connections without proper close frames
        let ping_timeout = Duration::from_secs(30); // Wait up to 30s for PONG after PING
        let mut awaiting_pong = false;
        let mut ping_sent_time = std::time::Instant::now();

        // For Gate.io debugging
        let mut last_ping_time = std::time::Instant::now();
        let mut message_count = 0u64;

        if self.config.exchange == Exchange::GateIO {
            debug!("Gate.io: ping interval set to {}ms (dual ping: WS + app-level)", self.config.ping_interval_ms);
        }

        // Track if we've received any message at all after connect
        let mut any_message_received = false;

        loop {
            // Check for stale connection
            if last_message_time.elapsed() > stale_timeout {
                warn!("{:?}: No messages received for {:?}, forcing reconnect",
                    self.config.exchange, last_message_time.elapsed());
                return Err(FeedError::Disconnected("Stale connection - no messages received".to_string()));
            }

            // Check for ping timeout (no PONG received after sending PING)
            if awaiting_pong && ping_sent_time.elapsed() > ping_timeout {
                warn!("{:?}: No PONG received for {:?} after PING, forcing reconnect",
                    self.config.exchange, ping_sent_time.elapsed());
                return Err(FeedError::Disconnected("Ping timeout - no PONG received".to_string()));
            }

            tokio::select! {
                msg = read.next() => {
                    any_message_received = true;
                    // Update last message time for any received message
                    last_message_time = std::time::Instant::now();

                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            message_count += 1;

                            // Gate.io debugging
                            if self.config.exchange == Exchange::GateIO {
                                // Log errors only
                                if text.contains("error") {
                                    warn!("Gate.io msg #{}: {}", message_count, &text[..text.len().min(200)]);
                                }
                            }

                            // Handle Gate.io application-level pong response (ignore it)
                            if self.config.exchange == Exchange::GateIO && text.contains("\"channel\":\"spot.pong\"") {
                                debug!("Gate.io: Received pong response (latency: {:?})", last_ping_time.elapsed());
                                continue;
                            }

                            // Handle Bybit application-level pong response
                            // Bybit responds with {"success":true,"ret_msg":"pong","conn_id":"...","op":"pong"}
                            if self.config.exchange == Exchange::Bybit && text.contains("\"op\":\"pong\"") {
                                awaiting_pong = false;
                                debug!("Bybit: Received pong response (latency: {:?})", ping_sent_time.elapsed());
                                continue;
                            }

                            // Handle Coinbase heartbeats channel messages (connection keep-alive)
                            // These are sent by the server every ~1s when subscribed to heartbeats channel
                            if self.config.exchange == Exchange::Coinbase && text.contains("\"channel\":\"heartbeats\"") {
                                // Heartbeat received - connection is alive
                                // No need to track awaiting_pong since Coinbase sends these automatically
                                continue;
                            }
                            // Use try_send to avoid blocking on channel full
                            // If channel is full, force reconnection to resync orderbook
                            // (dropping messages would break delta-based orderbook sync)
                            if let Err(e) = self.tx.try_send(WsMessage::Text(text)) {
                                match e {
                                    mpsc::error::TrySendError::Full(_) => {
                                        // Channel full - reconnect to resync orderbook
                                        // This is critical for delta-based exchanges (Bybit, Coinbase)
                                        warn!("{:?}: Channel full, forcing reconnect to resync orderbook",
                                            self.config.exchange);
                                        return Err(FeedError::Disconnected("Channel full - resync needed".to_string()));
                                    }
                                    mpsc::error::TrySendError::Closed(_) => {
                                        error!("{:?}: Channel closed", self.config.exchange);
                                        return Err(FeedError::Disconnected("Channel closed".to_string()));
                                    }
                                }
                            }
                        }
                        Some(Ok(Message::Binary(data))) => {
                            message_count += 1;
                            // Use try_send for binary data too - reconnect if full
                            if let Err(mpsc::error::TrySendError::Full(_)) = self.tx.try_send(WsMessage::Binary(data.clone())) {
                                warn!("{:?}: Channel full (binary), forcing reconnect", self.config.exchange);
                                return Err(FeedError::Disconnected("Channel full - resync needed".to_string()));
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            debug!("{:?}: Received WebSocket PING, sending PONG", self.config.exchange);
                            if let Err(e) = write.send(Message::Pong(data)).await {
                                error!("{:?}: Failed to send PONG: {}", self.config.exchange, e);
                                return Err(FeedError::ConnectionFailed(format!("PONG send failed: {}", e)));
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // Pong received, connection is alive
                            awaiting_pong = false;
                            debug!("{:?}: Received WebSocket PONG (latency: {:?})",
                                self.config.exchange, ping_sent_time.elapsed());
                        }
                        Some(Ok(Message::Close(frame))) => {
                            debug!("{:?}: Received close frame: {:?}", self.config.exchange, frame);
                            return Ok(());
                        }
                        Some(Err(e)) => {
                            error!("{:?}: WebSocket read error: {} (type: {:?})", self.config.exchange, e, std::any::type_name_of_val(&e));
                            return Err(FeedError::ConnectionFailed(e.to_string()));
                        }
                        None => {
                            warn!("{:?}: WebSocket stream ended", self.config.exchange);
                            return Err(FeedError::Disconnected("Stream ended".to_string()));
                        }
                        Some(Ok(other)) => {
                            // Catch-all for unexpected message types
                            warn!("{:?}: Unexpected message type: {:?}", self.config.exchange, other);
                        }
                    }
                }
                _ = ping_timer.tick() => {
                    // Gate.io requires BOTH WebSocket protocol ping AND application-level ping
                    if self.config.exchange == Exchange::GateIO {
                        // 1. Send WebSocket protocol-level ping
                        if let Err(e) = write.send(Message::Ping(vec![])).await {
                            error!("Gate.io: Failed to send WS PING: {}", e);
                            return Err(FeedError::ConnectionFailed(format!("WS PING failed: {}", e)));
                        }

                        // 2. Send application-level ping (spot.ping channel)
                        let ping_msg = Self::create_gateio_ping();
                        if let Err(e) = write.send(Message::Text(ping_msg)).await {
                            error!("Gate.io: Failed to send app-level ping: {}", e);
                            return Err(FeedError::ConnectionFailed(format!("App ping failed: {}", e)));
                        }
                        last_ping_time = std::time::Instant::now();
                        awaiting_pong = true;
                        ping_sent_time = std::time::Instant::now();
                    } else if self.config.exchange == Exchange::Bybit {
                        // Bybit requires application-level ping: {"op": "ping"}
                        // The server responds with {"op": "pong", ...}
                        // Timeout is 10 minutes but we ping every 20s for safety
                        if let Err(e) = write.send(Message::Text(r#"{"op": "ping"}"#.to_string())).await {
                            error!("Bybit: Failed to send app-level ping: {}", e);
                            return Err(FeedError::ConnectionFailed(format!("App ping failed: {}", e)));
                        }
                        awaiting_pong = true;
                        ping_sent_time = std::time::Instant::now();
                    } else {
                        // Other exchanges use WebSocket protocol-level ping
                        if let Err(e) = write.send(Message::Ping(vec![])).await {
                            error!("{:?}: Failed to send PING: {}", self.config.exchange, e);
                            return Err(FeedError::ConnectionFailed(format!("PING failed: {}", e)));
                        }
                        awaiting_pong = true;
                        ping_sent_time = std::time::Instant::now();
                    }
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

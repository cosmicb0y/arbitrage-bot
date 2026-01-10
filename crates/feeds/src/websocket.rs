//! WebSocket client for exchange connections.

use crate::{FeedConfig, FeedError};
use arbitrage_core::Exchange;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
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
    /// Circuit breaker opened - connection attempts are being blocked
    /// due to repeated failures. The Duration indicates when retry will be attempted.
    CircuitBreakerOpen(Duration),
}

/// Circuit breaker state for preventing reconnection storms during extended outages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - connections allowed
    Closed,
    /// Blocking connections due to repeated failures
    Open,
    /// Testing if service has recovered
    HalfOpen,
}

/// Circuit breaker to prevent infinite reconnection attempts during extended outages.
/// Implements the circuit breaker pattern with three states:
/// - Closed: Normal operation, connections allowed
/// - Open: Blocking connections after threshold failures
/// - HalfOpen: Testing if service recovered after timeout
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
    /// Time to wait before transitioning from Open to HalfOpen
    pub open_timeout: Duration,
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
            open_timeout: Duration::from_secs(300), // 5 minutes
            failure_threshold: 10,
        }
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker with custom thresholds.
    pub fn new(failure_threshold: u32, open_timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
            open_timeout,
            failure_threshold,
        }
    }

    /// Check if a connection attempt should be allowed.
    /// Returns Ok(()) if allowed, Err with time until retry if blocked.
    pub fn should_allow_connection(&mut self) -> Result<(), Duration> {
        match self.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure {
                    let elapsed = last_failure.elapsed();
                    if elapsed >= self.open_timeout {
                        info!("Circuit breaker transitioning to HalfOpen state");
                        self.state = CircuitState::HalfOpen;
                        Ok(())
                    } else {
                        let remaining = self.open_timeout - elapsed;
                        Err(remaining)
                    }
                } else {
                    // No recorded failure, shouldn't happen but allow connection
                    self.state = CircuitState::Closed;
                    Ok(())
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    /// Record a successful connection.
    pub fn record_success(&mut self) {
        if self.state == CircuitState::HalfOpen {
            info!("Circuit breaker closing - connection succeeded in HalfOpen state");
        }
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.last_failure = None;
    }

    /// Record a connection failure.
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    warn!(
                        "Circuit breaker opening after {} consecutive failures",
                        self.failure_count
                    );
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Failed in half-open state, go back to open
                warn!("Circuit breaker reopening - connection failed in HalfOpen state");
                self.state = CircuitState::Open;
                self.failure_count = self.failure_threshold; // Prevent immediate retry
            }
            CircuitState::Open => {
                // Already open, just update failure time
            }
        }
    }

    /// Get current state.
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Get current failure count.
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }
}

/// WebSocket client for a single exchange connection.
pub struct WsClient {
    config: FeedConfig,
    tx: mpsc::Sender<WsMessage>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
}

impl WsClient {
    /// Create a new WebSocket client.
    pub fn new(config: FeedConfig, tx: mpsc::Sender<WsMessage>) -> Self {
        Self {
            config,
            tx,
            shutdown_rx: None,
        }
    }

    /// Attach a shutdown receiver for graceful shutdown support.
    /// When a message is received on this channel, the client will send a
    /// WebSocket Close frame and terminate cleanly.
    pub fn with_shutdown(mut self, rx: oneshot::Receiver<()>) -> Self {
        self.shutdown_rx = Some(rx);
        self
    }

    /// Connect and run the WebSocket client.
    pub async fn run(self, subscribe_msg: Option<String>) -> Result<(), FeedError> {
        let msgs = subscribe_msg.map(|m| vec![m]);
        self.run_with_messages(msgs).await
    }

    /// Calculate backoff delay with exponential growth and random jitter.
    /// Jitter (0-25% of base delay) prevents "thundering herd" problem when
    /// multiple connections fail and reconnect simultaneously.
    fn calculate_backoff_with_jitter(base_ms: u64, attempt: u32, max_ms: u64) -> u64 {
        let backoff_power = attempt.min(8); // max 2^8 = 256x base delay
        let exponential = base_ms.saturating_mul(1 << backoff_power);
        let capped = exponential.min(max_ms);
        // Add 0-25% random jitter to spread out reconnection attempts
        let jitter = (capped as f64 * rand::thread_rng().gen::<f64>() * 0.25) as u64;
        capped + jitter
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
        let _sent_count_clone = sent_count.clone();

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
                                // Gate.io spot.obu sends both snapshots (full=true) and deltas
                                if text.contains("\"channel\":\"spot.obu\"") {
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
                    if text.contains("\"channel\":\"spot.obu\"") {
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
    /// If a shutdown signal is received, the client will gracefully close the connection.
    /// Circuit breaker prevents infinite reconnection attempts during extended outages.
    pub async fn run_with_messages(mut self, subscribe_msgs: Option<Vec<String>>) -> Result<(), FeedError> {
        let mut reconnect_attempts = 0u32;
        let mut connection_start: Instant;
        let mut has_connected_once = false;

        // Circuit breaker to prevent infinite reconnection attempts
        let mut circuit_breaker = CircuitBreaker::default();

        // Take ownership of shutdown receiver for the reconnect loop
        let mut shutdown_rx = self.shutdown_rx.take();

        loop {
            // Check for shutdown signal before attempting connection
            if let Some(ref mut rx) = shutdown_rx {
                if rx.try_recv().is_ok() {
                    info!("{:?}: Shutdown requested, not reconnecting", self.config.exchange);
                    return Ok(());
                }
            }

            // Check circuit breaker before attempting connection
            if let Err(wait_time) = circuit_breaker.should_allow_connection() {
                warn!(
                    "{:?}: Circuit breaker OPEN - waiting {:?} before retry (failures: {})",
                    self.config.exchange,
                    wait_time,
                    circuit_breaker.failure_count()
                );
                let _ = self.tx.send(WsMessage::CircuitBreakerOpen(wait_time)).await;

                // Wait for circuit breaker timeout, but also check for shutdown signal
                if let Some(ref mut rx) = shutdown_rx {
                    tokio::select! {
                        _ = tokio::time::sleep(wait_time) => {}
                        _ = rx => {
                            info!("{:?}: Shutdown requested during circuit breaker wait", self.config.exchange);
                            return Ok(());
                        }
                    }
                } else {
                    tokio::time::sleep(wait_time).await;
                }
                continue;
            }

            connection_start = Instant::now();
            let is_reconnect = has_connected_once;

            match self.connect_and_handle(&subscribe_msgs, is_reconnect, &mut shutdown_rx).await {
                Ok(()) => {
                    debug!("WebSocket connection closed normally for {:?}", self.config.exchange);
                    break;
                }
                Err(e) => {
                    let connection_duration = connection_start.elapsed();
                    has_connected_once = true;

                    // Record failure for circuit breaker
                    circuit_breaker.record_failure();

                    // Reset reconnect counter if connection was stable (5+ minutes)
                    if connection_duration > Duration::from_secs(300) {
                        info!(
                            "{:?}: Connection was stable for {:?}, resetting reconnect counter",
                            self.config.exchange,
                            connection_duration
                        );
                        reconnect_attempts = 0;
                        // Also reset circuit breaker on stable connection
                        circuit_breaker.record_success();
                    }

                    reconnect_attempts = reconnect_attempts.saturating_add(1);

                    // Calculate delay with exponential backoff + jitter, capped at 5 minutes
                    // Jitter prevents "thundering herd" when multiple connections reconnect simultaneously
                    let delay_ms = Self::calculate_backoff_with_jitter(
                        self.config.reconnect_delay_ms,
                        reconnect_attempts,
                        300_000, // max 5 minutes
                    );

                    warn!(
                        "{:?}: WebSocket error after {:?}: {}. Reconnecting in {:.1}s (attempt #{}, circuit: {:?})",
                        self.config.exchange,
                        connection_duration,
                        e,
                        delay_ms as f64 / 1000.0,
                        reconnect_attempts,
                        circuit_breaker.state()
                    );

                    let _ = self.tx.send(WsMessage::Disconnected).await;

                    // Wait for backoff delay, but also check for shutdown signal
                    if let Some(ref mut rx) = shutdown_rx {
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
                            _ = rx => {
                                info!("{:?}: Shutdown requested during backoff", self.config.exchange);
                                return Ok(());
                            }
                        }
                    } else {
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn connect_and_handle(
        &self,
        subscribe_msgs: &Option<Vec<String>>,
        is_reconnect: bool,
        shutdown_rx: &mut Option<oneshot::Receiver<()>>,
    ) -> Result<(), FeedError> {
        let is_gateio = self.config.exchange == Exchange::GateIO;
        debug!("Connecting to {:?}: {}", self.config.exchange, self.config.ws_url);

        // Apply connection timeout to prevent indefinite hangs
        let connect_timeout = Duration::from_millis(self.config.connect_timeout_ms);
        let (ws_stream, response) = tokio::time::timeout(connect_timeout, connect_async(&self.config.ws_url))
            .await
            .map_err(|_| {
                FeedError::Timeout(format!(
                    "Connection to {:?} timed out after {}ms",
                    self.config.exchange, self.config.connect_timeout_ms
                ))
            })??;
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

            // Build the select with optional shutdown handling
            // Using a macro-like approach to handle Option<Receiver>
            let shutdown_triggered = if let Some(ref mut rx) = shutdown_rx {
                tokio::select! {
                    msg = read.next() => {
                        Self::handle_ws_message(
                            &self.config.exchange,
                            &self.tx,
                            msg,
                            &mut message_count,
                            &mut last_message_time,
                            &mut awaiting_pong,
                            &ping_sent_time,
                            &last_ping_time,
                            &mut write,
                        ).await?;
                        false
                    }
                    _ = ping_timer.tick() => {
                        Self::handle_ping(
                            &self.config.exchange,
                            &mut write,
                            &mut awaiting_pong,
                            &mut ping_sent_time,
                            &mut last_ping_time,
                        ).await?;
                        false
                    }
                    _ = rx => {
                        true
                    }
                }
            } else {
                tokio::select! {
                    msg = read.next() => {
                        Self::handle_ws_message(
                            &self.config.exchange,
                            &self.tx,
                            msg,
                            &mut message_count,
                            &mut last_message_time,
                            &mut awaiting_pong,
                            &ping_sent_time,
                            &last_ping_time,
                            &mut write,
                        ).await?;
                        false
                    }
                    _ = ping_timer.tick() => {
                        Self::handle_ping(
                            &self.config.exchange,
                            &mut write,
                            &mut awaiting_pong,
                            &mut ping_sent_time,
                            &mut last_ping_time,
                        ).await?;
                        false
                    }
                }
            };

            if shutdown_triggered {
                info!("{:?}: Graceful shutdown requested, sending Close frame", self.config.exchange);
                let _ = write.send(Message::Close(None)).await;
                return Ok(());
            }
        }
    }

    /// Handle incoming WebSocket message
    async fn handle_ws_message<S>(
        exchange: &Exchange,
        tx: &mpsc::Sender<WsMessage>,
        msg: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
        message_count: &mut u64,
        last_message_time: &mut std::time::Instant,
        awaiting_pong: &mut bool,
        ping_sent_time: &std::time::Instant,
        last_ping_time: &std::time::Instant,
        write: &mut S,
    ) -> Result<(), FeedError>
    where
        S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        // Update last message time for any received message
        *last_message_time = std::time::Instant::now();

        match msg {
            Some(Ok(Message::Text(text))) => {
                *message_count += 1;

                // Gate.io debugging
                if *exchange == Exchange::GateIO {
                    // Log errors only
                    if text.contains("error") {
                        warn!("Gate.io msg #{}: {}", message_count, &text[..text.len().min(200)]);
                    }
                }

                // Handle Gate.io application-level pong response (ignore it)
                if *exchange == Exchange::GateIO && text.contains("\"channel\":\"spot.pong\"") {
                    debug!("Gate.io: Received pong response (latency: {:?})", last_ping_time.elapsed());
                    return Ok(());
                }

                // Handle Bybit application-level pong response
                // Bybit responds with {"success":true,"ret_msg":"pong","conn_id":"...","op":"pong"}
                if *exchange == Exchange::Bybit && text.contains("\"op\":\"pong\"") {
                    *awaiting_pong = false;
                    debug!("Bybit: Received pong response (latency: {:?})", ping_sent_time.elapsed());
                    return Ok(());
                }

                // Handle Coinbase heartbeats channel messages (connection keep-alive)
                // These are sent by the server every ~1s when subscribed to heartbeats channel
                if *exchange == Exchange::Coinbase && text.contains("\"channel\":\"heartbeats\"") {
                    // Heartbeat received - connection is alive
                    // No need to track awaiting_pong since Coinbase sends these automatically
                    return Ok(());
                }
                // Use try_send to avoid blocking on channel full
                // If channel is full, force reconnection to resync orderbook
                // (dropping messages would break delta-based orderbook sync)
                if let Err(e) = tx.try_send(WsMessage::Text(text)) {
                    match e {
                        mpsc::error::TrySendError::Full(_) => {
                            // Channel full - reconnect to resync orderbook
                            // This is critical for delta-based exchanges (Bybit, Coinbase)
                            warn!("{:?}: Channel full, forcing reconnect to resync orderbook", exchange);
                            return Err(FeedError::Disconnected("Channel full - resync needed".to_string()));
                        }
                        mpsc::error::TrySendError::Closed(_) => {
                            error!("{:?}: Channel closed", exchange);
                            return Err(FeedError::Disconnected("Channel closed".to_string()));
                        }
                    }
                }
            }
            Some(Ok(Message::Binary(data))) => {
                *message_count += 1;
                // Use try_send for binary data too - reconnect if full
                if let Err(mpsc::error::TrySendError::Full(_)) = tx.try_send(WsMessage::Binary(data.clone())) {
                    warn!("{:?}: Channel full (binary), forcing reconnect", exchange);
                    return Err(FeedError::Disconnected("Channel full - resync needed".to_string()));
                }
            }
            Some(Ok(Message::Ping(data))) => {
                debug!("{:?}: Received WebSocket PING, sending PONG", exchange);
                if let Err(e) = write.send(Message::Pong(data)).await {
                    error!("{:?}: Failed to send PONG: {}", exchange, e);
                    return Err(FeedError::ConnectionFailed(format!("PONG send failed: {}", e)));
                }
            }
            Some(Ok(Message::Pong(_))) => {
                // Pong received, connection is alive
                *awaiting_pong = false;
                debug!("{:?}: Received WebSocket PONG (latency: {:?})", exchange, ping_sent_time.elapsed());
            }
            Some(Ok(Message::Close(frame))) => {
                debug!("{:?}: Received close frame: {:?}", exchange, frame);
                // Return a special marker - we use Ok but caller should handle graceful close
                return Err(FeedError::Disconnected("Server closed connection".to_string()));
            }
            Some(Err(e)) => {
                error!("{:?}: WebSocket read error: {}", exchange, e);
                return Err(FeedError::ConnectionFailed(e.to_string()));
            }
            None => {
                warn!("{:?}: WebSocket stream ended", exchange);
                return Err(FeedError::Disconnected("Stream ended".to_string()));
            }
            Some(Ok(other)) => {
                // Catch-all for unexpected message types
                warn!("{:?}: Unexpected message type: {:?}", exchange, other);
            }
        }
        Ok(())
    }

    /// Handle ping timer tick
    async fn handle_ping<S>(
        exchange: &Exchange,
        write: &mut S,
        awaiting_pong: &mut bool,
        ping_sent_time: &mut std::time::Instant,
        last_ping_time: &mut std::time::Instant,
    ) -> Result<(), FeedError>
    where
        S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        // Gate.io requires BOTH WebSocket protocol ping AND application-level ping
        if *exchange == Exchange::GateIO {
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
            *last_ping_time = std::time::Instant::now();
            *awaiting_pong = true;
            *ping_sent_time = std::time::Instant::now();
        } else if *exchange == Exchange::Bybit {
            // Bybit requires application-level ping: {"op": "ping"}
            // The server responds with {"op": "pong", ...}
            // Timeout is 10 minutes but we ping every 20s for safety
            if let Err(e) = write.send(Message::Text(r#"{"op": "ping"}"#.to_string())).await {
                error!("Bybit: Failed to send app-level ping: {}", e);
                return Err(FeedError::ConnectionFailed(format!("App ping failed: {}", e)));
            }
            *awaiting_pong = true;
            *ping_sent_time = std::time::Instant::now();
        } else {
            // Other exchanges use WebSocket protocol-level ping
            if let Err(e) = write.send(Message::Ping(vec![])).await {
                error!("{:?}: Failed to send PING: {}", exchange, e);
                return Err(FeedError::ConnectionFailed(format!("PING failed: {}", e)));
            }
            *awaiting_pong = true;
            *ping_sent_time = std::time::Instant::now();
        }
        Ok(())
    }
}

/// Multi-exchange WebSocket manager.
pub struct WsManager {
    #[allow(dead_code)]
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

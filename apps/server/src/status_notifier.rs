//! Status notifier for WebSocket connection events.
//!
//! Sends Telegram notifications for connection status changes:
//! - WebSocket disconnections
//! - WebSocket reconnections
//! - Circuit breaker events

use arbitrage_core::Exchange;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Status event types for notification.
#[derive(Debug, Clone)]
pub enum StatusEvent {
    /// WebSocket connected (initial connection)
    Connected(Exchange),
    /// WebSocket disconnected
    Disconnected(Exchange),
    /// WebSocket reconnected after disconnection
    Reconnected(Exchange),
    /// Circuit breaker opened - connection attempts blocked
    CircuitBreakerOpen(Exchange, Duration),
    /// Server started
    ServerStarted,
    /// Server stopping
    ServerStopping,
}

/// Configuration for status notifications.
#[derive(Debug, Clone)]
pub struct StatusNotifierConfig {
    /// Telegram bot token for status notifications
    pub bot_token: String,
    /// Telegram chat ID to send notifications to
    pub chat_id: String,
    /// Whether to send notifications on connect events
    pub notify_on_connect: bool,
    /// Whether to send notifications on disconnect events
    pub notify_on_disconnect: bool,
    /// Whether to send notifications on reconnect events
    pub notify_on_reconnect: bool,
    /// Whether to send notifications on circuit breaker events
    pub notify_on_circuit_breaker: bool,
}

impl StatusNotifierConfig {
    /// Create config from environment variables.
    /// Uses TELEGRAM_STATUS_BOT_TOKEN and TELEGRAM_STATUS_CHAT_ID.
    pub fn from_env() -> Option<Self> {
        let bot_token = std::env::var("TELEGRAM_STATUS_BOT_TOKEN").ok()?;
        let chat_id = std::env::var("TELEGRAM_STATUS_CHAT_ID").ok()?;

        if bot_token.is_empty() || chat_id.is_empty() {
            return None;
        }

        Some(Self {
            bot_token,
            chat_id,
            notify_on_connect: false, // Usually too noisy
            notify_on_disconnect: true,
            notify_on_reconnect: true,
            notify_on_circuit_breaker: true,
        })
    }
}

/// Status notifier that sends Telegram messages for connection events.
pub struct StatusNotifier {
    config: StatusNotifierConfig,
    http_client: reqwest::Client,
    hostname: String,
}

impl StatusNotifier {
    /// Create a new status notifier.
    pub fn new(config: StatusNotifierConfig) -> Self {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            config,
            http_client: reqwest::Client::new(),
            hostname,
        }
    }

    /// Send a status notification.
    pub async fn notify(&self, event: &StatusEvent) {
        let message = match event {
            StatusEvent::Connected(exchange) => {
                if !self.config.notify_on_connect {
                    return;
                }
                format!("✅ <b>{:?}</b> WebSocket connected", exchange)
            }
            StatusEvent::Disconnected(exchange) => {
                if !self.config.notify_on_disconnect {
                    return;
                }
                format!("⚠️ <b>{:?}</b> WebSocket disconnected", exchange)
            }
            StatusEvent::Reconnected(exchange) => {
                if !self.config.notify_on_reconnect {
                    return;
                }
                format!("🔄 <b>{:?}</b> WebSocket reconnected", exchange)
            }
            StatusEvent::CircuitBreakerOpen(exchange, wait_time) => {
                if !self.config.notify_on_circuit_breaker {
                    return;
                }
                format!(
                    "🚫 <b>{:?}</b> Circuit breaker OPEN\nRetry in {:?}",
                    exchange, wait_time
                )
            }
            StatusEvent::ServerStarted | StatusEvent::ServerStopping => {
                // Don't send notifications for server start/stop
                return;
            }
        };

        // Add hostname and timestamp
        let full_message = match event {
            StatusEvent::ServerStarted | StatusEvent::ServerStopping => message,
            _ => {
                let now = chrono::Utc::now();
                format!(
                    "<b>{}</b>\n{}\n\n⏰ {}",
                    self.hostname,
                    message,
                    now.format("%Y-%m-%d %H:%M:%S UTC")
                )
            }
        };

        if let Err(e) = self.send_telegram_message(&full_message).await {
            error!("Failed to send status notification: {}", e);
        }
    }

    /// Send a message via Telegram Bot API.
    async fn send_telegram_message(&self, message: &str) -> Result<(), reqwest::Error> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.config.bot_token
        );

        let params = [
            ("chat_id", self.config.chat_id.as_str()),
            ("text", message),
            ("parse_mode", "HTML"),
            ("disable_web_page_preview", "true"),
        ];

        let response = self.http_client.post(&url).form(&params).send().await?;

        if !response.status().is_success() {
            warn!(
                "Telegram API returned non-success status: {}",
                response.status()
            );
        }

        Ok(())
    }
}

/// Shared status notifier handle for sending events from multiple tasks.
#[derive(Clone)]
pub struct StatusNotifierHandle {
    tx: mpsc::Sender<StatusEvent>,
}

impl StatusNotifierHandle {
    /// Send a status event.
    pub async fn send(&self, event: StatusEvent) {
        if let Err(e) = self.tx.send(event).await {
            warn!("Failed to send status event: {}", e);
        }
    }

    /// Send a status event (non-blocking).
    pub fn try_send(&self, event: StatusEvent) {
        if let Err(e) = self.tx.try_send(event) {
            warn!("Failed to send status event (try_send): {}", e);
        }
    }
}

/// Start the status notifier background task.
/// Returns a handle that can be cloned and used to send events.
pub fn start_status_notifier(config: StatusNotifierConfig) -> StatusNotifierHandle {
    let (tx, mut rx) = mpsc::channel::<StatusEvent>(100);

    let notifier = Arc::new(StatusNotifier::new(config));

    tokio::spawn(async move {
        info!("Status notifier started");

        while let Some(event) = rx.recv().await {
            notifier.notify(&event).await;
        }

        info!("Status notifier stopped");
    });

    StatusNotifierHandle { tx }
}

/// Try to create and start a status notifier from environment variables.
/// Returns None if the required environment variables are not set.
pub fn try_start_status_notifier() -> Option<StatusNotifierHandle> {
    match StatusNotifierConfig::from_env() {
        Some(config) => {
            info!(
                "Status notifier enabled (chat_id: {})",
                &config.chat_id[..config.chat_id.len().min(6)]
            );
            Some(start_status_notifier(config))
        }
        None => {
            info!("Status notifier disabled (TELEGRAM_STATUS_BOT_TOKEN or TELEGRAM_STATUS_CHAT_ID not set)");
            None
        }
    }
}

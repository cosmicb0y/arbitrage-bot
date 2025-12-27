//! Error types for feed operations.

use thiserror::Error;

/// Errors that can occur during feed operations.
#[derive(Debug, Error)]
pub enum FeedError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(String),

    #[error("WebSocket disconnected: {0}")]
    Disconnected(String),

    #[error("Failed to parse message: {0}")]
    ParseError(String),

    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("Exchange not supported: {0}")]
    UnsupportedExchange(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Channel closed")]
    ChannelClosed,
}

impl From<tokio_tungstenite::tungstenite::Error> for FeedError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        FeedError::ConnectionFailed(err.to_string())
    }
}

impl From<serde_json::Error> for FeedError {
    fn from(err: serde_json::Error) -> Self {
        FeedError::ParseError(err.to_string())
    }
}

impl From<url::ParseError> for FeedError {
    fn from(err: url::ParseError) -> Self {
        FeedError::ConnectionFailed(err.to_string())
    }
}

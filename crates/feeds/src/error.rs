//! Error types for feed operations.

use std::time::Duration;
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

impl FeedError {
    /// Returns true if this error is transient and likely to succeed on retry.
    /// Use this to decide whether to retry the operation or escalate.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            FeedError::ConnectionFailed(_)
                | FeedError::Disconnected(_)
                | FeedError::Timeout(_)
                | FeedError::RateLimitExceeded
        )
    }

    /// Returns true if this error is permanent and requires manual intervention.
    /// Operations with permanent errors should not be retried automatically.
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            FeedError::AuthenticationFailed(_) | FeedError::UnsupportedExchange(_)
        )
    }

    /// Returns a suggested retry delay for this error type, if applicable.
    /// Returns None for permanent errors that should not be retried.
    pub fn suggested_retry_delay(&self) -> Option<Duration> {
        match self {
            FeedError::RateLimitExceeded => Some(Duration::from_secs(60)),
            FeedError::ConnectionFailed(_) => Some(Duration::from_secs(5)),
            FeedError::Disconnected(_) => Some(Duration::from_secs(2)),
            FeedError::Timeout(_) => Some(Duration::from_secs(2)),
            FeedError::SubscriptionFailed(_) => Some(Duration::from_secs(5)),
            // Permanent errors - no retry
            FeedError::AuthenticationFailed(_)
            | FeedError::UnsupportedExchange(_)
            | FeedError::ParseError(_)
            | FeedError::ChannelClosed => None,
        }
    }
}

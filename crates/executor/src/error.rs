//! Error types for execution operations.

use thiserror::Error;

/// Errors that can occur during trade execution.
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Order submission failed: {0}")]
    SubmissionFailed(String),

    #[error("Order rejected: {0}")]
    OrderRejected(String),

    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance { needed: u64, available: u64 },

    #[error("Order timeout: {0}")]
    Timeout(String),

    #[error("Order cancelled: {0}")]
    Cancelled(String),

    #[error("Exchange error: {0}")]
    ExchangeError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid order parameters: {0}")]
    InvalidParameters(String),

    #[error("Slippage exceeded: expected {expected_bps} bps, got {actual_bps} bps")]
    SlippageExceeded { expected_bps: u16, actual_bps: u16 },

    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Result type for executor operations.
pub type ExecutorResult<T> = Result<T, ExecutorError>;

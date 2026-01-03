//! Alert configuration types.

use serde::{Deserialize, Serialize};

/// User alert configuration stored in database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Database ID
    pub id: i64,
    /// Telegram chat ID for notifications
    pub telegram_chat_id: String,
    /// Minimum premium in basis points to trigger alert
    pub min_premium_bps: i32,
    /// Symbols to monitor (empty = all)
    pub symbols: Vec<String>,
    /// Symbols to exclude from alerts (blacklist)
    pub excluded_symbols: Vec<String>,
    /// Exchanges to monitor (empty = all)
    pub exchanges: Vec<String>,
    /// Whether alerts are enabled
    pub enabled: bool,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            id: 0,
            telegram_chat_id: String::new(),
            min_premium_bps: 400,
            symbols: Vec::new(),
            excluded_symbols: Vec::new(),
            exchanges: Vec::new(),
            enabled: true,
        }
    }
}

impl AlertConfig {
    /// Create a new config for a chat.
    pub fn new(chat_id: impl Into<String>) -> Self {
        Self {
            telegram_chat_id: chat_id.into(),
            ..Default::default()
        }
    }

    /// Check if a symbol should trigger alerts.
    /// Returns false if symbol is in excluded list.
    /// Returns true if symbols list is empty (all allowed) or symbol is in allowed list.
    pub fn should_alert_symbol(&self, symbol: &str) -> bool {
        // Check blacklist first
        if self.excluded_symbols.iter().any(|s| s.eq_ignore_ascii_case(symbol)) {
            return false;
        }
        // Check whitelist (empty = all allowed)
        self.symbols.is_empty() || self.symbols.iter().any(|s| s.eq_ignore_ascii_case(symbol))
    }

    /// Check if an exchange should trigger alerts.
    pub fn should_alert_exchange(&self, exchange: &str) -> bool {
        self.exchanges.is_empty()
            || self
                .exchanges
                .iter()
                .any(|e| e.eq_ignore_ascii_case(exchange))
    }
}

/// Alert history entry for deduplication.
#[derive(Debug, Clone)]
pub struct AlertHistory {
    pub id: i64,
    pub symbol: String,
    pub source_exchange: String,
    pub target_exchange: String,
    pub premium_bps: i32,
    pub source_price: f64,
    pub target_price: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

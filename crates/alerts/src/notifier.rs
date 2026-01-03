//! Alert notification logic.

use crate::config::AlertHistory;
use crate::db::Database;
use crate::telegram::{format_alert_message, TelegramBot};
pub use crate::telegram::ExchangeRates;
use arbitrage_core::{ArbitrageOpportunity, FixedPoint};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum NotifierError {
    #[error("Database error: {0}")]
    Db(#[from] crate::db::DbError),
    #[error("Telegram error: {0}")]
    Telegram(#[from] crate::telegram::TelegramError),
}

/// Function type to check if a transfer path exists between exchanges.
pub type TransferPathChecker = Box<dyn Fn(&str, &str, &str) -> bool + Send + Sync>;

/// Configuration for the notifier.
#[derive(Clone)]
pub struct NotifierConfig {
    /// Cooldown in minutes before sending another alert for the same opportunity.
    pub cooldown_minutes: i64,
    /// Days to keep alert history.
    pub history_retention_days: i64,
    /// Only send alerts for opportunities with valid transfer paths.
    pub require_transfer_path: bool,
}

impl std::fmt::Debug for NotifierConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotifierConfig")
            .field("cooldown_minutes", &self.cooldown_minutes)
            .field("history_retention_days", &self.history_retention_days)
            .field("require_transfer_path", &self.require_transfer_path)
            .finish()
    }
}

impl Default for NotifierConfig {
    fn default() -> Self {
        Self {
            cooldown_minutes: 5,
            history_retention_days: 30,
            require_transfer_path: true,
        }
    }
}

/// Alert notifier that sends Telegram notifications.
pub struct Notifier {
    db: Database,
    bot: Arc<TelegramBot>,
    config: NotifierConfig,
    transfer_path_checker: Option<TransferPathChecker>,
    exchange_rates: std::sync::RwLock<ExchangeRates>,
}

impl Notifier {
    /// Create a new notifier.
    pub fn new(db: Database, bot: Arc<TelegramBot>, config: NotifierConfig) -> Self {
        Self {
            db,
            bot,
            config,
            transfer_path_checker: None,
            exchange_rates: std::sync::RwLock::new(ExchangeRates::default()),
        }
    }

    /// Set the transfer path checker function.
    pub fn with_transfer_path_checker(mut self, checker: TransferPathChecker) -> Self {
        self.transfer_path_checker = Some(checker);
        self
    }

    /// Update exchange rates for raw price display.
    pub fn update_exchange_rates(&self, rates: ExchangeRates) {
        if let Ok(mut lock) = self.exchange_rates.write() {
            *lock = rates;
        }
    }

    /// Process an arbitrage opportunity and send alerts if needed.
    pub async fn process_opportunity(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<u32, NotifierError> {
        let symbol = opportunity.asset.symbol.as_str();
        let source_exchange = format!("{:?}", opportunity.source_exchange);
        let target_exchange = format!("{:?}", opportunity.target_exchange);
        let premium_bps = opportunity.premium_bps;

        // Skip if depth is not available on both sides
        if opportunity.source_depth == 0 || opportunity.target_depth == 0 {
            debug!(
                symbol = symbol,
                source = source_exchange,
                target = target_exchange,
                source_depth = opportunity.source_depth,
                target_depth = opportunity.target_depth,
                "Skipping alert: depth not available on both sides"
            );
            return Ok(0);
        }

        // Check transfer path if configured
        if self.config.require_transfer_path {
            if let Some(ref checker) = self.transfer_path_checker {
                if !checker(symbol, &source_exchange, &target_exchange) {
                    debug!(
                        symbol = symbol,
                        source = source_exchange,
                        target = target_exchange,
                        "Skipping alert: no transfer path available"
                    );
                    return Ok(0);
                }
            }
        }

        // Check if this opportunity is already active (still above threshold)
        // Only send alert when opportunity first crosses threshold, not while it remains above
        let is_already_active = self
            .db
            .is_opportunity_active(symbol, &source_exchange, &target_exchange)
            .await?;

        if is_already_active {
            // Update the last_seen timestamp but don't send alert
            let _ = self
                .db
                .mark_opportunity_active(symbol, &source_exchange, &target_exchange, premium_bps)
                .await;
            debug!(
                symbol = symbol,
                source = source_exchange,
                target = target_exchange,
                premium_bps = premium_bps,
                "Skipping alert: opportunity still active"
            );
            return Ok(0);
        }

        // Get all enabled configs that match this opportunity
        let configs = self.db.get_all_enabled_configs().await?;
        let mut sent_count = 0u32;

        for config in configs {
            // Check if opportunity meets config criteria
            if premium_bps < config.min_premium_bps {
                continue;
            }

            if !config.should_alert_symbol(symbol) {
                continue;
            }

            if !config.should_alert_exchange(&source_exchange)
                && !config.should_alert_exchange(&target_exchange)
            {
                continue;
            }

            // Format and send alert
            let source_price = FixedPoint(opportunity.source_price).to_f64();
            let target_price = FixedPoint(opportunity.target_price).to_f64();
            let source_depth = if opportunity.source_depth > 0 {
                Some(FixedPoint(opportunity.source_depth).to_f64())
            } else {
                None
            };
            let target_depth = if opportunity.target_depth > 0 {
                Some(FixedPoint(opportunity.target_depth).to_f64())
            } else {
                None
            };

            let source_quote = format!("{:?}", opportunity.source_quote);
            let target_quote = format!("{:?}", opportunity.target_quote);

            // Get current exchange rates for raw price display
            let rates = self.exchange_rates.read().ok().map(|r| r.clone());

            let message = format_alert_message(
                symbol,
                &source_exchange,
                &target_exchange,
                &source_quote,
                &target_quote,
                source_price,
                target_price,
                premium_bps,
                source_depth,
                target_depth,
                rates.as_ref(),
            );

            match self.bot.send_alert(&config.telegram_chat_id, &message).await {
                Ok(_) => {
                    info!(
                        chat_id = config.telegram_chat_id,
                        symbol = symbol,
                        premium_bps = premium_bps,
                        "Alert sent"
                    );
                    sent_count += 1;
                }
                Err(e) => {
                    error!(
                        chat_id = config.telegram_chat_id,
                        error = %e,
                        "Failed to send alert"
                    );
                }
            }
        }

        // Record in history and mark as active (only if we sent at least one alert)
        if sent_count > 0 {
            let history = AlertHistory {
                id: 0,
                symbol: symbol.to_string(),
                source_exchange: source_exchange.clone(),
                target_exchange: target_exchange.clone(),
                premium_bps,
                source_price: FixedPoint(opportunity.source_price).to_f64(),
                target_price: FixedPoint(opportunity.target_price).to_f64(),
                created_at: chrono::Utc::now(),
            };
            self.db.record_alert(&history).await?;

            // Mark this opportunity as active so we don't send duplicate alerts
            self.db
                .mark_opportunity_active(symbol, &source_exchange, &target_exchange, premium_bps)
                .await?;
        }

        Ok(sent_count)
    }

    /// Clean up old history entries and stale active opportunities.
    pub async fn cleanup(&self) -> Result<(u64, u64), NotifierError> {
        let history_deleted = self
            .db
            .cleanup_old_history(self.config.history_retention_days)
            .await?;
        if history_deleted > 0 {
            info!(deleted = history_deleted, "Cleaned up old alert history");
        }

        // Clean up opportunities not seen in last 10 minutes (they likely fell below threshold)
        let opportunities_deleted = self.db.cleanup_stale_opportunities(10).await?;
        if opportunities_deleted > 0 {
            info!(
                deleted = opportunities_deleted,
                "Cleaned up stale active opportunities"
            );
        }

        Ok((history_deleted, opportunities_deleted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notifier_config_default() {
        let config = NotifierConfig::default();
        assert_eq!(config.cooldown_minutes, 5);
        assert_eq!(config.history_retention_days, 30);
    }
}

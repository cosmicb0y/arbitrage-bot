//! SQLite database for alert configuration and history.

use crate::config::{AlertConfig, AlertHistory};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Configuration not found for chat: {0}")]
    ConfigNotFound(String),
}

/// Database connection for alerts.
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Connect to SQLite database at the given path.
    pub async fn connect(database_url: &str) -> Result<Self, DbError> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> Result<(), DbError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS alert_config (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                telegram_chat_id TEXT NOT NULL UNIQUE,
                min_premium_bps INTEGER NOT NULL DEFAULT 400,
                symbols TEXT NOT NULL DEFAULT '[]',
                excluded_symbols TEXT NOT NULL DEFAULT '[]',
                exchanges TEXT NOT NULL DEFAULT '[]',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migration: add excluded_symbols column if it doesn't exist
        let _ = sqlx::query(
            "ALTER TABLE alert_config ADD COLUMN excluded_symbols TEXT NOT NULL DEFAULT '[]'",
        )
        .execute(&self.pool)
        .await;

        // Migration: add min_profit_usd column if it doesn't exist
        let _ = sqlx::query(
            "ALTER TABLE alert_config ADD COLUMN min_profit_usd REAL NOT NULL DEFAULT 0.0",
        )
        .execute(&self.pool)
        .await;

        // Active opportunities table - tracks currently active opportunities to avoid duplicate alerts
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS active_opportunities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                source_exchange TEXT NOT NULL,
                target_exchange TEXT NOT NULL,
                last_premium_bps INTEGER NOT NULL,
                first_seen_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_seen_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(symbol, source_exchange, target_exchange)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS alert_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                source_exchange TEXT NOT NULL,
                target_exchange TEXT NOT NULL,
                premium_bps INTEGER NOT NULL,
                source_price REAL NOT NULL,
                target_price REAL NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_recent_alerts
            ON alert_history(symbol, source_exchange, target_exchange, created_at)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get or create config for a chat.
    pub async fn get_or_create_config(&self, chat_id: &str) -> Result<AlertConfig, DbError> {
        let existing = sqlx::query_as::<_, (i64, String, i32, f64, String, String, String, bool)>(
            "SELECT id, telegram_chat_id, min_premium_bps, min_profit_usd, symbols, excluded_symbols, exchanges, enabled FROM alert_config WHERE telegram_chat_id = ?",
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((
            id,
            telegram_chat_id,
            min_premium_bps,
            min_profit_usd,
            symbols_json,
            excluded_symbols_json,
            exchanges_json,
            enabled,
        )) = existing
        {
            let symbols: Vec<String> = serde_json::from_str(&symbols_json).unwrap_or_default();
            let excluded_symbols: Vec<String> =
                serde_json::from_str(&excluded_symbols_json).unwrap_or_default();
            let exchanges: Vec<String> = serde_json::from_str(&exchanges_json).unwrap_or_default();

            return Ok(AlertConfig {
                id,
                telegram_chat_id,
                min_premium_bps,
                min_profit_usd,
                symbols,
                excluded_symbols,
                exchanges,
                enabled,
            });
        }

        let result = sqlx::query("INSERT INTO alert_config (telegram_chat_id) VALUES (?)")
            .bind(chat_id)
            .execute(&self.pool)
            .await?;

        Ok(AlertConfig {
            id: result.last_insert_rowid(),
            telegram_chat_id: chat_id.to_string(),
            ..Default::default()
        })
    }

    /// Update config.
    pub async fn update_config(&self, config: &AlertConfig) -> Result<(), DbError> {
        let symbols_json = serde_json::to_string(&config.symbols).unwrap_or_default();
        let excluded_symbols_json =
            serde_json::to_string(&config.excluded_symbols).unwrap_or_default();
        let exchanges_json = serde_json::to_string(&config.exchanges).unwrap_or_default();

        sqlx::query(
            r#"
            UPDATE alert_config
            SET min_premium_bps = ?, min_profit_usd = ?, symbols = ?, excluded_symbols = ?, exchanges = ?, enabled = ?
            WHERE telegram_chat_id = ?
            "#,
        )
        .bind(config.min_premium_bps)
        .bind(config.min_profit_usd)
        .bind(&symbols_json)
        .bind(&excluded_symbols_json)
        .bind(&exchanges_json)
        .bind(config.enabled)
        .bind(&config.telegram_chat_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all enabled configs.
    pub async fn get_all_enabled_configs(&self) -> Result<Vec<AlertConfig>, DbError> {
        let rows = sqlx::query_as::<_, (i64, String, i32, f64, String, String, String, bool)>(
            "SELECT id, telegram_chat_id, min_premium_bps, min_profit_usd, symbols, excluded_symbols, exchanges, enabled FROM alert_config WHERE enabled = 1",
        )
        .fetch_all(&self.pool)
        .await?;

        let configs = rows
            .into_iter()
            .map(
                |(
                    id,
                    telegram_chat_id,
                    min_premium_bps,
                    min_profit_usd,
                    symbols_json,
                    excluded_symbols_json,
                    exchanges_json,
                    enabled,
                )| {
                    let symbols: Vec<String> =
                        serde_json::from_str(&symbols_json).unwrap_or_default();
                    let excluded_symbols: Vec<String> =
                        serde_json::from_str(&excluded_symbols_json).unwrap_or_default();
                    let exchanges: Vec<String> =
                        serde_json::from_str(&exchanges_json).unwrap_or_default();
                    AlertConfig {
                        id,
                        telegram_chat_id,
                        min_premium_bps,
                        min_profit_usd,
                        symbols,
                        excluded_symbols,
                        exchanges,
                        enabled,
                    }
                },
            )
            .collect();

        Ok(configs)
    }

    /// Check if this opportunity is already active (above threshold).
    /// Returns true if the opportunity was already tracked and still active.
    /// This prevents duplicate alerts while an opportunity remains above threshold.
    pub async fn is_opportunity_active(
        &self,
        symbol: &str,
        source_exchange: &str,
        target_exchange: &str,
    ) -> Result<bool, DbError> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM active_opportunities
            WHERE symbol = ? AND source_exchange = ? AND target_exchange = ?
            "#,
        )
        .bind(symbol)
        .bind(source_exchange)
        .bind(target_exchange)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists > 0)
    }

    /// Mark an opportunity as active (first time above threshold).
    pub async fn mark_opportunity_active(
        &self,
        symbol: &str,
        source_exchange: &str,
        target_exchange: &str,
        premium_bps: i32,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO active_opportunities (symbol, source_exchange, target_exchange, last_premium_bps)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(symbol, source_exchange, target_exchange)
            DO UPDATE SET last_premium_bps = ?, last_seen_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(symbol)
        .bind(source_exchange)
        .bind(target_exchange)
        .bind(premium_bps)
        .bind(premium_bps)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove an opportunity from active tracking (fell below threshold).
    pub async fn mark_opportunity_inactive(
        &self,
        symbol: &str,
        source_exchange: &str,
        target_exchange: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            DELETE FROM active_opportunities
            WHERE symbol = ? AND source_exchange = ? AND target_exchange = ?
            "#,
        )
        .bind(symbol)
        .bind(source_exchange)
        .bind(target_exchange)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all currently active opportunities.
    pub async fn get_all_active_opportunities(
        &self,
    ) -> Result<Vec<(String, String, String)>, DbError> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT symbol, source_exchange, target_exchange FROM active_opportunities",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Clean up stale active opportunities (not seen for a while).
    pub async fn cleanup_stale_opportunities(&self, minutes: i64) -> Result<u64, DbError> {
        let result = sqlx::query(
            "DELETE FROM active_opportunities WHERE last_seen_at < datetime('now', ? || ' minutes')",
        )
        .bind(-minutes)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Record an alert in history.
    pub async fn record_alert(&self, history: &AlertHistory) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO alert_history (symbol, source_exchange, target_exchange, premium_bps, source_price, target_price)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&history.symbol)
        .bind(&history.source_exchange)
        .bind(&history.target_exchange)
        .bind(history.premium_bps)
        .bind(history.source_price)
        .bind(history.target_price)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clean up old history entries (older than days).
    pub async fn cleanup_old_history(&self, days: i64) -> Result<u64, DbError> {
        let result = sqlx::query(
            "DELETE FROM alert_history WHERE created_at < datetime('now', ? || ' days')",
        )
        .bind(-days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connect() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let config = db.get_or_create_config("123456").await.unwrap();
        assert_eq!(config.telegram_chat_id, "123456");
        assert_eq!(config.min_premium_bps, 400);
    }

    #[tokio::test]
    async fn test_update_config() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let mut config = db.get_or_create_config("123456").await.unwrap();

        config.min_premium_bps = 100;
        config.symbols = vec!["BTC".to_string(), "ETH".to_string()];
        db.update_config(&config).await.unwrap();

        let updated = db.get_or_create_config("123456").await.unwrap();
        assert_eq!(updated.min_premium_bps, 100);
        assert_eq!(updated.symbols, vec!["BTC", "ETH"]);
    }

    #[tokio::test]
    async fn test_active_opportunity_tracking() {
        let db = Database::connect("sqlite::memory:").await.unwrap();

        // Initially not active
        let is_active = db
            .is_opportunity_active("BTC", "Binance", "Upbit")
            .await
            .unwrap();
        assert!(!is_active);

        // Mark as active
        db.mark_opportunity_active("BTC", "Binance", "Upbit", 100)
            .await
            .unwrap();

        // Now it should be active
        let is_active = db
            .is_opportunity_active("BTC", "Binance", "Upbit")
            .await
            .unwrap();
        assert!(is_active);

        // Different symbol should not be active
        let is_active = db
            .is_opportunity_active("ETH", "Binance", "Upbit")
            .await
            .unwrap();
        assert!(!is_active);

        // Mark as inactive
        db.mark_opportunity_inactive("BTC", "Binance", "Upbit")
            .await
            .unwrap();

        // Should no longer be active
        let is_active = db
            .is_opportunity_active("BTC", "Binance", "Upbit")
            .await
            .unwrap();
        assert!(!is_active);
    }
}

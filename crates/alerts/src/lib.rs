//! Telegram alert system for arbitrage opportunities.
//!
//! This crate provides:
//! - SQLite-based configuration storage
//! - Telegram bot integration for notifications
//! - Alert filtering and deduplication

pub mod config;
pub mod db;
pub mod notifier;
pub mod telegram;

pub use config::AlertConfig;
pub use db::Database;
pub use notifier::{Notifier, NotifierConfig, TransferPathChecker};
pub use telegram::TelegramBot;

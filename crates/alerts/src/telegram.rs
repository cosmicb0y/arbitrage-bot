//! Telegram bot handlers.

use crate::db::Database;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelegramError {
    #[error("Telegram API error: {0}")]
    Api(#[from] teloxide::RequestError),
    #[error("Database error: {0}")]
    Db(#[from] crate::db::DbError),
}

/// Bot commands.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Start the bot and register for alerts")]
    Start,
    #[command(description = "Show current configuration")]
    Config,
    #[command(description = "Set minimum premium (bps). Usage: /premium 50")]
    Premium(String),
    #[command(description = "Set symbols to monitor. Usage: /symbols BTC,ETH (or 'all')")]
    Symbols(String),
    #[command(description = "Exclude symbols from alerts. Usage: /exclude BTC,DOGE (or 'clear')")]
    Exclude(String),
    #[command(description = "Set exchanges to monitor. Usage: /exchanges Binance,Upbit")]
    Exchanges(String),
    #[command(description = "Pause alerts")]
    Pause,
    #[command(description = "Resume alerts")]
    Resume,
    #[command(description = "Show help")]
    Help,
}

/// Telegram bot wrapper.
pub struct TelegramBot {
    bot: Bot,
    db: Database,
}

impl TelegramBot {
    /// Create a new bot with the given token.
    pub fn new(token: &str, db: Database) -> Self {
        let bot = Bot::new(token);
        Self { bot, db }
    }

    /// Get the underlying bot for sending messages.
    pub fn bot(&self) -> &Bot {
        &self.bot
    }

    /// Get the database reference.
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Send an alert message to a chat.
    pub async fn send_alert(&self, chat_id: &str, message: &str) -> Result<(), TelegramError> {
        let chat_id: ChatId = ChatId(chat_id.parse().unwrap_or(0));
        self.bot
            .send_message(chat_id, message)
            .parse_mode(ParseMode::Html)
            .await?;
        Ok(())
    }

    /// Run the bot command handler.
    pub async fn run(self: Arc<Self>) {
        let bot = self.bot.clone();
        let handler = Update::filter_message().filter_command::<Command>().endpoint(
            move |bot: Bot, msg: Message, cmd: Command| {
                let this = Arc::clone(&self);
                async move { this.handle_command(bot, msg, cmd).await }
            },
        );

        Dispatcher::builder(bot, handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    }

    async fn handle_command(
        &self,
        bot: Bot,
        msg: Message,
        cmd: Command,
    ) -> Result<(), TelegramError> {
        let chat_id = msg.chat.id.to_string();

        match cmd {
            Command::Start => {
                let config = self.db.get_or_create_config(&chat_id).await?;
                let text = format!(
                    "Welcome to Arbitrage Alert Bot!\n\n\
                     Your chat is now registered.\n\
                     Current settings:\n\
                     - Min premium: {} bps\n\
                     - Symbols: {}\n\
                     - Exchanges: {}\n\n\
                     Use /help to see available commands.",
                    config.min_premium_bps,
                    if config.symbols.is_empty() {
                        "All".to_string()
                    } else {
                        config.symbols.join(", ")
                    },
                    if config.exchanges.is_empty() {
                        "All".to_string()
                    } else {
                        config.exchanges.join(", ")
                    }
                );
                bot.send_message(msg.chat.id, text).await?;
            }

            Command::Config => {
                let config = self.db.get_or_create_config(&chat_id).await?;
                let status = if config.enabled { "Active" } else { "Paused" };
                let text = format!(
                    "<b>Current Configuration</b>\n\n\
                     Status: {}\n\
                     Min Premium: {} bps\n\
                     Symbols: {}\n\
                     Excluded: {}\n\
                     Exchanges: {}",
                    status,
                    config.min_premium_bps,
                    if config.symbols.is_empty() {
                        "All".to_string()
                    } else {
                        config.symbols.join(", ")
                    },
                    if config.excluded_symbols.is_empty() {
                        "None".to_string()
                    } else {
                        config.excluded_symbols.join(", ")
                    },
                    if config.exchanges.is_empty() {
                        "All".to_string()
                    } else {
                        config.exchanges.join(", ")
                    }
                );
                bot.send_message(msg.chat.id, text)
                    .parse_mode(ParseMode::Html)
                    .await?;
            }

            Command::Premium(value) => {
                let value = value.trim();
                if let Ok(bps) = value.parse::<i32>() {
                    if bps >= 0 && bps <= 10000 {
                        let mut config = self.db.get_or_create_config(&chat_id).await?;
                        config.min_premium_bps = bps;
                        self.db.update_config(&config).await?;
                        bot.send_message(
                            msg.chat.id,
                            format!("Minimum premium set to {} bps", bps),
                        )
                        .await?;
                    } else {
                        bot.send_message(msg.chat.id, "Premium must be between 0 and 10000 bps")
                            .await?;
                    }
                } else {
                    bot.send_message(msg.chat.id, "Usage: /premium <number>\nExample: /premium 50")
                        .await?;
                }
            }

            Command::Symbols(value) => {
                let value = value.trim();
                let mut config = self.db.get_or_create_config(&chat_id).await?;

                if value.is_empty() || value.eq_ignore_ascii_case("all") {
                    config.symbols = Vec::new();
                    self.db.update_config(&config).await?;
                    bot.send_message(msg.chat.id, "Monitoring all symbols")
                        .await?;
                } else {
                    let symbols: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_uppercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                    config.symbols = symbols.clone();
                    self.db.update_config(&config).await?;
                    bot.send_message(
                        msg.chat.id,
                        format!("Monitoring symbols: {}", symbols.join(", ")),
                    )
                    .await?;
                }
            }

            Command::Exclude(value) => {
                let value = value.trim();
                let mut config = self.db.get_or_create_config(&chat_id).await?;

                if value.is_empty() || value.eq_ignore_ascii_case("clear") {
                    config.excluded_symbols = Vec::new();
                    self.db.update_config(&config).await?;
                    bot.send_message(msg.chat.id, "Exclusion list cleared")
                        .await?;
                } else {
                    let symbols: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_uppercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                    config.excluded_symbols = symbols.clone();
                    self.db.update_config(&config).await?;
                    bot.send_message(
                        msg.chat.id,
                        format!("Excluding symbols: {}", symbols.join(", ")),
                    )
                    .await?;
                }
            }

            Command::Exchanges(value) => {
                let value = value.trim();
                let mut config = self.db.get_or_create_config(&chat_id).await?;

                if value.is_empty() || value.eq_ignore_ascii_case("all") {
                    config.exchanges = Vec::new();
                    self.db.update_config(&config).await?;
                    bot.send_message(msg.chat.id, "Monitoring all exchanges")
                        .await?;
                } else {
                    let exchanges: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    config.exchanges = exchanges.clone();
                    self.db.update_config(&config).await?;
                    bot.send_message(
                        msg.chat.id,
                        format!("Monitoring exchanges: {}", exchanges.join(", ")),
                    )
                    .await?;
                }
            }

            Command::Pause => {
                let mut config = self.db.get_or_create_config(&chat_id).await?;
                config.enabled = false;
                self.db.update_config(&config).await?;
                bot.send_message(msg.chat.id, "Alerts paused. Use /resume to re-enable.")
                    .await?;
            }

            Command::Resume => {
                let mut config = self.db.get_or_create_config(&chat_id).await?;
                config.enabled = true;
                self.db.update_config(&config).await?;
                bot.send_message(msg.chat.id, "Alerts resumed!").await?;
            }

            Command::Help => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
        }

        Ok(())
    }
}

/// Format price with appropriate precision based on magnitude.
fn format_price(price: f64) -> String {
    if price == 0.0 {
        return "$0".to_string();
    }
    let abs_price = price.abs();
    if abs_price >= 1000.0 {
        format!("${:.2}", price)
    } else if abs_price >= 1.0 {
        format!("${:.4}", price)
    } else if abs_price >= 0.01 {
        format!("${:.6}", price)
    } else if abs_price >= 0.0001 {
        format!("${:.8}", price)
    } else {
        // Very small prices - use scientific notation or more decimals
        format!("${:.10}", price)
    }
}

/// Format an opportunity as an alert message.
pub fn format_alert_message(
    symbol: &str,
    source_exchange: &str,
    target_exchange: &str,
    source_price: f64,
    target_price: f64,
    premium_bps: i32,
    source_depth: Option<f64>,
    target_depth: Option<f64>,
) -> String {
    let premium_pct = premium_bps as f64 / 100.0;

    let mut msg = format!(
        "🚨 <b>Arbitrage Alert!</b>\n\n\
         <b>Symbol:</b> {}\n\
         📈 <b>Buy:</b> {} @ {}\n\
         📉 <b>Sell:</b> {} @ {}\n\n\
         <b>Premium:</b> {} bps ({:.2}%)",
        symbol,
        source_exchange,
        format_price(source_price),
        target_exchange,
        format_price(target_price),
        premium_bps,
        premium_pct
    );

    if let (Some(src), Some(tgt)) = (source_depth, target_depth) {
        let depth = src.min(tgt);
        msg.push_str(&format!("\n<b>Depth:</b> {:.4} available", depth));
    }

    let now = chrono::Utc::now();
    msg.push_str(&format!("\n\n⏰ {}", now.format("%Y-%m-%d %H:%M:%S UTC")));

    msg
}

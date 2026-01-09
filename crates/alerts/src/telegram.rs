//! Telegram bot handlers.

use crate::db::Database;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{LinkPreviewOptions, ParseMode};
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
    #[command(description = "Set minimum profit (USD). Usage: /minprofit 10 (or 0 to disable)")]
    Minprofit(String),
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
            .link_preview_options(LinkPreviewOptions {
                is_disabled: true,
                url: None,
                prefer_small_media: false,
                prefer_large_media: false,
                show_above_text: false,
            })
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
                let min_profit_str = if config.min_profit_usd > 0.0 {
                    format!("${:.2}", config.min_profit_usd)
                } else {
                    "Disabled".to_string()
                };
                let text = format!(
                    "Welcome to Arbitrage Alert Bot!\n\n\
                     Your chat is now registered.\n\
                     Current settings:\n\
                     - Min premium: {} bps\n\
                     - Min profit: {}\n\
                     - Symbols: {}\n\
                     - Exchanges: {}\n\n\
                     Use /help to see available commands.",
                    config.min_premium_bps,
                    min_profit_str,
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
                let min_profit_str = if config.min_profit_usd > 0.0 {
                    format!("${:.2}", config.min_profit_usd)
                } else {
                    "Disabled".to_string()
                };
                let text = format!(
                    "<b>Current Configuration</b>\n\n\
                     Status: {}\n\
                     Min Premium: {} bps\n\
                     Min Profit: {}\n\
                     Symbols: {}\n\
                     Excluded: {}\n\
                     Exchanges: {}\n\n\
                     <i>Alert triggers when: Premium >= {} bps{}</i>",
                    status,
                    config.min_premium_bps,
                    min_profit_str,
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
                    },
                    config.min_premium_bps,
                    if config.min_profit_usd > 0.0 {
                        format!(" OR Profit >= ${:.2}", config.min_profit_usd)
                    } else {
                        String::new()
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

            Command::Minprofit(value) => {
                let value = value.trim();
                if let Ok(usd) = value.parse::<f64>() {
                    if usd >= 0.0 && usd <= 1_000_000.0 {
                        let mut config = self.db.get_or_create_config(&chat_id).await?;
                        config.min_profit_usd = usd;
                        self.db.update_config(&config).await?;
                        if usd > 0.0 {
                            bot.send_message(
                                msg.chat.id,
                                format!("Minimum profit set to ${:.2} USD\n\nAlerts will trigger when:\n• Premium >= {} bps OR\n• Expected profit >= ${:.2}", usd, config.min_premium_bps, usd),
                            )
                            .await?;
                        } else {
                            bot.send_message(
                                msg.chat.id,
                                "Minimum profit disabled. Alerts will only use premium threshold.",
                            )
                            .await?;
                        }
                    } else {
                        bot.send_message(msg.chat.id, "Profit must be between 0 and 1,000,000 USD")
                            .await?;
                    }
                } else {
                    bot.send_message(msg.chat.id, "Usage: /minprofit <number>\nExample: /minprofit 10\nSet to 0 to disable profit-based alerts")
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
/// Format USD price with comma separators to prevent phone number auto-detection on mobile.
fn format_price(price: f64) -> String {
    if price == 0.0 {
        return "$0".to_string();
    }
    let abs_price = price.abs();
    if abs_price >= 1000.0 {
        // Use comma separators for large numbers (e.g., $99,450.00)
        // This prevents iOS/Android from detecting as phone number
        let int_part = price.trunc() as i64;
        let frac_part = ((price.fract().abs() * 100.0).round() as i64).min(99);
        format!("${}.{:02}", format_with_commas(int_part), frac_part)
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

/// Format KRW price with adaptive precision based on magnitude.
fn format_krw_price(price: f64) -> String {
    if price < 0.0001 {
        format!("₩{:.6}", price)
    } else if price < 0.01 {
        format!("₩{:.4}", price)
    } else if price < 1.0 {
        format!("₩{:.2}", price)
    } else if price < 100.0 {
        format!("₩{:.1}", price)
    } else {
        // Large KRW prices - use comma separator
        format!("₩{}", format_with_commas(price as i64))
    }
}

/// Format USDT/USDC price with appropriate precision.
/// Uses comma separators for large numbers to prevent phone number auto-detection on mobile.
fn format_stablecoin_price(price: f64, symbol: &str) -> String {
    if price == 0.0 {
        return format!("0 {}", symbol);
    }
    let abs_price = price.abs();
    if abs_price >= 1000.0 {
        // Use comma separators for large numbers (e.g., 273,397.40)
        // This prevents iOS/Android from detecting as phone number
        let int_part = price.trunc() as i64;
        let frac_part = ((price.fract().abs() * 100.0).round() as i64).min(99);
        format!("{}.{:02} {}", format_with_commas(int_part), frac_part, symbol)
    } else if abs_price >= 1.0 {
        format!("{:.4} {}", price, symbol)
    } else if abs_price >= 0.01 {
        format!("{:.6} {}", price, symbol)
    } else if abs_price >= 0.0001 {
        format!("{:.8} {}", price, symbol)
    } else {
        format!("{:.10} {}", price, symbol)
    }
}

/// Format number with comma separators.
fn format_with_commas(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Format USD value with appropriate precision.
fn format_usd_value(value: f64) -> String {
    if value >= 1000.0 {
        format!("${:.0}", value)
    } else if value >= 100.0 {
        format!("${:.1}", value)
    } else {
        format!("${:.2}", value)
    }
}

/// Get the trade URL for an exchange and symbol.
fn get_exchange_trade_url(exchange: &str, symbol: &str) -> Option<String> {
    let symbol_upper = symbol.to_uppercase();
    match exchange {
        "Binance" => Some(format!("https://www.binance.com/en/trade/{}USDT", symbol_upper)),
        "Coinbase" => Some(format!("https://www.coinbase.com/advanced-trade/spot/{}-USD", symbol_upper)),
        "Upbit" => Some(format!("https://upbit.com/exchange?code=CRIX.UPBIT.KRW-{}", symbol_upper)),
        "Bithumb" => Some(format!("https://www.bithumb.com/trade/order/{}_KRW", symbol_upper)),
        "Bybit" => Some(format!("https://www.bybit.com/en/trade/spot/{}/USDT", symbol_upper)),
        "GateIO" => Some(format!("https://www.gate.io/trade/{}_USDT", symbol_upper)),
        "Kraken" => Some(format!("https://pro.kraken.com/app/trade/{}-usd", symbol.to_lowercase())),
        "Okx" => Some(format!("https://www.okx.com/trade-spot/{}-usdt", symbol.to_lowercase())),
        _ => None,
    }
}

/// Format exchange name with link if available.
fn format_exchange_with_link(exchange: &str, symbol: &str) -> String {
    if let Some(url) = get_exchange_trade_url(exchange, symbol) {
        format!("<a href=\"{}\">{}</a>", url, exchange)
    } else {
        exchange.to_string()
    }
}

/// Exchange rate information for price conversion.
#[derive(Debug, Clone, Default)]
pub struct ExchangeRates {
    /// USDT/KRW rate from Upbit
    pub upbit_usdt_krw: f64,
    /// USDT/KRW rate from Bithumb
    pub bithumb_usdt_krw: f64,
    /// USDT/USD rate
    pub usdt_usd: f64,
    /// USDC/USD rate
    pub usdc_usd: f64,
}

/// Convert USD price back to raw quote currency price.
fn convert_usd_to_raw(
    usd_price: f64,
    exchange: &str,
    quote: &str,
    rates: &ExchangeRates,
) -> Option<f64> {
    match quote {
        "KRW" => {
            // USD -> KRW via USDT/KRW
            let usdt_krw = if exchange == "Upbit" {
                rates.upbit_usdt_krw
            } else if exchange == "Bithumb" {
                rates.bithumb_usdt_krw
            } else {
                0.0
            };
            if usdt_krw > 0.0 && rates.usdt_usd > 0.0 {
                // usd_price was: krw_price / usdt_krw * usdt_usd
                // So: krw_price = usd_price / usdt_usd * usdt_krw
                Some((usd_price / rates.usdt_usd) * usdt_krw)
            } else {
                None
            }
        }
        "USDT" => {
            // USD -> USDT
            if rates.usdt_usd > 0.0 {
                Some(usd_price / rates.usdt_usd)
            } else {
                None
            }
        }
        "USDC" => {
            // USD -> USDC
            if rates.usdc_usd > 0.0 {
                Some(usd_price / rates.usdc_usd)
            } else {
                None
            }
        }
        _ => None, // USD - no conversion needed
    }
}

/// Format price with raw quote and USD conversion.
fn format_price_with_raw(
    usd_price: f64,
    exchange: &str,
    quote: &str,
    rates: &ExchangeRates,
) -> String {
    if let Some(raw_price) = convert_usd_to_raw(usd_price, exchange, quote, rates) {
        let raw_str = match quote {
            "KRW" => format_krw_price(raw_price),
            "USDT" => format_stablecoin_price(raw_price, "USDT"),
            "USDC" => format_stablecoin_price(raw_price, "USDC"),
            _ => format_price(usd_price),
        };
        format!("{}\n        ({})", raw_str, format_price(usd_price))
    } else {
        format_price(usd_price)
    }
}

/// Format timestamp as human-readable string.
fn format_timestamp(timestamp_ms: u64) -> String {
    if timestamp_ms == 0 {
        return "unknown".to_string();
    }
    let secs = (timestamp_ms / 1000) as i64;
    let nanos = ((timestamp_ms % 1000) * 1_000_000) as u32;
    match chrono::DateTime::from_timestamp(secs, nanos) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        None => "invalid".to_string(),
    }
}

/// Format price for display, preferring raw price if available.
/// Falls back to conversion-based display if raw price is not provided.
fn format_price_display(
    raw_price: Option<f64>,
    usd_price: f64,
    exchange: &str,
    quote: &str,
    rates: &ExchangeRates,
) -> String {
    if let Some(raw) = raw_price {
        // Use the actual raw price from the exchange directly
        let raw_str = match quote {
            "KRW" => format_krw_price(raw),
            "USDT" => format_stablecoin_price(raw, "USDT"),
            "USDC" => format_stablecoin_price(raw, "USDC"),
            _ => format_price(raw),
        };
        format!("{}\n        ({})", raw_str, format_price(usd_price))
    } else {
        // Fallback: convert USD price back to raw quote currency
        format_price_with_raw(usd_price, exchange, quote, rates)
    }
}

/// Format an opportunity as an alert message.
pub fn format_alert_message(
    symbol: &str,
    source_exchange: &str,
    target_exchange: &str,
    source_quote: &str,
    target_quote: &str,
    source_price: f64,
    target_price: f64,
    source_raw_price: Option<f64>,
    target_raw_price: Option<f64>,
    premium_bps: i32,
    optimal_size: Option<f64>,
    optimal_profit: Option<f64>,
    rates: Option<&ExchangeRates>,
    source_timestamp_ms: Option<u64>,
    target_timestamp_ms: Option<u64>,
) -> String {
    let premium_pct = premium_bps as f64 / 100.0;

    let source_link = format_exchange_with_link(source_exchange, symbol);
    let target_link = format_exchange_with_link(target_exchange, symbol);

    // Format market names like BTC/USDT, BTC/KRW
    let buy_market = format!("{}/{}", symbol, source_quote);
    let sell_market = format!("{}/{}", symbol, target_quote);

    // Format prices: use raw prices directly if available, otherwise fallback to conversion
    let default_rates = ExchangeRates::default();
    let rates = rates.unwrap_or(&default_rates);

    let source_price_str = format_price_display(source_raw_price, source_price, source_exchange, source_quote, rates);
    let target_price_str = format_price_display(target_raw_price, target_price, target_exchange, target_quote, rates);

    // Format timestamps
    let source_ts_str = source_timestamp_ms
        .filter(|&ts| ts > 0)
        .map(format_timestamp)
        .unwrap_or_default();
    let target_ts_str = target_timestamp_ms
        .filter(|&ts| ts > 0)
        .map(format_timestamp)
        .unwrap_or_default();

    // Build price lines with optional timestamps
    let source_ts_line = if source_ts_str.is_empty() {
        String::new()
    } else {
        format!(" ({})", source_ts_str)
    };
    let target_ts_line = if target_ts_str.is_empty() {
        String::new()
    } else {
        format!(" ({})", target_ts_str)
    };

    let mut msg = format!(
        "🚨 <b>Arbitrage Alert!</b>\n\n\
         📈 <b>Buy:</b> {} @ {}{}\n        ({})\n\
         📉 <b>Sell:</b> {} @ {}{}\n        ({})\n\n\
         <b>Premium:</b> {:.2}%",
        buy_market,
        source_price_str,
        source_ts_line,
        source_link,
        sell_market,
        target_price_str,
        target_ts_line,
        target_link,
        premium_pct
    );

    // Show optimal trade size and expected profit
    if let (Some(size), Some(profit)) = (optimal_size, optimal_profit) {
        if size > 0.0 && profit > 0.0 {
            let notional_value = size * source_price;
            msg.push_str(&format!(
                "\n\n<b>Optimal Trade:</b>\n  Size: {:.4} {} ({})\n  Expected Profit: {}",
                size, symbol, format_usd_value(notional_value),
                format_usd_value(profit)
            ));
        }
    }

    let now = chrono::Utc::now();
    msg.push_str(&format!("\n\n⏰ {}", now.format("%Y-%m-%d %H:%M:%S UTC")));

    msg
}

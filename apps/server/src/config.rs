//! Application configuration.

use arbitrage_core::Exchange;
use arbitrage_engine::DetectorConfig;
use serde::{Deserialize, Serialize};

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Detector configuration.
    pub detector: DetectorSettings,
    /// Execution configuration.
    pub execution: ExecutionSettings,
    /// Exchange configurations.
    pub exchanges: Vec<ExchangeSettings>,
    /// Logging level.
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            detector: DetectorSettings::default(),
            execution: ExecutionSettings::default(),
            exchanges: vec![
                ExchangeSettings::new(Exchange::Binance),
                ExchangeSettings::new(Exchange::Coinbase),
            ],
            log_level: "info".to_string(),
        }
    }
}

/// Detector settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorSettings {
    /// Minimum premium in basis points.
    pub min_premium_bps: i32,
    /// Maximum price staleness in milliseconds.
    pub max_staleness_ms: u64,
    /// Scan interval in milliseconds.
    pub scan_interval_ms: u64,
}

impl Default for DetectorSettings {
    fn default() -> Self {
        Self {
            min_premium_bps: 30,
            max_staleness_ms: 5000,
            scan_interval_ms: 100,
        }
    }
}

impl From<&DetectorSettings> for DetectorConfig {
    fn from(settings: &DetectorSettings) -> Self {
        DetectorConfig {
            min_premium_bps: settings.min_premium_bps,
            max_staleness_ms: settings.max_staleness_ms,
            ..Default::default()
        }
    }
}

/// Execution settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSettings {
    /// Execution mode.
    pub mode: ExecutionMode,
    /// Maximum position in USD.
    pub max_position_usd: u64,
    /// Maximum slippage in basis points.
    pub max_slippage_bps: u16,
    /// Minimum profit in basis points to execute.
    pub min_profit_bps: i32,
}

impl Default for ExecutionSettings {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::AlertOnly,
            max_position_usd: 10000,
            max_slippage_bps: 50,
            min_profit_bps: 20,
        }
    }
}

/// Per-exchange settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeSettings {
    /// Exchange identifier.
    pub exchange: Exchange,
    /// Whether enabled.
    pub enabled: bool,
    /// API key (optional).
    pub api_key: Option<String>,
    /// API secret (optional).
    pub api_secret: Option<String>,
    /// Trading pairs to monitor.
    pub pairs: Vec<String>,
}

impl ExchangeSettings {
    pub fn new(exchange: Exchange) -> Self {
        Self {
            exchange,
            enabled: true,
            api_key: None,
            api_secret: None,
            pairs: vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()],
        }
    }
}

/// Execution mode for the bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    /// Automatically execute trades.
    Auto,
    /// Require manual approval.
    ManualApproval,
    /// Only show alerts, no execution.
    #[default]
    AlertOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.detector.min_premium_bps, 30);
        assert_eq!(config.execution.mode, ExecutionMode::AlertOnly);
        assert!(!config.exchanges.is_empty());
    }

    #[test]
    fn test_detector_settings_to_config() {
        let settings = DetectorSettings::default();
        let config: DetectorConfig = (&settings).into();
        assert_eq!(config.min_premium_bps, settings.min_premium_bps);
    }

    #[test]
    fn test_exchange_settings_new() {
        let settings = ExchangeSettings::new(Exchange::Binance);
        assert_eq!(settings.exchange, Exchange::Binance);
        assert!(settings.enabled);
        assert!(!settings.pairs.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.detector.min_premium_bps, config.detector.min_premium_bps);
    }
}

//! Fee configuration for exchanges.
//!
//! This module provides fee structures for trading and withdrawals,
//! with default values and support for runtime updates from exchange APIs.

use arbitrage_core::{Exchange, FixedPoint};
use std::collections::HashMap;

/// Trading and withdrawal fee configuration for an exchange.
#[derive(Debug, Clone, Copy)]
pub struct FeeConfig {
    /// Maker fee in basis points (negative means rebate).
    pub maker_fee_bps: i32,
    /// Taker fee in basis points.
    pub taker_fee_bps: i32,
    /// Withdrawal fee in base asset units (FixedPoint scale).
    /// This varies by asset, so 0 means "use per-asset fee".
    pub withdrawal_fee: u64,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            maker_fee_bps: 10, // 0.1%
            taker_fee_bps: 10, // 0.1%
            withdrawal_fee: 0,
        }
    }
}

impl FeeConfig {
    /// Create a new fee config with specified taker fees.
    pub fn new(taker_fee_bps: i32) -> Self {
        Self {
            maker_fee_bps: taker_fee_bps,
            taker_fee_bps,
            withdrawal_fee: 0,
        }
    }

    /// Create fee config with separate maker/taker fees.
    pub fn with_maker_taker(maker_fee_bps: i32, taker_fee_bps: i32) -> Self {
        Self {
            maker_fee_bps,
            taker_fee_bps,
            withdrawal_fee: 0,
        }
    }

    /// Set withdrawal fee.
    pub fn with_withdrawal_fee(mut self, fee: FixedPoint) -> Self {
        self.withdrawal_fee = fee.0;
        self
    }

    /// Get default fee config for an exchange.
    pub fn default_for_exchange(exchange: Exchange) -> Self {
        match exchange {
            // Global exchanges
            Exchange::Binance => Self::with_maker_taker(10, 10), // 0.1% / 0.1%
            Exchange::Coinbase => Self::with_maker_taker(40, 60), // 0.4% / 0.6%
            Exchange::Kraken => Self::with_maker_taker(16, 26),  // 0.16% / 0.26%
            Exchange::Bybit => Self::with_maker_taker(10, 10),   // 0.1% / 0.1%
            Exchange::Okx => Self::with_maker_taker(8, 10),      // 0.08% / 0.1%
            Exchange::GateIO => Self::with_maker_taker(20, 20),  // 0.2% / 0.2%

            // Korean exchanges (typically lower fees)
            Exchange::Upbit => Self::with_maker_taker(5, 5),     // 0.05% / 0.05%
            Exchange::Bithumb => Self::with_maker_taker(4, 4),   // 0.04% / 0.04%

            // Default for unknown exchanges
            _ => Self::default(),
        }
    }
}

/// Per-asset withdrawal fee configuration.
#[derive(Debug, Clone)]
pub struct AssetWithdrawalFee {
    /// Asset symbol (e.g., "BTC", "ETH").
    pub asset: String,
    /// Network/chain for withdrawal.
    pub network: Option<String>,
    /// Withdrawal fee amount in asset units (FixedPoint scale).
    pub fee: u64,
    /// Minimum withdrawal amount (FixedPoint scale).
    pub min_withdrawal: u64,
}

/// Fee manager for all exchanges.
#[derive(Debug, Default)]
pub struct FeeManager {
    /// Base fee config per exchange.
    exchange_fees: HashMap<Exchange, FeeConfig>,
    /// Per-asset withdrawal fees: (exchange, asset) -> fee.
    withdrawal_fees: HashMap<(Exchange, String), AssetWithdrawalFee>,
}

impl FeeManager {
    /// Create a new fee manager with default fees for all known exchanges.
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.initialize_defaults();
        manager
    }

    /// Initialize default fees for all exchanges.
    fn initialize_defaults(&mut self) {
        let exchanges = [
            Exchange::Binance,
            Exchange::Coinbase,
            Exchange::Kraken,
            Exchange::Bybit,
            Exchange::Okx,
            Exchange::GateIO,
            Exchange::Upbit,
            Exchange::Bithumb,
        ];

        for exchange in exchanges {
            self.exchange_fees
                .insert(exchange, FeeConfig::default_for_exchange(exchange));
        }

        // Initialize some common withdrawal fees (can be updated via API)
        self.set_default_withdrawal_fees();
    }

    /// Set common default withdrawal fees.
    fn set_default_withdrawal_fees(&mut self) {
        // Binance BTC withdrawal
        self.withdrawal_fees.insert(
            (Exchange::Binance, "BTC".to_string()),
            AssetWithdrawalFee {
                asset: "BTC".to_string(),
                network: Some("BTC".to_string()),
                fee: FixedPoint::from_f64(0.0001).0, // 0.0001 BTC
                min_withdrawal: FixedPoint::from_f64(0.001).0,
            },
        );

        // Binance ETH withdrawal
        self.withdrawal_fees.insert(
            (Exchange::Binance, "ETH".to_string()),
            AssetWithdrawalFee {
                asset: "ETH".to_string(),
                network: Some("ETH".to_string()),
                fee: FixedPoint::from_f64(0.0005).0, // 0.0005 ETH
                min_withdrawal: FixedPoint::from_f64(0.01).0,
            },
        );

        // Upbit BTC withdrawal (higher due to regulatory requirements)
        self.withdrawal_fees.insert(
            (Exchange::Upbit, "BTC".to_string()),
            AssetWithdrawalFee {
                asset: "BTC".to_string(),
                network: Some("BTC".to_string()),
                fee: FixedPoint::from_f64(0.0005).0, // 0.0005 BTC
                min_withdrawal: FixedPoint::from_f64(0.001).0,
            },
        );

        // Upbit ETH withdrawal
        self.withdrawal_fees.insert(
            (Exchange::Upbit, "ETH".to_string()),
            AssetWithdrawalFee {
                asset: "ETH".to_string(),
                network: Some("ETH".to_string()),
                fee: FixedPoint::from_f64(0.01).0, // 0.01 ETH
                min_withdrawal: FixedPoint::from_f64(0.02).0,
            },
        );
    }

    /// Get trading fee config for an exchange.
    pub fn get_trading_fees(&self, exchange: Exchange) -> FeeConfig {
        self.exchange_fees
            .get(&exchange)
            .copied()
            .unwrap_or_else(|| FeeConfig::default_for_exchange(exchange))
    }

    /// Get withdrawal fee for a specific asset on an exchange.
    pub fn get_withdrawal_fee(&self, exchange: Exchange, asset: &str) -> Option<&AssetWithdrawalFee> {
        self.withdrawal_fees.get(&(exchange, asset.to_string()))
    }

    /// Get withdrawal fee amount for a specific asset.
    pub fn get_withdrawal_fee_amount(&self, exchange: Exchange, asset: &str) -> u64 {
        self.withdrawal_fees
            .get(&(exchange, asset.to_string()))
            .map(|f| f.fee)
            .unwrap_or(0)
    }

    /// Update trading fees for an exchange.
    pub fn update_trading_fees(&mut self, exchange: Exchange, fees: FeeConfig) {
        self.exchange_fees.insert(exchange, fees);
    }

    /// Update withdrawal fee for an asset.
    pub fn update_withdrawal_fee(
        &mut self,
        exchange: Exchange,
        asset: &str,
        fee: u64,
        min_withdrawal: u64,
        network: Option<String>,
    ) {
        self.withdrawal_fees.insert(
            (exchange, asset.to_string()),
            AssetWithdrawalFee {
                asset: asset.to_string(),
                network,
                fee,
                min_withdrawal,
            },
        );
    }

    /// Get fees for an arbitrage pair (buy exchange -> sell exchange).
    /// Returns (buy_fee_bps, sell_fee_bps, withdrawal_fee).
    pub fn get_arbitrage_fees(
        &self,
        buy_exchange: Exchange,
        sell_exchange: Exchange,
        asset: &str,
    ) -> (u32, u32, u64) {
        let buy_fees = self.get_trading_fees(buy_exchange);
        let sell_fees = self.get_trading_fees(sell_exchange);

        // Use taker fees (market orders for arbitrage)
        let buy_fee = buy_fees.taker_fee_bps.max(0) as u32;
        let sell_fee = sell_fees.taker_fee_bps.max(0) as u32;

        // Get withdrawal fee from buy exchange (where we buy, then withdraw)
        let withdrawal_fee = self.get_withdrawal_fee_amount(buy_exchange, asset);

        (buy_fee, sell_fee, withdrawal_fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fee_config() {
        let fee = FeeConfig::default();
        assert_eq!(fee.taker_fee_bps, 10);
        assert_eq!(fee.maker_fee_bps, 10);
    }

    #[test]
    fn test_exchange_specific_fees() {
        assert_eq!(FeeConfig::default_for_exchange(Exchange::Binance).taker_fee_bps, 10);
        assert_eq!(FeeConfig::default_for_exchange(Exchange::Coinbase).taker_fee_bps, 60);
        assert_eq!(FeeConfig::default_for_exchange(Exchange::Upbit).taker_fee_bps, 5);
    }

    #[test]
    fn test_fee_manager() {
        let manager = FeeManager::new();

        let binance_fees = manager.get_trading_fees(Exchange::Binance);
        assert_eq!(binance_fees.taker_fee_bps, 10);

        let upbit_fees = manager.get_trading_fees(Exchange::Upbit);
        assert_eq!(upbit_fees.taker_fee_bps, 5);
    }

    #[test]
    fn test_withdrawal_fees() {
        let manager = FeeManager::new();

        let btc_fee = manager.get_withdrawal_fee(Exchange::Binance, "BTC");
        assert!(btc_fee.is_some());
        assert!(btc_fee.unwrap().fee > 0);
    }

    #[test]
    fn test_arbitrage_fees() {
        let manager = FeeManager::new();

        let (buy_fee, sell_fee, withdrawal) =
            manager.get_arbitrage_fees(Exchange::Binance, Exchange::Upbit, "BTC");

        assert_eq!(buy_fee, 10);  // Binance taker
        assert_eq!(sell_fee, 5);  // Upbit taker
        assert!(withdrawal > 0); // Binance BTC withdrawal
    }

    #[test]
    fn test_update_fees() {
        let mut manager = FeeManager::new();

        // Update trading fees
        manager.update_trading_fees(Exchange::Binance, FeeConfig::new(5));
        assert_eq!(manager.get_trading_fees(Exchange::Binance).taker_fee_bps, 5);

        // Update withdrawal fee
        manager.update_withdrawal_fee(
            Exchange::Binance,
            "SOL",
            FixedPoint::from_f64(0.01).0,
            FixedPoint::from_f64(0.1).0,
            Some("SOL".to_string()),
        );
        let sol_fee = manager.get_withdrawal_fee(Exchange::Binance, "SOL");
        assert!(sol_fee.is_some());
    }
}

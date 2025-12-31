//! Quote currency types for multi-quote market support.

use serde::{Deserialize, Serialize};

/// Quote currency for trading pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum QuoteCurrency {
    /// US Dollar (native USD pairs like Coinbase)
    USD = 1,
    /// Tether (most common stablecoin)
    USDT = 2,
    /// USD Coin
    USDC = 3,
    /// Binance USD (legacy)
    BUSD = 4,
    /// Korean Won
    KRW = 10,
}

impl QuoteCurrency {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "USD" => Some(QuoteCurrency::USD),
            "USDT" => Some(QuoteCurrency::USDT),
            "USDC" => Some(QuoteCurrency::USDC),
            "BUSD" => Some(QuoteCurrency::BUSD),
            "KRW" => Some(QuoteCurrency::KRW),
            _ => None,
        }
    }

    /// Get the quote currency ID.
    pub fn id(self) -> u8 {
        self as u8
    }

    /// Create from ID.
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(QuoteCurrency::USD),
            2 => Some(QuoteCurrency::USDT),
            3 => Some(QuoteCurrency::USDC),
            4 => Some(QuoteCurrency::BUSD),
            10 => Some(QuoteCurrency::KRW),
            _ => None,
        }
    }

    /// Check if this is a USD-pegged stablecoin (USDT, USDC, BUSD).
    pub fn is_usd_stablecoin(self) -> bool {
        matches!(
            self,
            QuoteCurrency::USDT | QuoteCurrency::USDC | QuoteCurrency::BUSD
        )
    }

    /// Check if this is USD or a USD-pegged stablecoin.
    pub fn is_usd_equivalent(self) -> bool {
        matches!(
            self,
            QuoteCurrency::USD
                | QuoteCurrency::USDT
                | QuoteCurrency::USDC
                | QuoteCurrency::BUSD
        )
    }

    /// Get display name.
    pub fn as_str(self) -> &'static str {
        match self {
            QuoteCurrency::USD => "USD",
            QuoteCurrency::USDT => "USDT",
            QuoteCurrency::USDC => "USDC",
            QuoteCurrency::BUSD => "BUSD",
            QuoteCurrency::KRW => "KRW",
        }
    }
}

impl Default for QuoteCurrency {
    fn default() -> Self {
        QuoteCurrency::USD
    }
}

impl std::fmt::Display for QuoteCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(QuoteCurrency::from_str("USD"), Some(QuoteCurrency::USD));
        assert_eq!(QuoteCurrency::from_str("usdt"), Some(QuoteCurrency::USDT));
        assert_eq!(QuoteCurrency::from_str("USDC"), Some(QuoteCurrency::USDC));
        assert_eq!(QuoteCurrency::from_str("krw"), Some(QuoteCurrency::KRW));
        assert_eq!(QuoteCurrency::from_str("INVALID"), None);
    }

    #[test]
    fn test_id_roundtrip() {
        for quote in [
            QuoteCurrency::USD,
            QuoteCurrency::USDT,
            QuoteCurrency::USDC,
            QuoteCurrency::BUSD,
            QuoteCurrency::KRW,
        ] {
            assert_eq!(QuoteCurrency::from_id(quote.id()), Some(quote));
        }
    }

    #[test]
    fn test_is_usd_stablecoin() {
        assert!(!QuoteCurrency::USD.is_usd_stablecoin());
        assert!(QuoteCurrency::USDT.is_usd_stablecoin());
        assert!(QuoteCurrency::USDC.is_usd_stablecoin());
        assert!(QuoteCurrency::BUSD.is_usd_stablecoin());
        assert!(!QuoteCurrency::KRW.is_usd_stablecoin());
    }

    #[test]
    fn test_is_usd_equivalent() {
        assert!(QuoteCurrency::USD.is_usd_equivalent());
        assert!(QuoteCurrency::USDT.is_usd_equivalent());
        assert!(QuoteCurrency::USDC.is_usd_equivalent());
        assert!(QuoteCurrency::BUSD.is_usd_equivalent());
        assert!(!QuoteCurrency::KRW.is_usd_equivalent());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", QuoteCurrency::USD), "USD");
        assert_eq!(format!("{}", QuoteCurrency::USDT), "USDT");
        assert_eq!(format!("{}", QuoteCurrency::KRW), "KRW");
    }
}

//! Asset and trading pair definitions.

use crate::{Chain, Exchange};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Token or native asset information.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Asset {
    /// Token symbol (e.g., "ETH", "BTC", "USDT")
    pub symbol: CompactString,
    /// Blockchain where this asset exists
    pub chain: Chain,
    /// Decimal places (e.g., 18 for ETH, 6 for USDT)
    pub decimals: u8,
    /// Contract address for tokens, None for native assets
    pub contract_address: Option<[u8; 32]>,
}

impl Asset {
    /// Create a native asset (no contract address).
    pub fn native(symbol: &str, chain: Chain, decimals: u8) -> Self {
        Self {
            symbol: CompactString::new(symbol),
            chain,
            decimals,
            contract_address: None,
        }
    }

    /// Create a token asset with contract address.
    pub fn token(symbol: &str, chain: Chain, decimals: u8, contract_address: [u8; 32]) -> Self {
        Self {
            symbol: CompactString::new(symbol),
            chain,
            decimals,
            contract_address: Some(contract_address),
        }
    }

    /// Check if this is a native asset.
    #[inline]
    pub fn is_native(&self) -> bool {
        self.contract_address.is_none()
    }

    /// Create BTC asset (no specific chain for CEX).
    pub fn btc() -> Self {
        Self::native("BTC", Chain::Ethereum, 8) // Using Ethereum as placeholder
    }

    /// Create ETH asset on Ethereum mainnet.
    pub fn eth() -> Self {
        Self::native("ETH", Chain::Ethereum, 18)
    }

    /// Create SOL asset on Solana.
    pub fn sol() -> Self {
        Self::native("SOL", Chain::Solana, 9)
    }
}

/// Trading pair representing base/quote assets on an exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPair {
    /// Base asset (e.g., ETH in ETH/USDT)
    pub base: Asset,
    /// Quote asset (e.g., USDT in ETH/USDT)
    pub quote: Asset,
    /// Exchange where this pair is traded
    pub exchange: Exchange,
    /// Pool address for DEX pairs
    pub pool_address: Option<[u8; 32]>,
}

impl TradingPair {
    /// Create a new trading pair without pool address.
    pub fn new(base: Asset, quote: Asset, exchange: Exchange) -> Self {
        Self {
            base,
            quote,
            exchange,
            pool_address: None,
        }
    }

    /// Create a trading pair with a pool address (for DEX).
    pub fn with_pool(
        base: Asset,
        quote: Asset,
        exchange: Exchange,
        pool_address: [u8; 32],
    ) -> Self {
        Self {
            base,
            quote,
            exchange,
            pool_address: Some(pool_address),
        }
    }

    /// Get the trading pair symbol (e.g., "ETH/USDT").
    pub fn symbol(&self) -> String {
        format!("{}/{}", self.base.symbol, self.quote.symbol)
    }

    /// Generate a unique ID for this trading pair.
    /// Uses a simple hash of symbol + exchange.
    pub fn id(&self) -> u32 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.base.symbol.hash(&mut hasher);
        self.quote.symbol.hash(&mut hasher);
        self.exchange.hash(&mut hasher);
        hasher.finish() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Asset tests ===

    #[test]
    fn test_asset_new_native() {
        let eth = Asset::native("ETH", Chain::Ethereum, 18);
        assert_eq!(eth.symbol.as_str(), "ETH");
        assert_eq!(eth.chain, Chain::Ethereum);
        assert_eq!(eth.decimals, 18);
        assert!(eth.contract_address.is_none());
        assert!(eth.is_native());
    }

    #[test]
    fn test_asset_new_token() {
        let usdt_addr: [u8; 32] = [0xda; 32]; // mock address
        let usdt = Asset::token("USDT", Chain::Ethereum, 6, usdt_addr);
        assert_eq!(usdt.symbol.as_str(), "USDT");
        assert_eq!(usdt.decimals, 6);
        assert!(usdt.contract_address.is_some());
        assert!(!usdt.is_native());
    }

    #[test]
    fn test_asset_btc() {
        let btc = Asset::btc();
        assert_eq!(btc.symbol.as_str(), "BTC");
        assert_eq!(btc.decimals, 8);
    }

    #[test]
    fn test_asset_eth() {
        let eth = Asset::eth();
        assert_eq!(eth.symbol.as_str(), "ETH");
        assert_eq!(eth.chain, Chain::Ethereum);
        assert_eq!(eth.decimals, 18);
    }

    #[test]
    fn test_asset_sol() {
        let sol = Asset::sol();
        assert_eq!(sol.symbol.as_str(), "SOL");
        assert_eq!(sol.chain, Chain::Solana);
        assert_eq!(sol.decimals, 9);
    }

    // === TradingPair tests ===

    #[test]
    fn test_trading_pair_new() {
        let base = Asset::eth();
        let quote = Asset::native("USDT", Chain::Ethereum, 6);
        let pair = TradingPair::new(base, quote, Exchange::Binance);

        assert_eq!(pair.base.symbol.as_str(), "ETH");
        assert_eq!(pair.quote.symbol.as_str(), "USDT");
        assert_eq!(pair.exchange, Exchange::Binance);
        assert!(pair.pool_address.is_none());
    }

    #[test]
    fn test_trading_pair_with_pool() {
        let pool_addr: [u8; 32] = [0xab; 32];
        let base = Asset::eth();
        let quote = Asset::native("USDC", Chain::Ethereum, 6);
        let pair = TradingPair::with_pool(base, quote, Exchange::UniswapV3, pool_addr);

        assert!(pair.pool_address.is_some());
        assert_eq!(pair.pool_address.unwrap(), pool_addr);
    }

    #[test]
    fn test_trading_pair_symbol() {
        let base = Asset::eth();
        let quote = Asset::native("USDT", Chain::Ethereum, 6);
        let pair = TradingPair::new(base, quote, Exchange::Binance);

        assert_eq!(pair.symbol(), "ETH/USDT");
    }

    #[test]
    fn test_trading_pair_id() {
        let base = Asset::eth();
        let quote = Asset::native("USDT", Chain::Ethereum, 6);
        let pair = TradingPair::new(base, quote, Exchange::Binance);

        // Same pair should have same ID
        let pair2 = TradingPair::new(Asset::eth(), Asset::native("USDT", Chain::Ethereum, 6), Exchange::Binance);
        assert_eq!(pair.id(), pair2.id());

        // Different exchange should have different ID
        let pair3 = TradingPair::new(Asset::eth(), Asset::native("USDT", Chain::Ethereum, 6), Exchange::Coinbase);
        assert_ne!(pair.id(), pair3.id());
    }
}

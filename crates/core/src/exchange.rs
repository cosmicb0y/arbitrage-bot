//! Exchange identifiers and types.

use crate::Chain;
use serde::{Deserialize, Serialize};

/// Type of exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ExchangeType {
    Cex = 1,       // Centralized exchange (Binance, Coinbase)
    CpmmDex = 2,   // Constant Product AMM (Uniswap V2)
    ClmmDex = 3,   // Concentrated Liquidity AMM (Uniswap V3)
    PerpDex = 4,   // Perpetual DEX (dYdX, GMX)
    Orderbook = 5, // On-chain orderbook (Serum)
}

impl ExchangeType {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(ExchangeType::Cex),
            2 => Some(ExchangeType::CpmmDex),
            3 => Some(ExchangeType::ClmmDex),
            4 => Some(ExchangeType::PerpDex),
            5 => Some(ExchangeType::Orderbook),
            _ => None,
        }
    }

    #[inline]
    pub fn id(self) -> u8 {
        self as u8
    }

    #[inline]
    pub fn is_dex(self) -> bool {
        !matches!(self, ExchangeType::Cex)
    }
}

/// Exchange identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum Exchange {
    // CEX (100-199)
    Binance = 100,
    Coinbase = 101,
    Kraken = 102,
    Okx = 103,
    Bybit = 104,
    Upbit = 105,
    Bithumb = 106,
    GateIO = 107,

    // DEX - EVM CPMM (200-209)
    UniswapV2 = 200,
    SushiSwap = 202,

    // DEX - EVM CLMM (210-219)
    UniswapV3 = 201,
    Curve = 203,
    Balancer = 204,

    // DEX - Solana (300-399)
    Raydium = 300,
    Orca = 301,
    Jupiter = 302,

    // PerpDEX (400-499)
    Dydx = 400,
    Gmx = 401,
    Hyperliquid = 402,
}

impl Exchange {
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            100 => Some(Exchange::Binance),
            101 => Some(Exchange::Coinbase),
            102 => Some(Exchange::Kraken),
            103 => Some(Exchange::Okx),
            104 => Some(Exchange::Bybit),
            105 => Some(Exchange::Upbit),
            106 => Some(Exchange::Bithumb),
            107 => Some(Exchange::GateIO),
            200 => Some(Exchange::UniswapV2),
            201 => Some(Exchange::UniswapV3),
            202 => Some(Exchange::SushiSwap),
            203 => Some(Exchange::Curve),
            204 => Some(Exchange::Balancer),
            300 => Some(Exchange::Raydium),
            301 => Some(Exchange::Orca),
            302 => Some(Exchange::Jupiter),
            400 => Some(Exchange::Dydx),
            401 => Some(Exchange::Gmx),
            402 => Some(Exchange::Hyperliquid),
            _ => None,
        }
    }

    #[inline]
    pub fn id(self) -> u16 {
        self as u16
    }

    pub fn exchange_type(self) -> ExchangeType {
        match self {
            Exchange::Binance
            | Exchange::Coinbase
            | Exchange::Kraken
            | Exchange::Okx
            | Exchange::Bybit
            | Exchange::Upbit
            | Exchange::Bithumb
            | Exchange::GateIO => ExchangeType::Cex,

            Exchange::UniswapV2 | Exchange::SushiSwap => ExchangeType::CpmmDex,

            Exchange::UniswapV3 | Exchange::Curve | Exchange::Balancer => ExchangeType::ClmmDex,

            Exchange::Raydium | Exchange::Orca | Exchange::Jupiter => ExchangeType::CpmmDex,

            Exchange::Dydx | Exchange::Gmx | Exchange::Hyperliquid => ExchangeType::PerpDex,
        }
    }

    pub fn chain(self) -> Option<Chain> {
        match self {
            // CEX - no specific chain
            Exchange::Binance
            | Exchange::Coinbase
            | Exchange::Kraken
            | Exchange::Okx
            | Exchange::Bybit
            | Exchange::Upbit
            | Exchange::Bithumb
            | Exchange::GateIO => None,

            // EVM DEXes on Ethereum mainnet
            Exchange::UniswapV2
            | Exchange::UniswapV3
            | Exchange::SushiSwap
            | Exchange::Curve
            | Exchange::Balancer => Some(Chain::Ethereum),

            // Solana DEXes
            Exchange::Raydium | Exchange::Orca | Exchange::Jupiter => Some(Chain::Solana),

            // PerpDEX - varies
            Exchange::Dydx => None, // Multi-chain
            Exchange::Gmx => Some(Chain::Arbitrum),
            Exchange::Hyperliquid => None, // Own chain
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Exchange::Binance => "Binance",
            Exchange::Coinbase => "Coinbase",
            Exchange::Kraken => "Kraken",
            Exchange::Okx => "OKX",
            Exchange::Bybit => "Bybit",
            Exchange::UniswapV2 => "Uniswap V2",
            Exchange::UniswapV3 => "Uniswap V3",
            Exchange::SushiSwap => "SushiSwap",
            Exchange::Curve => "Curve",
            Exchange::Balancer => "Balancer",
            Exchange::Raydium => "Raydium",
            Exchange::Orca => "Orca",
            Exchange::Jupiter => "Jupiter",
            Exchange::Dydx => "dYdX",
            Exchange::Gmx => "GMX",
            Exchange::Hyperliquid => "Hyperliquid",
            Exchange::Upbit => "Upbit",
            Exchange::Bithumb => "Bithumb",
            Exchange::GateIO => "Gate.io",
        }
    }

    pub fn all_cex() -> &'static [Exchange] {
        &[
            Exchange::Binance,
            Exchange::Coinbase,
            Exchange::Kraken,
            Exchange::Okx,
            Exchange::Bybit,
            Exchange::Upbit,
            Exchange::Bithumb,
            Exchange::GateIO,
        ]
    }

    pub fn all_dex() -> &'static [Exchange] {
        &[
            Exchange::UniswapV2,
            Exchange::UniswapV3,
            Exchange::SushiSwap,
            Exchange::Curve,
            Exchange::Balancer,
            Exchange::Raydium,
            Exchange::Orca,
            Exchange::Jupiter,
        ]
    }

    pub fn all_perp() -> &'static [Exchange] {
        &[Exchange::Dydx, Exchange::Gmx, Exchange::Hyperliquid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === ExchangeType tests ===

    #[test]
    fn test_exchange_type_from_id() {
        assert_eq!(ExchangeType::from_id(1), Some(ExchangeType::Cex));
        assert_eq!(ExchangeType::from_id(2), Some(ExchangeType::CpmmDex));
        assert_eq!(ExchangeType::from_id(3), Some(ExchangeType::ClmmDex));
        assert_eq!(ExchangeType::from_id(4), Some(ExchangeType::PerpDex));
        assert_eq!(ExchangeType::from_id(5), Some(ExchangeType::Orderbook));
        assert_eq!(ExchangeType::from_id(255), None);
    }

    #[test]
    fn test_exchange_type_is_dex() {
        assert!(!ExchangeType::Cex.is_dex());
        assert!(ExchangeType::CpmmDex.is_dex());
        assert!(ExchangeType::ClmmDex.is_dex());
        assert!(ExchangeType::PerpDex.is_dex());
        assert!(ExchangeType::Orderbook.is_dex());
    }

    // === Exchange tests ===

    #[test]
    fn test_exchange_from_id() {
        assert_eq!(Exchange::from_id(100), Some(Exchange::Binance));
        assert_eq!(Exchange::from_id(200), Some(Exchange::UniswapV2));
        assert_eq!(Exchange::from_id(300), Some(Exchange::Raydium));
        assert_eq!(Exchange::from_id(400), Some(Exchange::Dydx));
        assert_eq!(Exchange::from_id(9999), None);
    }

    #[test]
    fn test_exchange_id() {
        assert_eq!(Exchange::Binance.id(), 100);
        assert_eq!(Exchange::UniswapV3.id(), 201);
        assert_eq!(Exchange::Hyperliquid.id(), 402);
    }

    #[test]
    fn test_exchange_type() {
        assert_eq!(Exchange::Binance.exchange_type(), ExchangeType::Cex);
        assert_eq!(Exchange::Coinbase.exchange_type(), ExchangeType::Cex);
        assert_eq!(Exchange::UniswapV2.exchange_type(), ExchangeType::CpmmDex);
        assert_eq!(Exchange::UniswapV3.exchange_type(), ExchangeType::ClmmDex);
        assert_eq!(Exchange::Gmx.exchange_type(), ExchangeType::PerpDex);
    }

    #[test]
    fn test_exchange_chain() {
        // CEX has no specific chain
        assert_eq!(Exchange::Binance.chain(), None);
        assert_eq!(Exchange::Coinbase.chain(), None);

        // EVM DEXes
        assert_eq!(Exchange::UniswapV2.chain(), Some(Chain::Ethereum));
        assert_eq!(Exchange::UniswapV3.chain(), Some(Chain::Ethereum));

        // Solana DEXes
        assert_eq!(Exchange::Raydium.chain(), Some(Chain::Solana));
        assert_eq!(Exchange::Orca.chain(), Some(Chain::Solana));
    }

    #[test]
    fn test_exchange_as_str() {
        assert_eq!(Exchange::Binance.as_str(), "Binance");
        assert_eq!(Exchange::UniswapV2.as_str(), "Uniswap V2");
        assert_eq!(Exchange::Hyperliquid.as_str(), "Hyperliquid");
    }

    #[test]
    fn test_exchange_all_cex() {
        let cexes = Exchange::all_cex();
        assert!(cexes.contains(&Exchange::Binance));
        assert!(cexes.contains(&Exchange::Coinbase));
        assert!(!cexes.contains(&Exchange::UniswapV2));
    }

    #[test]
    fn test_exchange_all_dex() {
        let dexes = Exchange::all_dex();
        assert!(dexes.contains(&Exchange::UniswapV2));
        assert!(dexes.contains(&Exchange::Raydium));
        assert!(!dexes.contains(&Exchange::Binance));
    }
}

//! Blockchain chain identifiers and utilities.

use serde::{Deserialize, Serialize};

/// Blockchain network identifier.
/// Uses u8 representation for compact serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Chain {
    // EVM chains (1-9)
    Ethereum = 1,
    Arbitrum = 2,
    Optimism = 3,
    Base = 4,
    Polygon = 5,
    Avalanche = 6,
    Bsc = 7,

    // Non-EVM chains (10+)
    Solana = 10,

    // Cosmos ecosystem (20+)
    Cosmos = 20,
    Osmosis = 21,
}

impl Chain {
    /// Create Chain from u8 ID.
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Chain::Ethereum),
            2 => Some(Chain::Arbitrum),
            3 => Some(Chain::Optimism),
            4 => Some(Chain::Base),
            5 => Some(Chain::Polygon),
            6 => Some(Chain::Avalanche),
            7 => Some(Chain::Bsc),
            10 => Some(Chain::Solana),
            20 => Some(Chain::Cosmos),
            21 => Some(Chain::Osmosis),
            _ => None,
        }
    }

    /// Get u8 ID of this chain.
    #[inline]
    pub fn id(self) -> u8 {
        self as u8
    }

    /// Check if this chain is EVM-compatible.
    #[inline]
    pub fn is_evm(self) -> bool {
        matches!(
            self,
            Chain::Ethereum
                | Chain::Arbitrum
                | Chain::Optimism
                | Chain::Base
                | Chain::Polygon
                | Chain::Avalanche
                | Chain::Bsc
        )
    }

    /// Get string representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Chain::Ethereum => "Ethereum",
            Chain::Arbitrum => "Arbitrum",
            Chain::Optimism => "Optimism",
            Chain::Base => "Base",
            Chain::Polygon => "Polygon",
            Chain::Avalanche => "Avalanche",
            Chain::Bsc => "BSC",
            Chain::Solana => "Solana",
            Chain::Cosmos => "Cosmos",
            Chain::Osmosis => "Osmosis",
        }
    }

    /// Get all chain variants.
    pub fn all() -> &'static [Chain] {
        &[
            Chain::Ethereum,
            Chain::Arbitrum,
            Chain::Optimism,
            Chain::Base,
            Chain::Polygon,
            Chain::Avalanche,
            Chain::Bsc,
            Chain::Solana,
            Chain::Cosmos,
            Chain::Osmosis,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_from_id() {
        // Chain::from_id should return Some for valid IDs
        assert_eq!(Chain::from_id(1), Some(Chain::Ethereum));
        assert_eq!(Chain::from_id(2), Some(Chain::Arbitrum));
        assert_eq!(Chain::from_id(10), Some(Chain::Solana));
        // Invalid ID should return None
        assert_eq!(Chain::from_id(255), None);
    }

    #[test]
    fn test_chain_to_id() {
        // Chain should convert to its u8 ID
        assert_eq!(Chain::Ethereum.id(), 1);
        assert_eq!(Chain::Arbitrum.id(), 2);
        assert_eq!(Chain::Solana.id(), 10);
    }

    #[test]
    fn test_chain_is_evm() {
        // EVM chains
        assert!(Chain::Ethereum.is_evm());
        assert!(Chain::Arbitrum.is_evm());
        assert!(Chain::Optimism.is_evm());
        assert!(Chain::Base.is_evm());
        assert!(Chain::Polygon.is_evm());
        assert!(Chain::Avalanche.is_evm());
        assert!(Chain::Bsc.is_evm());

        // Non-EVM chains
        assert!(!Chain::Solana.is_evm());
        assert!(!Chain::Cosmos.is_evm());
        assert!(!Chain::Osmosis.is_evm());
    }

    #[test]
    fn test_chain_display() {
        assert_eq!(Chain::Ethereum.as_str(), "Ethereum");
        assert_eq!(Chain::Solana.as_str(), "Solana");
    }

    #[test]
    fn test_chain_all_variants() {
        // Ensure all variants are covered
        let all = Chain::all();
        assert!(all.len() >= 10);
        assert!(all.contains(&Chain::Ethereum));
        assert!(all.contains(&Chain::Solana));
    }
}

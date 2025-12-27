//! Bridge protocol definitions for cross-chain transfers.

use crate::{Asset, Chain};
use serde::{Deserialize, Serialize};

/// Bridge protocol identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BridgeProtocol {
    Native = 0,     // Exchange's own bridge
    LayerZero = 1,
    Wormhole = 2,
    Stargate = 3,
    Across = 4,
    Hop = 5,
    Synapse = 6,
    Celer = 7,
    Axelar = 8,
}

impl BridgeProtocol {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(BridgeProtocol::Native),
            1 => Some(BridgeProtocol::LayerZero),
            2 => Some(BridgeProtocol::Wormhole),
            3 => Some(BridgeProtocol::Stargate),
            4 => Some(BridgeProtocol::Across),
            5 => Some(BridgeProtocol::Hop),
            6 => Some(BridgeProtocol::Synapse),
            7 => Some(BridgeProtocol::Celer),
            8 => Some(BridgeProtocol::Axelar),
            _ => None,
        }
    }

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BridgeProtocol::Native => "Native",
            BridgeProtocol::LayerZero => "LayerZero",
            BridgeProtocol::Wormhole => "Wormhole",
            BridgeProtocol::Stargate => "Stargate",
            BridgeProtocol::Across => "Across",
            BridgeProtocol::Hop => "Hop",
            BridgeProtocol::Synapse => "Synapse",
            BridgeProtocol::Celer => "Celer",
            BridgeProtocol::Axelar => "Axelar",
        }
    }
}

/// Bridge route configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeRoute {
    pub protocol: BridgeProtocol,
    pub source_chain: Chain,
    pub dest_chain: Chain,
    pub supported_assets: Vec<Asset>,
    /// Fee in basis points
    pub fee_bps: u16,
    /// Estimated transfer time in seconds
    pub estimated_time_secs: u32,
    /// Available liquidity
    pub liquidity: u64,
    /// Whether the route is currently active
    pub is_active: bool,
}

impl BridgeRoute {
    /// Create a new bridge route.
    pub fn new(
        protocol: BridgeProtocol,
        source_chain: Chain,
        dest_chain: Chain,
        fee_bps: u16,
        estimated_time_secs: u32,
    ) -> Self {
        Self {
            protocol,
            source_chain,
            dest_chain,
            supported_assets: Vec::new(),
            fee_bps,
            estimated_time_secs,
            liquidity: 0,
            is_active: true,
        }
    }

    /// Add a supported asset.
    pub fn add_supported_asset(&mut self, asset: Asset) {
        if !self.supported_assets.contains(&asset) {
            self.supported_assets.push(asset);
        }
    }

    /// Check if an asset is supported.
    pub fn supports_asset(&self, asset: &Asset) -> bool {
        self.supported_assets.contains(asset)
    }

    /// Calculate the fee for a given amount (in fixed-point).
    pub fn calculate_fee(&self, amount: u64) -> u64 {
        (amount as u128 * self.fee_bps as u128 / 10000) as u64
    }

    /// Check if this route supports the given direction.
    pub fn is_valid_direction(&self, from: Chain, to: Chain) -> bool {
        self.source_chain == from && self.dest_chain == to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_protocol_from_id() {
        assert_eq!(BridgeProtocol::from_id(0), Some(BridgeProtocol::Native));
        assert_eq!(BridgeProtocol::from_id(1), Some(BridgeProtocol::LayerZero));
        assert_eq!(BridgeProtocol::from_id(2), Some(BridgeProtocol::Wormhole));
        assert_eq!(BridgeProtocol::from_id(255), None);
    }

    #[test]
    fn test_bridge_protocol_as_str() {
        assert_eq!(BridgeProtocol::LayerZero.as_str(), "LayerZero");
        assert_eq!(BridgeProtocol::Wormhole.as_str(), "Wormhole");
        assert_eq!(BridgeProtocol::Stargate.as_str(), "Stargate");
    }

    #[test]
    fn test_bridge_route_new() {
        let route = BridgeRoute::new(
            BridgeProtocol::Stargate,
            Chain::Ethereum,
            Chain::Arbitrum,
            50, // 0.5% fee
            300, // 5 minutes
        );

        assert_eq!(route.protocol, BridgeProtocol::Stargate);
        assert_eq!(route.source_chain, Chain::Ethereum);
        assert_eq!(route.dest_chain, Chain::Arbitrum);
        assert_eq!(route.fee_bps, 50);
        assert_eq!(route.estimated_time_secs, 300);
        assert!(route.is_active);
    }

    #[test]
    fn test_bridge_route_supports_asset() {
        let mut route = BridgeRoute::new(
            BridgeProtocol::Stargate,
            Chain::Ethereum,
            Chain::Arbitrum,
            50,
            300,
        );

        let eth = Asset::eth();
        let sol = Asset::sol();

        route.add_supported_asset(eth.clone());

        assert!(route.supports_asset(&eth));
        assert!(!route.supports_asset(&sol));
    }

    #[test]
    fn test_bridge_route_fee_calculation() {
        let route = BridgeRoute::new(
            BridgeProtocol::Stargate,
            Chain::Ethereum,
            Chain::Arbitrum,
            50, // 0.5% fee = 50 bps
            300,
        );

        // 100 ETH, 0.5% fee = 0.5 ETH
        let amount = 100_00000000u64; // 100 in fixed-point (8 decimals)
        let fee = route.calculate_fee(amount);
        assert_eq!(fee, 50000000u64); // 0.5 in fixed-point
    }

    #[test]
    fn test_bridge_route_is_valid_direction() {
        let route = BridgeRoute::new(
            BridgeProtocol::Stargate,
            Chain::Ethereum,
            Chain::Arbitrum,
            50,
            300,
        );

        assert!(route.is_valid_direction(Chain::Ethereum, Chain::Arbitrum));
        assert!(!route.is_valid_direction(Chain::Arbitrum, Chain::Ethereum));
        assert!(!route.is_valid_direction(Chain::Ethereum, Chain::Solana));
    }
}

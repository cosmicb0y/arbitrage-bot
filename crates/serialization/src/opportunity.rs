//! Arbitrage opportunity serialization using efficient binary format.
//!
//! Layout:
//! - Header: [magic: u32, version: u8, opp_count: u32, batch_id: u64, timestamp: u64]
//! - Opportunities: [OpportunityData; opp_count]

use arbitrage_core::{ArbitrageOpportunity, Asset, Chain, Exchange, FixedPoint};
use thiserror::Error;

/// Magic bytes for format identification
const MAGIC: u32 = 0x4F505054; // "OPPT" - Opportunity
/// Current format version
const VERSION: u8 = 1;
/// Header size: magic(4) + version(1) + count(4) + batch_id(8) + timestamp(8) = 25
const HEADER_SIZE: usize = 25;

/// Serialization errors for opportunities.
#[derive(Debug, Error)]
pub enum OpportunitySerializationError {
    #[error("Invalid magic bytes")]
    InvalidMagic,
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u8),
    #[error("Buffer too small: expected {expected}, got {actual}")]
    BufferTooSmall { expected: usize, actual: usize },
    #[error("Unknown exchange ID: {0}")]
    UnknownExchange(u16),
    #[error("Unknown chain ID: {0}")]
    UnknownChain(u8),
    #[error("Empty batch")]
    EmptyBatch,
}

/// Fixed-size opportunity data for serialization.
/// This captures the core fields needed for transmission.
/// Size: 2+2+8+8+8+8+8+8+4+1+8+8+8+8+8+1+4+1+32 = 127 bytes (variable due to symbol)
#[repr(C)]
#[derive(Clone, Debug)]
struct OpportunityData {
    // Core identifiers (18 bytes)
    source_exchange: u16,
    target_exchange: u16,
    id: u64,
    discovered_at_ms: u64,

    // Price data (24 bytes)
    source_price: u64,
    target_price: u64,
    premium_bps: i32,
    _pad1: u32, // padding for alignment

    // Cost analysis (32 bytes)
    estimated_gas_cost: u64,
    estimated_bridge_fee: u64,
    estimated_trading_fee: u64,
    net_profit_estimate: i64,

    // Execution conditions (17 bytes)
    min_amount: u64,
    max_amount: u64,
    confidence_score: u8,

    // Asset info (variable, stored separately)
    asset_chain: u8,
    asset_decimals: u8,
    asset_symbol_len: u8,
    // asset_symbol follows (variable length, max 16 bytes)
}

// Fixed portion: 2+2+8+8 + 8+8+4+4 + 8+8+8+8 + 8+8+1+1+1+1 = 96 bytes
const OPPORTUNITY_FIXED_SIZE: usize = 96;
const MAX_SYMBOL_LEN: usize = 16;

impl OpportunityData {
    fn from_opportunity(opp: &ArbitrageOpportunity) -> (Self, Vec<u8>) {
        let symbol_bytes = opp.asset.symbol.as_bytes();
        let symbol_len = symbol_bytes.len().min(MAX_SYMBOL_LEN) as u8;

        let data = Self {
            source_exchange: opp.source_exchange as u16,
            target_exchange: opp.target_exchange as u16,
            id: opp.id,
            discovered_at_ms: opp.discovered_at_ms,
            source_price: opp.source_price,
            target_price: opp.target_price,
            premium_bps: opp.premium_bps,
            _pad1: 0,
            estimated_gas_cost: opp.estimated_gas_cost,
            estimated_bridge_fee: opp.estimated_bridge_fee,
            estimated_trading_fee: opp.estimated_trading_fee,
            net_profit_estimate: opp.net_profit_estimate,
            min_amount: opp.min_amount,
            max_amount: opp.max_amount,
            confidence_score: opp.confidence_score,
            asset_chain: opp.asset.chain as u8,
            asset_decimals: opp.asset.decimals,
            asset_symbol_len: symbol_len,
        };

        let symbol_vec = symbol_bytes[..symbol_len as usize].to_vec();
        (data, symbol_vec)
    }

    fn to_opportunity(&self, symbol: &str) -> Result<ArbitrageOpportunity, OpportunitySerializationError> {
        let source_exchange = Exchange::from_id(self.source_exchange)
            .ok_or(OpportunitySerializationError::UnknownExchange(self.source_exchange))?;
        let target_exchange = Exchange::from_id(self.target_exchange)
            .ok_or(OpportunitySerializationError::UnknownExchange(self.target_exchange))?;
        let chain = Chain::from_id(self.asset_chain)
            .ok_or(OpportunitySerializationError::UnknownChain(self.asset_chain))?;

        let asset = Asset::native(symbol, chain, self.asset_decimals);

        let mut opp = ArbitrageOpportunity::new(
            self.id,
            source_exchange,
            target_exchange,
            asset,
            FixedPoint(self.source_price),
            FixedPoint(self.target_price),
        );

        // Restore additional fields
        opp.discovered_at_ms = self.discovered_at_ms;
        opp.estimated_gas_cost = self.estimated_gas_cost;
        opp.estimated_bridge_fee = self.estimated_bridge_fee;
        opp.estimated_trading_fee = self.estimated_trading_fee;
        opp.net_profit_estimate = self.net_profit_estimate;
        opp.min_amount = self.min_amount;
        opp.max_amount = self.max_amount;
        opp.confidence_score = self.confidence_score;

        Ok(opp)
    }

    fn to_bytes(&self) -> [u8; OPPORTUNITY_FIXED_SIZE] {
        let mut buf = [0u8; OPPORTUNITY_FIXED_SIZE];
        let mut offset = 0;

        // Write each field
        buf[offset..offset + 2].copy_from_slice(&self.source_exchange.to_le_bytes());
        offset += 2;
        buf[offset..offset + 2].copy_from_slice(&self.target_exchange.to_le_bytes());
        offset += 2;
        buf[offset..offset + 8].copy_from_slice(&self.id.to_le_bytes());
        offset += 8;
        buf[offset..offset + 8].copy_from_slice(&self.discovered_at_ms.to_le_bytes());
        offset += 8;
        buf[offset..offset + 8].copy_from_slice(&self.source_price.to_le_bytes());
        offset += 8;
        buf[offset..offset + 8].copy_from_slice(&self.target_price.to_le_bytes());
        offset += 8;
        buf[offset..offset + 4].copy_from_slice(&self.premium_bps.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&self._pad1.to_le_bytes());
        offset += 4;
        buf[offset..offset + 8].copy_from_slice(&self.estimated_gas_cost.to_le_bytes());
        offset += 8;
        buf[offset..offset + 8].copy_from_slice(&self.estimated_bridge_fee.to_le_bytes());
        offset += 8;
        buf[offset..offset + 8].copy_from_slice(&self.estimated_trading_fee.to_le_bytes());
        offset += 8;
        buf[offset..offset + 8].copy_from_slice(&self.net_profit_estimate.to_le_bytes());
        offset += 8;
        buf[offset..offset + 8].copy_from_slice(&self.min_amount.to_le_bytes());
        offset += 8;
        buf[offset..offset + 8].copy_from_slice(&self.max_amount.to_le_bytes());
        offset += 8;
        buf[offset] = self.confidence_score;
        offset += 1;
        buf[offset] = self.asset_chain;
        offset += 1;
        buf[offset] = self.asset_decimals;
        offset += 1;
        buf[offset] = self.asset_symbol_len;

        buf
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut offset = 0;

        let source_exchange = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let target_exchange = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let id = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let discovered_at_ms = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let source_price = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let target_price = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let premium_bps = i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let _pad1 = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let estimated_gas_cost = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let estimated_bridge_fee = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let estimated_trading_fee = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let net_profit_estimate = i64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let min_amount = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let max_amount = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let confidence_score = bytes[offset];
        offset += 1;
        let asset_chain = bytes[offset];
        offset += 1;
        let asset_decimals = bytes[offset];
        offset += 1;
        let asset_symbol_len = bytes[offset];

        Self {
            source_exchange,
            target_exchange,
            id,
            discovered_at_ms,
            source_price,
            target_price,
            premium_bps,
            _pad1,
            estimated_gas_cost,
            estimated_bridge_fee,
            estimated_trading_fee,
            net_profit_estimate,
            min_amount,
            max_amount,
            confidence_score,
            asset_chain,
            asset_decimals,
            asset_symbol_len,
        }
    }
}

/// Serializer for single ArbitrageOpportunity.
pub struct OpportunitySerializer;

impl OpportunitySerializer {
    /// Serialize an ArbitrageOpportunity to bytes.
    pub fn to_bytes(opp: &ArbitrageOpportunity) -> Vec<u8> {
        OpportunityBatchSerializer::to_bytes(&[opp.clone()], 0)
    }

    /// Deserialize bytes to an ArbitrageOpportunity.
    pub fn from_bytes(bytes: &[u8]) -> Result<ArbitrageOpportunity, OpportunitySerializationError> {
        let opps = OpportunityBatchSerializer::from_bytes(bytes)?;
        opps.into_iter()
            .next()
            .ok_or(OpportunitySerializationError::EmptyBatch)
    }
}

/// Serializer for batches of ArbitrageOpportunities.
pub struct OpportunityBatchSerializer;

impl OpportunityBatchSerializer {
    /// Serialize a batch of ArbitrageOpportunities to bytes.
    pub fn to_bytes(opportunities: &[ArbitrageOpportunity], batch_id: u64) -> Vec<u8> {
        let opp_count = opportunities.len() as u32;

        // Estimate capacity
        let estimated_size = HEADER_SIZE + opportunities.len() * (OPPORTUNITY_FIXED_SIZE + MAX_SYMBOL_LEN);
        let mut buf = Vec::with_capacity(estimated_size);

        // Write header
        buf.extend_from_slice(&MAGIC.to_le_bytes());
        buf.push(VERSION);
        buf.extend_from_slice(&opp_count.to_le_bytes());
        buf.extend_from_slice(&batch_id.to_le_bytes());

        let timestamp = opportunities
            .first()
            .map(|o| o.discovered_at_ms)
            .unwrap_or(0);
        buf.extend_from_slice(&timestamp.to_le_bytes());

        // Write opportunities
        for opp in opportunities {
            let (data, symbol) = OpportunityData::from_opportunity(opp);
            buf.extend_from_slice(&data.to_bytes());
            buf.extend_from_slice(&symbol);
        }

        buf
    }

    /// Deserialize bytes to a batch of ArbitrageOpportunities.
    pub fn from_bytes(bytes: &[u8]) -> Result<Vec<ArbitrageOpportunity>, OpportunitySerializationError> {
        if bytes.len() < HEADER_SIZE {
            return Err(OpportunitySerializationError::BufferTooSmall {
                expected: HEADER_SIZE,
                actual: bytes.len(),
            });
        }

        // Read header
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != MAGIC {
            return Err(OpportunitySerializationError::InvalidMagic);
        }

        let version = bytes[4];
        if version != VERSION {
            return Err(OpportunitySerializationError::UnsupportedVersion(version));
        }

        let opp_count = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;
        // batch_id at bytes[9..17] - not used
        // timestamp at bytes[17..25] - not used

        // Read opportunities
        let mut opportunities = Vec::with_capacity(opp_count);
        let mut offset = HEADER_SIZE;

        for _ in 0..opp_count {
            if offset + OPPORTUNITY_FIXED_SIZE > bytes.len() {
                return Err(OpportunitySerializationError::BufferTooSmall {
                    expected: offset + OPPORTUNITY_FIXED_SIZE,
                    actual: bytes.len(),
                });
            }

            let data = OpportunityData::from_bytes(&bytes[offset..offset + OPPORTUNITY_FIXED_SIZE]);
            offset += OPPORTUNITY_FIXED_SIZE;

            let symbol_len = data.asset_symbol_len as usize;
            if offset + symbol_len > bytes.len() {
                return Err(OpportunitySerializationError::BufferTooSmall {
                    expected: offset + symbol_len,
                    actual: bytes.len(),
                });
            }

            let symbol = std::str::from_utf8(&bytes[offset..offset + symbol_len])
                .unwrap_or("???");
            offset += symbol_len;

            opportunities.push(data.to_opportunity(symbol)?);
        }

        Ok(opportunities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrage_core::{Exchange, FixedPoint};

    fn create_test_opportunity() -> ArbitrageOpportunity {
        let asset = Asset::eth();
        ArbitrageOpportunity::new(
            1,
            Exchange::Binance,
            Exchange::Coinbase,
            asset,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(50500.0),
        )
    }

    #[test]
    fn test_opportunity_to_bytes() {
        let opp = create_test_opportunity();
        let bytes = OpportunitySerializer::to_bytes(&opp);
        assert!(!bytes.is_empty());
        assert!(bytes.len() >= HEADER_SIZE);
    }

    #[test]
    fn test_opportunity_roundtrip() {
        let original = create_test_opportunity();

        let bytes = OpportunitySerializer::to_bytes(&original);
        let decoded = OpportunitySerializer::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.source_exchange, original.source_exchange);
        assert_eq!(decoded.target_exchange, original.target_exchange);
        assert_eq!(decoded.source_price, original.source_price);
        assert_eq!(decoded.target_price, original.target_price);
        assert_eq!(decoded.premium_bps, original.premium_bps);
    }

    #[test]
    fn test_opportunity_batch_roundtrip() {
        let opportunities = vec![
            {
                let asset = Asset::eth();
                ArbitrageOpportunity::new(
                    1,
                    Exchange::Binance,
                    Exchange::Coinbase,
                    asset,
                    FixedPoint::from_f64(50000.0),
                    FixedPoint::from_f64(50500.0),
                )
            },
            {
                let asset = Asset::btc();
                ArbitrageOpportunity::new(
                    2,
                    Exchange::Kraken,
                    Exchange::Okx,
                    asset,
                    FixedPoint::from_f64(60000.0),
                    FixedPoint::from_f64(60300.0),
                )
            },
        ];

        let bytes = OpportunityBatchSerializer::to_bytes(&opportunities, 1);
        let decoded = OpportunityBatchSerializer::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].id, 1);
        assert_eq!(decoded[1].id, 2);
        assert_eq!(decoded[0].source_exchange, Exchange::Binance);
        assert_eq!(decoded[1].source_exchange, Exchange::Kraken);
    }

    #[test]
    fn test_serialization_is_compact() {
        let opp = create_test_opportunity();
        let bytes = OpportunitySerializer::to_bytes(&opp);
        // Should be reasonably compact (less than 256 bytes for single opportunity)
        assert!(bytes.len() < 256);
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = vec![0x00; HEADER_SIZE];
        bytes[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        let result = OpportunityBatchSerializer::from_bytes(&bytes);
        assert!(matches!(
            result,
            Err(OpportunitySerializationError::InvalidMagic)
        ));
    }

    #[test]
    fn test_buffer_too_small() {
        let bytes = vec![0x4F, 0x50, 0x50, 0x54]; // Just magic
        let result = OpportunityBatchSerializer::from_bytes(&bytes);
        assert!(matches!(
            result,
            Err(OpportunitySerializationError::BufferTooSmall { .. })
        ));
    }
}

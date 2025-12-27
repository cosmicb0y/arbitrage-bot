//! Price data serialization using efficient binary format.
//!
//! Layout:
//! - Header: [magic: u32, version: u8, tick_count: u32, batch_id: u64, timestamp: u64]
//! - Ticks: [PriceTickData; tick_count]

use arbitrage_core::{Exchange, FixedPoint, PriceTick};
use thiserror::Error;

/// Magic bytes for format identification
const MAGIC: u32 = 0x50544B42; // "PTKB" - PriceTick Batch
/// Current format version
const VERSION: u8 = 1;
/// Header size: magic(4) + version(1) + tick_count(4) + batch_id(8) + timestamp(8) = 25
const HEADER_SIZE: usize = 25;

/// Serialization errors.
#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("Invalid magic bytes")]
    InvalidMagic,
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u8),
    #[error("Buffer too small: expected {expected}, got {actual}")]
    BufferTooSmall { expected: usize, actual: usize },
    #[error("Unknown exchange ID: {0}")]
    UnknownExchange(u16),
    #[error("Empty batch")]
    EmptyBatch,
}

/// Raw PriceTick data for binary serialization.
/// Size: 2 + 4 + 8*6 = 54 bytes
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
struct PriceTickData {
    exchange: u16,
    pair_id: u32,
    price: u64,
    bid: u64,
    ask: u64,
    volume_24h: u64,
    liquidity: u64,
    timestamp_ms: u64,
}

const TICK_DATA_SIZE: usize = std::mem::size_of::<PriceTickData>();

impl PriceTickData {
    fn from_tick(tick: &PriceTick) -> Self {
        Self {
            exchange: tick.exchange() as u16,
            pair_id: tick.pair_id(),
            price: tick.price().0,
            bid: tick.bid().0,
            ask: tick.ask().0,
            volume_24h: tick.volume_24h().0,
            liquidity: tick.liquidity().0,
            timestamp_ms: tick.timestamp_ms(),
        }
    }

    fn to_tick(&self) -> Result<PriceTick, SerializationError> {
        let exchange = Exchange::from_id(self.exchange)
            .ok_or(SerializationError::UnknownExchange(self.exchange))?;

        Ok(PriceTick::new(
            exchange,
            self.pair_id,
            FixedPoint(self.price),
            FixedPoint(self.bid),
            FixedPoint(self.ask),
        ))
    }

    fn to_bytes(&self) -> [u8; TICK_DATA_SIZE] {
        unsafe { std::mem::transmute_copy(self) }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const Self) }
    }
}

/// Serializer for single PriceTick.
pub struct PriceTickSerializer;

impl PriceTickSerializer {
    /// Serialize a PriceTick to bytes.
    pub fn to_bytes(tick: &PriceTick) -> Vec<u8> {
        PriceTickBatchSerializer::to_bytes(&[*tick], 0)
    }

    /// Deserialize bytes to a PriceTick.
    pub fn from_bytes(bytes: &[u8]) -> Result<PriceTick, SerializationError> {
        let ticks = PriceTickBatchSerializer::from_bytes(bytes)?;
        ticks.into_iter().next().ok_or(SerializationError::EmptyBatch)
    }
}

/// Serializer for batches of PriceTicks.
pub struct PriceTickBatchSerializer;

impl PriceTickBatchSerializer {
    /// Serialize a batch of PriceTicks to bytes.
    pub fn to_bytes(ticks: &[PriceTick], batch_id: u64) -> Vec<u8> {
        let tick_count = ticks.len() as u32;
        let total_size = HEADER_SIZE + ticks.len() * TICK_DATA_SIZE;
        let mut buf = Vec::with_capacity(total_size);

        // Write header
        buf.extend_from_slice(&MAGIC.to_le_bytes());
        buf.push(VERSION);
        buf.extend_from_slice(&tick_count.to_le_bytes());
        buf.extend_from_slice(&batch_id.to_le_bytes());

        let timestamp = ticks.first().map(|t| t.timestamp_ms()).unwrap_or(0);
        buf.extend_from_slice(&timestamp.to_le_bytes());

        // Write ticks
        for tick in ticks {
            let data = PriceTickData::from_tick(tick);
            buf.extend_from_slice(&data.to_bytes());
        }

        buf
    }

    /// Deserialize bytes to a batch of PriceTicks.
    pub fn from_bytes(bytes: &[u8]) -> Result<Vec<PriceTick>, SerializationError> {
        if bytes.len() < HEADER_SIZE {
            return Err(SerializationError::BufferTooSmall {
                expected: HEADER_SIZE,
                actual: bytes.len(),
            });
        }

        // Read header
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != MAGIC {
            return Err(SerializationError::InvalidMagic);
        }

        let version = bytes[4];
        if version != VERSION {
            return Err(SerializationError::UnsupportedVersion(version));
        }

        let tick_count = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;
        // batch_id at bytes[9..17] - not used for reading
        // timestamp at bytes[17..25] - not used for reading

        let expected_size = HEADER_SIZE + tick_count * TICK_DATA_SIZE;
        if bytes.len() < expected_size {
            return Err(SerializationError::BufferTooSmall {
                expected: expected_size,
                actual: bytes.len(),
            });
        }

        // Read ticks
        let mut ticks = Vec::with_capacity(tick_count);
        let mut offset = HEADER_SIZE;

        for _ in 0..tick_count {
            let data = PriceTickData::from_bytes(&bytes[offset..offset + TICK_DATA_SIZE]);
            ticks.push(data.to_tick()?);
            offset += TICK_DATA_SIZE;
        }

        Ok(ticks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrage_core::Exchange;

    #[test]
    fn test_price_tick_to_bytes() {
        let tick = PriceTick::new(
            Exchange::Binance,
            12345,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50001.0),
        );

        let bytes = PriceTickSerializer::to_bytes(&tick);
        assert!(!bytes.is_empty());
        // Header (25) + 1 tick (54) = 79 bytes
        assert!(bytes.len() >= 20);
    }

    #[test]
    fn test_price_tick_roundtrip() {
        let original = PriceTick::new(
            Exchange::Binance,
            12345,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50001.0),
        );

        let bytes = PriceTickSerializer::to_bytes(&original);
        let decoded = PriceTickSerializer::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.exchange(), original.exchange());
        assert_eq!(decoded.pair_id(), original.pair_id());
        assert_eq!(decoded.price().0, original.price().0);
        assert_eq!(decoded.bid().0, original.bid().0);
        assert_eq!(decoded.ask().0, original.ask().0);
    }

    #[test]
    fn test_price_tick_batch_roundtrip() {
        let ticks = vec![
            PriceTick::new(
                Exchange::Binance,
                1,
                FixedPoint::from_f64(50000.0),
                FixedPoint::from_f64(49999.0),
                FixedPoint::from_f64(50001.0),
            ),
            PriceTick::new(
                Exchange::Coinbase,
                2,
                FixedPoint::from_f64(50100.0),
                FixedPoint::from_f64(50099.0),
                FixedPoint::from_f64(50101.0),
            ),
        ];

        let bytes = PriceTickBatchSerializer::to_bytes(&ticks, 1);
        let decoded = PriceTickBatchSerializer::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].exchange(), Exchange::Binance);
        assert_eq!(decoded[1].exchange(), Exchange::Coinbase);
    }

    #[test]
    fn test_serialization_is_compact() {
        let tick = PriceTick::new(
            Exchange::Binance,
            1,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(49999.0),
            FixedPoint::from_f64(50001.0),
        );

        let bytes = PriceTickSerializer::to_bytes(&tick);
        // Header (25) + 1 tick (54) = 79 bytes, well under 100
        assert!(bytes.len() < 100);
    }

    #[test]
    fn test_batch_serialization_performance() {
        // Create 1000 ticks
        let ticks: Vec<PriceTick> = (0..1000)
            .map(|i| {
                PriceTick::new(
                    Exchange::Binance,
                    i,
                    FixedPoint::from_f64(50000.0 + i as f64),
                    FixedPoint::from_f64(49999.0 + i as f64),
                    FixedPoint::from_f64(50001.0 + i as f64),
                )
            })
            .collect();

        let start = std::time::Instant::now();
        let bytes = PriceTickBatchSerializer::to_bytes(&ticks, 1);
        let serialize_time = start.elapsed();

        let start = std::time::Instant::now();
        let _decoded = PriceTickBatchSerializer::from_bytes(&bytes).unwrap();
        let deserialize_time = start.elapsed();

        // Should complete in under 10ms for 1000 ticks
        assert!(
            serialize_time.as_millis() < 10,
            "Serialization took {:?}",
            serialize_time
        );
        assert!(
            deserialize_time.as_millis() < 10,
            "Deserialization took {:?}",
            deserialize_time
        );
    }

    #[test]
    fn test_invalid_magic() {
        // Header size is 25 bytes, so we need at least 25 bytes to test magic
        let mut bytes = vec![0x00; HEADER_SIZE];
        bytes[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Invalid magic
        let result = PriceTickBatchSerializer::from_bytes(&bytes);
        assert!(matches!(result, Err(SerializationError::InvalidMagic)));
    }

    #[test]
    fn test_buffer_too_small() {
        let bytes = vec![0x50, 0x54, 0x4B, 0x42]; // Just magic, no version
        let result = PriceTickBatchSerializer::from_bytes(&bytes);
        assert!(matches!(result, Err(SerializationError::BufferTooSmall { .. })));
    }
}

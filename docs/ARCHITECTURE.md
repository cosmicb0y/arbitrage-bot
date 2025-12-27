# Crypto Arbitrage Bot - Architecture

## Overview
Multi-chain cryptocurrency arbitrage detection and execution system.

## Tech Stack
- **Language**: Rust
- **GUI**: Tauri v2 + React + TypeScript
- **Serialization**: FlatBuffers (zero-copy, 10,000+ msg/sec)
- **Data Feed**: WebSocket direct connections

## Project Structure

```
arbitrage-bot/
├── crates/
│   ├── core/           # Core data types (shared)
│   ├── serialization/  # FlatBuffers schemas
│   ├── feeds/          # WebSocket data collection
│   ├── engine/         # Arbitrage detection
│   └── executor/       # Trade execution
├── apps/
│   ├── server/         # Headless server
│   └── desktop/        # Tauri desktop app
└── docs/               # Documentation
```

## Core Data Types

### Enums
| Type | Size | Description |
|------|------|-------------|
| `Chain` | u8 | Blockchain identifier (Ethereum=1, Solana=10, etc.) |
| `ExchangeType` | u8 | CEX, CPMM DEX, CLMM DEX, PerpDex, Orderbook |
| `Exchange` | u16 | Exchange identifier (Binance=100, UniswapV2=200, etc.) |

### Price Data
| Type | Size | Description |
|------|------|-------------|
| `PriceTick` | 54 bytes | Real-time price tick (packed struct) |
| `OrderbookSnapshot` | variable | Orderbook bids/asks |

### Arbitrage
| Type | Description |
|------|-------------|
| `PremiumMatrix` | All exchange-pair premiums |
| `ArbitrageOpportunity` | Detected opportunity + route |
| `RouteStep` | Trade/Bridge/Withdraw/Deposit step |

## Fixed-Point Arithmetic
All prices stored as `u64` with 18 decimal places:
- `1.0` = `1_000_000_000_000_000_000`
- `50000.50` = `50000_500_000_000_000_000_000`

## Development Methodology
TDD (Test-Driven Development):
1. Write test first
2. Run test, confirm failure
3. Implement minimum code
4. Run test, confirm success
5. Refactor

## Test Commands
```bash
cargo test --workspace           # All tests
cargo test -p arbitrage-core     # Core crate only
cargo tarpaulin --workspace      # Coverage
```

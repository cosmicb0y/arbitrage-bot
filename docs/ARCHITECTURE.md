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

## Crate Dependencies & Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           EXTERNAL DATA SOURCES                         │
│                    (Binance, Coinbase, Upbit WebSocket)                 │
└─────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            arbitrage-feeds                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  websocket  │  │   adapter   │  │ aggregator  │  │  discovery  │     │
│  │ (WS 연결)   │  │ (파싱/변환) │  │ (가격 취합) │  │ (마켓 탐색) │     │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘     │
│                         │                                               │
│                         ▼                                               │
│                    PriceTick 생성                                       │
└─────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           arbitrage-engine                              │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐                     │
│  │  detector   │  │   premium    │  │    route    │                     │
│  │ (기회 탐지) │  │(프리미엄계산)│  │ (경로 최적) │                     │
│  └─────────────┘  └──────────────┘  └─────────────┘                     │
│         │                                                               │
│         ▼                                                               │
│   ArbitrageOpportunity 생성                                             │
└─────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          arbitrage-executor                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                      │
│  │    order    │  │     cex     │  │     dex     │                      │
│  │ (주문 관리) │  │(CEX 거래소) │  │(DEX 온체인) │                      │
│  └─────────────┘  └─────────────┘  └─────────────┘                      │
│                         │                                               │
│                         ▼                                               │
│                   실제 주문 실행                                        │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                            arbitrage-core                               │
│           (모든 crate가 의존하는 공통 데이터 타입)                      │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐  │
│  │   Asset   │ │ Exchange  │ │ PriceTick │ │Opportunity│ │FixedPoint │  │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘ └───────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Dependency Graph

```
core ◄─────────────────────────────────────────┐
  ▲                                            │
  │                                            │
feeds ◄──────────────┐                         │
  ▲                  │                         │
  │                  │                         │
engine ◄─────────────┤                         │
  ▲                  │                         │
  │                  │                         │
executor ────────────┴─────────────────────────┘
```

### Crate Responsibilities

| Crate             | Role                    | Input                  | Output                                                   |
| ----------------- | ----------------------- | ---------------------- | -------------------------------------------------------- |
| **core**          | Common type definitions | -                      | `PriceTick`, `Exchange`, `Asset`, `ArbitrageOpportunity` |
| **feeds**         | Price data collection   | Exchange WebSocket     | `PriceTick` (price, bid/ask, volume)                     |
| **engine**        | Arbitrage detection     | `PriceTick` stream     | `ArbitrageOpportunity` (premium, route)                  |
| **executor**      | Order execution         | `ArbitrageOpportunity` | Trade execution                                          |
| **serialization** | Binary serialization    | core types             | Binary data (currently unused)                           |

## Core Data Types

### Enums

| Type           | Size | Description                                            |
| -------------- | ---- | ------------------------------------------------------ |
| `Chain`        | u8   | Blockchain identifier (Ethereum=1, Solana=10, etc.)    |
| `ExchangeType` | u8   | CEX, CPMM DEX, CLMM DEX, PerpDex, Orderbook            |
| `Exchange`     | u16  | Exchange identifier (Binance=100, UniswapV2=200, etc.) |

### Price Data

| Type                | Size     | Description                          |
| ------------------- | -------- | ------------------------------------ |
| `PriceTick`         | 54 bytes | Real-time price tick (packed struct) |
| `OrderbookSnapshot` | variable | Orderbook bids/asks                  |

### Arbitrage

| Type                   | Description                        |
| ---------------------- | ---------------------------------- |
| `PremiumMatrix`        | All exchange-pair premiums         |
| `ArbitrageOpportunity` | Detected opportunity + route       |
| `RouteStep`            | Trade/Bridge/Withdraw/Deposit step |

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

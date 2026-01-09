# Crypto Arbitrage Bot - Architecture

## Overview

Multi-chain cryptocurrency arbitrage detection and execution system with real-time Telegram alerts.

## Tech Stack

- **Language**: Rust
- **GUI**: Tauri v2 + React + TypeScript
- **Database**: SQLite (alerts configuration)
- **Data Feed**: WebSocket direct connections to 6 exchanges
- **Alerts**: Telegram Bot API

## Supported Exchanges

| Exchange | Type | Quote Currency | Data Source      | Status     |
| -------- | ---- | -------------- | ---------------- | ---------- |
| Binance  | CEX  | USDT, USDC     | WebSocket + REST | ✅ Active  |
| Coinbase | CEX  | USDT, USDC     | WebSocket (L2)   | ✅ Active  |
| Bybit    | CEX  | USDT, USDC     | WebSocket        | ✅ Active  |
| Gate.io  | CEX  | USDT, USDC     | WebSocket        | ✅ Active  |
| Upbit    | CEX  | KRW            | WebSocket        | ✅ Active  |
| Bithumb  | CEX  | KRW            | WebSocket        | ✅ Active  |
| Kraken   | CEX  | USD            | -                | 🚧 Planned |
| OKX      | CEX  | USDT           | -                | 🚧 Planned |

## Project Structure

```
arbitrage-bot/
├── crates/
│   ├── core/           # Core data types (shared)
│   ├── feeds/          # WebSocket data collection
│   ├── engine/         # Arbitrage detection
│   ├── executor/       # Trade execution
│   └── alerts/         # Telegram notifications
├── apps/
│   ├── server/         # Headless server (main entry point)
│   └── desktop/        # Tauri desktop app
└── docs/               # Documentation
```

## Crate Dependencies & Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           EXTERNAL DATA SOURCES                         │
│     (Binance, Coinbase, Bybit, GateIO, Upbit, Bithumb WebSocket)       │
└─────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            arbitrage-feeds                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  websocket  │  │   adapter   │  │ aggregator  │  │  discovery  │     │
│  │ (WS 연결)   │  │ (파싱/변환) │  │ (가격 취합) │  │ (마켓 탐색) │     │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘     │
│  ┌─────────────┐  ┌─────────────┐                                       │
│  │    rest     │  │symbol_mapping│                                      │
│  │(REST 폴백)  │  │ (심볼 변환) │                                       │
│  └─────────────┘  └─────────────┘                                       │
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
                         ┌───────────┴───────────┐
                         ▼                       ▼
┌─────────────────────────────────┐  ┌─────────────────────────────────┐
│       arbitrage-executor        │  │        arbitrage-alerts         │
│  ┌─────────────┐ ┌────────────┐ │  │  ┌──────────┐  ┌─────────────┐  │
│  │    order    │ │    cex     │ │  │  │ telegram │  │  notifier   │  │
│  │ (주문 관리) │ │(CEX 거래소)│ │  │  │ (봇 핸들)│  │ (알림 발송) │  │
│  └─────────────┘ └────────────┘ │  │  └──────────┘  └─────────────┘  │
│  ┌─────────────┐               │  │  ┌──────────┐  ┌─────────────┐  │
│  │     dex     │               │  │  │    db    │  │   config    │  │
│  │(DEX 온체인) │               │  │  │ (SQLite) │  │ (사용자설정)│  │
│  └─────────────┘               │  │  └──────────┘  └─────────────┘  │
└─────────────────────────────────┘  └─────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                            arbitrage-core                               │
│           (모든 crate가 의존하는 공통 데이터 타입)                      │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐  │
│  │   Asset   │ │ Exchange  │ │ PriceTick │ │Opportunity│ │FixedPoint │  │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘ └───────────┘  │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐                              │
│  │QuoteCurrency│ │  Chain   │ │  Bridge   │                              │
│  └───────────┘ └───────────┘ └───────────┘                              │
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
engine ◄─────────────┼─────────────────────────┤
  ▲                  │                         │
  │                  │                         │
executor ────────────┤                         │
                     │                         │
alerts ──────────────┴─────────────────────────┘
```

### Crate Responsibilities

| Crate        | Role                    | Input                  | Output                                                   |
| ------------ | ----------------------- | ---------------------- | -------------------------------------------------------- |
| **core**     | Common type definitions | -                      | `PriceTick`, `Exchange`, `Asset`, `ArbitrageOpportunity` |
| **feeds**    | Price data collection   | Exchange WebSocket     | `PriceTick` (price, bid/ask, depth)                      |
| **engine**   | Arbitrage detection     | `PriceTick` stream     | `ArbitrageOpportunity` (premium, route)                  |
| **executor** | Order execution         | `ArbitrageOpportunity` | Trade execution                                          |
| **alerts**   | Telegram notifications  | `ArbitrageOpportunity` | Telegram messages                                        |

### Module Details

#### arbitrage-feeds

| Module           | Description                                                                          |
| ---------------- | ------------------------------------------------------------------------------------ |
| `websocket`      | WebSocket connection management                                                      |
| `adapter`        | Exchange-specific message parsing (Binance, Coinbase, Bybit, GateIO, Upbit, Bithumb) |
| `aggregator`     | Price aggregation across exchanges                                                   |
| `discovery`      | Common market discovery                                                              |
| `rest`           | REST API fallback for orderbook depth                                                |
| `symbol_mapping` | Symbol normalization across exchanges                                                |
| `manager`        | Connection state and config                                                          |

#### arbitrage-engine

| Module      | Description                                    |
| ----------- | ---------------------------------------------- |
| `detector`  | Opportunity detection with multi-quote support |
| `premium`   | Premium matrix calculation                     |
| `depth`     | Orderbook depth analysis                       |
| `fee`       | Trading fee calculation                        |
| `orderbook` | Orderbook management                           |
| `route`     | Route optimization (placeholder)               |

#### arbitrage-alerts

| Module     | Description                       |
| ---------- | --------------------------------- |
| `telegram` | Telegram bot command handler      |
| `notifier` | Alert dispatch with deduplication |
| `db`       | SQLite configuration storage      |
| `config`   | User alert configuration          |

## Core Data Types

### Enums

| Type            | Size | Description                                         |
| --------------- | ---- | --------------------------------------------------- |
| `Chain`         | u8   | Blockchain identifier (Ethereum=1, Solana=10, etc.) |
| `ExchangeType`  | u8   | CEX, CPMM DEX, CLMM DEX, PerpDex, Orderbook         |
| `Exchange`      | u16  | Exchange identifier (Binance=100, GateIO=107, etc.) |
| `QuoteCurrency` | u8   | Quote currency (USD=1, USDT=2, USDC=3, KRW=10)      |

### Price Data

| Type                | Size     | Description                          |
| ------------------- | -------- | ------------------------------------ |
| `PriceTick`         | 71 bytes | Real-time price tick (packed struct) |
| `OrderbookSnapshot` | variable | Orderbook bids/asks                  |
| `FixedPoint`        | 8 bytes  | Fixed-point number (8 decimals)      |

### Arbitrage

| Type                   | Description                             |
| ---------------------- | --------------------------------------- |
| `PremiumMatrix`        | All exchange-pair premiums              |
| `ArbitrageOpportunity` | Detected opportunity with depth + route |
| `RouteStep`            | Trade/Bridge/Withdraw/Deposit step      |
| `ExchangePairPremium`  | Premium between two exchanges           |

## Fixed-Point Arithmetic

All prices stored as `u64` with 8 decimal places:

- `1.0` = `100_000_000`
- `50000.50` = `5_000_050_000_000`

## Price Normalization

### Multi-Currency Support

각 거래소는 서로 다른 호가 통화(Quote Currency)를 사용합니다:

| Quote Currency | Exchanges              | Conversion          |
| -------------- | ---------------------- | ------------------- |
| USD            | Coinbase               | 기준 (1:1)          |
| USDT           | Binance, Bybit, GateIO, Coinbase | USDT/USD 환율 적용  |
| USDC           | Binance, Bybit, GateIO, Coinbase | USD와 1:1 (Coinbase), 환율 적용 (기타) |
| KRW            | Upbit, Bithumb         | USDT/KRW → USD 변환 |

### Price Storage (DenominatedPrices)

모든 가격은 세 가지 형태로 저장됩니다:

| Field     | Description         | Example                    |
| --------- | ------------------- | -------------------------- |
| `raw`     | 원본 거래소 가격    | 34,800 USDT (Binance)      |
| `usd`     | USD 정규화 가격     | 34,730.40 USD (USDT=0.998) |
| `usdlike` | USDT/USDC 환산 가격 | 34,800 USDT                |

### Stablecoin Conversion

해외 거래소의 USDT/USDC 가격은 각 거래소의 스테이블코인 환율로 USD 변환됩니다:

```
USD Price = Raw Price × (Stablecoin/USD Rate)
```

디페깅 발생 시 (예: USDT/USD = 0.998):

- Binance BTC/USDT 34,800 → USD 34,730.40
- 정확한 거래소 간 프리미엄 계산 가능

## Premium Types

| Premium        | Description                                        |
| -------------- | -------------------------------------------------- |
| Raw Premium    | Direct price comparison (target - source) / source |
| Kimchi Premium | KRW price via official USD/KRW rate vs overseas    |
| Tether Premium | KRW price via USDT/KRW rate vs overseas            |

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
cargo test -p arbitrage-feeds    # Feeds crate only
cargo tarpaulin --workspace      # Coverage
```

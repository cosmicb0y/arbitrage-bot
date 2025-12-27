# Arbitrage Bot

고성능 암호화폐 차익거래 탐지 및 실행 시스템

## 개요

다중 거래소 간 가격 차이를 실시간으로 모니터링하고 차익거래 기회를 탐지하는 Rust 기반 시스템입니다.

### 주요 기능

- **실시간 가격 피드**: WebSocket을 통한 거래소 연결 (Binance, Coinbase, Kraken, OKX)
- **차익거래 탐지**: 거래소 간 프리미엄 계산 및 기회 탐지
- **데스크톱 GUI**: Tauri + React 기반 모니터링 대시보드
- **CLI 서버**: 헤드리스 모드 지원

## 프로젝트 구조

```
arbitrage-bot/
├── crates/
│   ├── core/           # 핵심 데이터 타입 (Exchange, Chain, FixedPoint 등)
│   ├── serialization/  # 바이너리 직렬화
│   ├── feeds/          # WebSocket 가격 피드 수집
│   ├── engine/         # 차익거래 탐지 엔진
│   └── executor/       # 거래 실행 (CEX/DEX)
├── apps/
│   ├── server/         # CLI 헤드리스 서버
│   └── desktop/        # Tauri 데스크톱 앱
│       ├── src-tauri/  # Rust 백엔드
│       └── src/        # React 프론트엔드
└── Cargo.toml          # 워크스페이스 설정
```

## 요구 사항

- Rust 1.75+
- Node.js 18+
- pnpm

### macOS 추가 요구 사항 (Tauri)

```bash
xcode-select --install
```

## 빠른 시작

### 1. 의존성 설치

```bash
# Rust 크레이트 빌드
cargo build

# 프론트엔드 의존성 (데스크톱 앱용)
cd apps/desktop
pnpm install
```

### 2. CLI 서버 실행

```bash
# 시뮬레이션 모드 (기본)
cargo run -p arbitrage-server

# 라이브 WebSocket 피드
cargo run -p arbitrage-server -- --live

# 옵션
cargo run -p arbitrage-server -- --help
```

### 3. 데스크톱 앱 실행

```bash
cd apps/desktop
pnpm tauri dev
```

## CLI 옵션

```
arbitrage-bot [OPTIONS]

Options:
  -c, --config <FILE>      설정 파일 경로 [default: config.json]
  -p, --min-premium <BPS>  최소 프리미엄 (basis points) [default: 30]
  -m, --mode <MODE>        실행 모드: auto, manual, alert [default: alert]
  -l, --log-level <LEVEL>  로그 레벨: trace, debug, info, warn, error [default: info]
      --dry-run            시뮬레이션 모드 (실제 거래 없음) [default: true]
      --live               라이브 WebSocket 피드 사용
```

## 핵심 개념

### Fixed-Point 가격

모든 가격은 `FixedPoint(u64)`로 저장됩니다 (8자리 소수점).

```rust
let price = FixedPoint::from_f64(50000.0);  // 5000000000000
let value = price.to_f64();                  // 50000.0
```

### 프리미엄 계산

프리미엄은 basis points (bps)로 표현됩니다. 100 bps = 1%.

```rust
// Binance에서 $50,000, Coinbase에서 $50,500이면
// 프리미엄 = (50500 - 50000) / 50000 * 10000 = 100 bps
```

### 거래소 타입

```rust
pub enum Exchange {
    Binance,    // CEX
    Coinbase,   // CEX
    Kraken,     // CEX
    Okx,        // CEX
    Bybit,      // CEX
    UniswapV2,  // DEX
    UniswapV3,  // DEX
    // ...
}
```

## 테스트

```bash
# 전체 테스트
cargo test --workspace

# 특정 크레이트 테스트
cargo test -p arbitrage-core
cargo test -p arbitrage-engine

# 상세 출력
cargo test --workspace -- --nocapture
```

## 아키텍처

```
┌─────────────────────────────────────────────────────────────┐
│                     Desktop App (Tauri)                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Dashboard  │  │Opportunities│  │      Settings       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ IPC Commands
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Rust Backend                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │  Feeds   │  │  Engine  │  │ Executor │  │   Core   │     │
│  │(WebSocket)│  │(Detector)│  │(CEX/DEX) │  │ (Types)  │     │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
└─────────────────────────────────────────────────────────────┘
         │                                         │
         │ WebSocket                               │ REST/Blockchain
         ▼                                         ▼
┌─────────────────┐                    ┌─────────────────────┐
│    Exchanges    │                    │    Execution        │
│ Binance,Coinbase│                    │  (Future)           │
└─────────────────┘                    └─────────────────────┘
```

## 크레이트 상세

### arbitrage-core

핵심 데이터 타입 정의:
- `Exchange`, `Chain`, `ExchangeType`
- `FixedPoint` (고정소수점 가격)
- `PriceTick`, `ArbitrageOpportunity`
- `Asset`, `TradingPair`

### arbitrage-feeds

거래소 WebSocket 연결:
- `WsClient`: WebSocket 클라이언트 (자동 재연결)
- `BinanceAdapter`, `CoinbaseAdapter`: 거래소별 메시지 파서
- `FeedConfig`: 연결 설정

### arbitrage-engine

차익거래 탐지 로직:
- `OpportunityDetector`: 기회 탐지기
- `PremiumMatrix`: 거래소 간 프리미엄 계산
- `RouteFinder`: 최적 경로 탐색

### arbitrage-executor

거래 실행 (개발 중):
- `CexExecutor`: 중앙화 거래소 주문
- `DexExecutor`: DEX 스왑 실행
- `Order`, `OrderStatus`: 주문 관리

## 개발 가이드

### 새 거래소 추가

1. `crates/core/src/exchange.rs`에 거래소 추가
2. `crates/feeds/src/adapter.rs`에 어댑터 구현
3. `crates/feeds/src/feed.rs`에 피드 설정 추가

### 코드 스타일

```bash
# 포맷팅
cargo fmt

# 린트
cargo clippy --workspace
```

## 라이선스

MIT

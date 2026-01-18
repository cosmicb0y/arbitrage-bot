# Arbitrage Bot - 개발 가이드

## 개요

이 문서는 Arbitrage Bot 프로젝트의 개발 환경 설정 및 개발 워크플로우를 안내합니다.

---

## 요구 사항

### 필수 도구

| 도구 | 버전 | 설치 |
|------|------|------|
| **Rust** | 1.75+ | [rustup.rs](https://rustup.rs) |
| **Node.js** | 18+ | [nodejs.org](https://nodejs.org) |
| **pnpm** | 8+ | `npm install -g pnpm` |

### macOS 추가 요구 사항 (Tauri)

```bash
xcode-select --install
```

### Linux 추가 요구 사항 (Tauri)

```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

---

## 프로젝트 설정

### 1. 저장소 클론

```bash
git clone https://github.com/user/arbitrage-bot.git
cd arbitrage-bot
```

### 2. Rust 의존성 빌드

```bash
# 개발 빌드
cargo build

# 릴리스 빌드
cargo build --release
```

### 3. 프론트엔드 의존성 설치

```bash
cd apps/desktop
pnpm install
```

---

## 실행 모드

### CLI 서버

```bash
# 시뮬레이션 모드 (기본)
cargo run -p arbitrage-server

# 라이브 WebSocket 피드
cargo run -p arbitrage-server -- --live

# 모든 옵션 확인
cargo run -p arbitrage-server -- --help
```

#### CLI 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `-c, --config` | `config.json` | 설정 파일 경로 |
| `-p, --min-premium` | 30 | 최소 프리미엄 (bps) |
| `-m, --mode` | `alert` | 실행 모드: auto/manual/alert |
| `-l, --log-level` | `info` | 로그 레벨 |
| `--dry-run` | true | 시뮬레이션 모드 |
| `--live` | false | 라이브 WebSocket 피드 |
| `--ws-port` | 9001 | WebSocket 서버 포트 |
| `--telegram` | false | 텔레그램 알림 활성화 |
| `--db-path` | `data/alerts.db` | SQLite DB 경로 |

### 데스크톱 앱

```bash
cd apps/desktop

# 개발 모드 (핫 리로드)
pnpm tauri dev

# 프로덕션 빌드
pnpm tauri build
```

---

## 환경 변수 설정

프로젝트 루트에 `.env` 파일을 생성합니다:

```bash
# 텔레그램 알림
TELEGRAM_BOT_TOKEN=your_bot_token
TELEGRAM_STATUS_BOT_TOKEN=your_status_bot_token
TELEGRAM_STATUS_CHAT_ID=your_chat_id

# Binance
BINANCE_API_KEY=your_api_key
BINANCE_SECRET_KEY=your_secret_key

# Coinbase (CDP API)
COINBASE_API_KEY_ID=organizations/{org}/apiKeys/{id}
COINBASE_SECRET_KEY="-----BEGIN EC PRIVATE KEY-----
...
-----END EC PRIVATE KEY-----"

# Upbit
UPBIT_ACCESS_KEY=your_access_key
UPBIT_SECRET_KEY=your_secret_key

# Bithumb
BITHUMB_API_KEY=your_api_key
BITHUMB_SECRET_KEY=your_secret_key

# Bybit
BYBIT_API_KEY=your_api_key
BYBIT_SECRET_KEY=your_secret_key

# Gate.io
GATEIO_API_KEY=your_api_key
GATEIO_SECRET_KEY=your_secret_key
```

---

## 테스트

### 전체 워크스페이스 테스트

```bash
cargo test --workspace
```

### 특정 크레이트 테스트

```bash
cargo test -p arbitrage-core
cargo test -p arbitrage-feeds
cargo test -p arbitrage-engine
cargo test -p arbitrage-executor
cargo test -p arbitrage-alerts
```

### 상세 출력

```bash
cargo test --workspace -- --nocapture
```

### 단일 테스트 실행

```bash
cargo test -p arbitrage-engine test_optimal_size_basic
```

---

## 코드 스타일

### 포맷팅

```bash
# 전체 포맷
cargo fmt

# 포맷 확인 (CI용)
cargo fmt -- --check
```

### 린트

```bash
# 전체 린트
cargo clippy --workspace

# 경고를 오류로 처리 (CI용)
cargo clippy --workspace -- -D warnings
```

### 권장 IDE 설정

#### VS Code

`.vscode/settings.json`:
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

---

## 새 거래소 추가

### 1. Exchange enum 추가

`crates/core/src/exchange.rs`:
```rust
pub enum Exchange {
    // ... 기존 거래소
    NewExchange = 108,  // 새 ID 할당
}
```

### 2. 어댑터 구현

`crates/feeds/src/adapter/new_exchange.rs`:
```rust
use crate::adapter::ExchangeAdapter;
use arbitrage_core::{Exchange, QuoteCurrency};

pub struct NewExchangeAdapter;

impl ExchangeAdapter for NewExchangeAdapter {
    fn exchange(&self) -> Exchange {
        Exchange::NewExchange
    }

    fn parse_message(&self, msg: &str) -> Option<ParsedTick> {
        // WebSocket 메시지 파싱
    }

    fn subscription_message(&self, symbols: &[&str]) -> String {
        // 구독 메시지 생성
    }
}
```

### 3. 러너 구현

`crates/feeds/src/runner/new_exchange.rs`:
```rust
pub async fn run_new_exchange_runner(
    mut rx: mpsc::Receiver<WsMessage>,
    tx: mpsc::Sender<FeedMessage>,
) {
    let adapter = NewExchangeAdapter;

    while let Some(msg) = rx.recv().await {
        match msg {
            WsMessage::Text(text) => {
                if let Some(tick) = adapter.parse_message(&text) {
                    let _ = tx.send(FeedMessage::Tick(tick)).await;
                }
            }
            WsMessage::Connected => {
                let _ = tx.send(FeedMessage::Event(
                    ConnectionEvent::Connected(Exchange::NewExchange)
                )).await;
            }
            // ... 기타 이벤트 처리
        }
    }
}
```

### 4. 서버에 핸들러 추가

`apps/server/src/main.rs`:
```rust
// WebSocket 피드 스폰
let (new_exchange_tx, new_exchange_rx) = mpsc::channel(5000);
let new_exchange_client = WsClient::new(
    FeedConfig::new_exchange(),
    new_exchange_tx,
);

tokio::spawn(new_exchange_client.run(shutdown_rx.clone()));
tokio::spawn(run_new_exchange_runner(new_exchange_rx, feed_tx.clone()));
```

### 5. 데스크톱 API 클라이언트 추가

`apps/desktop/src-tauri/src/exchange_client.rs`:
```rust
pub async fn fetch_new_exchange_wallet(
    api_key: &str,
    secret_key: &str,
) -> Result<ExchangeWalletInfo, String> {
    // API 호출 구현
}
```

---

## 프로젝트 구조 상세

### 워크스페이스 레이아웃

```
arbitrage-bot/
├── Cargo.toml              # 워크스페이스 정의
├── crates/
│   ├── core/               # 핵심 데이터 타입
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── asset.rs        # Asset, TradingPair
│   │       ├── chain.rs        # Chain enum
│   │       ├── exchange.rs     # Exchange, ExchangeType
│   │       ├── price.rs        # FixedPoint, PriceTick
│   │       ├── quote_currency.rs
│   │       ├── opportunity.rs  # ArbitrageOpportunity
│   │       ├── execution.rs    # ExecutionConfig
│   │       └── bridge.rs       # BridgeProtocol
│   ├── feeds/              # WebSocket 피드
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── websocket.rs    # WsClient, CircuitBreaker
│   │       ├── message.rs      # FeedMessage, ParsedTick
│   │       ├── adapter/        # 거래소별 파서
│   │       ├── runner/         # 거래소별 러너
│   │       ├── aggregator.rs   # PriceAggregator
│   │       └── error.rs
│   ├── engine/             # 탐지 엔진
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── detector.rs     # OpportunityDetector
│   │       ├── premium.rs      # PremiumMatrix
│   │       ├── depth.rs        # Optimal size 계산
│   │       ├── orderbook.rs    # OrderbookCache
│   │       ├── fee.rs          # FeeManager
│   │       └── route.rs        # RouteFinder
│   ├── executor/           # 거래 실행 (개발 중)
│   └── alerts/             # 텔레그램 알림
├── apps/
│   ├── server/             # CLI 서버
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── state.rs        # AppState
│   │       ├── config.rs       # AppConfig
│   │       ├── ws_server.rs    # WebSocket 브로드캐스트
│   │       ├── feeds/          # FeedHandler
│   │       ├── wallet_status.rs
│   │       └── exchange_rate.rs
│   └── desktop/            # Tauri 앱
│       ├── package.json
│       ├── src/            # React 프론트엔드
│       │   ├── App.tsx
│       │   ├── components/
│       │   └── hooks/
│       │       └── useTauri.ts
│       └── src-tauri/      # Rust 백엔드
│           ├── Cargo.toml
│           └── src/
│               ├── main.rs
│               ├── state.rs
│               ├── commands.rs
│               ├── exchange_client.rs
│               └── credentials.rs
└── docs/                   # 문서
```

---

## 디버깅

### 로그 레벨 설정

```bash
# 상세 디버그 로그
cargo run -p arbitrage-server -- --log-level trace

# 특정 모듈만 디버그
RUST_LOG=arbitrage_feeds=debug cargo run -p arbitrage-server
```

### WebSocket 연결 디버깅

```bash
# wscat으로 WebSocket 테스트
npm install -g wscat
wscat -c ws://localhost:9001/ws
```

### 메모리 프로파일링

```bash
# macOS
cargo instruments -t Allocations --release -p arbitrage-server

# Linux
valgrind --tool=massif ./target/release/arbitrage-bot
```

---

## CI/CD

### GitHub Actions 워크플로우 예제

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt -- --check
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo test --workspace

  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release --workspace
```

---

## 문제 해결

### 빌드 오류

#### OpenSSL 관련 오류 (Linux)
```bash
sudo apt install pkg-config libssl-dev
```

#### Tauri 빌드 오류 (macOS)
```bash
xcode-select --install
```

### 런타임 오류

#### WebSocket 연결 실패
- 포트 9001이 사용 중인지 확인
- 방화벽 설정 확인

#### 거래소 연결 실패
- API 키 유효성 확인
- 거래소 상태 페이지 확인
- VPN/프록시 연결 확인

---

## 기여 가이드라인

### 커밋 컨벤션

```
feat: 새 기능 추가
fix: 버그 수정
docs: 문서 변경
refactor: 리팩토링
test: 테스트 추가/수정
chore: 빌드/설정 변경
```

### PR 프로세스

1. 이슈 생성 또는 기존 이슈 할당
2. feature 브랜치 생성: `git checkout -b feat/new-feature`
3. 변경 사항 커밋
4. 테스트 통과 확인: `cargo test --workspace`
5. PR 생성 및 리뷰 요청

---

*문서 생성일: 2026-01-11 | 버전: 0.1.0*

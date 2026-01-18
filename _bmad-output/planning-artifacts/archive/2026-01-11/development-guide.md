# Development Guide - 개발 가이드

**생성일**: 2026-01-11

이 문서는 arbitrage-bot 프로젝트의 개발 환경 설정 및 개발 가이드를 제공합니다.

---

## 요구 사항

### 필수

| 도구 | 버전 | 용도 |
|------|------|------|
| Rust | 1.75+ | 백엔드 개발 |
| Node.js | 18+ | 프론트엔드 빌드 |
| pnpm | 8+ | Node.js 패키지 관리 |

### macOS 추가 (Tauri용)

```bash
xcode-select --install
```

### Windows 추가 (Tauri용)

- Visual Studio Build Tools
- WebView2 Runtime

---

## 환경 설정

### 1. 저장소 클론

```bash
git clone https://github.com/user/arbitrage-bot.git
cd arbitrage-bot
```

### 2. Rust 크레이트 빌드

```bash
# 개발 빌드
cargo build

# 릴리스 빌드
cargo build --release

# 특정 크레이트 빌드
cargo build -p arbitrage-core
cargo build -p arbitrage-server
```

### 3. 프론트엔드 의존성 설치

```bash
cd apps/desktop
pnpm install
```

### 4. 환경 변수 설정

프로젝트 루트에 `.env` 파일을 생성합니다:

```bash
# 텔레그램 봇 (선택적)
TELEGRAM_BOT_TOKEN=your_bot_token

# 거래소 API 키 (선택적, 자격증명 관리는 Settings에서)
# BINANCE_API_KEY=...
# BINANCE_SECRET_KEY=...
```

---

## 개발 실행

### CLI 서버

```bash
# 시뮬레이션 모드 (기본)
cargo run -p arbitrage-server

# 라이브 WebSocket 피드
cargo run -p arbitrage-server -- --live

# 상세 로그
cargo run -p arbitrage-server -- --log-level debug

# 전체 옵션
cargo run -p arbitrage-server -- --help
```

**주요 옵션**:

| 옵션 | 설명 | 기본값 |
|------|------|--------|
| `--live` | 라이브 피드 사용 | false |
| `--ws-port` | WebSocket 포트 | 9001 |
| `--min-premium` | 최소 프리미엄 (bps) | 30 |
| `--telegram` | 텔레그램 알림 | false |
| `--log-level` | 로그 레벨 | info |

### 데스크톱 앱

```bash
cd apps/desktop

# 개발 모드 (핫 리로드)
pnpm tauri dev

# 프론트엔드만 개발
pnpm dev

# 프로덕션 빌드
pnpm tauri build
```

---

## 테스트

### 전체 테스트

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

### 테스트 커버리지

```bash
cargo tarpaulin --workspace
```

---

## 코드 품질

### 포맷팅

```bash
cargo fmt
cargo fmt --check  # CI용
```

### 린트

```bash
cargo clippy --workspace
cargo clippy --workspace -- -D warnings  # 경고를 오류로
```

### 프론트엔드 린트

```bash
cd apps/desktop
pnpm lint
```

---

## 새 거래소 추가

### 1. Core에 거래소 추가

`crates/core/src/exchange.rs`:

```rust
#[repr(u16)]
pub enum Exchange {
    // 기존 거래소...
    NewExchange = 108,  // 새 ID 할당
}

impl Exchange {
    pub fn as_str(self) -> &'static str {
        match self {
            // ...
            Self::NewExchange => "NewExchange",
        }
    }
}
```

### 2. 피드 어댑터 생성

`crates/feeds/src/adapter/new_exchange.rs`:

```rust
use crate::{FeedMessage, ParsedTick};
use arbitrage_core::{Exchange, FixedPoint};

pub struct NewExchangeAdapter;

impl NewExchangeAdapter {
    pub fn parse_message(&self, msg: &str) -> Option<ParsedTick> {
        // 거래소별 메시지 파싱 로직
        todo!()
    }

    pub fn subscribe_message(&self, symbols: &[String]) -> String {
        // 구독 메시지 생성
        todo!()
    }
}
```

### 3. 어댑터 Export

`crates/feeds/src/adapter/mod.rs`:

```rust
mod new_exchange;
pub use new_exchange::NewExchangeAdapter;
```

### 4. 서버에 피드 핸들러 추가

`apps/server/src/main.rs`의 `spawn_live_feeds()`:

```rust
// NewExchange 피드 추가
let new_exchange_runner = FeedRunner::new_exchange(
    aggregator.clone(),
    symbols.clone(),
);
// 러너 스포닝...
```

---

## 아키텍처 패턴

### Event-Driven 구조

```
Exchange WebSocket → FeedRunner → FeedHandler → SharedState
                                       ↓
                                  Detector
                                       ↓
                              ArbitrageOpportunity
                                    ↓   ↓
                              Alerts   WsBroadcast
```

### 동시성 패턴

| 패턴 | 사용처 | 목적 |
|------|--------|------|
| DashMap | Detector, State | Lock-free 동시 접근 |
| mpsc Channel | FeedRunner → Handler | 비동기 메시지 전달 |
| broadcast Channel | WsServer | 다중 클라이언트 브로드캐스트 |
| RwLock | Executor orders | 주문 상태 관리 |

### Fixed-Point 산술

모든 가격은 `FixedPoint(u64)`로 저장됩니다 (8자리 소수점):

```rust
let price = FixedPoint::from_f64(50000.0);  // 5000000000000
let value = price.to_f64();                  // 50000.0

// 프리미엄 계산 (bps)
let premium = FixedPoint::premium_bps(buy_price, sell_price);
```

---

## 디버깅

### 로그 레벨

```bash
# 특정 모듈 디버그
RUST_LOG=arbitrage_feeds=debug cargo run -p arbitrage-server

# 전체 trace
RUST_LOG=trace cargo run -p arbitrage-server
```

### WebSocket 디버깅

```bash
# wscat으로 연결
wscat -c ws://localhost:9001/ws

# 메시지 확인
{"type":"prices","data":[...]}
```

### Tauri 디버깅

```bash
# 개발 도구 열기
pnpm tauri dev

# Rust 백엔드 로그
RUST_BACKTRACE=1 pnpm tauri dev
```

---

## 빌드 & 배포

### Docker

```bash
# 이미지 빌드
docker build -t arbitrage-bot .

# 컨테이너 실행
docker run -p 9001:9001 arbitrage-bot

# Docker Compose
docker-compose up -d
```

### 릴리스 빌드

```bash
# CLI 서버
cargo build --release -p arbitrage-server

# 데스크톱 앱
cd apps/desktop
pnpm tauri build
```

### 바이너리 위치

| 플랫폼 | 경로 |
|--------|------|
| CLI Server | `target/release/arbitrage-bot` |
| macOS App | `apps/desktop/src-tauri/target/release/bundle/macos/` |
| Windows | `apps/desktop/src-tauri/target/release/bundle/msi/` |
| Linux | `apps/desktop/src-tauri/target/release/bundle/deb/` |

---

## 코드 스타일

### Rust

- Edition 2021
- rustfmt 기본 설정
- clippy 권장사항 준수
- 모든 pub 아이템에 문서 주석

### TypeScript

- ESLint 설정 준수
- 컴포넌트는 함수형 + Hooks
- 타입 명시 (any 금지)

### 커밋 메시지

```
feat(feeds): add GateIO WebSocket adapter
fix(engine): correct premium calculation for KRW pairs
docs: update API contracts documentation
refactor(server): extract feed handler to separate module
test(core): add FixedPoint arithmetic tests
```

---

## 문제 해결

### WebSocket 연결 실패

1. 서버가 실행 중인지 확인
2. 포트 충돌 확인 (`lsof -i :9001`)
3. CORS 설정 확인

### Tauri 빌드 실패

```bash
# Xcode 도구 재설치
xcode-select --install

# Rust 타겟 추가
rustup target add aarch64-apple-darwin
```

### 거래소 연결 실패

1. 네트워크 연결 확인
2. API 키 유효성 확인
3. IP 화이트리스트 확인
4. Rate limit 확인

---

## 리소스

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Tauri Guides](https://tauri.app/v1/guides/)
- [React Docs](https://react.dev/)

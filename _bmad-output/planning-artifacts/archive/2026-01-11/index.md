# arbitrage-bot - 프로젝트 문서 인덱스

**생성일**: 2026-01-11
**스캔 레벨**: Exhaustive (전체 소스 분석)
**워크플로우 버전**: 1.2.0

---

## 프로젝트 개요

| 항목 | 값 |
|------|-----|
| **프로젝트명** | arbitrage-bot |
| **도메인** | 암호화폐 차익거래 시스템 |
| **저장소 타입** | Monorepo (Cargo Workspace) |
| **주요 언어** | Rust 2021, TypeScript 5.5 |
| **아키텍처** | Event-Driven, WebSocket 기반 |
| **라이선스** | Private |

### 핵심 기능

- **실시간 가격 모니터링**: 6개 거래소 WebSocket 피드
- **차익거래 감지**: 프리미엄, 뎁스, 라우트 기반 기회 탐지
- **거래 실행**: CEX/DEX 통합 실행 엔진
- **알림 시스템**: Telegram 봇 + SQLite 저장
- **데스크톱 앱**: Tauri 2.0 + React 대시보드

---

## 문서 구조

### 생성된 문서

| 문서 | 설명 | 대상 |
|------|------|------|
| [project-overview.md](project-overview.md) | 프로젝트 요약, 기술 스택, 구조 | 전체 이해 |
| [api-contracts.md](api-contracts.md) | WebSocket 메시지, CLI 옵션, 백그라운드 작업 | API 개발자 |
| [component-inventory.md](component-inventory.md) | React 컴포넌트, Hooks, Tauri 명령어 | 프론트엔드 개발 |
| [source-tree-analysis.md](source-tree-analysis.md) | 디렉토리 구조, 파일 목적, LOC | 탐색/온보딩 |
| [development-guide.md](development-guide.md) | 환경 설정, 개발 실행, 테스트, 디버깅 | 개발자 |

### 기존 문서

| 문서 | 설명 |
|------|------|
| [README.md](../README.md) | 빠른 시작, 기능 요약 |
| [ARCHITECTURE.md](ARCHITECTURE.md) | 상세 아키텍처, 크레이트 의존성 |
| [DATA_MODEL.md](DATA_MODEL.md) | ERD, 데이터 구조, WebSocket 형식 |

---

## 프로젝트 파트별 참조

### Server (apps/server)

**타입**: Backend CLI Server
**기술**: Rust + Tokio + Axum

| 주제 | 참조 문서 |
|------|----------|
| CLI 옵션 | [api-contracts.md#cli-인터페이스](api-contracts.md#cli-인터페이스) |
| WebSocket API | [api-contracts.md#websocket-메시지-타입](api-contracts.md#websocket-메시지-타입) |
| 실행 방법 | [development-guide.md#cli-서버](development-guide.md#cli-서버) |

### Desktop (apps/desktop)

**타입**: Tauri Desktop App
**기술**: Tauri 2.0 + React 18 + TypeScript

| 주제 | 참조 문서 |
|------|----------|
| UI 컴포넌트 | [component-inventory.md#react-컴포넌트](component-inventory.md#react-컴포넌트) |
| 커스텀 Hooks | [component-inventory.md#커스텀-hooks](component-inventory.md#커스텀-hooks) |
| Tauri 명령어 | [component-inventory.md#tauri-ipc-명령어](component-inventory.md#tauri-ipc-명령어) |
| 실행 방법 | [development-guide.md#데스크톱-앱](development-guide.md#데스크톱-앱) |

### Libraries (crates/)

**타입**: Rust Library Crates
**크레이트 수**: 5개

| 크레이트 | 역할 | 참조 |
|----------|------|------|
| `arbitrage-core` | 데이터 타입 (Exchange, FixedPoint, PriceTick) | [project-overview.md](project-overview.md) |
| `arbitrage-feeds` | 거래소 WebSocket 어댑터 (6개) | [api-contracts.md](api-contracts.md) |
| `arbitrage-engine` | 차익거래 감지 (프리미엄, 뎁스, 라우트) | [ARCHITECTURE.md](ARCHITECTURE.md) |
| `arbitrage-executor` | 거래 실행 (CEX/DEX) | [ARCHITECTURE.md](ARCHITECTURE.md) |
| `arbitrage-alerts` | Telegram 알림 + SQLite | [api-contracts.md](api-contracts.md) |

---

## 빠른 시작

### 필수 요구 사항

```bash
# Rust 1.75+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js 18+ & pnpm
brew install node pnpm

# macOS: Xcode CLI 도구
xcode-select --install
```

### 개발 환경 실행

```bash
# 저장소 클론
git clone https://github.com/user/arbitrage-bot.git
cd arbitrage-bot

# CLI 서버 실행 (시뮬레이션 모드)
cargo run -p arbitrage-server

# 라이브 피드로 실행
cargo run -p arbitrage-server -- --live

# 데스크톱 앱 실행
cd apps/desktop
pnpm install
pnpm tauri dev
```

### 테스트 실행

```bash
# 전체 테스트
cargo test --workspace

# 특정 크레이트 테스트
cargo test -p arbitrage-core
cargo test -p arbitrage-feeds
```

---

## 아키텍처 개요

```
┌─────────────────────────────────────────────────────────────────┐
│                        Data Flow                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Exchange WebSocket ──→ FeedRunner ──→ FeedHandler ──→ SharedState
│  (Binance, Upbit,       (tokio task)   (parse/update)   (DashMap) │
│   Bithumb, Bybit,                                                │
│   Coinbase, GateIO)                                              │
│                                                                  │
│                              ↓                                   │
│                          Detector                                │
│                    (premium/depth/route)                         │
│                              ↓                                   │
│                    ArbitrageOpportunity                          │
│                         ↓       ↓                                │
│                     Alerts   WsBroadcast ──→ Desktop/Web Client  │
│                   (Telegram)   (Axum)                            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 주요 패턴

### 동시성

| 패턴 | 사용처 | 목적 |
|------|--------|------|
| `DashMap` | Detector, SharedState | Lock-free 동시 접근 |
| `mpsc Channel` | FeedRunner → Handler | 비동기 메시지 전달 |
| `broadcast Channel` | WsServer | 다중 클라이언트 브로드캐스트 |
| `RwLock` | Executor orders | 주문 상태 관리 |

### Fixed-Point 산술

모든 가격은 `FixedPoint(u64)`로 저장 (8자리 소수점):

```rust
let price = FixedPoint::from_f64(50000.0);  // 5000000000000
let premium = FixedPoint::premium_bps(buy_price, sell_price);
```

---

## 새 거래소 추가

1. **Core에 거래소 추가**: `crates/core/src/exchange.rs`
2. **피드 어댑터 생성**: `crates/feeds/src/adapter/new_exchange.rs`
3. **어댑터 Export**: `crates/feeds/src/adapter/mod.rs`
4. **서버에 핸들러 추가**: `apps/server/src/main.rs`

상세 가이드: [development-guide.md#새-거래소-추가](development-guide.md#새-거래소-추가)

---

## 문서 메타데이터

| 항목 | 값 |
|------|-----|
| 생성 도구 | BMAD Document-Project Workflow |
| 스캔 모드 | initial_scan |
| 스캔 레벨 | exhaustive |
| 총 파일 수 | ~15,400 LOC |
| 지원 거래소 | 6개 (Binance, Upbit, Bithumb, Bybit, Coinbase, GateIO) |

---

*이 문서는 AI 에이전트의 빠른 컨텍스트 로딩을 위해 최적화되었습니다.*

# Arbitrage Bot - 소스 트리 분석

## 개요

이 문서는 프로젝트의 디렉토리 구조와 각 파일의 역할을 상세히 설명합니다.

---

## 전체 구조

```
arbitrage-bot/
├── Cargo.toml                 # 워크스페이스 루트 설정
├── README.md                  # 프로젝트 README
├── .env                       # 환경 변수 (Git 제외)
│
├── crates/                    # Rust 라이브러리 크레이트
│   ├── core/                  # 핵심 데이터 타입
│   ├── feeds/                 # WebSocket 가격 피드
│   ├── engine/                # 차익거래 탐지 엔진
│   ├── executor/              # 거래 실행 (개발 중)
│   └── alerts/                # 텔레그램 알림
│
├── apps/                      # 애플리케이션
│   ├── server/                # CLI 헤드리스 서버
│   └── desktop/               # Tauri 데스크톱 앱
│
├── docs/                      # 프로젝트 문서
│
└── data/                      # 런타임 데이터 (Git 제외)
    └── alerts.db              # SQLite DB
```

---

## crates/core/ - 핵심 데이터 타입

**역할**: 모든 크레이트가 공유하는 기본 데이터 타입 정의

```
crates/core/
├── Cargo.toml
└── src/
    ├── lib.rs                 # 모듈 재export
    ├── asset.rs               # Asset, TradingPair
    ├── bridge.rs              # BridgeProtocol, BridgeRoute
    ├── chain.rs               # Chain enum (10+ 블록체인)
    ├── exchange.rs            # Exchange, ExchangeType (17 거래소)
    ├── execution.rs           # ExecutionConfig, OrderStatus
    ├── opportunity.rs         # ArbitrageOpportunity, RouteStep
    ├── price.rs               # FixedPoint, PriceTick, OrderbookSnapshot
    └── quote_currency.rs      # QuoteCurrency (USD, USDT, USDC, KRW)
```

### 주요 파일 설명

| 파일 | 주요 타입 | 설명 |
|------|----------|------|
| `asset.rs` | `Asset`, `TradingPair` | 자산 및 거래 페어 정의 |
| `chain.rs` | `Chain` | EVM, Solana, Cosmos 등 체인 |
| `exchange.rs` | `Exchange`, `ExchangeType` | CEX, DEX, PerpDEX 분류 |
| `price.rs` | `FixedPoint`, `PriceTick` | 8자리 고정소수점, 가격 틱 |
| `opportunity.rs` | `ArbitrageOpportunity` | 기회 + 라우트 + 최적 사이즈 |
| `execution.rs` | `ExecutionConfig` | 실행 모드, 슬리피지, 포지션 한도 |
| `bridge.rs` | `BridgeProtocol` | LayerZero, Wormhole 등 9개 브릿지 |

---

## crates/feeds/ - WebSocket 가격 피드

**역할**: 6개 거래소 WebSocket 연결 및 메시지 파싱

```
crates/feeds/
├── Cargo.toml
└── src/
    ├── lib.rs                 # 모듈 재export
    ├── websocket.rs           # WsClient, CircuitBreaker
    ├── message.rs             # FeedMessage, ParsedTick, ConnectionEvent
    ├── feed.rs                # FeedHandler 트레이트
    ├── manager.rs             # FeedConfig, ConnectionState
    ├── aggregator.rs          # PriceAggregator (DashMap 캐시)
    ├── discovery.rs           # MarketInfo, CommonMarkets
    ├── rest.rs                # REST API 오더북 페처
    ├── symbol_mapping.rs      # 심볼 매핑 (동명이인 처리)
    ├── error.rs               # FeedError, 재시도 분류
    │
    ├── adapter/               # 거래소별 메시지 파서
    │   ├── mod.rs
    │   ├── binance.rs         # Binance WebSocket 파서
    │   ├── coinbase.rs        # Coinbase (L2 오더북 유지)
    │   ├── bybit.rs           # Bybit (스냅샷/델타)
    │   ├── gateio.rs          # Gate.io (듀얼 핑)
    │   ├── upbit.rs           # Upbit (MessagePack)
    │   └── bithumb.rs         # Bithumb (MessagePack)
    │
    └── runner/                # 거래소별 메시지 처리기
        ├── mod.rs
        ├── binance.rs         # Binance 러너
        ├── coinbase.rs        # Coinbase 러너 (BTreeMap 오더북)
        ├── bybit.rs           # Bybit 러너
        ├── gateio.rs          # Gate.io 러너
        ├── upbit.rs           # Upbit 러너
        └── bithumb.rs         # Bithumb 러너
```

### Runner/Adapter 분리 패턴

| 구성요소 | 역할 | 특징 |
|----------|------|------|
| **Adapter** | 메시지 파싱 | 거래소 JSON → 구조체 |
| **Runner** | 상태 관리 | 오더북 유지, 스테이블코인 감지 |

### WebSocket 클라이언트 기능

- **Circuit Breaker**: 10회 실패 → 5분 차단
- **지수 백오프**: 1초 ~ 5분
- **스테일 감지**: 2분 무응답 시 재연결
- **핑/퐁**: 프로토콜 + 앱 레벨

---

## crates/engine/ - 차익거래 탐지 엔진

**역할**: 가격 비교, 프리미엄 계산, 최적 거래량 분석

```
crates/engine/
├── Cargo.toml
└── src/
    ├── lib.rs                 # 모듈 재export
    ├── detector.rs            # OpportunityDetector (DashMap 기반)
    ├── premium.rs             # PremiumMatrix, DenominatedPrices
    ├── depth.rs               # calculate_optimal_size(), DepthFeeConfig
    ├── orderbook.rs           # OrderbookCache (BTreeMap)
    ├── fee.rs                 # FeeConfig, FeeManager
    └── route.rs               # RouteFinder, RouteBuilder
```

### 핵심 알고리즘

| 파일 | 알고리즘 | 설명 |
|------|----------|------|
| `detector.rs` | Event-Driven Detection | pair_id별 탐지 (전체 스캔 X) |
| `premium.rs` | Multi-Denomination | Raw, USDlike, Kimchi 프리미엄 |
| `depth.rs` | Depth Walking | 오더북 깊이 분석으로 최적 사이즈 계산 |
| `fee.rs` | Fee Management | 8개 거래소 기본 수수료 + 자산별 출금 수수료 |

---

## crates/executor/ - 거래 실행 (개발 중)

**역할**: CEX/DEX 거래 실행

```
crates/executor/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── cex.rs                 # CexExecutor (중앙화 거래소)
    ├── dex.rs                 # DexExecutor (탈중앙화 거래소)
    └── order.rs               # Order, OrderBuilder
```

---

## crates/alerts/ - 텔레그램 알림

**역할**: 차익거래 기회 및 연결 상태 알림

```
crates/alerts/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── bot.rs                 # TelegramBot (teloxide)
    ├── notifier.rs            # Notifier (중복 제거, 필터링)
    └── database.rs            # SQLite 설정 저장
```

---

## apps/server/ - CLI 헤드리스 서버

**역할**: 가격 피드 수집, 기회 탐지, WebSocket 브로드캐스트

```
apps/server/
├── Cargo.toml
└── src/
    ├── main.rs                # 엔트리포인트, 태스크 스폰
    ├── state.rs               # AppState (Lock-Free)
    ├── config.rs              # AppConfig, DetectorSettings
    ├── ws_server.rs           # Axum WebSocket 서버
    ├── exchange_rate.rs       # USD/KRW 환율 조회
    ├── wallet_status.rs       # 지갑 입출금 상태 조회
    ├── status_notifier.rs     # 연결 상태 텔레그램 알림
    │
    └── feeds/                 # 피드 핸들러
        ├── mod.rs
        ├── handler.rs         # FeedHandler (상태 업데이트)
        └── common.rs          # 공통 유틸리티
```

### main.rs 초기화 흐름

1. `.env` 로드
2. CLI 인자 파싱 (clap)
3. `AppState` 생성
4. WebSocket 서버 시작 (:9001)
5. 백그라운드 태스크 스폰:
   - Event-driven 탐지기
   - Stats 리포터 (10초)
   - 환율 업데이터 (5분)
   - 지갑 상태 업데이터 (5분)
   - 마켓 디스커버리 (5분)
   - 스테일 가격 정리 (10초)
6. WebSocket 피드 시작 (라이브 모드)

### AppState 구조

```rust
pub struct AppState {
    // Lock-Free (DashMap, Atomic)
    prices: PriceAggregator,
    detector: OpportunityDetector,
    orderbook_cache: DashMap<...>,
    stablecoin_prices: DashMap<...>,
    stats: AtomicU64 fields,

    // RwLock (Read-Heavy)
    config: RwLock<AppConfig>,
    common_markets: RwLock<...>,
    fee_manager: RwLock<FeeManager>,

    // 채널
    price_update_tx: mpsc::Sender<...>,
}
```

---

## apps/desktop/ - Tauri 데스크톱 앱

**역할**: React GUI + Rust 백엔드

```
apps/desktop/
├── package.json               # Node.js 의존성
├── tsconfig.json              # TypeScript 설정
├── vite.config.ts             # Vite 빌드 설정
├── tailwind.config.js         # Tailwind CSS 설정
│
├── src/                       # React 프론트엔드
│   ├── main.tsx               # React 엔트리
│   ├── App.tsx                # 루트 컴포넌트 (탭 네비게이션)
│   ├── index.css              # Tailwind 임포트
│   │
│   ├── components/            # UI 컴포넌트
│   │   ├── Header.tsx         # 헤더 (통계, 봇 제어)
│   │   ├── Dashboard.tsx      # 대시보드 (프리미엄 매트릭스)
│   │   ├── Opportunities.tsx  # 기회 테이블
│   │   ├── Markets.tsx        # 마켓 목록
│   │   ├── Wallets.tsx        # 지갑 잔액
│   │   └── Settings.tsx       # 설정 (자격 증명)
│   │
│   └── hooks/
│       └── useTauri.ts        # 모든 React 훅 정의
│
└── src-tauri/                 # Rust 백엔드
    ├── Cargo.toml
    ├── tauri.conf.json        # Tauri 앱 설정
    │
    └── src/
        ├── main.rs            # Tauri 엔트리
        ├── state.rs           # AppState + WebSocket 클라이언트
        ├── commands.rs        # 21개 IPC 명령
        ├── exchange_client.rs # 6개 거래소 API 클라이언트
        ├── credentials.rs     # .env 자격 증명 관리
        └── symbol_mapping.rs  # 심볼 매핑 관리
```

### React 훅 (useTauri.ts)

| 훅 | 역할 | 데이터 소스 |
|-----|------|------------|
| `usePrices()` | 가격 캐시 | DashMap 스타일, 10 FPS |
| `useOpportunities()` | 기회 목록 | 글로벌 캐시, 50개 제한 |
| `useStats()` | 봇 통계 | Tauri 이벤트 |
| `useBotControl()` | 시작/정지 | Tauri 명령 |
| `useConfig()` | 실행 설정 | Tauri 명령 |
| `useExchangeRate()` | 환율 | Tauri 이벤트 |
| `useCommonMarkets()` | 공통 마켓 | 글로벌 캐시 |
| `useCredentials()` | 자격 증명 | .env 파일 |
| `useWalletInfo()` | 지갑 잔액 | On-demand API |
| `useWalletStatus()` | 입출금 상태 | 5분 캐시 |
| `useSymbolMappings()` | 심볼 매핑 | JSON 파일 |
| `usePremiumMatrix()` | 프리미엄 매트릭스 | 서버 계산 |

### Tauri 명령 (commands.rs)

**데이터 조회**:
- `get_prices`, `get_opportunities`, `get_stats`
- `get_config`, `get_exchange_rate`
- `get_common_markets`, `get_wallet_status`

**설정 관리**:
- `update_config`, `set_server_url`

**지갑 조회**:
- `get_wallet_info`, `get_all_wallets`

**심볼 매핑**:
- `get_symbol_mappings`, `upsert_symbol_mapping`
- `remove_symbol_mapping`, `save_symbol_mappings`

**봇 제어** (TODO):
- `start_bot`, `stop_bot`, `execute_opportunity`

---

## docs/ - 프로젝트 문서

```
docs/
├── index.md                   # 마스터 인덱스
├── project-overview.md        # 프로젝트 개요
├── architecture.md            # 시스템 아키텍처
├── api-contracts.md           # WebSocket API 계약
├── development-guide.md       # 개발 가이드
└── source-tree-analysis.md    # 소스 트리 분석 (이 문서)
```

---

## 주요 파일별 LOC 추정

| 파일 | LOC | 설명 |
|------|-----|------|
| `apps/server/src/state.rs` | ~1000 | AppState + 모든 상태 관리 |
| `apps/server/src/ws_server.rs` | ~1100 | WebSocket 서버 + 브로드캐스트 |
| `apps/server/src/wallet_status.rs` | ~1200 | 6개 거래소 지갑 API |
| `apps/desktop/src-tauri/src/exchange_client.rs` | ~1700 | 6개 거래소 API 클라이언트 |
| `crates/feeds/src/adapter/coinbase.rs` | ~500 | Coinbase L2 오더북 |
| `crates/engine/src/premium.rs` | ~1100 | 프리미엄 계산 |
| `crates/core/src/opportunity.rs` | ~400 | ArbitrageOpportunity |

---

## 의존성 관계도

```
                         arbitrage-core
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
       arbitrage-feeds  arbitrage-engine  arbitrage-alerts
              │               │
              └───────┬───────┘
                      │
                      ▼
             arbitrage-executor
                      │
       ┌──────────────┼──────────────┐
       │                             │
       ▼                             ▼
  apps/server              apps/desktop/src-tauri
       │                             │
       └──────────┬──────────────────┘
                  │
                  ▼
           클라이언트 (WebSocket)
```

---

## Git 제외 파일

```
# .gitignore
target/                        # Rust 빌드 아티팩트
node_modules/                  # Node.js 의존성
dist/                          # Vite 빌드 출력
.env                           # 환경 변수 (API 키)
data/                          # 런타임 데이터
*.db                           # SQLite 데이터베이스
```

---

*문서 생성일: 2026-01-11 | 버전: 0.1.0*

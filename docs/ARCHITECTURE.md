# Arbitrage Bot - 시스템 아키텍처

## 개요

Arbitrage Bot은 마이크로서비스 + 이벤트 드리븐 아키텍처를 채택한 고성능 암호화폐 차익거래 시스템입니다. Rust의 비동기 런타임(Tokio)을 활용하여 lock-free 동시성과 저지연 처리를 구현합니다.

---

## 시스템 아키텍처

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              EXCHANGE LAYER                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐│
│  │ Binance  │ │ Coinbase │ │  Bybit   │ │  GateIO  │ │  Upbit   │ │Bithumb ││
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └───┬────┘│
│       │            │            │            │            │           │     │
└───────┼────────────┼────────────┼────────────┼────────────┼───────────┼─────┘
        │ WebSocket  │ WebSocket  │ WebSocket  │ WebSocket  │ WebSocket │
        └────────────┴────────────┴─────┬──────┴────────────┴───────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           FEED LAYER (arbitrage-feeds)                       │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                         WsClient (재연결 + Circuit Breaker)             │ │
│  │  - 지수 백오프 + 지터                                                   │ │
│  │  - 스테일 감지 (2분 타임아웃)                                           │ │
│  │  - 채널 백프레셔 (가득 차면 재연결)                                     │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                        │                                     │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
│  │ BinanceRunner│ │CoinbaseRunner│ │ BybitRunner │ │UpbitRunner  │  ...      │
│  │ (파싱 전용) │ │ (오더북 유지)│ │ (스냅샷/델타)│ │(MessagePack)│           │
│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘            │
│         │               │               │               │                    │
│         └───────────────┴───────┬───────┴───────────────┘                    │
│                                 │                                            │
│                          FeedMessage 채널                                    │
└─────────────────────────────────┼────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SERVER LAYER (apps/server)                            │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        FeedHandler (상태 업데이트)                      │ │
│  │  - 통화 변환 (KRW→USD, 스테이블코인 정규화)                            │ │
│  │  - 오더북 스냅샷/델타 관리                                              │ │
│  │  - PriceUpdateEvent 발행                                                │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                 │                                            │
│                    PriceUpdateEvent 채널                                     │
│                                 │                                            │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                  AppState (Lock-Free 동시 상태)                         │ │
│  │                                                                          │ │
│  │  prices: PriceAggregator (DashMap)        # Lock-free 가격 저장          │ │
│  │  detector: OpportunityDetector (DashMap)  # Lock-free 기회 탐지          │ │
│  │  orderbook_cache: DashMap                 # Lock-free 오더북             │ │
│  │  stablecoin_prices: DashMap               # Lock-free 환율               │ │
│  │  config: RwLock<AppConfig>                # Read-heavy 설정              │ │
│  │  stats: AtomicU64 fields                  # 완전 Lock-free 통계          │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                 │                                            │
│  ┌──────────────────────────────┴───────────────────────────────────────┐   │
│  │                    Event-Driven Detector                              │   │
│  │                                                                        │   │
│  │  1. PriceUpdateEvent 수신                                              │   │
│  │  2. 해당 pair_id만 기회 탐지 (전체 스캔 X)                             │   │
│  │  3. Optimal Size 계산 (Depth Walking)                                  │   │
│  │  4. 프리미엄 매트릭스 계산                                             │   │
│  │  5. WebSocket 클라이언트에 브로드캐스트                                │   │
│  │  6. 텔레그램 알림 전송                                                 │   │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                 │                                            │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │              WebSocket Server (Axum + Tokio Broadcast)                  │ │
│  │  - 포트: 9001                                                           │ │
│  │  - 브로드캐스트 채널 용량: 1000 메시지                                  │ │
│  │  - 초기 동기화: prices, opportunities, stats, common_markets            │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────┼────────────────────────────────────────────┘
                                  │
                    WebSocket (ws://localhost:9001/ws)
                                  │
          ┌───────────────────────┴───────────────────────┐
          │                                               │
          ▼                                               ▼
┌─────────────────────────────────┐    ┌─────────────────────────────────┐
│      DESKTOP LAYER (Tauri)      │    │      ALERT LAYER (Telegram)     │
│                                 │    │                                 │
│  ┌───────────────────────────┐  │    │  ┌───────────────────────────┐  │
│  │     Tauri Backend (Rust)  │  │    │  │    Notifier (SQLite)      │  │
│  │  - WebSocket 클라이언트   │  │    │  │  - 알림 중복 제거         │  │
│  │  - AppState (DashMap)     │  │    │  │  - 사용자별 설정          │  │
│  │  - Exchange API 클라이언트│  │    │  │  - Transfer Path 필터     │  │
│  │  - IPC 명령 핸들러        │  │    │  └───────────────────────────┘  │
│  └───────────────────────────┘  │    │                                 │
│              │                  │    │  ┌───────────────────────────┐  │
│        Tauri IPC + Events       │    │  │  StatusNotifier            │  │
│              │                  │    │  │  - 연결 상태 알림          │  │
│  ┌───────────────────────────┐  │    │  │  - Circuit Breaker 알림   │  │
│  │   React Frontend (TS)     │  │    │  └───────────────────────────┘  │
│  │  - usePrices() 훅         │  │    │                                 │
│  │  - useOpportunities() 훅  │  │    └─────────────────────────────────┘
│  │  - 10 FPS 배칭            │  │
│  │  - 글로벌 캐시            │  │
│  └───────────────────────────┘  │
│                                 │
└─────────────────────────────────┘
```

---

## 크레이트 의존성 그래프

```
                    arbitrage-core
                    (핵심 데이터 타입)
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
   arbitrage-feeds  arbitrage-engine  arbitrage-alerts
   (WebSocket 피드)  (탐지 엔진)      (텔레그램)
          │               │
          └───────┬───────┘
                  │
                  ▼
         arbitrage-executor
         (거래 실행 - 개발 중)
                  │
                  ▼
    ┌─────────────┴─────────────┐
    │                           │
    ▼                           ▼
apps/server              apps/desktop/src-tauri
(CLI 서버)               (Tauri 백엔드)
```

---

## 핵심 크레이트 상세

### 1. arbitrage-core

**역할**: 모든 크레이트가 공유하는 핵심 데이터 타입 정의

| 모듈 | 주요 타입 | 설명 |
|------|----------|------|
| `asset` | `Asset`, `TradingPair` | 자산 및 거래 페어 |
| `chain` | `Chain` | 블록체인 네트워크 (10+ 체인) |
| `exchange` | `Exchange`, `ExchangeType` | 거래소 식별자 (17개) |
| `price` | `FixedPoint`, `PriceTick`, `OrderbookSnapshot` | 가격 데이터 (8자리 고정소수점) |
| `quote_currency` | `QuoteCurrency` | 호가 통화 (USD, USDT, USDC, KRW) |
| `opportunity` | `ArbitrageOpportunity`, `RouteStep` | 차익거래 기회 |
| `execution` | `ExecutionConfig`, `OrderStatus` | 실행 설정 및 상태 |
| `bridge` | `BridgeProtocol`, `BridgeRoute` | 크로스체인 브릿지 (9개) |

**설계 패턴**:
- `#[repr(u8/u16)]` enum: 컴팩트 직렬화
- `FixedPoint`: 부동소수점 정밀도 오류 방지
- Builder 패턴: `ArbitrageOpportunity::new().with_depth().with_optimal_size()`

### 2. arbitrage-feeds

**역할**: 거래소 WebSocket 연결 및 메시지 파싱

| 컴포넌트 | 역할 |
|----------|------|
| `WsClient` | 재연결, Circuit Breaker, 핑/퐁 |
| `*Adapter` | 거래소별 메시지 파싱 |
| `*Runner` | 어댑터 → `ParsedTick` 변환 |
| `PriceAggregator` | 가격 캐시 (DashMap) |

**Runner/Handler 분리**:
- **Runner**: 순수 파싱 로직 (외부 의존성 없음)
- **Handler**: 상태 업데이트 및 브로드캐스트

**오더북 관리**:
| 거래소 | 방식 |
|--------|------|
| Binance | 스냅샷 기반 (20 레벨) |
| Coinbase | 델타 업데이트 (BTreeMap 유지) |
| Bybit/GateIO | 스냅샷 + 델타 구분 |
| Upbit/Bithumb | 스냅샷 전용 |

### 3. arbitrage-engine

**역할**: 차익거래 기회 탐지 및 분석

| 모듈 | 주요 기능 |
|------|----------|
| `detector` | `OpportunityDetector` - DashMap 기반 lock-free 탐지 |
| `premium` | `PremiumMatrix` - 다중 통화 프리미엄 계산 |
| `depth` | `calculate_optimal_size()` - 오더북 깊이 분석 |
| `orderbook` | `OrderbookCache` - BTreeMap 기반 캐시 |
| `fee` | `FeeManager` - 거래소별 수수료 관리 |
| `route` | `RouteFinder` - 최적 경로 탐색 |

**프리미엄 유형**:
```rust
// Raw Premium: 직접 가격 비교
let raw = (sell_bid - buy_ask) / buy_ask * 10000;

// USDlike Premium: 동일 스테이블코인 기준
let usdlike = (sell_bid_usdt - buy_ask_usdt) / buy_ask_usdt * 10000;

// Kimchi Premium: USD 환율 기반
let kimchi = (sell_bid_usd - buy_ask_usd) / buy_ask_usd * 10000;
```

**Depth Walking 알고리즘**:
```
1. 양쪽 오더북 최상위 레벨에서 시작
2. effective_sell > effective_buy 인 동안:
   - 수량 = min(buy_remaining, sell_remaining)
   - 이익 누적
   - 레벨 소진시 다음 레벨로 이동
3. 출금 수수료 차감
4. 평균 매수/매도 가격 및 소비 레벨 반환
```

### 4. apps/server

**역할**: 헤드리스 CLI 서버

| 모듈 | 역할 |
|------|------|
| `main.rs` | 초기화 및 태스크 스폰 |
| `state.rs` | AppState (lock-free 상태 관리) |
| `ws_server.rs` | WebSocket 브로드캐스트 |
| `feeds/` | FeedHandler 구현 |
| `wallet_status.rs` | 입출금 상태 조회 |
| `exchange_rate.rs` | USD/KRW 환율 |

**백그라운드 태스크**:
- Event-driven 기회 탐지기
- Stats 리포터 (10초)
- 환율 업데이터 (5분)
- 지갑 상태 업데이터 (5분)
- 마켓 디스커버리 (5분)
- 스테일 가격 정리 (10초)

### 5. apps/desktop

**역할**: Tauri 데스크톱 앱

#### Rust 백엔드 (`src-tauri/`)
| 모듈 | 역할 |
|------|------|
| `state.rs` | AppState + WebSocket 클라이언트 |
| `commands.rs` | 21개 Tauri 명령 |
| `exchange_client.rs` | 6개 거래소 API |
| `credentials.rs` | .env 자격 증명 관리 |

#### React 프론트엔드 (`src/`)
| 컴포넌트 | 역할 |
|----------|------|
| `Dashboard` | 마켓 개요 + 프리미엄 매트릭스 |
| `Opportunities` | 실시간 기회 테이블 |
| `Markets` | 거래 페어 가용성 |
| `Wallets` | 잔액 + 입출금 상태 |
| `Settings` | 설정 + 자격 증명 |

**Hook 기반 상태 관리**:
```typescript
usePrices()         // DashMap 스타일 가격 캐시
useOpportunities()  // 글로벌 캐시 + 10 FPS 배칭
useStats()          // 봇 통계
useExchangeRate()   // 환율 데이터
useWalletStatus()   // 입출금 상태
usePremiumMatrix()  // 서버 계산 프리미엄
```

---

## 동시성 모델

### Lock-Free 설계

| 구조 | 용도 | 이유 |
|------|------|------|
| `DashMap` | 가격, 오더북, 탐지기 | 고빈도 쓰기, 동시 읽기 |
| `AtomicU64` | 통계, 환율 | 단일 값, 빈번한 갱신 |
| `AtomicBool` | 연결 상태, 실행 플래그 | 불리언 토글 |

### 최소 잠금

| 구조 | 용도 | 이유 |
|------|------|------|
| `RwLock` | 설정, 마켓, 기회 목록 | Read-heavy, 가끔 쓰기 |

### 채널 크기

| 채널 | 용량 | 용도 |
|------|------|------|
| Broadcast | 1000 | 클라이언트 브로드캐스트 |
| PriceUpdate | 1024 | 탐지기 이벤트 |
| Feed | 5000-30000 | 거래소별 메시지 버퍼 |

---

## 복원력 패턴

### Circuit Breaker

```
Closed (정상) → 10회 연속 실패 → Open (차단, 5분)
                                    │
                                    ▼
                               Half-Open (테스트)
                                    │
                    ┌───────────────┴───────────────┐
                    │                               │
                성공 → Closed                    실패 → Open
```

### 재연결 전략

- **지수 백오프**: 기본 1초, 최대 5분
- **지터**: 0-25% 무작위 지연
- **스테일 감지**: 2분 무응답시 재연결
- **핑 타임아웃**: 30초 내 Pong 없으면 재연결

### 캐시 무효화

- **Reconnected 이벤트**: 해당 거래소 모든 캐시 삭제
- **오더북 델타**: 스냅샷 없이 델타 수신시 무시
- **채널 가득 참**: 재연결 (오더북 재동기화)

---

## 성능 특성

### 레이턴시 목표

| 단계 | 목표 |
|------|------|
| WebSocket 수신 → 파싱 | < 1ms |
| 파싱 → 상태 업데이트 | < 1ms |
| 상태 → 기회 탐지 | < 5ms |
| 탐지 → 브로드캐스트 | < 1ms |
| **총 레이턴시** | **< 10ms** |

### 처리량

| 메트릭 | 값 |
|--------|-----|
| 가격 업데이트 | 1000+ /초 |
| 기회 탐지 | 이벤트 드리븐 (가격당 1회) |
| WebSocket 클라이언트 | 100+ 동시 |

### 메모리

| 컴포넌트 | 예상 사용량 |
|----------|-------------|
| 가격 캐시 (6 거래소 × 100 심볼) | ~5 MB |
| 오더북 캐시 (20 레벨 × 600 페어) | ~20 MB |
| 기회 목록 (최대 50개) | < 1 MB |

---

## 보안 고려사항

### API 키 관리

- `.env` 파일에 저장 (Git 제외)
- UI 표시시 마스킹 (`first4...last4`)
- 프로세스 환경 변수로 로드

### 인증 방식

| 거래소 | 방식 |
|--------|------|
| Binance | HMAC-SHA256 |
| Coinbase | ES256 (ECDSA P-256) |
| Upbit | JWT + HMAC-SHA256 |
| Bithumb | JWT + HMAC-SHA256 |
| Bybit | HMAC-SHA256 |
| GateIO | HMAC-SHA512 |

### 네트워크

- WebSocket 서버: localhost 전용 (기본)
- CORS: 모든 오리진 허용 (개발용)
- TLS: 거래소 연결은 `native-tls`

---

## 확장 포인트

### 새 거래소 추가

1. `crates/core/src/exchange.rs`에 `Exchange` enum 추가
2. `crates/feeds/src/adapter/`에 어댑터 구현
3. `crates/feeds/src/runner/`에 러너 구현
4. `apps/server/src/main.rs`에 피드 핸들러 추가
5. `apps/desktop/src-tauri/src/exchange_client.rs`에 API 추가

### 새 전략 추가

1. `crates/engine/`에 전략 모듈 구현
2. `OpportunityDetector`에서 전략 호출
3. `ArbitrageOpportunity`에 전략별 필드 추가

### 새 알림 채널 추가

1. `crates/alerts/`에 채널 구현
2. `apps/server/src/main.rs`에서 초기화

---

*문서 생성일: 2026-01-11 | 버전: 0.1.0*

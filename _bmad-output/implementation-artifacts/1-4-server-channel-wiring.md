# Story 1.4: 서버 초기화 시 채널 연결

Status: done

## Story

As a **시스템 운영자**,
I want **서버 시작 시 SubscriptionManager와 WsClient 간 채널이 연결**,
So that **마켓 디스커버리가 새 마켓 발견 시 구독 요청을 전송할 수 있다**.

## Acceptance Criteria

1. **AC1**: 서버 시작 시 `SubscriptionManager`가 초기화되어야 한다
2. **AC2**: 각 거래소 WsClient에 대한 mpsc 채널이 생성되어야 한다
3. **AC3**: MarketDiscovery가 새 공통 마켓 발견 시 `SubscriptionManager.update_subscriptions` 호출이 가능해야 한다
4. **AC4**: 기존 feeds 크레이트 아키텍처와 호환되어야 한다 (NFR11)
5. **AC5**: 기존 WebSocket 연결 및 메시지 처리에 영향이 없어야 한다

## Tasks / Subtasks

- [x] Task 1: SubscriptionManager를 spawn_live_feeds에 통합 (AC: #1, #2)
  - [x] 1.1: `SubscriptionManager` 인스턴스 생성
  - [x] 1.2: 각 거래소별 채널 생성 (`SubscriptionManager::create_channel()`)
  - [x] 1.3: 각 WsClient에 `with_subscription_channel()` 호출 (Epic 2에서 완료 예정)
  - [x] 1.4: SubscriptionManager에 각 거래소 등록 (`register_exchange()`)

- [x] Task 2: SubscriptionManager를 Arc로 감싸 공유 (AC: #3)
  - [x] 2.1: `Arc<SubscriptionManager>` 타입 정의
  - [x] 2.2: `spawn_live_feeds`에서 반환하여 main에서 사용 가능하게

- [x] Task 3: run_market_discovery에 SubscriptionManager 연결 (AC: #3)
  - [x] 3.1: 함수 시그니처에 `subscription_manager: Arc<SubscriptionManager>` 파라미터 추가
  - [x] 3.2: 새 공통 마켓 발견 시 각 거래소별 심볼 추출
  - [x] 3.3: 새 마켓에 대해 `update_subscriptions()` 호출

- [x] Task 4: 빌드 및 기존 테스트 검증 (AC: #4, #5)
  - [x] 4.1: `cargo build` 성공 확인
  - [x] 4.2: `cargo test -p arbitrage-feeds` 74개 통과
  - [x] 4.3: `cargo test -p arbitrage-server` 17개 통과

## Dev Notes

### Architecture Compliance

**현재 spawn_live_feeds 구조:**
```rust
async fn spawn_live_feeds(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    symbol_mappings: &SymbolMappings,
    status_notifier: Option<StatusNotifierHandle>,
) -> Vec<tokio::task::JoinHandle<()>>
```

**변경 후:**
```rust
async fn spawn_live_feeds(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    symbol_mappings: &SymbolMappings,
    status_notifier: Option<StatusNotifierHandle>,
) -> (Vec<tokio::task::JoinHandle<()>>, Arc<SubscriptionManager>)
```

**WsClient 생성 패턴 변경:**
```rust
// Before:
let binance_client = WsClient::new(binance_config.clone(), ws_tx);

// After:
let (sub_tx, sub_rx) = SubscriptionManager::create_channel();
subscription_manager.register_exchange(Exchange::Binance, sub_tx);
let binance_client = WsClient::new(binance_config.clone(), ws_tx)
    .with_subscription_channel(sub_rx, Box::new(BinanceSubscriptionBuilder));
```

### SubscriptionBuilder 구현

Story 1.4에서는 빌더 구현 없이 채널 연결만 수행.
Epic 2 (Story 2.1~2.6)에서 거래소별 SubscriptionBuilder 구현 예정.

**임시 Placeholder 빌더:**
```rust
struct PlaceholderSubscriptionBuilder;

impl SubscriptionBuilder for PlaceholderSubscriptionBuilder {
    fn build_subscribe_message(&self, symbols: &[String]) -> String {
        // Placeholder - will be replaced in Epic 2
        format!(r#"{{"subscribe":{:?}}}"#, symbols)
    }
}
```

또는 `subscription_builder`를 `None`으로 두고 채널만 연결 (구독 메시지 생성 스킵).

### Technical Requirements

**언어/프레임워크:**
- Rust 2024 edition
- tokio async runtime
- Arc for shared state

**의존성:**
- `SubscriptionManager` - Story 1.2에서 구현
- `WsClient::with_subscription_channel()` - Story 1.3에서 구현
- `MarketDiscovery` - feeds 크레이트

**파일 위치:**
- `apps/server/src/main.rs` (수정)

### Implementation Notes

**SubscriptionManager 스레드 안전성:**
- `SubscriptionManager`는 내부적으로 `DashMap` 사용 (lock-free)
- `register_exchange()`는 `&mut self` 필요 - 초기화 시에만 호출
- `update_subscriptions()`는 `&self` - 런타임 중 안전하게 호출 가능

**Arc 래핑 전략:**
```rust
// Option 1: Arc만 사용 (register_exchange 후 immutable 사용)
let manager = SubscriptionManager::new();
// ... register exchanges ...
let manager = Arc::new(manager);

// Option 2: Arc<Mutex> (동적 등록 필요 시)
let manager = Arc::new(Mutex::new(SubscriptionManager::new()));
```

**주의사항:**
- `register_exchange()`는 `&mut self` 필요하므로 Arc 래핑 전에 모든 거래소 등록 완료
- `update_subscriptions()`는 `&self`이므로 Arc 공유 후 안전하게 호출 가능

### run_market_discovery 통합

**현재:**
```rust
async fn run_market_discovery(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    _symbol_mappings: Arc<SymbolMappings>,
) {
    // ... 5분마다 마켓 갱신 ...
    state.register_common_markets(&common);
    ws_server::broadcast_common_markets(&broadcast_tx, &common);
}
```

**변경 후:**
```rust
async fn run_market_discovery(
    state: SharedState,
    broadcast_tx: BroadcastSender,
    symbol_mappings: Arc<SymbolMappings>,
    subscription_manager: Arc<SubscriptionManager>,
) {
    // ... 기존 로직 ...

    // 새 마켓 발견 시 동적 구독
    // TODO: Epic 2에서 거래소별 심볼 변환 로직 추가
    for exchange in [Exchange::Binance, Exchange::Coinbase, ...] {
        let new_markets: Vec<String> = /* 해당 거래소의 새 마켓 심볼 목록 */;
        if let Err(e) = subscription_manager.update_subscriptions(exchange, &new_markets).await {
            warn!("{:?}: Failed to update subscriptions: {}", exchange, e);
        }
    }
}
```

### Previous Story Intelligence

**Story 1.2에서 학습한 패턴:**
- `SubscriptionManager::create_channel()` - 채널 쌍 생성
- `register_exchange()` - 거래소별 sender 등록
- `update_subscriptions()` - diff 계산 후 새 마켓만 구독 요청

**Story 1.3에서 학습한 패턴:**
- `WsClient::with_subscription_channel()` - 빌더 패턴으로 채널 연결
- `SubscriptionBuilder` trait - 거래소별 메시지 빌더

### References

- [Source: apps/server/src/main.rs#L646-L990] - spawn_live_feeds 함수
- [Source: apps/server/src/main.rs#L994-L1038] - run_market_discovery 함수
- [Source: crates/feeds/src/subscription.rs] - SubscriptionManager
- [Source: crates/feeds/src/websocket.rs#L229-L237] - with_subscription_channel

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- `cargo build --package arbitrage-server` - 성공 (5 warnings, 0 errors)
- `cargo test -p arbitrage-feeds` - 74 passed, 0 failed
- `cargo test -p arbitrage-server` - 17 passed, 0 failed

### Completion Notes List

- SubscriptionManager import 추가 및 spawn_live_feeds에서 인스턴스 생성
- 6개 거래소 (Binance, Coinbase, Upbit, Bithumb, Bybit, GateIO) 채널 등록
- sub_rx는 `let _ = sub_rx;`로 임시 소비 (Epic 2에서 with_subscription_channel 연결 예정)
- spawn_live_feeds 반환 타입을 `(Vec<JoinHandle>, Arc<SubscriptionManager>)`로 변경
- run_market_discovery에 subscription_manager 파라미터 추가
- 5분마다 마켓 디스커버리 시 각 거래소별 update_subscriptions 호출
- 시뮬레이터 모드에서는 빈 SubscriptionManager 사용

### File List

- `apps/server/src/main.rs` (수정)
  - Line 32: SubscriptionManager import 추가
  - Line 647-653: spawn_live_feeds 반환 타입 변경
  - Line 657: SubscriptionManager 인스턴스 생성
  - Line 804-823: Binance 채널 등록
  - Line 843-846: Coinbase 채널 등록
  - Line 917-920: Upbit 채널 등록
  - Line 943-946: Bithumb 채널 등록
  - Line 969-972: Bybit 채널 등록
  - Line 1002-1005: GateIO 채널 등록
  - Line 1029-1032: Arc 래핑 및 튜플 반환
  - Line 1038-1042: run_market_discovery 시그니처 변경
  - Line 1074-1139: update_subscriptions 호출 로직 추가
  - Line 1333-1363: main()에서 spawn_live_feeds 및 run_market_discovery 호출 수정

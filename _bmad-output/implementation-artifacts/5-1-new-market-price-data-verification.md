# Story 5.1: 새 마켓 가격 데이터 수신 확인

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **시스템 운영자**,
I want **새로 구독된 마켓의 가격 데이터가 정상 수신되는지 확인**,
So that **구독이 성공적으로 완료되었음을 검증할 수 있다**.

## Acceptance Criteria

1. **AC1:** Given 새 마켓 구독이 완료되었을 때, When 거래소에서 가격 데이터를 전송하면, Then FeedHandler가 해당 마켓의 가격 데이터를 정상적으로 파싱해야 한다
2. **AC2:** Given 새 마켓의 가격 데이터가 파싱되었을 때, When update 메서드가 호출되면, Then PriceAggregator에 새 마켓 가격이 저장되어야 한다
3. **AC3:** Given 새 마켓 발견부터 구독 완료까지, When 전체 흐름이 실행되면, Then 10초 이내에 완료되어야 한다 (NFR3)
4. **AC4:** Given 새 마켓 가격이 수신되었을 때, When OpportunityDetector가 확인하면, Then 해당 마켓의 PremiumMatrix가 생성되어야 한다
5. **AC5:** Given 구독 성공 시, When 로깅이 실행되면, Then INFO 레벨로 `[INFO] New market subscribed: {symbol} on [{exchanges}]` 형식으로 출력되어야 한다

## Tasks / Subtasks

- [ ] Task 1: FeedHandler 새 마켓 파싱 검증 (AC: #1)
  - [ ] Subtask 1.1: 동적 구독된 마켓의 심볼이 기존 FeedHandler에서 올바르게 파싱되는지 단위 테스트 작성
  - [ ] Subtask 1.2: 거래소별 어댑터(BinanceAdapter, UpbitAdapter 등)가 새 심볼을 처리할 수 있는지 확인
  - [ ] Subtask 1.3: pair_id 매핑이 동적으로 생성되는지 검증 (`symbol_to_pair_id` 함수 활용)

- [ ] Task 2: PriceAggregator 저장 검증 (AC: #2)
  - [ ] Subtask 2.1: `PriceAggregator.update()` 호출 시 새 마켓 가격이 저장되는지 테스트
  - [ ] Subtask 2.2: `get_price(exchange, pair_id)` 로 새 마켓 가격 조회 가능 확인
  - [ ] Subtask 2.3: 동시 다중 거래소 가격 업데이트 시 데이터 정합성 검증

- [ ] Task 3: End-to-End 구독 흐름 타이밍 검증 (AC: #3)
  - [ ] Subtask 3.1: MarketDiscovery → SubscriptionManager → WsClient → FeedHandler 전체 흐름 타이밍 측정 로직 추가
  - [ ] Subtask 3.2: 구독 완료 시간이 10초 이내인지 검증하는 통합 테스트 작성
  - [ ] Subtask 3.3: 타이밍 초과 시 WARN 로그 출력

- [ ] Task 4: OpportunityDetector 통합 (AC: #4)
  - [ ] Subtask 4.1: `OpportunityDetector.register_symbol()` 호출로 새 마켓 등록 확인
  - [ ] Subtask 4.2: `update_price_with_bid_ask()` 호출 시 PremiumMatrix 자동 생성 검증
  - [ ] Subtask 4.3: 새 마켓의 `get_matrix(pair_id)` 조회 가능 확인

- [ ] Task 5: 구독 성공 INFO 로깅 (AC: #5)
  - [ ] Subtask 5.1: `tracing::info!` 매크로를 사용한 구독 성공 로깅 구현
  - [ ] Subtask 5.2: 로그 포맷이 `[INFO] New market subscribed: {symbol} on [{exchanges}]` 형식 준수 확인
  - [ ] Subtask 5.3: 기존 Epic 4 로깅 패턴과 일관성 유지

## Dev Notes

### 기존 아키텍처 패턴

**Runner/Handler 분리 패턴:**
- `Runner`: WebSocket 메시지를 순수하게 파싱 (상태 없음)
- `Handler`: 파싱된 데이터를 상태에 업데이트 (PriceAggregator, OpportunityDetector)
- 위치: `crates/feeds/src/runner/*.rs`, `crates/feeds/src/feed.rs`

**DashMap 기반 Lock-free 상태 관리:**
- `PriceAggregator.prices`: `DashMap<(exchange_id, pair_id), PriceTick>`
- `OpportunityDetector.matrices`: `DashMap<u32, PremiumMatrix>`
- `OpportunityDetector.symbol_registry`: `DashMap<u32, String>`

**채널 기반 통신:**
- `FeedMessage` 패턴으로 WebSocket → 처리 핸들러 통신
- `SubscriptionChange` 패턴으로 SubscriptionManager → WsClient 통신
- `PriceUpdateEvent` 패턴으로 가격 업데이트 이벤트 브로드캐스트

### 핵심 파일 및 역할

| 파일 | 역할 |
|------|------|
| `crates/feeds/src/aggregator.rs` | PriceAggregator - 가격 저장/조회 |
| `crates/feeds/src/subscription.rs` | SubscriptionManager - 구독 상태 관리 |
| `crates/feeds/src/feed.rs` | FeedHandler 트레이트 및 PriceFeed 구현 |
| `crates/feeds/src/websocket.rs` | WsClient - WebSocket 연결 및 메시지 처리 |
| `crates/engine/src/detector.rs` | OpportunityDetector - 차익거래 기회 탐지 |
| `apps/server/src/state.rs` | SharedState - 서버 전역 상태 |
| `apps/server/src/main.rs` | 서버 초기화 및 채널 연결 |

### 주요 함수/메서드

**PriceAggregator:**
```rust
pub fn update(&self, tick: PriceTick) // 가격 저장
pub fn get_price(&self, exchange: Exchange, pair_id: u32) -> Option<PriceTick>
pub fn get_all_prices_for_pair(&self, pair_id: u32) -> Vec<PriceTick>
```

**OpportunityDetector:**
```rust
pub fn register_symbol(&self, symbol: &str) -> u32 // pair_id 반환
pub fn get_or_register_pair_id(&self, symbol: &str) -> u32
pub fn update_price_with_bid_ask(...) // 가격 업데이트 + Matrix 자동 생성
pub fn get_matrix(&self, pair_id: u32) -> Option<Ref<'_, u32, PremiumMatrix>>
```

**SubscriptionManager:**
```rust
pub async fn update_subscriptions(&self, exchange: Exchange, new_markets: &[String]) -> Result<usize, SubscriptionError>
```

### Project Structure Notes

**수정 필요 파일:**
- `apps/server/src/main.rs`: 가격 업데이트 핸들러에서 새 마켓 등록 로직 확인
- `apps/server/src/feeds/*.rs`: 각 거래소 피드 핸들러에서 OpportunityDetector 연동 검증

**테스트 위치:**
- `crates/feeds/src/aggregator.rs` (기존 단위 테스트 확장)
- `crates/engine/src/detector.rs` (기존 단위 테스트 확장)
- `apps/server/tests/` (통합 테스트 신규 작성)

### 이전 Epic 학습 사항

**Epic 1-4 완료된 구현:**
- SubscriptionChange enum 정의 (Story 1.1)
- SubscriptionManager 구조체 (Story 1.2)
- WsClient subscription_rx 채널 통합 (Story 1.3)
- 서버 초기화 시 채널 연결 (Story 1.4)
- 6개 거래소 동적 구독 빌더 (Epic 2)
- 지수 백오프 재시도 로직 (Story 3.1)
- Rate-limit 준수 로직 (Story 3.4, 3.5)
- 구독 로깅 패턴 (Epic 4)

**활용할 기존 패턴:**
- `tracing::info!` 매크로 사용
- `DashMap` lock-free 접근
- `mpsc` 채널 통신

### Git Intelligence

**최근 커밋 분석:**
- `feat(wts): scaffold stores and tests` - WTS 관련, 현재 스토리와 무관
- `feat(server): integrate dynamic subscription system with WebSocket feeds` - **핵심 참조**
- `feat(feeds): export subscription management types from lib.rs` - 구독 타입 export
- `feat(feeds): add runtime subscription support to WsClient` - WsClient 구독 지원
- `feat(feeds): add error handling and resilience for subscriptions (Epic 3)` - 에러 처리 패턴

**참조할 구현 패턴:**
- 서버의 동적 구독 통합은 `17e9c8c` 커밋 참조
- subscription.rs 모듈 구조는 `fcf0233` 커밋 참조

### References

- [Source: crates/feeds/src/aggregator.rs#PriceAggregator]
- [Source: crates/feeds/src/subscription.rs#SubscriptionManager]
- [Source: crates/engine/src/detector.rs#OpportunityDetector]
- [Source: crates/feeds/src/feed.rs#FeedHandler trait]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 5]
- [Source: docs/architecture.md#feeds 크레이트 구조]

### Technical Requirements

**Rust 버전:** 1.75+
**관련 크레이트:**
- `arbitrage-feeds`: WebSocket, 구독 관리, 어댑터
- `arbitrage-core`: Exchange enum, PriceTick, FixedPoint
- `arbitrage-engine`: OpportunityDetector, PremiumMatrix
- `dashmap`: Lock-free concurrent HashMap
- `tracing`: 로깅 인프라

**테스트 요구사항:**
- 단위 테스트: `cargo test -p arbitrage-feeds`
- 통합 테스트: `cargo test -p arbitrage-server --test integration`
- NFR3 타이밍 검증: 10초 이내 구독 완료

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List


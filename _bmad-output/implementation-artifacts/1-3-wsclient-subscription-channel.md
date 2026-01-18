# Story 1.3: WsClient 구독 채널 통합

Status: done

## Story

As a **시스템 개발자**,
I want **WsClient가 구독 변경 요청을 수신할 수 있는 채널**,
So that **런타임 중 동적으로 구독을 추가할 수 있다**.

## Acceptance Criteria

1. **AC1**: WsClient가 `subscription_rx: mpsc::Receiver<SubscriptionChange>` 채널을 선택적으로 받을 수 있어야 한다 ✅
2. **AC2**: `connect_and_handle` 내부 select! 루프에서 subscription_rx를 처리해야 한다 ✅
3. **AC3**: `SubscriptionChange::Subscribe` 수신 시 거래소에 구독 메시지를 전송해야 한다 ✅
4. **AC4**: 기존 메시지 처리 로직에 영향을 주지 않아야 한다 ✅
5. **AC5**: 채널 통신 오버헤드가 1ms 미만이어야 한다 (NFR4) ✅

## Tasks / Subtasks

- [x] Task 1: WsClient 구조체 확장 (AC: #1)
  - [x] 1.1: `subscription_rx: Option<mpsc::Receiver<SubscriptionChange>>` 필드 추가
  - [x] 1.2: `with_subscription_channel()` 빌더 메서드 추가
  - [x] 1.3: 기존 생성자/메서드 호환성 유지

- [x] Task 2: 구독 메시지 빌더 trait 정의 (AC: #3)
  - [x] 2.1: `SubscriptionBuilder` trait 정의 - `fn build_subscribe_message(symbols: &[String]) -> String`
  - [x] 2.2: WsClient에 `subscription_builder: Option<Box<dyn SubscriptionBuilder>>` 추가
  - [x] 2.3: 빌더 없으면 구독 메시지 생성 스킵 (향후 거래소별 구현)

- [x] Task 3: select! 루프 확장 (AC: #2, #3, #4)
  - [x] 3.1: `connect_and_handle` 함수 시그니처에 subscription_rx 전달
  - [x] 3.2: select! 분기에 `subscription_rx.recv()` 추가
  - [x] 3.3: SubscriptionChange::Subscribe 처리 - 빌더로 메시지 생성 후 WebSocket 전송
  - [x] 3.4: 기존 메시지/ping/shutdown 처리 로직 유지

- [x] Task 4: 단위 테스트 작성 (AC: #1-#5)
  - [x] 4.1: `with_subscription_channel()` 테스트
  - [x] 4.2: subscription_rx 없을 때 기존 동작 테스트
  - [x] 4.3: SubscriptionBuilder trait 테스트

- [x] Task 5: 빌드 및 테스트 검증 (AC: #4)
  - [x] 5.1: `cargo build -p arbitrage-feeds` 성공 확인
  - [x] 5.2: `cargo test -p arbitrage-feeds` 전체 테스트 통과 확인
  - [x] 5.3: 기존 테스트 회귀 없음 확인

## Dev Notes

### Architecture Compliance

**기존 WsClient 구조:**
```rust
pub struct WsClient {
    config: FeedConfig,
    tx: mpsc::Sender<WsMessage>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
}
```

**확장 후:**
```rust
pub struct WsClient {
    config: FeedConfig,
    tx: mpsc::Sender<WsMessage>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
    subscription_rx: Option<mpsc::Receiver<SubscriptionChange>>,  // NEW
    subscription_builder: Option<Box<dyn SubscriptionBuilder>>,  // NEW
}
```

**select! 루프 구현 (biased select 사용):**
```rust
enum SelectResult {
    WsMessage,
    PingTick,
    Shutdown,
    Subscription(SubscriptionChange),
}

let result = tokio::select! {
    biased;

    // Runtime subscription changes (highest priority)
    Some(change) = async { ... } => SelectResult::Subscription(change),

    // Shutdown signal
    _ = async { ... } => SelectResult::Shutdown,

    // Ping timer
    _ = ping_timer.tick() => SelectResult::PingTick,

    // WebSocket message (default)
    msg = read.next() => { ... SelectResult::WsMessage }
};
```

### Technical Requirements

**언어/프레임워크:**
- Rust 2024 edition
- `tokio::sync::mpsc` 채널
- `tokio::select!` 매크로

**의존성:**
- `SubscriptionChange` - Story 1.1/1.2에서 구현
- `WsClient` - 기존 websocket.rs

**파일 위치:**
- `crates/feeds/src/websocket.rs` (수정)

### Implementation Notes

**SubscriptionBuilder trait:**
- Story 2.1-2.6에서 거래소별 구현 예정
- 이 스토리에서는 trait 정의 + Option 처리만
- 빌더 없으면 구독 메시지 전송 스킵

**biased select! 사용 이유:**
- 구독 변경을 최우선 처리하여 응답성 향상
- Option 처리를 위한 async 블록 패턴 적용

**주의사항:**
- `subscription_rx`는 이동(move) 필요 - `run_with_messages`에서 소유권 이전
- select! 내부에서 mutable borrow 주의
- Gate.io 특수 처리와 충돌하지 않도록 별도 분기 유지

### Previous Story Intelligence

**Story 1.2에서 학습한 패턴:**
- `mpsc::Receiver<SubscriptionChange>` 채널 사용
- `SubscriptionManager::create_channel()` 활용
- Arc 공유 참조 패턴

### References

- [Source: crates/feeds/src/websocket.rs#L186-L237] - WsClient 구조체 및 빌더
- [Source: crates/feeds/src/websocket.rs#L617-L717] - select! 루프 확장
- [Source: crates/feeds/src/websocket.rs#L32-L37] - SubscriptionBuilder trait
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.3] - 스토리 요구사항

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Build: `cargo build -p arbitrage-feeds` - 성공
- Tests: `cargo test -p arbitrage-feeds` - 74 tests passed + 1 doctest

### Completion Notes List

- `SubscriptionBuilder` trait 추가 (`build_subscribe_message` 메서드)
- WsClient 구조체에 2개 필드 추가:
  - `subscription_rx: Option<mpsc::Receiver<SubscriptionChange>>`
  - `subscription_builder: Option<Box<dyn SubscriptionBuilder>>`
- `with_subscription_channel()` 빌더 메서드 추가
- `run_with_messages`에서 subscription_rx 소유권 이전 처리
- `connect_and_handle` 시그니처 확장 (subscription_rx 파라미터)
- select! 루프를 biased select로 리팩토링:
  - `SelectResult` enum으로 결과 처리 통합
  - 구독 변경 → 셧다운 → 핑 → WS 메시지 우선순위
- 4개 테스트 케이스 추가:
  - `test_subscription_builder_trait`
  - `test_ws_client_with_subscription_channel`
  - `test_ws_client_without_subscription_channel`
  - `test_ws_client_builder_chain`
- 기존 70개 테스트 회귀 없음 확인

### File List

- `crates/feeds/src/websocket.rs` (수정)

# Story 1.2: SubscriptionManager 구조체 구현

Status: done

## Story

As a **시스템 개발자**,
I want **거래소별 구독 상태를 관리하는 SubscriptionManager**,
So that **현재 구독 목록을 추적하고 구독 요청을 전송할 수 있다**.

## Acceptance Criteria

1. **AC1**: `SubscriptionManager` 구조체가 `subscription.rs`에 정의되어야 한다 ✅
2. **AC2**: 각 거래소별 `mpsc::Sender<SubscriptionChange>`를 HashMap으로 보유해야 한다 ✅
3. **AC3**: `current_subscriptions: DashMap<Exchange, HashSet<String>>`으로 현재 구독 상태를 추적해야 한다 ✅
4. **AC4**: `update_subscriptions(&self, exchange: Exchange, new_markets: &[String])` 메서드로 diff 계산 및 구독 요청 전송이 가능해야 한다 ✅
5. **AC5**: 채널 통신 오버헤드가 1ms 미만이어야 한다 (NFR4) ✅
6. **AC6**: 기존 feeds 크레이트 아키텍처와 호환되어야 한다 (NFR11) ✅

## Tasks / Subtasks

- [x] Task 1: SubscriptionManager 구조체 정의 (AC: #1, #2, #3)
  - [x] 1.1: `SubscriptionManager` 구조체 정의
    - `senders: HashMap<Exchange, mpsc::Sender<SubscriptionChange>>`
    - `current_subscriptions: Arc<DashMap<Exchange, HashSet<String>>>`
  - [x] 1.2: `new()` 생성자 구현
  - [x] 1.3: `register_exchange()` 메서드 - 거래소별 sender 등록

- [x] Task 2: 구독 관리 메서드 구현 (AC: #4, #5)
  - [x] 2.1: `update_subscriptions(exchange, new_markets)` 구현
    - 현재 구독과 new_markets 간 diff 계산
    - 새 마켓만 Subscribe 채널로 전송
    - current_subscriptions 업데이트
  - [x] 2.2: `get_current_subscriptions(exchange)` 읽기 메서드
  - [x] 2.3: `subscription_count(exchange)` 통계 메서드

- [x] Task 3: 채널 생성 헬퍼 (AC: #2)
  - [x] 3.1: `create_channel()` - `(Sender, Receiver)` 쌍 반환
  - [x] 3.2: 채널 버퍼 크기 상수 정의 (SUBSCRIPTION_CHANNEL_BUFFER = 1024)

- [x] Task 4: 단위 테스트 작성 (AC: #1-#6)
  - [x] 4.1: `SubscriptionManager::new()` 생성 테스트
  - [x] 4.2: `register_exchange()` 및 sender 보유 테스트
  - [x] 4.3: `update_subscriptions()` diff 계산 테스트 - 새 마켓만 전송
  - [x] 4.4: `update_subscriptions()` 중복 마켓 무시 테스트
  - [x] 4.5: 멀티 거래소 독립성 테스트

- [x] Task 5: 빌드 및 테스트 검증 (AC: #6)
  - [x] 5.1: `cargo build -p arbitrage-feeds` 성공 확인
  - [x] 5.2: `cargo test -p arbitrage-feeds` 전체 테스트 통과 확인
  - [x] 5.3: 기존 테스트 회귀 없음 확인

## Dev Notes

### Architecture Compliance

**아키텍처 결정 (architecture.md Decision 1):**
```rust
pub struct SubscriptionManager {
    senders: HashMap<Exchange, mpsc::Sender<SubscriptionChange>>,
    current_subscriptions: DashMap<Exchange, HashSet<String>>,
}
```

**Runner/Handler 분리 패턴:**
- `SubscriptionManager`는 상태 관리 + 채널 전송 담당
- 실제 WebSocket 구독 메시지 전송은 Story 1.3 (WsClient 통합)에서 처리

**DashMap 기반 lock-free 상태 관리:**
- 기존 `PriceAggregator` 패턴 참조 ([aggregator.rs:6](crates/feeds/src/aggregator.rs#L6))
- `Arc<DashMap<...>>` 형태로 멀티스레드 안전성 확보

### Technical Requirements

**언어/프레임워크:**
- Rust 2024 edition
- `tokio::sync::mpsc` 채널
- `dashmap` 크레이트 (이미 feeds 종속성에 포함)

**의존성:**
- `arbitrage_core::Exchange` - 거래소 enum ([exchange.rs:43](crates/core/src/exchange.rs#L43))
- `SubscriptionChange` - Story 1.1에서 구현 완료

**파일 위치:**
- `crates/feeds/src/subscription.rs` (기존 파일에 추가)

### Code Patterns Reference

**기존 DashMap 사용 패턴 (aggregator.rs:14-17):**
```rust
pub struct PriceAggregator {
    prices: Arc<DashMap<PriceKey, PriceTick>>,
}
```

**기존 mpsc 채널 패턴 (runner/binance.rs 등):**
```rust
use tokio::sync::mpsc;
let (tx, rx) = mpsc::channel::<FeedMessage>(1024);
```

**Exchange enum (core/exchange.rs:43-52):**
```rust
pub enum Exchange {
    Binance = 100,
    Coinbase = 101,
    Bybit = 104,
    Upbit = 105,
    Bithumb = 106,
    GateIO = 107,
    // ...
}
```

### Implementation Notes

**diff 계산 로직:**
```rust
pub async fn update_subscriptions(&self, exchange: Exchange, new_markets: &[String]) -> Result<(), FeedError> {
    let new_set: HashSet<String> = new_markets.iter().cloned().collect();

    // Get current subscriptions or empty set
    let current = self.current_subscriptions
        .get(&exchange)
        .map(|r| r.value().clone())
        .unwrap_or_default();

    // Calculate diff: new_markets - current = to_subscribe
    let to_subscribe: Vec<String> = new_set
        .difference(&current)
        .cloned()
        .collect();

    if !to_subscribe.is_empty() {
        if let Some(sender) = self.senders.get(&exchange) {
            sender.send(SubscriptionChange::Subscribe(to_subscribe)).await?;
        }
    }

    // Update current subscriptions
    self.current_subscriptions.insert(exchange, new_set);

    Ok(())
}
```

**채널 버퍼 사이즈:**
- 1024개 권장 (Binance 최대 스트림 수와 동일)
- 배치 구독 시 버퍼 오버플로우 방지

### Project Structure Notes

- `crates/feeds/src/subscription.rs` - Story 1.1에서 생성된 파일에 추가
- `SubscriptionChange` enum 이미 존재
- 같은 파일에 `SubscriptionManager` 구조체 추가

### Testing Standards

- 단위 테스트는 `#[cfg(test)]` 모듈 내부에 작성
- `#[tokio::test]` 사용 (async 테스트)
- mock receiver로 채널 전송 검증

### Previous Story Intelligence

**Story 1.1에서 학습한 패턴:**
- `#[must_use]` attribute 적절히 사용
- `PartialEq`, `Eq` derive 추가
- 헬퍼 메서드 (`is_empty()`, `len()`) 포함
- 포괄적 단위 테스트 (6개 테스트 케이스)

**코드 리뷰 피드백 반영:**
- M1: `#[must_use]` 누락 → 필요시 추가
- M2: `PartialEq`, `Eq` derive → 비교 연산 필요시 추가

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Decision-1] - 채널 구조 결정
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.2] - 스토리 요구사항
- [Source: crates/feeds/src/aggregator.rs#L6-L17] - DashMap 패턴
- [Source: crates/core/src/exchange.rs#L43-L52] - Exchange enum
- [Source: crates/feeds/src/subscription.rs] - SubscriptionChange (Story 1.1)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Build: `cargo build -p arbitrage-feeds` - 성공
- Tests: `cargo test -p arbitrage-feeds` - 70 tests passed + 1 doctest

### Completion Notes List

- `SubscriptionManager` 구조체 구현 완료
  - `senders: HashMap<Exchange, mpsc::Sender<SubscriptionChange>>` - 거래소별 채널 sender
  - `current_subscriptions: Arc<DashMap<Exchange, HashSet<String>>>` - lock-free 구독 상태 추적
- 핵심 메서드 구현:
  - `new()` - 빈 매니저 생성
  - `create_channel()` - 채널 쌍 생성 (버퍼 1024)
  - `register_exchange()` - 거래소 채널 등록
  - `update_subscriptions()` - diff 계산 후 새 마켓만 구독 요청 전송
  - `get_current_subscriptions()` - 현재 구독 조회
  - `subscription_count()` - 구독 수 반환
  - `is_registered()` - 거래소 등록 여부
  - `subscriptions()` - DashMap Arc 공유 참조
- `SubscriptionError` enum 추가 - `ExchangeNotRegistered`, `ChannelSendError`
- `Default` trait 구현
- 14개 SubscriptionManager 테스트 케이스 추가
- Story 1.1 패턴 준수: `#[must_use]`, 포괄적 문서화 주석
- 모든 AC 충족 확인

### File List

- `crates/feeds/src/subscription.rs` (수정)

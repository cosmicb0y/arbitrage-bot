# Story 2.1: Binance 동적 구독 메시지 빌더

Status: done

## Story

As a **시스템 개발자**,
I want **Binance WebSocket에 런타임 구독 메시지를 생성하는 빌더**,
So that **새 마켓을 Binance에 동적으로 구독할 수 있다**.

## Acceptance Criteria

1. **AC1**: `BinanceSubscriptionBuilder`가 `SubscriptionBuilder` trait을 구현해야 한다
2. **AC2**: `build_subscribe_message`가 `{"method":"SUBSCRIBE","params":["btcusdt@depth20@100ms"],"id":N}` 형식을 생성해야 한다
3. **AC3**: main.rs에서 WsClient에 `with_subscription_channel(sub_rx, Box::new(BinanceSubscriptionBuilder))` 연결
4. **AC4**: 기존 Binance WebSocket 동작에 영향이 없어야 한다
5. **AC5**: rate-limit 5 msg/sec 준수 (현재 단일 메시지 전송으로 충족)

## Tasks / Subtasks

- [x] Task 1: BinanceSubscriptionBuilder 구현 (AC: #1, #2)
  - [x] 1.1: `subscription.rs`에 `BinanceSubscriptionBuilder` struct 정의
  - [x] 1.2: `SubscriptionBuilder` trait 구현
  - [x] 1.3: 기존 `BinanceAdapter::subscribe_messages` 로직 활용
  - [x] 1.4: lib.rs에서 export

- [x] Task 2: main.rs에서 WsClient 연결 (AC: #3)
  - [x] 2.1: `BinanceSubscriptionBuilder` import
  - [x] 2.2: `with_subscription_channel(sub_rx, Box::new(BinanceSubscriptionBuilder::new()))` 호출
  - [x] 2.3: `let _ = sub_rx;` 제거

- [x] Task 3: 빌드 및 테스트 검증 (AC: #4, #5)
  - [x] 3.1: `cargo build` 성공 확인
  - [x] 3.2: `cargo test -p arbitrage-feeds` 통과 확인 (83 tests)
  - [x] 3.3: 단위 테스트 추가 (9개 테스트 케이스)

## Dev Notes

### Architecture Compliance

**SubscriptionBuilder trait (Story 1.3에서 정의됨):**
```rust
pub trait SubscriptionBuilder: Send + Sync {
    fn build_subscribe_message(&self, symbols: &[String]) -> String;
}
```

**기존 BinanceAdapter 구독 메시지 생성:**
```rust
// crates/feeds/src/adapter/binance.rs
impl BinanceAdapter {
    fn subscribe_messages(symbols: &[String]) -> Vec<String> {
        // depth20@100ms 스트림으로 구독
        for chunk in symbols.chunks(50) {
            let depth_streams: Vec<String> = chunk
                .iter()
                .map(|s| format!("\"{}@depth20@100ms\"", s.to_lowercase()))
                .collect();
            messages.push(format!(
                r#"{{"method": "SUBSCRIBE", "params": [{}], "id": {}}}"#,
                depth_streams.join(", "),
                id
            ));
        }
    }
}
```

**구현 방향:**
- `BinanceSubscriptionBuilder::new()` - stateless struct
- `build_subscribe_message`에서 `BinanceAdapter::subscribe_messages` 첫 번째 메시지 반환
- 또는 단일 메시지로 간소화 (50개 이하 심볼)

### Binance WebSocket API

**구독 메시지 형식:**
```json
{
  "method": "SUBSCRIBE",
  "params": ["btcusdt@depth20@100ms", "ethusdt@depth20@100ms"],
  "id": 1
}
```

**스트림 종류:**
- `@depth20@100ms` - 오더북 depth (현재 사용 중)
- `@ticker` - 가격 ticker
- `@trade` - 체결 정보

**Rate Limit:**
- 5 msg/sec per connection
- 런타임 구독은 마켓 디스커버리 5분 주기이므로 충분

### Implementation Notes

**BinanceSubscriptionBuilder 구현:**
```rust
pub struct BinanceSubscriptionBuilder {
    id_counter: std::sync::atomic::AtomicU32,
}

impl BinanceSubscriptionBuilder {
    pub fn new() -> Self {
        Self {
            id_counter: std::sync::atomic::AtomicU32::new(1),
        }
    }
}

impl SubscriptionBuilder for BinanceSubscriptionBuilder {
    fn build_subscribe_message(&self, symbols: &[String]) -> String {
        let id = self.id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("\"{}@depth20@100ms\"", s.to_lowercase()))
            .collect();
        format!(
            r#"{{"method":"SUBSCRIBE","params":[{}],"id":{}}}"#,
            streams.join(","),
            id
        )
    }
}
```

### Previous Story Intelligence

**Story 1.3에서 학습한 패턴:**
- `SubscriptionBuilder` trait - `Send + Sync` bounds
- `WsClient::with_subscription_channel(rx, builder)` - 빌더 패턴

**Story 1.4에서 준비된 연결점:**
```rust
// apps/server/src/main.rs
let binance_client = WsClient::new(binance_config.clone(), ws_tx);
// TODO: Add .with_subscription_channel(sub_rx, Box::new(BinanceSubscriptionBuilder)) in Epic 2
let _ = sub_rx; // 이 라인 제거 예정
```

### References

- [Source: crates/feeds/src/adapter/binance.rs#L60-75] - subscribe_messages 함수
- [Source: crates/feeds/src/websocket.rs#L20-40] - SubscriptionBuilder trait
- [Source: apps/server/src/main.rs#L814-823] - Binance WsClient 설정

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- `cargo build -p arbitrage-feeds` - 성공
- `cargo test -p arbitrage-feeds` - 83 tests passed + 2 doctests
- `cargo build -p arbitrage-server` - 성공 (5 warnings, 0 errors)
- `cargo test -p arbitrage-server` - 17 tests passed

### Completion Notes List

- `BinanceSubscriptionBuilder` struct 추가 (AtomicU32 id_counter 사용)
- `SubscriptionBuilder` trait 구현 - `build_subscribe_message` 메서드
- 메시지 형식: `{"method":"SUBSCRIBE","params":["symbol@depth20@100ms",...],"id":N}`
- 소문자 변환 및 증가하는 ID 지원
- 9개 단위 테스트 추가 (Send+Sync 검증 포함)
- lib.rs에서 `BinanceSubscriptionBuilder` export 추가
- main.rs에서 Binance WsClient에 `with_subscription_channel` 연결 완료
- `let _ = sub_rx;` 임시 코드 제거

### File List

- `crates/feeds/src/subscription.rs` (수정)
  - Line 29: import 추가 (SubscriptionBuilder, AtomicU32, Ordering)
  - Line 293-340: BinanceSubscriptionBuilder 구현
  - Line 670-761: 9개 테스트 케이스 추가
- `crates/feeds/src/lib.rs` (수정)
  - Line 38-41: BinanceSubscriptionBuilder export 추가
- `apps/server/src/main.rs` (수정)
  - Line 32: BinanceSubscriptionBuilder import 추가
  - Line 819-821: with_subscription_channel 연결

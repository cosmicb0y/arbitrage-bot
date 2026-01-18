# Story 1.1: SubscriptionChange 데이터 구조 정의

Status: done

## Story

As a **시스템 개발자**,
I want **구독 변경 요청을 표현하는 데이터 구조**,
So that **마켓 구독 추가/제거를 타입 안전하게 전달할 수 있다**.

## Acceptance Criteria

1. **AC1**: feeds 크레이트에 `subscription.rs` 모듈이 생성되어야 한다
2. **AC2**: `SubscriptionChange` enum이 `Subscribe(Vec<String>)`와 `Unsubscribe(Vec<String>)` 변형을 가져야 한다
3. **AC3**: `lib.rs`에서 `subscription` 모듈이 export 되어야 한다
4. **AC4**: 기존 feeds 크레이트 아키텍처와 호환되어야 한다 (NFR11)

## Tasks / Subtasks

- [x] Task 1: SubscriptionChange enum 정의 (AC: #1, #2)
  - [x] 1.1: `crates/feeds/src/subscription.rs` 파일 생성
  - [x] 1.2: `SubscriptionChange` enum 정의 - `Subscribe(Vec<String>)`, `Unsubscribe(Vec<String>)`
  - [x] 1.3: Debug, Clone derive 추가
  - [x] 1.4: 모듈 문서화 주석 추가

- [x] Task 2: 모듈 export 설정 (AC: #3)
  - [x] 2.1: `crates/feeds/src/lib.rs`에 `pub mod subscription;` 추가
  - [x] 2.2: `pub use subscription::*;` 추가하여 re-export

- [x] Task 3: 단위 테스트 작성 (AC: #2, #4)
  - [x] 3.1: `SubscriptionChange::Subscribe` 생성 및 pattern match 테스트
  - [x] 3.2: `SubscriptionChange::Unsubscribe` 생성 및 pattern match 테스트
  - [x] 3.3: Clone trait 동작 테스트
  - [x] 3.4: 빈 벡터 케이스 테스트

- [x] Task 4: 빌드 및 테스트 검증 (AC: #4)
  - [x] 4.1: `cargo build -p arbitrage-feeds` 성공 확인
  - [x] 4.2: `cargo test -p arbitrage-feeds` 전체 테스트 통과 확인
  - [x] 4.3: 기존 테스트 회귀 없음 확인

## Dev Notes

### Architecture Compliance

**Runner/Handler 분리 패턴 준수:**
- `SubscriptionChange`는 순수 데이터 타입 (상태 없음)
- 실제 구독 로직은 Story 1.2 (SubscriptionManager)와 Story 1.3 (WsClient 통합)에서 구현

**채널 기반 통신:**
- `SubscriptionChange`는 `mpsc::Sender<SubscriptionChange>`를 통해 전달될 예정
- Story 1.2에서 DashMap 기반 상태 관리와 결합

### Technical Requirements

**언어/프레임워크:**
- Rust 2024 edition
- No external dependencies (core type only)

**파일 위치:**
- `crates/feeds/src/subscription.rs` (신규)
- `crates/feeds/src/lib.rs` (수정)

### Code Patterns Reference

**기존 message.rs 패턴 참조:**
```rust
// 유사한 enum 패턴: FeedMessage, ConnectionEvent
#[derive(Debug, Clone)]
pub enum SubscriptionChange {
    Subscribe(Vec<String>),
    Unsubscribe(Vec<String>),
}
```

**lib.rs export 패턴 참조:**
```rust
// 기존 패턴 (line 22-23 참조)
pub mod subscription;
pub use subscription::*;
```

### Project Structure Notes

- `crates/feeds/` 크레이트는 WebSocket 연결 및 메시지 파싱 담당
- 모듈 구조: `adapter/`, `runner/`, `message.rs`, `websocket.rs` 등
- 새 `subscription.rs`는 동일 레벨에 추가

### Testing Standards

- 단위 테스트는 `#[cfg(test)]` 모듈 내부에 작성
- 기존 `message.rs` 테스트 패턴 참조 (line 197-273)
- 모든 enum variant에 대한 테스트 필수

### References

- [Source: docs/architecture.md#arbitrage-feeds] - feeds 크레이트 역할 정의
- [Source: crates/feeds/src/message.rs#L12-L17] - FeedMessage enum 패턴
- [Source: crates/feeds/src/lib.rs#L12-L38] - 모듈 export 패턴
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.1] - 스토리 요구사항

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Build: `cargo build -p arbitrage-feeds` - 성공
- Tests: `cargo test -p arbitrage-feeds` - 55 passed + 5 subscription tests + 1 doc test = 61 tests 통과

### Completion Notes List

- `SubscriptionChange` enum 구현 완료 - `Subscribe(Vec<String>)`, `Unsubscribe(Vec<String>)` 변형
- 헬퍼 메서드 추가: `is_subscribe()`, `is_unsubscribe()`, `symbols()`, `is_empty()`, `len()`
- 포괄적인 단위 테스트 5개 작성 (subscribe, unsubscribe, clone, empty_vector, debug)
- 모듈 문서화 주석 및 doctest 예제 포함
- 기존 `message.rs` 패턴과 일관된 코드 스타일 적용
- 모든 AC (Acceptance Criteria) 충족 확인

### Code Review Fixes (2026-01-12)

- **M1 Fixed**: `#[must_use]` attribute 추가 (`is_empty()`, `len()`)
- **M2 Fixed**: `PartialEq`, `Eq` derive 추가 - 직접 비교 연산 가능
- **M4 Addressed**: 빈 벡터 경고 문서화 (`is_empty()` doc 주석에 설명 추가)
- **Test Added**: `test_subscription_change_equality` 추가 (6 tests total)

### File List

- `crates/feeds/src/subscription.rs` (신규)
- `crates/feeds/src/lib.rs` (수정)

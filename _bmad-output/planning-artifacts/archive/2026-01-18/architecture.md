---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/planning-artifacts/research/technical-dynamic-market-subscription-research-2026-01-11.md'
  - 'docs/index.md'
  - 'docs/project-overview.md'
  - 'docs/architecture.md'
  - 'docs/api-contracts.md'
  - 'docs/development-guide.md'
  - 'docs/source-tree-analysis.md'
workflowType: 'architecture'
project_name: 'arbitrage-bot'
user_name: 'Hyowon'
date: '2026-01-11'
featureScope: 'runtime-dynamic-market-subscription'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
- 마켓 발견 및 구독 관리 (FR1-3): MarketDiscovery 주기마다 새 공통 마켓 감지, diff 계산
- 거래소별 동적 구독 (FR4-9): 6개 거래소 런타임 WebSocket 구독 지원
- 에러 처리 및 복구 (FR10-12): 지수 백오프 재시도, 재연결 시 전체 재구독
- Rate-limit 관리 (FR13-14): 거래소별 제한 준수, 배치 처리
- 로깅 및 모니터링 (FR15-18): 구독 변경 이벤트 추적
- 기회 탐지 통합 (FR19-20): 새 마켓 가격 데이터 → 차익거래 기회 탐지

**Non-Functional Requirements:**
- 성능: 구독 요청 → 확인 < 5초, 채널 오버헤드 < 1ms
- 안정성: 24시간+ 무중단, 구독 실패율 < 1%
- 통합: 기존 feeds 크레이트 아키텍처 호환

**Scale & Complexity:**

- Primary domain: Backend/API (Rust/Tokio WebSocket)
- Complexity level: Medium
- Estimated architectural components: 4 (SubscriptionManager, WsClient 수정, 거래소별 빌더, 채널 통합)

### Technical Constraints & Dependencies

**거래소별 제약:**
- Binance: 5 msg/sec rate limit, 최대 1024 스트림, 초과 시 연결 해제
- Coinbase: JWT 인증 필요
- Bybit: 500 연결/5분, heartbeat 필수
- Upbit: 새 구독 시 이전 구독 대체 (전체 목록 재전송 필요)
- Bithumb: 제한적 API, 재연결 방식 권장
- Gate.io: 명시적 unsubscribe 전까지 구독 유지

**기존 아키텍처 종속성:**
- WsClient: subscription_rx 채널 추가, select! 확장 필요
- MarketDiscovery: diff 계산 로직 추가
- AppState: SubscriptionManager 초기화 및 채널 연결

### Cross-Cutting Concerns Identified

- **에러 처리**: 지수 백오프 재시도, 최대 재시도 제한, 에러 분류
- **Rate-limit 준수**: Binance 5 msg/sec, Bybit 연결 제한 → 배치 처리, 큐잉
- **로깅 표준**: INFO (성공), WARN (실패/재시도), ERROR (최대 재시도 초과)
- **재연결 동기화**: 연결 끊김 후 재연결 시 현재 공통 마켓 전체 재구독

## Starter Template Evaluation

### Primary Technology Domain

Brownfield Extension - 기존 Rust/Tokio WebSocket 아키텍처 확장

### Architecture Extension Strategy

이 기능은 새 프로젝트가 아닌 기존 아키텍처 확장입니다.

**기존 아키텍처 활용:**
- 크레이트 구조: `crates/feeds/` 확장
- 통신 패턴: mpsc 채널 + Tokio select!
- 상태 관리: DashMap 기반 lock-free
- 에러 처리: 기존 Circuit Breaker + 지수 백오프

**신규 모듈 추가:**
- `crates/feeds/src/subscription.rs`: SubscriptionManager, SubscriptionChange

**기존 파일 수정:**
- `crates/feeds/src/websocket.rs`: subscription_rx 채널 추가
- `crates/feeds/src/lib.rs`: subscription 모듈 export
- `apps/server/src/main.rs`: SubscriptionManager 초기화

### Architectural Decisions Inherited

**Language & Runtime:**
- Rust 1.75+, Tokio async runtime

**Communication:**
- mpsc::channel for subscription changes
- Broadcast channel for client notifications

**State Management:**
- DashMap for lock-free concurrent access
- AtomicU64 for statistics

**Error Handling:**
- Exponential backoff with jitter
- Circuit breaker pattern for connection failures

**Note:** No starter template initialization required - extending existing codebase.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
1. 채널 구조: SubscriptionManager 중앙화
2. 거래소별 구독 메시지 빌더 구현

**Important Decisions (Shape Architecture):**
3. 재시도 전략: 지수 백오프 + 지터
4. Upbit/Bithumb 특수 처리

**Deferred Decisions (Post-MVP):**
- Unsubscribe 로직 (상장 폐지 대응)
- 구독 상태 모니터링 UI

### Communication Architecture

**Decision 1: 채널 구조 - SubscriptionManager 중앙화**

| 항목 | 결정 |
|------|------|
| 패턴 | SubscriptionManager가 거래소별 mpsc::Sender 보유 |
| 이유 | 중앙 집중 관리, diff 계산 용이, 거래소별 독립성 유지 |

```rust
pub struct SubscriptionManager {
    senders: HashMap<Exchange, mpsc::Sender<SubscriptionChange>>,
    current_subscriptions: DashMap<Exchange, HashSet<String>>,
}
```

**Decision 2: 데이터 구조**

```rust
pub enum SubscriptionChange {
    Subscribe(Vec<String>),   // 새 심볼 구독
    Unsubscribe(Vec<String>), // 심볼 구독 해제 (Future)
}
```

### Error Handling & Resilience

**Decision 3: 재시도 전략 - 지수 백오프 + 지터**

| 파라미터 | 값 | 이유 |
|----------|-----|------|
| Base delay | 2초 | 즉시 재시도 방지 |
| Max delay | 5분 | 무한 대기 방지 |
| Jitter | 25% | 동시 재시도 분산 |
| Max retries | 5회 | 무한 재시도 방지 |

```rust
fn calculate_backoff(attempt: u32) -> Duration {
    let base = Duration::from_secs(2);
    let max = Duration::from_secs(300);
    let delay = base * 2u32.pow(attempt);
    let jitter = rand::thread_rng().gen_range(0.75..1.25);
    min(delay.mul_f64(jitter), max)
}
```

### Exchange-Specific Handling

**Decision 4: Upbit 특수 처리 - 전체 목록 재전송**

| 항목 | 결정 |
|------|------|
| 전략 | 새 마켓 추가 시 현재 구독 전체 + 새 마켓 전송 |
| 이유 | 연결 안정성 유지, Upbit API 특성 준수 |

```rust
// Upbit 구독 메시지 빌더
fn build_upbit_subscribe(&self, all_symbols: &[String]) -> String {
    json!([
        {"ticket": self.ticket_id},
        {"type": "ticker", "codes": all_symbols},
        {"format": "SIMPLE"}
    ]).to_string()
}
```

**Decision 5: Bithumb 처리 전략 - 재연결 방식**

| 항목 | 결정 |
|------|------|
| 전략 | 구독 변경 시 연결 끊고 새 구독 목록으로 재연결 |
| 이유 | API 제한으로 동적 구독 불가, 일관된 동작 보장 |

### Rate-limit Management

**Decision 6: 배치 큐 방식**

| 거래소 | Rate Limit | 배치 전략 |
|--------|-----------|-----------|
| Binance | 5 msg/sec | 1초마다 최대 5개 전송 |
| Bybit | 500 연결/5분 | 연결 수 모니터링 |
| 기타 | - | 즉시 전송 |

```rust
impl SubscriptionManager {
    async fn flush_pending_binance(&mut self) {
        let batch: Vec<_> = self.pending_binance
            .drain(..5.min(self.pending_binance.len()))
            .collect();
        if !batch.is_empty() {
            self.send_to_binance(batch).await;
        }
    }
}
```

### Decision Impact Analysis

**Implementation Sequence:**
1. `SubscriptionChange` enum 정의
2. `SubscriptionManager` 구현
3. `WsClient`에 subscription_rx 추가
4. 거래소별 구독 메시지 빌더 구현 (Binance → Bybit → GateIO → Coinbase → Upbit → Bithumb)
5. `main.rs`에서 채널 연결

**Cross-Component Dependencies:**
- SubscriptionManager → WsClient: mpsc 채널
- MarketDiscovery → SubscriptionManager: update_subscriptions() 호출
- WsClient → 거래소: 구독 메시지 전송


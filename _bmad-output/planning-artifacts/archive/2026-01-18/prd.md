---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-03-success', 'step-04-journeys', 'step-05-domain', 'step-06-innovation', 'step-07-project-type', 'step-08-scoping', 'step-09-functional', 'step-10-nonfunctional', 'step-11-polish']
status: complete
classification:
  projectType: api_backend
  domain: fintech
  complexity: medium
  projectContext: brownfield
  featureScope: runtime-dynamic-market-subscription
inputDocuments:
  - '_bmad-output/planning-artifacts/research/technical-dynamic-market-subscription-research-2026-01-11.md'
  - 'docs/index.md'
  - 'docs/project-overview.md'
  - 'docs/architecture.md'
  - 'docs/api-contracts.md'
  - 'docs/development-guide.md'
  - 'docs/source-tree-analysis.md'
workflowType: 'prd'
documentCounts:
  briefs: 0
  research: 1
  brainstorming: 0
  projectDocs: 6
---

# Product Requirements Document - arbitrage-bot

**Feature:** Runtime Dynamic Market Subscription
**Author:** Hyowon
**Date:** 2026-01-11

---

## Executive Summary

### 문제 정의

현재 arbitrage-bot은 서버 시작 시점의 공통 마켓만 WebSocket으로 구독합니다. 새로운 코인이 거래소에 상장되더라도 서버를 재시작하기 전까지는 해당 마켓의 가격 피드를 받을 수 없어 차익거래 기회를 놓치게 됩니다.

### 솔루션

런타임 중 동적으로 새 마켓을 WebSocket 구독에 추가하는 기능을 구현합니다. MarketDiscovery가 새 공통 마켓을 발견하면 SubscriptionManager가 각 거래소의 WsClient에 구독 요청을 전송하고, 서버 재시작 없이 새 코인의 차익거래 기회를 포착할 수 있습니다.

### 핵심 가치

| 항목 | 설명 |
|------|------|
| **비즈니스 가치** | 새 상장 코인 차익거래 기회 포착, 운영 부담 감소 |
| **기술적 범위** | 4개 파일 수정/추가, 6개 거래소 지원 |
| **복잡도** | 중간 (기존 아키텍처 확장) |

### 성공 지표

- 새 마켓 상장 후 **5분 이내** Opportunity 탐지 시작
- 구독 실패율 **< 1%** (재시도 포함)
- 거래소별 rate-limit 위반 **0건**

---

## Success Criteria

### User Success

| 기준 | 목표 |
|------|------|
| 새 공통 마켓 감지 → Opportunity 생성 | 자동화 (수동 개입 없음) |
| 감지 속도 | 거래소별 rate-limit 내 최대 속도 |
| 사용자 인지 | 새 마켓 추가 시 로그/알림으로 확인 가능 |

### Technical Success

| 기준 | 목표 |
|------|------|
| 구독 실패 시 복구 | 자동 재시도 (지수 백오프) |
| 로깅 수준 | 구독 변경 이벤트 추적 가능 (INFO 레벨) |
| Rate-limit 준수 | 거래소별 제한 내 동작 (예: Binance 5 msg/sec) |
| 안정성 | 24시간+ 무중단 운영 |

### Business Success

| 기준 | 목표 |
|------|------|
| 차익거래 기회 손실 감소 | 새 상장 코인 기회 포착 가능 |
| 운영 부담 감소 | 서버 재시작 불필요 |

### Measurable Outcomes

- 새 마켓 상장 후 5분 이내 Opportunity 탐지 시작
- 구독 실패율 < 1% (재시도 포함)
- 거래소별 rate-limit 위반 0건

## Product Scope

### MVP - Minimum Viable Product

- 마켓 디스커버리 → WebSocket 재구독 연결
- 6개 거래소 동적 구독 지원 (Binance, Coinbase, Bybit, GateIO, Upbit, Bithumb)
- 구독 실패 시 재시도 로직 (지수 백오프)
- 구독 변경 로깅 (INFO 레벨)

### Growth Features (Post-MVP)

- 텔레그램 알림: "새 마켓 상장: SOL/USDT [Binance, Upbit, Bybit]"
- 대시보드에 새 마켓 하이라이트 표시

### Vision (Future)

- 상장 폐지(Delisting) 자동 unsubscribe
- 거래소별 구독 상태 모니터링 UI

## User Journeys

### Journey 1: 새 마켓 상장 - 성공 시나리오

**상황:** Binance, Upbit, Bybit에 새로운 코인 "NEWCOIN"이 거의 동시에 상장됨

**시스템 흐름:**
1. MarketDiscovery가 5분 주기로 각 거래소 마켓 목록 조회
2. 새로운 공통 마켓 "NEWCOIN/USDT" 발견
3. SubscriptionManager가 현재 구독 목록과 diff 계산
4. 각 거래소 WsClient에 구독 요청 채널로 전송
5. WsClient가 거래소에 SUBSCRIBE 메시지 전송 (rate-limit 준수)
6. 거래소가 구독 확인 응답
7. 새 코인 가격 데이터 수신 시작
8. OpportunityDetector가 NEWCOIN에 대한 차익거래 기회 탐지
9. 로그: `[INFO] New market subscribed: NEWCOIN/USDT on [Binance, Upbit, Bybit]`

**결과:** 서버 재시작 없이 새 코인 차익거래 기회 포착 가능

### Journey 2: 구독 실패 - 복구 시나리오

**상황:** Binance 구독 요청이 rate-limit으로 실패

**시스템 흐름:**
1. WsClient가 Binance에 SUBSCRIBE 전송
2. Binance가 rate-limit 초과 오류 반환
3. 로그: `[WARN] Subscription failed for NEWCOIN@trade: rate limit exceeded`
4. 시스템이 지수 백오프로 재시도 스케줄 (2초 후)
5. 로그: `[INFO] Retry #1 for NEWCOIN@trade in 2s`
6. 재시도 성공
7. 로그: `[INFO] Successfully subscribed: NEWCOIN@trade after 1 retry`

**실패 지속 시:**
- 최대 재시도 횟수 초과
- 로그: `[ERROR] Max retries exceeded for NEWCOIN@trade - manual intervention required`
- 다른 거래소 구독은 정상 진행

### Journey 3: 운영자 모니터링

**상황:** 운영자가 시스템 상태 확인

**운영자 행동:**
1. 서버 로그 확인
2. 새 마켓 구독 이벤트 확인: `[INFO] New market subscribed: ...`
3. 구독 실패/재시도 이벤트 확인: `[WARN] Subscription failed: ...`
4. 현재 구독 중인 마켓 수 확인 (기존 stats 로그에 포함)

**확인 가능 정보:**
- 새로 추가된 마켓 목록
- 구독 성공/실패 현황
- Rate-limit 관련 경고

### Journey Requirements Summary

| 여정 | 필요 기능 |
|------|----------|
| 새 마켓 상장 | SubscriptionManager, 거래소별 구독 메시지 빌더, 채널 통신 |
| 구독 실패 복구 | 재시도 로직 (지수 백오프), 에러 분류, 최대 재시도 제한 |
| 운영자 모니터링 | 구독 변경 로깅, 상태 추적 |

## Domain-Specific Requirements

### API/거래소 제약

| 거래소 | Rate Limit | 최대 스트림 | 특이사항 |
|--------|-----------|------------|----------|
| Binance | 5 msg/sec | 1024/연결 | 초과 시 연결 해제 |
| Coinbase | - | - | JWT 인증 필요 |
| Bybit | 500 연결/5분 | - | heartbeat 필수 |
| Upbit | - | - | 새 구독 시 이전 구독 대체 (누적 아님) |
| Bithumb | 제한적 | - | 재연결 방식 권장 |
| Gate.io | - | - | 명시적 unsubscribe 전까지 유지 |

### 기술적 제약

- **Rate-limit 준수**: 거래소별 제한 내에서 동작
- **연결 안정성**: 기존 Circuit Breaker 패턴 활용
- **Upbit 특수 처리**: 전체 구독 목록 재전송 방식

### 리스크 및 완화

| 리스크 | 영향 | 완화 방안 |
|--------|------|----------|
| 다수 마켓 동시 상장 | rate-limit 초과 | 배치 처리 + 큐잉 |
| 구독 중 연결 끊김 | 새 마켓 누락 | 재연결 시 전체 공통 마켓 재구독 |
| 거래소 API 변경 | 구독 실패 | 기존 어댑터 패턴으로 격리 |

## Technical Requirements

### 내부 인터페이스

| 컴포넌트 | 인터페이스 | 설명 |
|----------|-----------|------|
| SubscriptionManager | `update_subscriptions(&CommonMarkets)` | 새 마켓 구독 요청 |
| WsClient | `subscription_rx: Receiver<SubscriptionChange>` | 구독 변경 수신 채널 |
| 거래소별 빌더 | `build_*_subscribe(&[String])` | 구독 메시지 생성 |

### 데이터 구조

```rust
enum SubscriptionChange {
    Subscribe(Vec<String>),   // 새 심볼 구독
    Unsubscribe(Vec<String>), // 심볼 구독 해제 (Future)
}

struct SubscriptionManager {
    senders: HashMap<Exchange, mpsc::Sender<SubscriptionChange>>,
    current_subscriptions: DashMap<Exchange, HashSet<String>>,
}
```

### 에러 처리

| 에러 유형 | 처리 방식 |
|----------|----------|
| Rate limit exceeded | 지수 백오프 재시도 (base: 2초, max: 5분) |
| Connection lost | 재연결 시 전체 재구독 |
| Invalid symbol | 로그 경고, 건너뛰기 |
| Max retries exceeded | 에러 로그, 다른 거래소 계속 진행 |

### 구현 변경 파일

| 파일 | 변경 유형 | 설명 |
|------|----------|------|
| `crates/feeds/src/subscription.rs` | 신규 | SubscriptionManager, SubscriptionChange |
| `crates/feeds/src/websocket.rs` | 수정 | subscription_rx 추가, select! 확장 |
| `crates/feeds/src/lib.rs` | 수정 | subscription 모듈 export |
| `apps/server/src/main.rs` | 수정 | SubscriptionManager 초기화, 채널 연결 |

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**MVP 접근법:** Problem-Solving MVP
- 현재 문제(서버 재시작 필요)를 해결하는 최소 기능 집합
- 기존 아키텍처 확장으로 구현 (새 패턴 도입 최소화)

### MVP Feature Set (Phase 1)

**지원 핵심 여정:**
- Journey 1: 새 마켓 상장 - 성공 시나리오
- Journey 2: 구독 실패 - 복구 시나리오
- Journey 3: 운영자 모니터링

**필수 기능:**
- SubscriptionManager 구현
- 6개 거래소 동적 구독 메시지 빌더
- WsClient subscription_rx 채널 통합
- 구독 실패 재시도 로직 (지수 백오프)
- 구독 변경 로깅 (INFO 레벨)

### Post-MVP Features

**Phase 2 (Growth):**
- 텔레그램 알림: "새 마켓 상장: SOL/USDT [Binance, Upbit, Bybit]"
- 대시보드에 새 마켓 하이라이트 표시
- 구독 통계 stats에 포함

**Phase 3 (Vision):**
- 상장 폐지(Delisting) 자동 unsubscribe
- 거래소별 구독 상태 모니터링 UI
- 구독 히스토리 로깅

### Risk Mitigation Strategy

**기술적 리스크:**
- Upbit 전체 재구독 방식 → 다른 거래소 먼저 구현, Upbit 마지막
- Bithumb 제한적 API → 재연결 방식 대안 준비

**운영 리스크:**
- 다수 마켓 동시 상장 → 배치 처리 + 큐잉으로 rate-limit 준수

**자원 리스크:**
- 단독 개발 → 거래소별 순차 구현 (Binance → Bybit → GateIO → Coinbase → Upbit → Bithumb)

## Functional Requirements

### 마켓 발견 및 구독 관리

- **FR1:** 시스템은 마켓 디스커버리 주기(5분)마다 새로운 공통 마켓을 감지할 수 있다
- **FR2:** 시스템은 현재 구독 목록과 새 공통 마켓 간의 차이(diff)를 계산할 수 있다
- **FR3:** 시스템은 새로 발견된 마켓에 대해 거래소별 WebSocket 구독을 요청할 수 있다

### 거래소별 동적 구독

- **FR4:** 시스템은 Binance WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR5:** 시스템은 Coinbase WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR6:** 시스템은 Bybit WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR7:** 시스템은 Gate.io WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR8:** 시스템은 Upbit WebSocket에 런타임 중 새 심볼을 구독할 수 있다 (전체 목록 재전송 방식)
- **FR9:** 시스템은 Bithumb WebSocket에 런타임 중 새 심볼을 구독할 수 있다

### 에러 처리 및 복구

- **FR10:** 시스템은 구독 실패 시 지수 백오프로 자동 재시도할 수 있다
- **FR11:** 시스템은 최대 재시도 횟수 초과 시 에러를 로깅하고 다른 거래소 구독을 계속할 수 있다
- **FR12:** 시스템은 연결 재시작 시 현재 공통 마켓 전체를 재구독할 수 있다

### Rate-limit 관리

- **FR13:** 시스템은 거래소별 rate-limit 제한을 준수하여 구독 요청을 전송할 수 있다
- **FR14:** 시스템은 다수 마켓 동시 상장 시 배치 처리로 rate-limit을 준수할 수 있다

### 로깅 및 모니터링

- **FR15:** 시스템은 새 마켓 구독 성공 시 INFO 레벨로 로깅할 수 있다
- **FR16:** 시스템은 구독 실패 시 WARN 레벨로 로깅할 수 있다
- **FR17:** 시스템은 재시도 시도 시 INFO 레벨로 로깅할 수 있다
- **FR18:** 시스템은 최대 재시도 초과 시 ERROR 레벨로 로깅할 수 있다

### 기회 탐지 통합

- **FR19:** 시스템은 새로 구독된 마켓의 가격 데이터를 수신할 수 있다
- **FR20:** 시스템은 새로 구독된 마켓에 대해 차익거래 기회를 탐지할 수 있다

## Non-Functional Requirements

### 성능

- **NFR1:** 구독 요청 → 확인 응답 대기 시간 < 5초
- **NFR2:** Rate-limit 위반 시 재시도 지연 2초 ~ 5분 (지수 백오프)
- **NFR3:** 새 마켓 발견 → 구독 완료 < 10초 (rate-limit 내)
- **NFR4:** 채널 통신 오버헤드 < 1ms (mpsc 채널)

### 안정성

- **NFR5:** 24시간+ 연속 운영 가능 (서버 재시작 없이)
- **NFR6:** 단일 거래소 장애가 다른 거래소 구독에 영향 없음
- **NFR7:** 구독 실패율 < 1% (재시도 포함)
- **NFR8:** 연결 끊김 후 자동 재연결 및 전체 재구독

### 통합

- **NFR9:** 6개 거래소 WebSocket API 프로토콜 준수
- **NFR10:** 거래소별 rate-limit 제한 내 동작
  - Binance: 5 msg/sec
  - Bybit: 500 연결/5분
  - 기타: 문서화된 제한 준수
- **NFR11:** 기존 feeds 크레이트 아키텍처와 호환
- **NFR12:** 기존 로깅 인프라 (tracing) 활용


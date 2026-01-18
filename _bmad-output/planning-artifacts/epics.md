---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories', 'step-04-final-validation']
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - 'docs/architecture.md'
status: complete
feature: runtime-dynamic-market-subscription
---

# Runtime Dynamic Market Subscription - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for Runtime Dynamic Market Subscription, decomposing the requirements from the PRD and Architecture into implementable stories.

## Requirements Inventory

### Functional Requirements

- **FR1:** 시스템은 마켓 디스커버리 주기(5분)마다 새로운 공통 마켓을 감지할 수 있다
- **FR2:** 시스템은 현재 구독 목록과 새 공통 마켓 간의 차이(diff)를 계산할 수 있다
- **FR3:** 시스템은 새로 발견된 마켓에 대해 거래소별 WebSocket 구독을 요청할 수 있다
- **FR4:** 시스템은 Binance WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR5:** 시스템은 Coinbase WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR6:** 시스템은 Bybit WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR7:** 시스템은 Gate.io WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR8:** 시스템은 Upbit WebSocket에 런타임 중 새 심볼을 구독할 수 있다 (전체 목록 재전송 방식)
- **FR9:** 시스템은 Bithumb WebSocket에 런타임 중 새 심볼을 구독할 수 있다
- **FR10:** 시스템은 구독 실패 시 지수 백오프로 자동 재시도할 수 있다
- **FR11:** 시스템은 최대 재시도 횟수 초과 시 에러를 로깅하고 다른 거래소 구독을 계속할 수 있다
- **FR12:** 시스템은 연결 재시작 시 현재 공통 마켓 전체를 재구독할 수 있다
- **FR13:** 시스템은 거래소별 rate-limit 제한을 준수하여 구독 요청을 전송할 수 있다
- **FR14:** 시스템은 다수 마켓 동시 상장 시 배치 처리로 rate-limit을 준수할 수 있다
- **FR15:** 시스템은 새 마켓 구독 성공 시 INFO 레벨로 로깅할 수 있다
- **FR16:** 시스템은 구독 실패 시 WARN 레벨로 로깅할 수 있다
- **FR17:** 시스템은 재시도 시도 시 INFO 레벨로 로깅할 수 있다
- **FR18:** 시스템은 최대 재시도 초과 시 ERROR 레벨로 로깅할 수 있다
- **FR19:** 시스템은 새로 구독된 마켓의 가격 데이터를 수신할 수 있다
- **FR20:** 시스템은 새로 구독된 마켓에 대해 차익거래 기회를 탐지할 수 있다

### Non-Functional Requirements

- **NFR1:** 구독 요청 → 확인 응답 대기 시간 < 5초
- **NFR2:** Rate-limit 위반 시 재시도 지연 2초 ~ 5분 (지수 백오프)
- **NFR3:** 새 마켓 발견 → 구독 완료 < 10초 (rate-limit 내)
- **NFR4:** 채널 통신 오버헤드 < 1ms (mpsc 채널)
- **NFR5:** 24시간+ 연속 운영 가능 (서버 재시작 없이)
- **NFR6:** 단일 거래소 장애가 다른 거래소 구독에 영향 없음
- **NFR7:** 구독 실패율 < 1% (재시도 포함)
- **NFR8:** 연결 끊김 후 자동 재연결 및 전체 재구독
- **NFR9:** 6개 거래소 WebSocket API 프로토콜 준수
- **NFR10:** 거래소별 rate-limit 제한 내 동작 (Binance 5 msg/sec, Bybit 500 연결/5min)
- **NFR11:** 기존 feeds 크레이트 아키텍처와 호환
- **NFR12:** 기존 로깅 인프라 (tracing) 활용

### Additional Requirements

**Architecture 요구사항:**
- Runner/Handler 분리 패턴 준수 (Runner: 순수 파싱, Handler: 상태 업데이트)
- DashMap 기반 lock-free 상태 관리
- 채널 기반 통신 (FeedMessage, PriceUpdateEvent 패턴)
- 기존 WsClient Circuit Breaker 패턴 활용
- 지수 백오프 + 지터 재연결 전략 (기본 1초, 최대 5분)
- Upbit 특수 처리: 새 구독 시 전체 목록 재전송 (누적 아님)
- Bithumb 제한적 API: 재연결 방식 권장

**구현 변경 파일:**
- `crates/feeds/src/subscription.rs` (신규) - SubscriptionManager, SubscriptionChange
- `crates/feeds/src/websocket.rs` (수정) - subscription_rx 추가, select! 확장
- `crates/feeds/src/lib.rs` (수정) - subscription 모듈 export
- `apps/server/src/main.rs` (수정) - SubscriptionManager 초기화, 채널 연결

### FR Coverage Map

| FR | Epic | 설명 |
|----|------|------|
| FR1 | Epic 1 | 마켓 디스커버리 주기마다 새 공통 마켓 감지 |
| FR2 | Epic 1 | 현재 구독 목록과 diff 계산 |
| FR3 | Epic 1 | 거래소별 WebSocket 구독 요청 |
| FR4 | Epic 2 | Binance 동적 구독 |
| FR5 | Epic 2 | Coinbase 동적 구독 |
| FR6 | Epic 2 | Bybit 동적 구독 |
| FR7 | Epic 2 | GateIO 동적 구독 |
| FR8 | Epic 2 | Upbit 동적 구독 (전체 목록 재전송) |
| FR9 | Epic 2 | Bithumb 동적 구독 |
| FR10 | Epic 3 | 지수 백오프 재시도 |
| FR11 | Epic 3 | 최대 재시도 초과 처리 |
| FR12 | Epic 3 | 연결 재시작 시 전체 재구독 |
| FR13 | Epic 3 | 거래소별 rate-limit 준수 |
| FR14 | Epic 3 | 배치 처리로 rate-limit 준수 |
| FR15 | Epic 4 | 구독 성공 INFO 로깅 |
| FR16 | Epic 4 | 구독 실패 WARN 로깅 |
| FR17 | Epic 4 | 재시도 INFO 로깅 |
| FR18 | Epic 4 | 최대 재시도 초과 ERROR 로깅 |
| FR19 | Epic 5 | 새 마켓 가격 데이터 수신 |
| FR20 | Epic 5 | 차익거래 기회 탐지 |

## Epic List

### Epic 1: 구독 관리 인프라
시스템이 런타임 중 동적으로 마켓 구독을 관리할 수 있는 핵심 인프라 구축
**FRs:** FR1, FR2, FR3
**NFRs:** NFR4, NFR11

### Epic 2: 거래소별 동적 구독 구현
6개 거래소에서 런타임 중 새 심볼을 구독할 수 있음
**FRs:** FR4, FR5, FR6, FR7, FR8, FR9
**NFRs:** NFR9, NFR10

### Epic 3: 에러 처리 및 복원력
구독 실패 시 자동 복구로 안정적인 24시간+ 운영 보장
**FRs:** FR10, FR11, FR12, FR13, FR14
**NFRs:** NFR1, NFR2, NFR5, NFR6, NFR7, NFR8

### Epic 4: 로깅 및 운영 가시성
운영자가 구독 상태와 이벤트를 모니터링할 수 있음
**FRs:** FR15, FR16, FR17, FR18
**NFRs:** NFR12

### Epic 5: 기회 탐지 통합
새로 구독된 마켓에서 차익거래 기회 탐지 시작
**FRs:** FR19, FR20
**NFRs:** NFR3

---

## Epic 1: 구독 관리 인프라

시스템이 런타임 중 동적으로 마켓 구독을 관리할 수 있는 핵심 인프라 구축

### Story 1.1: SubscriptionChange 데이터 구조 정의

As a **시스템 개발자**,
I want **구독 변경 요청을 표현하는 데이터 구조**,
So that **마켓 구독 추가/제거를 타입 안전하게 전달할 수 있다**.

**Acceptance Criteria:**

**Given** feeds 크레이트에 subscription.rs 모듈이 없을 때
**When** SubscriptionChange enum을 정의하면
**Then** Subscribe(Vec<String>)와 Unsubscribe(Vec<String>) 변형이 존재해야 한다
**And** lib.rs에서 subscription 모듈이 export 되어야 한다

### Story 1.2: SubscriptionManager 구조체 구현

As a **시스템 개발자**,
I want **거래소별 구독 상태를 관리하는 SubscriptionManager**,
So that **현재 구독 목록을 추적하고 구독 요청을 전송할 수 있다**.

**Acceptance Criteria:**

**Given** SubscriptionChange 타입이 정의되어 있을 때
**When** SubscriptionManager를 생성하면
**Then** 각 거래소별 mpsc::Sender<SubscriptionChange>를 보유해야 한다
**And** current_subscriptions: DashMap<Exchange, HashSet<String>>으로 현재 구독 추적해야 한다
**And** update_subscriptions 메서드로 새 마켓과 현재 구독 간 diff 계산이 가능해야 한다

### Story 1.3: WsClient 구독 채널 통합

As a **시스템 개발자**,
I want **WsClient가 구독 변경 요청을 수신할 수 있는 채널**,
So that **런타임 중 동적으로 구독을 추가할 수 있다**.

**Acceptance Criteria:**

**Given** WsClient가 실행 중일 때
**When** subscription_rx 채널을 select! 루프에 추가하면
**Then** SubscriptionChange::Subscribe 수신 시 구독 메시지를 거래소에 전송해야 한다
**And** 기존 메시지 처리 로직에 영향을 주지 않아야 한다
**And** 채널 통신 오버헤드가 1ms 미만이어야 한다 (NFR4)

### Story 1.4: 서버 초기화 시 채널 연결

As a **시스템 운영자**,
I want **서버 시작 시 SubscriptionManager와 WsClient 간 채널이 연결**,
So that **마켓 디스커버리가 새 마켓 발견 시 구독 요청을 전송할 수 있다**.

**Acceptance Criteria:**

**Given** 서버가 시작될 때
**When** SubscriptionManager를 초기화하면
**Then** 각 거래소 WsClient에 대한 mpsc 채널이 생성되어야 한다
**And** MarketDiscovery가 새 공통 마켓 발견 시 SubscriptionManager.update_subscriptions 호출이 가능해야 한다
**And** 기존 feeds 크레이트 아키텍처와 호환되어야 한다 (NFR11)

---

## Epic 2: 거래소별 동적 구독 구현

6개 거래소에서 런타임 중 새 심볼을 구독할 수 있음

### Story 2.1: Binance 동적 구독 메시지 빌더

As a **시스템 개발자**,
I want **Binance WebSocket에 런타임 구독 메시지를 생성하는 빌더**,
So that **새 마켓을 Binance에 동적으로 구독할 수 있다**.

**Acceptance Criteria:**

**Given** 새 심볼 목록 ["BTCUSDT", "ETHUSDT"]가 있을 때
**When** build_binance_subscribe 함수를 호출하면
**Then** `{"method":"SUBSCRIBE","params":["btcusdt@trade","ethusdt@trade"],"id":N}` 형식의 JSON이 생성되어야 한다
**And** rate-limit 5 msg/sec를 준수해야 한다 (NFR10)

### Story 2.2: Coinbase 동적 구독 메시지 빌더

As a **시스템 개발자**,
I want **Coinbase WebSocket에 런타임 구독 메시지를 생성하는 빌더**,
So that **새 마켓을 Coinbase에 동적으로 구독할 수 있다**.

**Acceptance Criteria:**

**Given** 새 심볼 목록 ["BTC-USD", "ETH-USD"]가 있을 때
**When** build_coinbase_subscribe 함수를 호출하면
**Then** Coinbase subscribe 메시지 형식에 맞는 JSON이 생성되어야 한다
**And** API 프로토콜을 준수해야 한다 (NFR9)

### Story 2.3: Bybit 동적 구독 메시지 빌더

As a **시스템 개발자**,
I want **Bybit WebSocket에 런타임 구독 메시지를 생성하는 빌더**,
So that **새 마켓을 Bybit에 동적으로 구독할 수 있다**.

**Acceptance Criteria:**

**Given** 새 심볼 목록이 있을 때
**When** build_bybit_subscribe 함수를 호출하면
**Then** Bybit subscribe 메시지 형식에 맞는 JSON이 생성되어야 한다
**And** 500 연결/5분 rate-limit을 준수해야 한다 (NFR10)

### Story 2.4: GateIO 동적 구독 메시지 빌더

As a **시스템 개발자**,
I want **GateIO WebSocket에 런타임 구독 메시지를 생성하는 빌더**,
So that **새 마켓을 GateIO에 동적으로 구독할 수 있다**.

**Acceptance Criteria:**

**Given** 새 심볼 목록이 있을 때
**When** build_gateio_subscribe 함수를 호출하면
**Then** GateIO subscribe 메시지 형식에 맞는 JSON이 생성되어야 한다
**And** API 프로토콜을 준수해야 한다 (NFR9)

### Story 2.5: Upbit 동적 구독 메시지 빌더 (전체 목록 재전송)

As a **시스템 개발자**,
I want **Upbit WebSocket에 전체 구독 목록을 재전송하는 빌더**,
So that **새 마켓을 Upbit에 동적으로 구독할 수 있다**.

**Acceptance Criteria:**

**Given** 현재 구독 목록 + 새 심볼 목록이 있을 때
**When** build_upbit_subscribe 함수를 호출하면
**Then** 전체 심볼 목록을 포함한 Upbit subscribe 메시지가 생성되어야 한다 (Upbit는 누적이 아닌 대체 방식)
**And** MessagePack 포맷을 지원해야 한다

### Story 2.6: Bithumb 동적 구독 메시지 빌더

As a **시스템 개발자**,
I want **Bithumb WebSocket에 런타임 구독 메시지를 생성하는 빌더**,
So that **새 마켓을 Bithumb에 동적으로 구독할 수 있다**.

**Acceptance Criteria:**

**Given** 새 심볼 목록이 있을 때
**When** build_bithumb_subscribe 함수를 호출하면
**Then** Bithumb subscribe 메시지 형식에 맞는 JSON이 생성되어야 한다
**And** 제한적 API 특성을 고려해야 한다

---

## Epic 3: 에러 처리 및 복원력

구독 실패 시 자동 복구로 안정적인 24시간+ 운영 보장

### Story 3.1: 지수 백오프 재시도 로직 구현

As a **시스템 운영자**,
I want **구독 실패 시 지수 백오프로 자동 재시도**,
So that **일시적 오류에서 자동으로 복구할 수 있다**.

**Acceptance Criteria:**

**Given** 구독 요청이 실패했을 때
**When** 재시도 로직이 실행되면
**Then** 첫 재시도는 2초 후에 시도되어야 한다
**And** 이후 재시도는 2배씩 증가해야 한다 (2초 → 4초 → 8초...)
**And** 최대 지연은 5분을 초과하지 않아야 한다 (NFR2)

### Story 3.2: 최대 재시도 초과 시 우아한 실패 처리

As a **시스템 운영자**,
I want **최대 재시도 횟수 초과 시 해당 거래소만 실패 처리**,
So that **한 거래소 장애가 다른 거래소 구독에 영향을 주지 않는다**.

**Acceptance Criteria:**

**Given** 구독 재시도가 최대 횟수(예: 5회)를 초과했을 때
**When** 실패 처리 로직이 실행되면
**Then** 해당 거래소/심볼 구독은 실패 상태로 표시되어야 한다
**And** 다른 거래소 구독은 정상적으로 계속되어야 한다 (NFR6)
**And** 구독 실패율이 1% 미만이어야 한다 (NFR7)

### Story 3.3: 연결 재시작 시 전체 재구독

As a **시스템 운영자**,
I want **WebSocket 연결 재시작 시 현재 공통 마켓 전체를 재구독**,
So that **연결 끊김 후에도 모든 마켓 데이터를 수신할 수 있다**.

**Acceptance Criteria:**

**Given** WebSocket 연결이 끊어졌다가 재연결되었을 때
**When** 재연결 핸들러가 실행되면
**Then** SubscriptionManager의 current_subscriptions 전체를 다시 구독해야 한다
**And** 자동 재연결 및 재구독이 완료되어야 한다 (NFR8)

### Story 3.4: 거래소별 Rate-limit 준수 로직

As a **시스템 개발자**,
I want **거래소별 rate-limit을 준수하는 구독 요청 전송**,
So that **rate-limit 초과로 인한 연결 해제를 방지할 수 있다**.

**Acceptance Criteria:**

**Given** 다수의 구독 요청이 대기 중일 때
**When** 구독 요청을 전송하면
**Then** Binance는 5 msg/sec 제한을 준수해야 한다
**And** 구독 요청 응답 시간이 5초 미만이어야 한다 (NFR1)

### Story 3.5: 다수 마켓 동시 상장 시 배치 처리

As a **시스템 운영자**,
I want **다수 마켓 동시 상장 시 배치 처리로 rate-limit 준수**,
So that **대량 구독 요청 시에도 안정적으로 동작할 수 있다**.

**Acceptance Criteria:**

**Given** 10개 이상의 새 마켓이 동시에 발견되었을 때
**When** 구독 요청을 전송하면
**Then** 거래소별 rate-limit에 맞춰 배치로 나눠 전송되어야 한다
**And** 24시간+ 연속 운영이 가능해야 한다 (NFR5)

---

## Epic 4: 로깅 및 운영 가시성

운영자가 구독 상태와 이벤트를 모니터링할 수 있음

### Story 4.1: 구독 성공 INFO 로깅

As a **시스템 운영자**,
I want **새 마켓 구독 성공 시 INFO 레벨 로그**,
So that **어떤 마켓이 새로 추가되었는지 확인할 수 있다**.

**Acceptance Criteria:**

**Given** 새 마켓 구독이 성공했을 때
**When** 로깅이 실행되면
**Then** `[INFO] New market subscribed: {symbol} on [{exchanges}]` 형식으로 출력되어야 한다
**And** 기존 tracing 인프라를 활용해야 한다 (NFR12)

### Story 4.2: 구독 실패 WARN 로깅

As a **시스템 운영자**,
I want **구독 실패 시 WARN 레벨 로그**,
So that **문제 발생을 인지하고 조치할 수 있다**.

**Acceptance Criteria:**

**Given** 구독 요청이 실패했을 때
**When** 로깅이 실행되면
**Then** `[WARN] Subscription failed for {symbol}: {error_reason}` 형식으로 출력되어야 한다
**And** 에러 원인이 명확히 기록되어야 한다

### Story 4.3: 재시도 INFO 로깅

As a **시스템 운영자**,
I want **재시도 시도 시 INFO 레벨 로그**,
So that **복구 시도 상황을 추적할 수 있다**.

**Acceptance Criteria:**

**Given** 구독 재시도가 스케줄되었을 때
**When** 로깅이 실행되면
**Then** `[INFO] Retry #{n} for {symbol} in {delay}s` 형식으로 출력되어야 한다
**And** 재시도 횟수와 대기 시간이 포함되어야 한다

### Story 4.4: 최대 재시도 초과 ERROR 로깅

As a **시스템 운영자**,
I want **최대 재시도 초과 시 ERROR 레벨 로그**,
So that **수동 개입이 필요한 상황을 즉시 인지할 수 있다**.

**Acceptance Criteria:**

**Given** 최대 재시도 횟수를 초과했을 때
**When** 로깅이 실행되면
**Then** `[ERROR] Max retries exceeded for {symbol} - manual intervention required` 형식으로 출력되어야 한다
**And** 알림 시스템 연동이 가능해야 한다 (Future)

---

## Epic 5: 기회 탐지 통합

새로 구독된 마켓에서 차익거래 기회 탐지 시작

### Story 5.1: 새 마켓 가격 데이터 수신 확인

As a **시스템 운영자**,
I want **새로 구독된 마켓의 가격 데이터가 정상 수신되는지 확인**,
So that **구독이 성공적으로 완료되었음을 검증할 수 있다**.

**Acceptance Criteria:**

**Given** 새 마켓 구독이 완료되었을 때
**When** 거래소에서 가격 데이터를 전송하면
**Then** FeedHandler가 해당 마켓의 가격 데이터를 정상적으로 파싱해야 한다
**And** PriceAggregator에 새 마켓 가격이 저장되어야 한다
**And** 새 마켓 발견 → 구독 완료가 10초 이내여야 한다 (NFR3)

### Story 5.2: OpportunityDetector 새 마켓 통합

As a **트레이더**,
I want **새로 구독된 마켓에 대해 차익거래 기회가 탐지**,
So that **새 상장 코인에서도 수익 기회를 포착할 수 있다**.

**Acceptance Criteria:**

**Given** 새 마켓의 가격 데이터가 수신되고 있을 때
**When** 가격 차이가 임계값을 초과하면
**Then** OpportunityDetector가 해당 마켓의 차익거래 기회를 탐지해야 한다
**And** 기회가 WebSocket 클라이언트에 브로드캐스트되어야 한다
**And** 기존 이벤트 드리븐 탐지 로직과 동일하게 동작해야 한다

---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories']
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/planning-artifacts/architecture.md'
  - '_bmad-output/planning-artifacts/ux-design-specification.md'
feature: bloomberg-terminal-wts
---

# arbitrage-bot WTS - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for Bloomberg Terminal Style Web Trading System (WTS), decomposing the requirements from the PRD, Architecture, and UX Design into implementable stories.

## Requirements Inventory

### Functional Requirements

**거래소 관리 (FR1-3):**
- FR1: 사용자는 거래소 탭에서 거래소를 선택할 수 있다
- FR2: 사용자는 선택한 거래소의 연결 상태를 확인할 수 있다
- FR3: 시스템은 거래소 API 장애 시 상태를 표시한다

**잔고 조회 (FR4-6):**
- FR4: 사용자는 선택한 거래소의 자산별 잔고를 조회할 수 있다
- FR5: 사용자는 잔고 정보를 갱신할 수 있다
- FR6: 시스템은 거래 완료 후 잔고를 자동 갱신한다

**오더북 (FR7-9):**
- FR7: 사용자는 선택한 마켓의 실시간 호가창을 볼 수 있다
- FR8: 사용자는 거래할 마켓(예: BTC/KRW)을 선택할 수 있다
- FR9: 시스템은 WebSocket을 통해 오더북을 실시간 스트리밍한다

**주문 (FR10-16):**
- FR10: 사용자는 시장가 매수 주문을 실행할 수 있다
- FR11: 사용자는 시장가 매도 주문을 실행할 수 있다
- FR12: 사용자는 지정가 매수 주문을 실행할 수 있다
- FR13: 사용자는 지정가 매도 주문을 실행할 수 있다
- FR14: 사용자는 주문 수량과 가격을 입력할 수 있다
- FR15: 시스템은 주문 실행 전 확인 다이얼로그를 표시한다
- FR16: 시스템은 주문 결과를 콘솔에 즉시 표시한다

**입금 (FR17-20):**
- FR17: 사용자는 입금할 자산을 선택할 수 있다
- FR18: 사용자는 입금 네트워크를 선택할 수 있다
- FR19: 사용자는 입금 주소를 생성할 수 있다
- FR20: 사용자는 생성된 입금 주소를 복사할 수 있다

**출금 (FR21-27):**
- FR21: 사용자는 출금할 자산을 선택할 수 있다
- FR22: 사용자는 출금 네트워크를 선택할 수 있다
- FR23: 사용자는 출금 주소를 입력할 수 있다
- FR24: 사용자는 출금 수량을 입력할 수 있다
- FR25: 시스템은 출금 실행 전 확인 다이얼로그를 표시한다
- FR26: 사용자는 출금을 실행할 수 있다
- FR27: 시스템은 2FA 필요 시 안내 메시지를 표시한다

**콘솔 로그 (FR28-31):**
- FR28: 시스템은 모든 API 요청/응답을 콘솔에 기록한다
- FR29: 시스템은 에러 발생 시 타임스탬프와 함께 빨간색으로 표시한다
- FR30: 시스템은 성공 메시지를 구분하여 표시한다
- FR31: 사용자는 콘솔 로그를 스크롤하여 이전 기록을 볼 수 있다

**창 관리 (FR32-33):**
- FR32: 사용자는 모니터링 앱에서 WTS 창을 열 수 있다
- FR33: WTS 창은 모니터링 앱과 독립적으로 동작한다

**에러 처리 (FR34-36):**
- FR34: 시스템은 API 에러 코드별 명확한 메시지를 표시한다
- FR35: 시스템은 Rate Limit 초과 시 사용자에게 알린다
- FR36: 시스템은 네트워크 오류 시 사용자에게 알린다

### NonFunctional Requirements

**성능 (Performance):**
- NFR1: 주문 실행은 버튼 클릭 → API 호출 즉시 (내부 지연 없음)
- NFR2: 오더북 갱신은 WebSocket 메시지 수신 → UI 갱신 100ms 이내
- NFR3: 잔고 갱신은 거래 완료 후 1초 이내
- NFR4: 콘솔 로그는 이벤트 발생 → 표시 100ms 이내
- NFR5: UI 반응성은 사용자 인터랙션 → 피드백 200ms 이내
- NFR6: 배치 처리 금지 (모든 요청 개별 즉시 처리)

**보안 (Security):**
- NFR7: API 키는 .env 파일 기반 저장
- NFR8: API 키 전송은 HTTPS 통신만 사용
- NFR9: 출금 전 주소/수량 확인 다이얼로그 필수
- NFR10: 주문 전 확인 다이얼로그 필수
- NFR11: API 키 평문 로깅 금지

**통합 (Integration):**
- NFR12: 거래소 API는 REST + WebSocket 지원
- NFR13: 인증 방식은 거래소별 명세 준수 (JWT, HMAC-SHA256, ES256 등)
- NFR14: Rate Limit은 거래소별 호출 제한 준수
- NFR15: 에러 처리는 거래소별 에러 코드 파싱 및 표시
- NFR16: WebSocket 연결 끊김 시 자동 재연결

**안정성 (Reliability):**
- NFR17: 장시간 무중단 운영 목표
- NFR18: API 장애 감지 및 상태 표시
- NFR19: 일시적 네트워크 오류 시 재시도
- NFR20: 주문/출금 결과 데이터 정합성 보장

### Additional Requirements

**Architecture 요구사항:**

*프로젝트 컨텍스트:*
- Brownfield Extension - 기존 Tauri 2.0 데스크톱 앱 확장 (Starter Template 초기화 불필요)

*기술 스택 결정:*
- 상태 관리: Zustand (복잡한 WTS 상태, 선택적 리렌더링)
- 통신 아키텍처: 하이브리드 (WTS 직접 REST + 서버 WebSocket + myOrder WebSocket)
- UI 레이아웃: 고정 그리드 (CSS Grid 기반)
- 확인 다이얼로그: 커스텀 모달 (React)

*프로젝트 구조 요구사항:*
- 프론트엔드: `apps/desktop/src/wts/` 디렉토리 구조
- 백엔드: `apps/desktop/src-tauri/src/wts/` 모듈 구조
- Tauri 명령 접두사: `wts_*`
- 스토어 명명: `use{Domain}Store`

*Upbit API 제약 (MVP 대상):*
- Order Rate Limit: 8회/초
- Query Rate Limit: 30회/초
- Quotation Rate Limit: 10회/초 (IP)
- WebSocket Connect: 5회/초
- 입금 주소 생성: 비동기 (생성 직후 null 가능 → 재조회 필요)
- 출금 주소: 사전 등록 필수 (Upbit 웹에서)

*구현 패턴 요구사항:*
- 콘솔 로그: `ConsoleLogEntry` 형식 준수 (id, timestamp, level, category, message)
- 에러 처리: 콘솔 로깅 + Toast 알림 모두 수행
- 주문/출금: 확인 다이얼로그 필수 표시
- Tauri 이벤트: `wts:{category}:{action}` 형식

**UX Design 요구사항:**

*레이아웃 요구사항:*
- 콘솔 좌측 고정 + 3컬럼 레이아웃
- 최소 창 크기: 1280 x 720px
- 패널 리사이즈 가능 (드래그)
- 레이아웃 상태 localStorage 저장

*디자인 시스템:*
- shadcn/ui + Tailwind CSS 3.4
- 다크 테마 기본 (터미널 스타일)
- 폰트: JetBrains Mono (숫자/코드), Inter (UI)

*인터랙션 요구사항:*
- 호가 클릭 → 주문 폼 가격 자동 입력
- 수량 % 버튼: 25%, 50%, 75%, MAX
- 키보드 단축키: 1-6 (거래소 전환)
- 가격 변동 애니메이션: 300ms 플래시
- 잔고 변화 하이라이트: 2000ms

*접근성 요구사항:*
- WCAG AA 준수 (색상 대비 4.5:1 이상)
- 키보드 접근성: Tab 순서 논리적
- 색상만 의존 금지: 아이콘/텍스트 병행

*컴포넌트 요구사항:*
- Orderbook: depth bar 포함, 행 클릭 인터랙션
- OrderForm: 지정가/시장가 탭, 확인 다이얼로그
- Console: VS Code 스타일, 최대 1000개 로그
- ConfirmDialog: 주문/출금 정보 요약 표시

### FR Coverage Map

| FR | Epic | 설명 |
|----|------|------|
| FR1 | Epic 1 | 거래소 탭 선택 |
| FR2 | Epic 1 | 연결 상태 확인 |
| FR3 | Epic 1 | API 장애 상태 표시 |
| FR4 | Epic 2 | 자산별 잔고 조회 |
| FR5 | Epic 2 | 잔고 수동 갱신 |
| FR6 | Epic 2 | 거래 후 자동 갱신 |
| FR7 | Epic 2 | 실시간 오더북 표시 |
| FR8 | Epic 2 | 마켓 선택 |
| FR9 | Epic 2 | WebSocket 오더북 스트리밍 |
| FR10 | Epic 3 | 시장가 매수 |
| FR11 | Epic 3 | 시장가 매도 |
| FR12 | Epic 3 | 지정가 매수 |
| FR13 | Epic 3 | 지정가 매도 |
| FR14 | Epic 3 | 수량/가격 입력 |
| FR15 | Epic 3 | 주문 확인 다이얼로그 |
| FR16 | Epic 3 | 주문 결과 콘솔 표시 |
| FR17 | Epic 4 | 입금 자산 선택 |
| FR18 | Epic 4 | 입금 네트워크 선택 |
| FR19 | Epic 4 | 입금 주소 생성 |
| FR20 | Epic 4 | 입금 주소 복사 |
| FR21 | Epic 5 | 출금 자산 선택 |
| FR22 | Epic 5 | 출금 네트워크 선택 |
| FR23 | Epic 5 | 출금 주소 입력 |
| FR24 | Epic 5 | 출금 수량 입력 |
| FR25 | Epic 5 | 출금 확인 다이얼로그 |
| FR26 | Epic 5 | 출금 실행 |
| FR27 | Epic 5 | 2FA 안내 메시지 |
| FR28 | Epic 3 | API 요청/응답 콘솔 기록 |
| FR29 | Epic 3 | 에러 빨간색 표시 |
| FR30 | Epic 3 | 성공 메시지 구분 |
| FR31 | Epic 3 | 콘솔 스크롤 |
| FR32 | Epic 1 | WTS 창 열기 |
| FR33 | Epic 1 | 독립 창 동작 |
| FR34 | Epic 5 | API 에러 코드별 메시지 |
| FR35 | Epic 5 | Rate Limit 알림 |
| FR36 | Epic 5 | 네트워크 오류 알림 |

## Epic List

### Epic 1: WTS 기반 인프라 및 창 시스템
사용자가 모니터링 앱에서 독립적인 WTS 창을 열고, 거래소를 선택하며, 실시간 연결 상태를 확인할 수 있다.

**FRs covered:** FR1, FR2, FR3, FR32, FR33
**NFRs addressed:** NFR16, NFR17, NFR18

**User Outcome:**
- WTS 전용 창 열기/닫기
- 거래소 탭 전환 (Upbit MVP)
- 연결 상태 실시간 확인
- 콘솔 패널 기본 구조

**Implementation Notes:**
- Zustand 스토어 설정 (wtsStore, consoleStore)
- Tauri 다중 창 설정 (tauri.conf.json)
- 기본 6패널 그리드 레이아웃
- 콘솔 로그 기본 구조
- WTS Rust 모듈 구조 생성

---

### Epic 2: 잔고 조회 및 오더북 실시간 표시
사용자가 선택한 거래소의 잔고를 조회하고, 마켓을 선택하여 실시간 오더북을 확인할 수 있다.

**FRs covered:** FR4, FR5, FR6, FR7, FR8, FR9
**NFRs addressed:** NFR2, NFR3, NFR12, NFR14

**User Outcome:**
- 자산별 잔고 목록 확인
- 잔고 수동/자동 갱신
- 마켓(BTC/KRW 등) 선택
- 실시간 오더북(호가창) 표시
- 호가 클릭 → 가격 자동 입력 준비

**Implementation Notes:**
- BalancePanel 컴포넌트
- OrderbookPanel 컴포넌트 (depth bar)
- 기존 서버 WebSocket 연결
- wts_get_balance Tauri 명령
- balanceStore Zustand 스토어

---

### Epic 3: 주문 실행 시스템
사용자가 시장가/지정가 매수/매도 주문을 실행하고, 결과를 콘솔에서 확인할 수 있다.

**FRs covered:** FR10, FR11, FR12, FR13, FR14, FR15, FR16, FR28, FR29, FR30, FR31
**NFRs addressed:** NFR1, NFR4, NFR5, NFR6, NFR10, NFR11, NFR13, NFR14, NFR15, NFR19, NFR20

**User Outcome:**
- 시장가 매수/매도 주문 실행
- 지정가 매수/매도 주문 실행
- 수량/가격 입력 (%, MAX 버튼)
- 주문 확인 다이얼로그
- 콘솔에 주문 결과 즉시 표시
- 에러 시 명확한 메시지

**Implementation Notes:**
- OrderPanel 컴포넌트
- ConfirmDialog 컴포넌트
- ConsolePanel 완성
- wts_place_order Tauri 명령
- Upbit 주문 API 연동 (REST)
- orderStore Zustand 스토어

---

### Epic 4: 입금 기능
사용자가 자산과 네트워크를 선택하여 입금 주소를 생성하고 복사할 수 있다.

**FRs covered:** FR17, FR18, FR19, FR20
**NFRs addressed:** NFR12, NFR13, NFR14

**User Outcome:**
- 입금할 자산 선택
- 입금 네트워크 선택
- 입금 주소 생성/조회
- 주소 복사 기능

**Implementation Notes:**
- TransferPanel (입금 탭)
- wts_get_deposit_address, wts_generate_deposit_address 명령
- 비동기 주소 생성 처리 (재조회 로직)
- transferStore Zustand 스토어

---

### Epic 5: 출금 기능
사용자가 자산/네트워크/주소/수량을 입력하여 출금을 실행하고, 2FA 필요 시 안내를 받을 수 있다.

**FRs covered:** FR21, FR22, FR23, FR24, FR25, FR26, FR27, FR34, FR35, FR36
**NFRs addressed:** NFR7, NFR8, NFR9, NFR11, NFR13, NFR14, NFR15

**User Outcome:**
- 출금할 자산/네트워크 선택
- 출금 주소 입력
- 출금 수량 입력
- 출금 확인 다이얼로그
- 출금 실행
- 2FA 필요 시 안내 메시지
- API 에러/Rate Limit/네트워크 오류 처리

**Implementation Notes:**
- TransferPanel (출금 탭)
- ConfirmDialog (출금용)
- wts_withdraw, wts_get_withdraw_addresses 명령
- 에러 처리 강화 (errorHandler.ts, upbitErrors.ts)

---

## Epic 1: WTS 기반 인프라 및 창 시스템

사용자가 모니터링 앱에서 독립적인 WTS 창을 열고, 거래소를 선택하며, 실시간 연결 상태를 확인할 수 있다.

### Story 1.1: WTS 프로젝트 구조 및 Zustand 설정

As a **개발자**,
I want **WTS 전용 디렉토리 구조와 Zustand 상태 관리가 설정된 환경**,
So that **WTS 기능을 체계적으로 개발할 수 있다**.

**Acceptance Criteria:**

**Given** 기존 arbitrage-bot 프로젝트가 있을 때
**When** WTS 프로젝트 구조를 생성하면
**Then** `apps/desktop/src/wts/` 디렉토리 구조가 생성되어야 한다
**And** `apps/desktop/src-tauri/src/wts/` Rust 모듈 구조가 생성되어야 한다
**And** Zustand 패키지가 설치되고 기본 스토어(wtsStore, consoleStore)가 정의되어야 한다
**And** WTS 전용 TypeScript 타입 파일(types.ts)이 생성되어야 한다

---

### Story 1.2: Tauri 다중 창 설정 및 WTS 창 열기

As a **트레이더**,
I want **모니터링 앱에서 WTS 창을 별도로 열 수 있는 기능**,
So that **기존 모니터링을 유지하면서 거래 작업을 할 수 있다**.

**Acceptance Criteria:**

**Given** 모니터링 앱이 실행 중일 때
**When** "WTS 열기" 버튼을 클릭하면
**Then** 새로운 WTS 창이 독립적으로 열려야 한다
**And** WTS 창 최소 크기는 1280x720px이어야 한다
**And** tauri.conf.json에 WTS 창 설정이 추가되어야 한다
**And** WTS 창은 모니터링 앱과 독립적으로 닫을 수 있어야 한다

---

### Story 1.3: 6패널 그리드 레이아웃 구현

As a **트레이더**,
I want **Bloomberg 터미널 스타일의 6패널 레이아웃**,
So that **필요한 모든 정보를 한 화면에서 볼 수 있다**.

**Acceptance Criteria:**

**Given** WTS 창이 열렸을 때
**When** 화면이 렌더링되면
**Then** 콘솔(좌측), 오더북(중앙 상단), 잔고(중앙 하단), 주문(우측 상단), 미체결(우측 하단), 헤더(상단)가 배치되어야 한다
**And** CSS Grid 기반 고정 레이아웃이 적용되어야 한다
**And** 다크 테마(터미널 스타일)가 기본 적용되어야 한다
**And** shadcn/ui 컴포넌트 기반 스타일링이 적용되어야 한다

---

### Story 1.4: 거래소 탭 및 선택 기능

As a **트레이더**,
I want **상단 탭에서 거래소를 선택할 수 있는 기능**,
So that **원하는 거래소로 빠르게 전환할 수 있다**.

**Acceptance Criteria:**

**Given** WTS 창이 열렸을 때
**When** 거래소 탭을 클릭하면
**Then** 선택한 거래소가 활성화 상태로 표시되어야 한다
**And** wtsStore의 selectedExchange 상태가 업데이트되어야 한다
**And** MVP에서는 Upbit 탭만 활성화되고 나머지는 비활성화(Coming Soon)되어야 한다
**And** 키보드 단축키 1-6으로 거래소 전환이 가능해야 한다

---

### Story 1.5: 연결 상태 표시 및 API 장애 감지

As a **트레이더**,
I want **거래소 API 연결 상태를 실시간으로 확인할 수 있는 기능**,
So that **거래 전에 시스템 상태를 파악할 수 있다**.

**Acceptance Criteria:**

**Given** 거래소가 선택되어 있을 때
**When** API 연결 상태가 변경되면
**Then** 헤더에 연결 상태 인디케이터(녹색=연결됨, 빨강=끊김, 노랑=연결중)가 표시되어야 한다
**And** 연결 상태 변경 시 콘솔에 로그가 기록되어야 한다
**And** API 장애 감지 시 사용자에게 상태가 표시되어야 한다

---

### Story 1.6: 콘솔 패널 기본 구조

As a **트레이더**,
I want **VS Code 스타일의 콘솔 로그 패널**,
So that **시스템 이벤트와 거래 결과를 확인할 수 있다**.

**Acceptance Criteria:**

**Given** WTS 창이 열렸을 때
**When** 시스템 이벤트가 발생하면
**Then** 콘솔에 타임스탬프(HH:mm:ss.SSS)와 함께 로그가 표시되어야 한다
**And** 로그 레벨별 색상 구분(INFO=회색, SUCCESS=녹색, ERROR=빨강, WARN=노랑)이 되어야 한다
**And** 최대 1000개 로그를 FIFO 방식으로 관리해야 한다
**And** 스크롤하여 이전 로그를 볼 수 있어야 한다
**And** consoleStore에 ConsoleLogEntry 형식으로 저장되어야 한다

---

## Epic 2: 잔고 조회 및 오더북 실시간 표시

사용자가 선택한 거래소의 잔고를 조회하고, 마켓을 선택하여 실시간 오더북을 확인할 수 있다.

### Story 2.1: Upbit API 인증 및 잔고 조회 Rust 백엔드

As a **트레이더**,
I want **Upbit API를 통해 내 잔고를 조회하는 백엔드 기능**,
So that **보유 자산 현황을 확인할 수 있다**.

**Acceptance Criteria:**

**Given** Upbit API 키가 환경 변수에 설정되어 있을 때
**When** `wts_get_balance` Tauri 명령을 호출하면
**Then** JWT 토큰이 생성되어 Upbit API에 인증 요청이 전송되어야 한다
**And** 자산별 잔고(currency, balance, locked, avg_buy_price)가 반환되어야 한다
**And** API 에러 시 에러 코드와 메시지가 반환되어야 한다
**And** Rate Limit(30회/초)을 준수해야 한다

---

### Story 2.2: 잔고 패널 UI 및 상태 관리

As a **트레이더**,
I want **보유 자산 목록을 한눈에 볼 수 있는 잔고 패널**,
So that **거래 전 자산 상태를 파악할 수 있다**.

**Acceptance Criteria:**

**Given** WTS 창에서 거래소가 선택되어 있을 때
**When** 잔고 패널이 렌더링되면
**Then** 보유 자산별로 코인명, 수량, 평균 매수가, 평가금액이 표시되어야 한다
**And** 잔고가 0인 자산은 필터링 옵션으로 숨길 수 있어야 한다
**And** balanceStore에 잔고 데이터가 저장되어야 한다
**And** 잔고 변화 시 해당 행이 2000ms 하이라이트되어야 한다

---

### Story 2.3: 잔고 수동/자동 갱신

As a **트레이더**,
I want **잔고를 수동으로 새로고침하고 거래 후 자동 갱신되는 기능**,
So that **항상 최신 잔고 정보를 확인할 수 있다**.

**Acceptance Criteria:**

**Given** 잔고 패널이 표시되어 있을 때
**When** 새로고침 버튼을 클릭하면
**Then** 잔고가 즉시 갱신되어야 한다
**And** 갱신 중에는 로딩 인디케이터가 표시되어야 한다

**Given** 주문이 체결되었을 때
**When** 체결 이벤트가 수신되면
**Then** 잔고가 1초 이내에 자동 갱신되어야 한다
**And** 콘솔에 잔고 갱신 로그가 기록되어야 한다

---

### Story 2.4: 마켓 선택 기능

As a **트레이더**,
I want **거래할 마켓(BTC/KRW, ETH/KRW 등)을 선택하는 기능**,
So that **원하는 마켓의 호가와 주문을 관리할 수 있다**.

**Acceptance Criteria:**

**Given** WTS 창에서 거래소가 선택되어 있을 때
**When** 마켓 선택 드롭다운을 클릭하면
**Then** 사용 가능한 마켓 목록이 표시되어야 한다
**And** 마켓을 선택하면 wtsStore.selectedMarket이 업데이트되어야 한다
**And** 선택된 마켓에 따라 오더북과 주문 패널이 업데이트되어야 한다
**And** 콘솔에 마켓 변경 로그가 기록되어야 한다

---

### Story 2.5: 실시간 오더북 WebSocket 연동

As a **트레이더**,
I want **선택한 마켓의 실시간 호가창을 볼 수 있는 기능**,
So that **시장 상황을 실시간으로 파악할 수 있다**.

**Acceptance Criteria:**

**Given** 마켓이 선택되어 있을 때
**When** 오더북 WebSocket 연결이 설정되면
**Then** 매수/매도 호가 각 15단계가 실시간 표시되어야 한다
**And** 오더북 갱신은 WebSocket 수신 후 100ms 이내에 UI에 반영되어야 한다
**And** WebSocket 연결 끊김 시 자동 재연결이 시도되어야 한다
**And** 연결 상태가 콘솔에 로그로 기록되어야 한다

---

### Story 2.6: 오더북 패널 UI 및 호가 클릭 상호작용

As a **트레이더**,
I want **호가창에서 가격을 클릭하면 주문 폼에 자동 입력되는 기능**,
So that **빠르게 지정가 주문을 준비할 수 있다**.

**Acceptance Criteria:**

**Given** 오더북이 표시되어 있을 때
**When** 화면이 렌더링되면
**Then** depth bar가 수량 비율에 따라 표시되어야 한다
**And** 매수 호가는 녹색, 매도 호가는 빨간색으로 구분되어야 한다
**And** 가격 변동 시 300ms 플래시 애니메이션이 적용되어야 한다

**Given** 오더북 행이 클릭되었을 때
**When** 호가 가격을 클릭하면
**Then** 주문 폼의 가격 필드에 해당 가격이 자동 입력되어야 한다
**And** 지정가 모드가 자동 선택되어야 한다

---

## Epic 3: 주문 실행 시스템

사용자가 시장가/지정가 매수/매도 주문을 실행하고, 결과를 콘솔에서 확인할 수 있다.

### Story 3.1: 주문 API Rust 백엔드 구현

As a **트레이더**,
I want **Upbit 주문 API와 연동된 백엔드 기능**,
So that **매수/매도 주문을 실행할 수 있다**.

**Acceptance Criteria:**

**Given** Upbit API 키가 설정되어 있을 때
**When** `wts_place_order` Tauri 명령을 호출하면
**Then** 시장가/지정가, 매수/매도에 따른 적절한 API 파라미터가 전송되어야 한다
**And** JWT 토큰 생성 시 주문 정보 해시가 포함되어야 한다
**And** Rate Limit(8회/초)을 준수해야 한다
**And** 주문 결과(uuid, side, ord_type, price, state 등)가 반환되어야 한다
**And** API 평문 로깅이 금지되어야 한다 (키 정보 마스킹)

---

### Story 3.2: 주문 패널 UI (수량/가격 입력)

As a **트레이더**,
I want **수량과 가격을 입력하고 주문 유형을 선택하는 폼**,
So that **정확한 주문 조건을 설정할 수 있다**.

**Acceptance Criteria:**

**Given** 주문 패널이 표시되어 있을 때
**When** 화면이 렌더링되면
**Then** 지정가/시장가 탭이 표시되어야 한다
**And** 수량 입력 필드와 % 버튼(25%, 50%, 75%, MAX)이 표시되어야 한다
**And** 지정가 모드에서는 가격 입력 필드가 활성화되어야 한다
**And** 시장가 모드에서는 가격 입력 필드가 비활성화되어야 한다
**And** 입력값 변경 시 예상 총액이 계산되어 표시되어야 한다

---

### Story 3.3: 시장가 매수/매도 주문 실행

As a **트레이더**,
I want **시장가 매수/매도 주문을 즉시 실행하는 기능**,
So that **현재 시장 가격으로 빠르게 거래할 수 있다**.

**Acceptance Criteria:**

**Given** 시장가 모드가 선택되고 수량이 입력되어 있을 때
**When** 매수/매도 버튼을 클릭하면
**Then** 주문 확인 다이얼로그가 표시되어야 한다
**And** 다이얼로그에서 확인을 클릭하면 주문이 즉시 실행되어야 한다
**And** 주문 결과가 콘솔에 즉시 표시되어야 한다
**And** 성공 시 토스트 알림이 표시되어야 한다
**And** 버튼 클릭 → API 호출은 내부 지연 없이 즉시 수행되어야 한다

---

### Story 3.4: 지정가 매수/매도 주문 실행

As a **트레이더**,
I want **지정가 매수/매도 주문을 실행하는 기능**,
So that **원하는 가격에 주문을 걸어둘 수 있다**.

**Acceptance Criteria:**

**Given** 지정가 모드가 선택되고 가격과 수량이 입력되어 있을 때
**When** 매수/매도 버튼을 클릭하면
**Then** 주문 확인 다이얼로그가 표시되어야 한다
**And** 다이얼로그에 주문 유형, 마켓, 가격, 수량, 총액이 요약되어야 한다
**And** 확인 클릭 시 지정가 주문이 전송되어야 한다
**And** 주문 결과가 콘솔에 기록되어야 한다

---

### Story 3.5: 주문 확인 다이얼로그

As a **트레이더**,
I want **주문 실행 전 확인 다이얼로그**,
So that **실수로 잘못된 주문을 방지할 수 있다**.

**Acceptance Criteria:**

**Given** 주문 버튼이 클릭되었을 때
**When** 확인 다이얼로그가 표시되면
**Then** 마켓(예: BTC/KRW), 주문 유형(시장가/지정가), 방향(매수/매도)이 표시되어야 한다
**And** 수량, 가격(지정가), 예상 총액이 명확히 표시되어야 한다
**And** "확인"과 "취소" 버튼이 제공되어야 한다
**And** Enter 키로 확인, ESC 키로 취소가 가능해야 한다
**And** 매수는 녹색 계열, 매도는 빨간색 계열로 구분되어야 한다

---

### Story 3.6: 콘솔 로그 완성 (주문 결과 표시)

As a **트레이더**,
I want **모든 API 요청/응답이 콘솔에 기록되는 기능**,
So that **거래 이력과 오류를 추적할 수 있다**.

**Acceptance Criteria:**

**Given** API 호출이 발생할 때
**When** 요청/응답이 완료되면
**Then** 콘솔에 타임스탬프, 카테고리, 메시지가 표시되어야 한다
**And** 성공 메시지는 녹색(SUCCESS)으로 표시되어야 한다
**And** 에러 메시지는 빨간색(ERROR)으로 타임스탬프와 함께 표시되어야 한다
**And** 이벤트 발생 → 콘솔 표시는 100ms 이내여야 한다
**And** API 키 등 민감 정보는 마스킹되어야 한다

---

### Story 3.7: 주문 에러 처리 및 Rate Limit 알림

As a **트레이더**,
I want **주문 실패 시 명확한 에러 메시지와 Rate Limit 알림**,
So that **문제 원인을 파악하고 대응할 수 있다**.

**Acceptance Criteria:**

**Given** 주문 API 호출이 실패했을 때
**When** 에러 응답이 수신되면
**Then** Upbit 에러 코드에 따른 한국어 메시지가 표시되어야 한다
**And** 콘솔에 ERROR 레벨로 기록되어야 한다
**And** 토스트 알림이 표시되어야 한다

**Given** Rate Limit이 초과되었을 때
**When** 429 에러 또는 관련 에러 코드가 수신되면
**Then** "주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요." 메시지가 표시되어야 한다
**And** 사용자가 재시도 타이밍을 판단할 수 있어야 한다

---

## Epic 4: 입금 기능

사용자가 자산과 네트워크를 선택하여 입금 주소를 생성하고 복사할 수 있다.

### Story 4.1: 입금 API Rust 백엔드 구현

As a **트레이더**,
I want **Upbit 입금 주소 조회/생성 API와 연동된 백엔드**,
So that **입금 주소를 받아볼 수 있다**.

**Acceptance Criteria:**

**Given** Upbit API 키가 설정되어 있을 때
**When** `wts_get_deposit_address` 명령을 호출하면
**Then** 해당 자산/네트워크의 입금 주소가 반환되어야 한다
**And** 주소가 없는 경우(null) `wts_generate_deposit_address`로 생성 요청을 할 수 있어야 한다

**Given** 입금 주소 생성이 요청되었을 때
**When** 비동기 생성이 진행 중이면
**Then** 생성 중 상태가 반환되어야 한다
**And** 일정 시간 후 재조회 로직이 트리거되어야 한다

---

### Story 4.2: 입금 탭 UI (자산/네트워크 선택)

As a **트레이더**,
I want **입금할 자산과 네트워크를 선택하는 UI**,
So that **원하는 방식으로 입금을 준비할 수 있다**.

**Acceptance Criteria:**

**Given** Transfer 패널의 입금 탭이 선택되어 있을 때
**When** 화면이 렌더링되면
**Then** 자산 선택 드롭다운이 표시되어야 한다
**And** 자산 선택 후 해당 자산의 네트워크 목록이 표시되어야 한다
**And** 네트워크별 특징(수수료, 확인 시간 등)이 안내되어야 한다
**And** transferStore에 선택 상태가 저장되어야 한다

---

### Story 4.3: 입금 주소 표시 및 복사

As a **트레이더**,
I want **생성된 입금 주소를 복사하는 기능**,
So that **외부 지갑에서 쉽게 송금할 수 있다**.

**Acceptance Criteria:**

**Given** 자산과 네트워크가 선택되어 있을 때
**When** 입금 주소가 조회/생성되면
**Then** 입금 주소가 화면에 표시되어야 한다
**And** 주소 옆에 복사 버튼이 있어야 한다
**And** 복사 버튼 클릭 시 클립보드에 주소가 복사되어야 한다
**And** 복사 성공 시 토스트 알림이 표시되어야 한다
**And** 콘솔에 입금 주소 조회/생성 로그가 기록되어야 한다

---

### Story 4.4: 입금 주소 비동기 생성 처리

As a **트레이더**,
I want **입금 주소가 없을 때 자동으로 생성되고 결과를 받는 기능**,
So that **첫 입금도 원활하게 진행할 수 있다**.

**Acceptance Criteria:**

**Given** 선택한 자산/네트워크에 입금 주소가 없을 때
**When** "주소 생성" 버튼을 클릭하면
**Then** 생성 요청이 전송되고 로딩 상태가 표시되어야 한다
**And** 생성 완료 후 주소가 자동으로 표시되어야 한다
**And** 비동기 생성 중(null 반환)인 경우 3초 후 자동 재조회되어야 한다
**And** 최대 5회 재시도 후 실패 시 에러 메시지가 표시되어야 한다

---

## Epic 5: 출금 기능

사용자가 자산/네트워크/주소/수량을 입력하여 출금을 실행하고, 2FA 필요 시 안내를 받을 수 있다.

### Story 5.1: 출금 API Rust 백엔드 구현

As a **트레이더**,
I want **Upbit 출금 API와 연동된 백엔드 기능**,
So that **자산을 외부 지갑으로 출금할 수 있다**.

**Acceptance Criteria:**

**Given** Upbit API 키가 설정되어 있을 때
**When** `wts_withdraw` 명령을 호출하면
**Then** 출금 요청이 Upbit API로 전송되어야 한다
**And** 출금 결과(uuid, txid, state 등)가 반환되어야 한다
**And** 에러 시 에러 코드와 상세 메시지가 반환되어야 한다
**And** Rate Limit을 준수해야 한다

---

### Story 5.2: 출금 탭 UI (자산/네트워크/주소/수량 입력)

As a **트레이더**,
I want **출금에 필요한 모든 정보를 입력하는 폼**,
So that **출금 조건을 정확하게 설정할 수 있다**.

**Acceptance Criteria:**

**Given** Transfer 패널의 출금 탭이 선택되어 있을 때
**When** 화면이 렌더링되면
**Then** 자산 선택, 네트워크 선택, 출금 주소 입력, 수량 입력 필드가 표시되어야 한다
**And** 출금 가능 잔고가 표시되어야 한다
**And** 출금 수수료가 표시되어야 한다
**And** % 버튼(25%, 50%, 75%, MAX)으로 수량을 빠르게 설정할 수 있어야 한다
**And** 최소 출금 수량 안내가 표시되어야 한다

---

### Story 5.3: 출금 확인 다이얼로그

As a **트레이더**,
I want **출금 실행 전 상세 정보를 확인하는 다이얼로그**,
So that **실수로 잘못된 출금을 방지할 수 있다**.

**Acceptance Criteria:**

**Given** 출금 버튼이 클릭되었을 때
**When** 확인 다이얼로그가 표시되면
**Then** 출금 자산, 네트워크, 주소, 수량, 수수료, 실수령액이 명확히 표시되어야 한다
**And** 주소는 전체 표시되고 앞/뒤 일부가 강조되어야 한다
**And** "주소를 다시 확인하세요" 경고 문구가 표시되어야 한다
**And** "확인"과 "취소" 버튼이 제공되어야 한다
**And** 확인 버튼은 3초 후 활성화되어 실수를 방지해야 한다

---

### Story 5.4: 출금 실행 및 결과 처리

As a **트레이더**,
I want **출금을 실행하고 결과를 확인하는 기능**,
So that **출금 상태를 추적할 수 있다**.

**Acceptance Criteria:**

**Given** 출금 확인 다이얼로그에서 확인을 클릭했을 때
**When** 출금 API가 호출되면
**Then** 출금 요청이 전송되고 로딩 상태가 표시되어야 한다
**And** 성공 시 출금 UUID와 상태가 콘솔에 기록되어야 한다
**And** 성공 시 토스트 알림이 표시되어야 한다
**And** 잔고가 자동으로 갱신되어야 한다

---

### Story 5.5: 2FA 및 출금 제한 안내

As a **트레이더**,
I want **2FA 필요 시 명확한 안내 메시지**,
So that **출금 실패 원인을 알고 대응할 수 있다**.

**Acceptance Criteria:**

**Given** 출금 요청이 2FA를 요구할 때
**When** 관련 에러 코드가 반환되면
**Then** "Upbit 앱에서 2FA 인증이 필요합니다." 안내가 표시되어야 한다
**And** 콘솔에 WARN 레벨로 기록되어야 한다

**Given** 출금 주소가 사전 등록되지 않았을 때
**When** 관련 에러 코드가 반환되면
**Then** "출금 주소를 Upbit 웹에서 먼저 등록해주세요." 안내가 표시되어야 한다
**And** 등록 방법 안내 링크가 제공되어야 한다

---

### Story 5.6: 출금 에러 처리 및 네트워크 오류 대응

As a **트레이더**,
I want **출금 실패 시 명확한 에러 메시지와 네트워크 오류 안내**,
So that **문제를 파악하고 재시도할 수 있다**.

**Acceptance Criteria:**

**Given** 출금 API 호출이 실패했을 때
**When** 에러 응답이 수신되면
**Then** Upbit 에러 코드에 따른 한국어 메시지가 표시되어야 한다
**And** 콘솔에 ERROR 레벨로 기록되어야 한다
**And** 토스트 알림이 표시되어야 한다

**Given** 네트워크 오류가 발생했을 때
**When** 연결 실패 또는 타임아웃이 발생하면
**Then** "네트워크 연결을 확인하세요." 메시지가 표시되어야 한다
**And** 자동 재시도가 1회 수행되어야 한다
**And** 재시도 실패 시 사용자에게 수동 재시도 옵션이 제공되어야 한다

---


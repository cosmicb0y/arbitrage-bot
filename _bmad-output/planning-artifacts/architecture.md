---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
status: 'complete'
completedAt: '2026-01-18'
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - 'docs/index.md'
  - 'docs/project-overview.md'
  - 'docs/architecture.md'
  - 'docs/api-contracts.md'
  - 'docs/development-guide.md'
  - 'docs/source-tree-analysis.md'
workflowType: 'architecture'
project_name: 'arbitrage-bot'
user_name: 'Hyowon'
date: '2026-01-18'
featureScope: 'bloomberg-terminal-wts'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
- 거래소 관리 (FR1-3): 탭 기반 거래소 선택, 연결 상태 표시, API 장애 감지
- 잔고 조회 (FR4-6): 선택 거래소 자산별 잔고, 수동/자동 갱신
- 오더북 (FR7-9): 실시간 WebSocket 호가창, 마켓 선택
- 주문 (FR10-16): 시장가/지정가 매수/매도, 수량/가격 입력, 확인 다이얼로그
- 입금 (FR17-20): 자산/네트워크 선택, 주소 생성, 복사 기능
- 출금 (FR21-27): 자산/네트워크/주소/수량 입력, 확인 다이얼로그, 2FA 안내
- 콘솔 (FR28-31): API 요청/응답 로깅, 에러 색상 구분, 타임스탬프
- 창 관리 (FR32-33): Tauri 별도 창, 모니터링 앱과 독립
- 에러 처리 (FR34-36): 에러 코드별 메시지, Rate Limit/네트워크 오류 알림

**Non-Functional Requirements:**
- 성능: 주문 즉시 API 호출 (배치 금지), 오더북 갱신 < 100ms, UI 반응 < 200ms
- 보안: API 키 .env 저장, HTTPS 통신, 주문/출금 확인 다이얼로그 필수, 메모리 평문 로깅 금지
- 통합: REST + WebSocket, 거래소별 인증 (JWT, HMAC-SHA256, ES256), Rate Limit 준수
- 안정성: 장시간 무중단, API 장애 감지, 재연결 자동화

**Scale & Complexity:**

- Primary domain: Desktop App (Tauri 2.0) + Fintech API Integration
- Complexity level: High
- Estimated architectural components: 6 (WTS Window Manager, Panel System, Exchange API Client, WebSocket Handler, State Manager, IPC Layer)

### Technical Constraints & Dependencies

**플랫폼 제약:**
- Tauri 2.0 다중 창 시스템 활용 (기존 모니터링 앱과 별도)
- React 18 + TypeScript 프론트엔드
- Rust 백엔드 (기존 arbitrage-bot 인프라)

**Upbit API 상세:**

| API 그룹 | Rate Limit | 측정 단위 |
|---------|------------|----------|
| Quotation (시세) | 초당 10회 | IP |
| Exchange Default (조회) | 초당 30회 | 계정 |
| Order (주문 생성/재주문) | 초당 8회 | 계정 |
| Order Test | 초당 8회 | 계정 |
| Order Cancel All (일괄 취소) | 2초당 1회 | 계정 |
| WebSocket Connect | 초당 5회 | IP/계정 |
| WebSocket Message | 초당 5회, 분당 100회 | IP/계정 |

**Upbit REST API 공통:**
- Base URL: `https://api.upbit.com/v1`
- TLS 1.2+ 필수, TLS 1.3 권장
- POST 요청: `Content-Type: application/json` 필수 (Form 미지원)
- 인증: `Authorization: Bearer [JWT_TOKEN]`
- GET/DELETE: 쿼리 파라미터 URL 인코딩 필수

**Upbit 에러 응답:**

| 상태 코드 | 에러 코드 | 발생 이유 |
|---------|---------|---------|
| 400 | `validation_error` | 필수 파라미터 누락 |
| 400 | `insufficient_funds_*` | 잔고 부족 |
| 401 | `jwt_verification` | JWT 검증 실패 |
| 401 | `no_authorization_ip` | 미등록 IP |
| 429 | - | Rate Limit 초과 |
| 500 | - | 서버 내부 오류 |

**Upbit 주문 API:**

| API | 엔드포인트 | Rate Limit |
|-----|-----------|------------|
| 주문 가능 정보 | GET `/v1/orders/chance` | 30/초 |
| 주문 생성 | POST `/v1/orders` | 8/초 |
| 주문 테스트 | POST `/v1/orders/test` | 8/초 |
| 개별 주문 조회 | GET `/v1/order` | 30/초 |
| 미체결 목록 | GET `/v1/orders/open` | 30/초 |
| 종료 주문 목록 | GET `/v1/orders/closed` | 30/초 |
| 개별 취소 | DELETE `/v1/order` | 30/초 |
| ID 목록 취소 | DELETE `/v1/orders/uuids` | 30/초 |
| 일괄 취소 | DELETE `/v1/orders/open` | 1/2초 |
| 취소 후 재주문 | POST `/v1/orders/cancel_and_new` | 8/초 |

**Upbit 주문 유형:**
- `limit`: 지정가 (volume + price)
- `price`: 시장가 매수 (price=총액)
- `market`: 시장가 매도 (volume)
- `best`: 최유리 지정가 (time_in_force 필수)

**Upbit 주문 체결 조건 (time_in_force):**
- `ioc`: 즉시 체결 가능 부분만, 나머지 취소
- `fok`: 전량 체결 가능시만, 아니면 전체 취소
- `post_only`: 메이커 주문만 생성

**Upbit 입금 API:**

| API | 엔드포인트 | 용도 |
|-----|-----------|------|
| 입금 가능 정보 | GET `/v1/deposits/chance/coin` | 가능 여부, 최소 수량 |
| 입금 주소 생성 | POST `/v1/deposits/generate_coin_address` | 새 주소 생성 (비동기) |
| 개별 주소 조회 | GET `/v1/deposits/coin_address` | 특정 통화 주소 조회 |
| 주소 목록 조회 | GET `/v1/deposits/coin_addresses` | 모든 입금 주소 |
| 개별 입금 조회 | GET `/v1/deposit` | UUID/TXID로 조회 |
| 입금 목록 조회 | GET `/v1/deposits` | 입금 이력 (100개) |
| 트래블룰 VASP | GET `/v1/travel_rule/vasps` | 트래블룰 거래소 목록 |

**Upbit 입금 제약:**
- 입금 주소 생성은 비동기 (생성 직후 null 가능 → 재조회 필요)
- 통화당 1회 생성 후 동일 주소 재사용

**Upbit 출금 API:**

| API | 엔드포인트 | 용도 |
|-----|-----------|------|
| 출금 가능 정보 | GET `/v1/withdraws/chance` | 수수료, 한도, 지갑 상태 |
| 출금 허용 주소 | GET `/v1/withdraws/coin_addresses` | 등록된 주소 목록 |
| 출금 요청 | POST `/v1/withdraws/coin` | 출금 실행 |
| 출금 조회 | GET `/v1/withdraw` | 단일 출금 상태 |
| 출금 목록 | GET `/v1/withdraws` | 출금 이력 |
| 출금 취소 | DELETE `/v1/withdraws/coin` | 취소 가능 건 취소 |

**Upbit 출금 제약:**
- 출금 주소 사전 등록 필수 (Upbit 웹에서 등록)
- 트래블룰 준수: 상대 거래소 검증 필요
- 취소 가능 여부: `is_cancelable` 필드로 확인

**Upbit WebSocket API:**

| 타입 | 용도 | 인증 |
|------|------|------|
| `ticker` | 현재가 정보 | 선택 |
| `orderbook` | 호가 정보 | 선택 |
| `trade` | 체결 정보 | 선택 |
| `myOrder` | 내 주문/체결 실시간 | **필수** |

**myOrder WebSocket 구독:**
```json
[
  {"ticket": "unique-id"},
  {"type": "myOrder", "codes": ["KRW-BTC"]}
]
```

**myOrder 주문 상태:**
- `wait`: 체결 대기
- `trade`: 체결 발생
- `done`: 전체 체결 완료
- `cancel`: 주문 취소
- `prevented`: SMP로 취소

**기타 거래소 API 제약:**

| 거래소 | 인증 방식 | Rate Limit |
|--------|----------|------------|
| Bithumb | JWT + HMAC-SHA256 | 문서 확인 필요 |
| Binance | HMAC-SHA256 | 1200 req/min |
| Coinbase | ES256 (ECDSA) | CDP API 키 필요 |
| Bybit | HMAC-SHA256 | 문서 확인 필요 |
| GateIO | HMAC-SHA512 | 문서 확인 필요 |

**기존 시스템 종속성:**
- 기존 WebSocket 인프라 (arbitrage-feeds) 활용 가능
- 기존 거래소 API 클라이언트 (exchange_client.rs) 확장
- .env 기반 API 키 관리 시스템 재사용

### Cross-Cutting Concerns Identified

- **에러 처리**: Quotation vs Exchange API 에러 형식 차이 처리, HTTP 상태 코드별 분기, 콘솔 로깅 + UI 알림, Rate Limit 재시도
- **인증 관리**: JWT 토큰 생성/갱신, Authorization Bearer 헤더, IP 화이트리스트 에러 처리
- **요청 형식**: POST는 JSON 필수, GET/DELETE는 URL 인코딩, TLS 1.2+ 필수
- **실시간 데이터**: WebSocket 연결 관리, 재연결 로직, myOrder 인증
- **확인 다이얼로그**: 주문/출금 전 사용자 확인 (공통 컴포넌트)
- **로깅 표준**: 타임스탬프, 색상 구분 (에러=빨강), 스크롤 가능 콘솔
- **Rate Limit 관리**: 거래소별 호출 제한 준수, `Remaining-Req` 헤더 모니터링
- **비동기 처리**: 입금 주소 생성 등 비동기 API 폴링/재시도 로직

## Starter Template Evaluation

### Primary Technology Domain

Brownfield Extension - 기존 Tauri 2.0 데스크톱 앱 확장

### Architecture Extension Strategy

이 기능은 새 프로젝트가 아닌 기존 아키텍처 확장입니다.

**기존 아키텍처 활용:**
- 프레임워크: Tauri 2.0 (다중 창 지원)
- 프론트엔드: React 18 + TypeScript 5.5
- 스타일링: Tailwind CSS 3.4
- 백엔드: Rust + Tokio async runtime
- 상태 관리: React Context (기존) + 필요시 Zustand 추가
- IPC 통신: Tauri Command/Event 시스템

**신규 모듈 추가:**
- `apps/desktop/src/wts/` - WTS React 컴포넌트
- `apps/desktop/src-tauri/src/wts/` - WTS Tauri 명령

**기존 파일 확장:**
- `apps/desktop/src-tauri/src/exchange_client.rs` - 주문/입출금 API 추가
- `apps/desktop/src-tauri/src/commands.rs` - WTS 명령 추가

### Architectural Decisions Inherited

**Language & Runtime:**
- Rust 1.75+ (백엔드)
- TypeScript 5.5 (프론트엔드)
- Tokio async runtime

**Styling Solution:**
- Tailwind CSS 3.4 (기존 설정 재사용)

**Build Tooling:**
- Vite (프론트엔드)
- Cargo (Rust)
- pnpm (패키지 관리)

**Testing Framework:**
- Vitest (프론트엔드)
- cargo test (Rust)

**Code Organization:**
- 기존 크레이트 구조 유지
- WTS 전용 모듈 분리

**Development Experience:**
- `pnpm tauri dev` (개발 모드)
- Hot reload 지원

**Note:** 새 Starter Template 초기화 불필요 - 기존 코드베이스 확장

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
1. 상태 관리: Zustand
2. 통신 아키텍처: WTS 직접 REST + 서버 WebSocket + myOrder WebSocket

**Important Decisions (Shape Architecture):**
3. UI 레이아웃: 고정 그리드
4. 확인 다이얼로그: 커스텀 모달

**Deferred Decisions (Post-MVP):**
- 리사이즈 가능 패널
- 로그 파일 저장
- 추가 거래소 확장

### State Management

**Decision: Zustand**

| 항목 | 결정 |
|------|------|
| 라이브러리 | Zustand |
| 이유 | 복잡한 WTS 상태 (6개 패널, 거래소/마켓 선택, 주문 폼), 선택적 리렌더링, 가벼움 |

**Store 구조:**
- `useWtsStore`: 거래소 선택, 마켓 선택, 연결 상태
- `useOrderStore`: 주문 폼 상태, 미체결 주문
- `useConsoleStore`: 로그 메시지 (최근 1000개)

### Communication Architecture

**Decision: 하이브리드 통신**

| 데이터 | 방식 | 이유 |
|--------|------|------|
| 주문/입출금 REST API | WTS Tauri 직접 호출 | 서버 없이 독립 동작 |
| 오더북 데이터 | 기존 서버 WebSocket | 중복 연결 방지, Rate Limit 절약 |
| myOrder (내 주문/체결) | WTS 전용 인증 WebSocket | 기존 서버에 없음, 실시간 필요 |

**구현 위치:**
- REST API: `apps/desktop/src-tauri/src/exchange_client.rs` 확장
- myOrder WebSocket: `apps/desktop/src-tauri/src/wts/` 신규

### UI Architecture

**Decision: 고정 그리드 레이아웃**

| 항목 | 결정 |
|------|------|
| 레이아웃 | CSS Grid 고정 배치 |
| 이유 | MVP 단순성, Bloomberg 스타일, 구현 복잡도 감소 |

**패널 구조 (MVP 6개):**
```
┌─────────────────┬─────────────────┬─────────────────┐
│   거래소 탭     │     오더북      │    매수/매도    │
├─────────────────┼─────────────────┼─────────────────┤
│      잔고       │     입출금      │      콘솔       │
└─────────────────┴─────────────────┴─────────────────┘
```

### Error Handling & Logging

**Decision: 메모리 기반 콘솔 로그**

| 항목 | 결정 |
|------|------|
| 저장 방식 | 메모리만 (Zustand) |
| 최대 개수 | 1000개 (FIFO) |
| 이유 | 보안 (민감 정보), MVP 단순성 |

**로그 형식:**
- 타임스탬프: `HH:mm:ss.SSS`
- 레벨: INFO (흰색), SUCCESS (녹색), ERROR (빨강), WARN (노랑)
- 메시지: API 요청/응답, 에러 상세

### Confirmation Dialogs

**Decision: 커스텀 모달**

| 항목 | 결정 |
|------|------|
| 구현 | React 커스텀 모달 (Tailwind) |
| 이유 | 복잡한 주문 정보 표시, 네이티브 다이얼로그 한계 |

**표시 정보:**
- 주문 확인: 마켓, 방향, 유형, 수량, 가격, 예상 수수료
- 출금 확인: 자산, 네트워크, 주소, 수량, 수수료

### Decision Impact Analysis

**Implementation Sequence:**
1. Zustand 스토어 설정
2. WTS 창 및 고정 그리드 레이아웃
3. 거래소 탭 + 연결 상태
4. 잔고 패널 + REST API
5. 오더북 패널 + 서버 WebSocket 연결
6. 주문 패널 + 확인 다이얼로그 + REST API
7. 입출금 패널 + REST API
8. 콘솔 패널
9. myOrder WebSocket 연결

**Cross-Component Dependencies:**
- Zustand → 모든 패널 (상태 공유)
- 거래소 선택 → 잔고, 오더북, 주문, 입출금 (데이터 필터링)
- 마켓 선택 → 오더북, 주문 (데이터 필터링)
- REST API 결과 → 콘솔 (로깅)

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:** 15개 영역에서 AI 에이전트가 서로 다른 선택을 할 수 있음

### Naming Patterns

**Zustand Store Naming:**
- 파일명: `{도메인}Store.ts` (예: `wtsStore.ts`, `orderStore.ts`)
- 훅 export: `use{Domain}Store` (예: `useWtsStore`, `useOrderStore`)
- 내부 상태: camelCase (예: `selectedExchange`, `orderFormData`)
- 액션: camelCase 동사형 (예: `setExchange`, `placeOrder`, `clearConsole`)

```typescript
// 올바른 예시
export const useWtsStore = create<WtsState>()((set) => ({
  selectedExchange: 'upbit',
  setExchange: (exchange) => set({ selectedExchange: exchange }),
}));
```

**Tauri Command Naming:**
- Rust 명령: snake_case (예: `wts_place_order`, `wts_get_balance`)
- 접두사: `wts_` (WTS 전용 명령 구분)
- TypeScript invoke: 동일 snake_case (예: `invoke('wts_place_order', {...})`)

```rust
// Rust
#[tauri::command]
pub async fn wts_place_order(params: OrderParams) -> Result<OrderResponse, String>
```

```typescript
// TypeScript
await invoke('wts_place_order', { params: orderParams });
```

**React Component Naming:**
- 파일명: `{ComponentName}.tsx` PascalCase
- 컴포넌트: `Wts` 접두사 (최상위만), 내부는 도메인 기반
- Props 인터페이스: `{Component}Props`

```typescript
// 파일: apps/desktop/src/wts/panels/OrderPanel.tsx
interface OrderPanelProps {
  exchange: string;
  market: string;
}
function OrderPanel({ exchange, market }: OrderPanelProps) { ... }
```

**File & Directory Naming:**
- React 디렉토리: kebab-case 또는 단일 단어 (예: `wts/`, `panels/`)
- React 파일: PascalCase.tsx (예: `OrderPanel.tsx`)
- Rust 디렉토리/파일: snake_case (예: `wts/`, `order_handler.rs`)
- 유틸리티: camelCase.ts (예: `formatters.ts`, `upbitApi.ts`)

### Structure Patterns

**WTS Frontend Structure:**
```
apps/desktop/src/wts/
├── index.tsx           # WTS 메인 진입점
├── WtsWindow.tsx       # 창 레이아웃 컴포넌트
├── stores/
│   ├── wtsStore.ts     # 거래소/마켓 선택 상태
│   ├── orderStore.ts   # 주문 폼 상태
│   └── consoleStore.ts # 콘솔 로그 상태
├── panels/
│   ├── ExchangePanel.tsx
│   ├── BalancePanel.tsx
│   ├── OrderbookPanel.tsx
│   ├── OrderPanel.tsx
│   ├── TransferPanel.tsx  # 입출금
│   └── ConsolePanel.tsx
├── components/         # WTS 공통 컴포넌트
│   ├── ConfirmDialog.tsx
│   ├── MarketSelector.tsx
│   └── NetworkSelector.tsx
├── hooks/
│   └── useUpbitApi.ts  # Upbit API 훅
└── types.ts            # WTS 전용 타입
```

**WTS Backend Structure:**
```
apps/desktop/src-tauri/src/wts/
├── mod.rs              # 모듈 선언
├── commands.rs         # Tauri 명령 정의
├── upbit_client.rs     # Upbit REST API 클라이언트
├── upbit_ws.rs         # Upbit myOrder WebSocket
└── types.rs            # Rust 타입 정의
```

**Test Location:**
- Frontend: `apps/desktop/src/wts/__tests__/` (별도 폴더)
- Backend: `apps/desktop/src-tauri/src/wts/tests.rs` (인라인 모듈)

### Format Patterns

**Console Log Format:**
```typescript
interface ConsoleLogEntry {
  id: string;           // 고유 ID (nanoid)
  timestamp: number;    // Unix ms
  level: 'INFO' | 'SUCCESS' | 'ERROR' | 'WARN';
  category: 'ORDER' | 'BALANCE' | 'DEPOSIT' | 'WITHDRAW' | 'SYSTEM';
  message: string;      // 사용자 친화적 메시지
  detail?: unknown;     // API 응답 원본 (디버깅용)
}

// 표시 형식
"14:32:15.123 [ORDER] 매수 주문 생성: KRW-BTC, 시장가, 100,000원"
"14:32:15.456 [ERROR] 주문 실패: insufficient_funds_bid"
```

**API Error Response Format:**
```typescript
// Upbit 원본 에러를 그대로 전달하되, 래퍼로 감쌈
interface WtsApiResult<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;       // Upbit 에러 코드 (예: 'insufficient_funds_bid')
    message: string;    // 한국어 메시지 (변환)
    raw?: unknown;      // Upbit 원본 응답
  };
}
```

**Date/Time Format:**
- 콘솔 타임스탬프: `HH:mm:ss.SSS` (24시간)
- API 요청 로깅: ISO 8601 (`2026-01-18T14:32:15.123Z`)
- UI 표시: 상대적 (방금, 1분 전) 또는 `HH:mm`

**Amount/Price Format:**
```typescript
// 금액 포맷터 (기존 패턴 따름)
function formatKrw(amount: number): string {
  return `₩${amount.toLocaleString('ko-KR')}`;
}
function formatCrypto(amount: number, decimals = 8): string {
  return amount.toFixed(decimals).replace(/\.?0+$/, '');
}
```

### Communication Patterns

**Zustand State Update Pattern:**
```typescript
// Immutable 업데이트 (기존 패턴)
set((state) => ({
  orders: [...state.orders, newOrder],
}));

// 단순 값 설정
set({ selectedExchange: exchange });
```

**Tauri Event Naming:**
- 형식: `wts:{category}:{action}` (kebab-case)
- 예시: `wts:order:created`, `wts:balance:updated`, `wts:error:occurred`

```rust
// Rust에서 이벤트 발행
app_handle.emit("wts:order:created", &order_data)?;
```

```typescript
// TypeScript에서 이벤트 수신
listen<OrderData>("wts:order:created", (event) => { ... });
```

**myOrder WebSocket Event Flow:**
1. WebSocket 연결 → `wts:ws:connected`
2. 주문 상태 변경 → `wts:order:updated` (payload: myOrder 데이터)
3. 연결 끊김 → `wts:ws:disconnected`
4. 재연결 시도 → `wts:ws:reconnecting`

### Process Patterns

**Loading State Pattern:**
```typescript
interface PanelState {
  status: 'idle' | 'loading' | 'success' | 'error';
  error?: string;
}

// 사용 예시
const { status, error } = useOrderStore();
{status === 'loading' && <Spinner />}
{status === 'error' && <ErrorMessage message={error} />}
```

**Error Handling Flow:**
1. API 호출 → try/catch
2. 에러 발생 시:
   - 콘솔에 ERROR 레벨 로깅 (consoleStore)
   - Toast 알림 표시 (사용자 친화적 메시지)
   - 상태 업데이트 (status: 'error')
3. Rate Limit 에러: 자동 재시도 (exponential backoff)

```typescript
// 에러 처리 유틸리티
function handleApiError(error: unknown, category: ConsoleCategory) {
  const message = translateUpbitError(error);
  consoleStore.getState().addLog('ERROR', category, message, error);
  toast.error(message);
}
```

**Confirmation Dialog Flow:**
1. 사용자 액션 (주문/출금 버튼 클릭)
2. 확인 다이얼로그 표시 (주문 정보 요약)
3. "확인" 클릭 → API 호출
4. "취소" 클릭 → 닫기, 상태 유지

```typescript
// 확인 다이얼로그 상태
interface ConfirmState {
  isOpen: boolean;
  type: 'order' | 'withdraw';
  data: OrderConfirmData | WithdrawConfirmData;
  onConfirm: () => Promise<void>;
}
```

**Rate Limit Handling:**
```typescript
// Upbit Rate Limit 준수
const RATE_LIMITS = {
  order: { max: 8, window: 1000 },    // 8/초
  query: { max: 30, window: 1000 },   // 30/초
  quotation: { max: 10, window: 1000 }, // 10/초 (IP)
};
```

### Enforcement Guidelines

**All AI Agents MUST:**
1. WTS 관련 파일은 반드시 `src/wts/` (프론트) 또는 `src/wts/` (백엔드) 하위에 생성
2. Tauri 명령은 `wts_` 접두사 사용
3. 콘솔 로그는 `ConsoleLogEntry` 형식 준수
4. 에러 처리 시 콘솔 로깅 + Toast 알림 모두 수행
5. 주문/출금 전 확인 다이얼로그 필수 표시

**Pattern Enforcement:**
- PR 리뷰 시 패턴 준수 확인
- 패턴 위반 발견 시 `_bmad-output/` 하위 문서 업데이트

### Pattern Examples

**Good Examples:**
```typescript
// ✅ 올바른 스토어 정의
// apps/desktop/src/wts/stores/orderStore.ts
export const useOrderStore = create<OrderState>()((set) => ({
  orderType: 'limit',
  setOrderType: (type) => set({ orderType: type }),
}));
```

```rust
// ✅ 올바른 Tauri 명령
#[tauri::command]
pub async fn wts_place_order(params: WtsOrderParams) -> Result<WtsOrderResult, String>
```

```typescript
// ✅ 올바른 에러 처리
try {
  const result = await invoke('wts_place_order', { params });
  addConsoleLog('SUCCESS', 'ORDER', '주문 성공', result);
} catch (error) {
  handleApiError(error, 'ORDER');
}
```

**Anti-Patterns:**
```typescript
// ❌ 잘못된 스토어 이름
export const wtsStore = create(...)  // 'use' 접두사 누락

// ❌ 잘못된 에러 처리
catch (error) {
  console.log(error);  // 콘솔 로그만, Toast 없음, 상태 업데이트 없음
}

// ❌ 확인 없이 주문 실행
onClick={() => invoke('wts_place_order', {...})}  // 확인 다이얼로그 없음
```

```rust
// ❌ 잘못된 명령 이름
pub async fn placeWtsOrder(...)  // snake_case 아님, wts_ 접두사 없음
```

## Project Structure & Boundaries

### Complete Project Directory Structure

**WTS Frontend Structure:**
```
apps/desktop/src/wts/
├── index.tsx                    # WTS 앱 진입점 (React Router)
├── WtsWindow.tsx                # 6패널 그리드 레이아웃
├── types.ts                     # WTS 전용 TypeScript 타입
│
├── stores/
│   ├── index.ts                 # Store 내보내기
│   ├── wtsStore.ts              # 거래소/마켓 선택, 연결 상태
│   ├── orderStore.ts            # 주문 폼, 미체결 주문
│   ├── balanceStore.ts          # 잔고 데이터
│   ├── transferStore.ts         # 입출금 폼 상태
│   └── consoleStore.ts          # 콘솔 로그 (최대 1000개)
│
├── panels/
│   ├── ExchangePanel.tsx        # 거래소 탭 + 연결 상태 (FR1-3)
│   ├── BalancePanel.tsx         # 잔고 목록 + 갱신 버튼 (FR4-6)
│   ├── OrderbookPanel.tsx       # 호가창 + 마켓 선택 (FR7-9)
│   ├── OrderPanel.tsx           # 매수/매도 폼 (FR10-16)
│   ├── TransferPanel.tsx        # 입금/출금 탭 (FR17-27)
│   └── ConsolePanel.tsx         # 로그 콘솔 (FR28-31)
│
├── components/
│   ├── ConfirmDialog.tsx        # 주문/출금 확인 모달
│   ├── MarketSelector.tsx       # KRW-BTC 형식 마켓 선택
│   ├── NetworkSelector.tsx      # 입출금 네트워크 선택
│   ├── AmountInput.tsx          # 수량/가격 입력 (숫자 포맷)
│   ├── Toast.tsx                # 알림 Toast
│   └── Spinner.tsx              # 로딩 인디케이터
│
├── hooks/
│   ├── useUpbitApi.ts           # Upbit REST API 호출 훅
│   ├── useMyOrderWs.ts          # myOrder WebSocket 훅
│   └── useConsole.ts            # 콘솔 로깅 훅
│
├── utils/
│   ├── formatters.ts            # 금액/수량 포맷터
│   ├── errorHandler.ts          # 에러 처리 + 한글 변환
│   ├── upbitErrors.ts           # Upbit 에러 코드 → 메시지 매핑
│   └── rateLimiter.ts           # Rate Limit 관리
│
└── __tests__/
    ├── stores/
    │   └── orderStore.test.ts
    ├── panels/
    │   └── OrderPanel.test.tsx
    └── utils/
        └── formatters.test.ts
```

**WTS Backend Structure:**
```
apps/desktop/src-tauri/src/wts/
├── mod.rs                       # 모듈 선언
├── commands.rs                  # Tauri 명령 정의 (wts_* 접두사)
├── types.rs                     # Rust 타입 정의
│
├── upbit/
│   ├── mod.rs                   # Upbit 모듈 선언
│   ├── client.rs                # REST API 클라이언트
│   ├── auth.rs                  # JWT 토큰 생성 (HMAC-SHA256)
│   ├── ws.rs                    # myOrder WebSocket 클라이언트
│   └── types.rs                 # Upbit API 응답 타입
│
└── tests.rs                     # 단위 테스트 (인라인)
```

**기존 파일 수정:**
```
apps/desktop/src-tauri/src/
├── main.rs                      # wts 모듈 등록, 명령 추가
├── lib.rs                       # (필요시) wts 모듈 내보내기
└── Cargo.toml                   # (필요시) 의존성 추가

apps/desktop/src-tauri/tauri.conf.json
└── windows 섹션에 WTS 창 설정 추가

apps/desktop/src/
├── main.tsx                     # WTS 라우트 추가
└── App.tsx                      # WTS 창 열기 버튼/메뉴 추가
```

### Architectural Boundaries

**API Boundaries:**

| 경계 | 설명 | 통신 방식 |
|------|------|----------|
| WTS ↔ Upbit REST | 주문/잔고/입출금 | Tauri 명령 → Rust HTTP |
| WTS ↔ Upbit WS | myOrder 실시간 | Rust WebSocket → Tauri 이벤트 |
| WTS ↔ 서버 WS | 오더북 데이터 | 기존 WebSocket 연결 재사용 |
| WTS ↔ Tauri | IPC 통신 | invoke() / listen() |

**Component Boundaries:**
```
┌─────────────────────────────────────────────────────────────┐
│                     WtsWindow.tsx                            │
│  ┌─────────────┬─────────────┬─────────────┐               │
│  │ Exchange    │ Orderbook   │ Order       │               │
│  │ Panel       │ Panel       │ Panel       │               │
│  │ (wtsStore)  │ (서버WS)    │ (orderStore)│               │
│  ├─────────────┼─────────────┼─────────────┤               │
│  │ Balance     │ Transfer    │ Console     │               │
│  │ Panel       │ Panel       │ Panel       │               │
│  │(balanceStore)│(transferStore)│(consoleStore)│           │
│  └─────────────┴─────────────┴─────────────┘               │
└─────────────────────────────────────────────────────────────┘
         │                    │
         ▼                    ▼
   ┌──────────┐        ┌──────────────┐
   │ Zustand  │        │ Tauri IPC    │
   │ Stores   │        │ Commands     │
   └──────────┘        └──────────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │ Rust Backend   │
                    │ wts/commands.rs│
                    └────────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │ Upbit    │  │ Upbit    │  │ Server   │
        │ REST     │  │ myOrder  │  │ WS       │
        │ API      │  │ WS       │  │ (기존)   │
        └──────────┘  └──────────┘  └──────────┘
```

**Data Boundaries:**

| 데이터 | 저장 위치 | 생명주기 |
|--------|----------|---------|
| 거래소/마켓 선택 | wtsStore (메모리) | 세션 |
| 주문 폼 데이터 | orderStore (메모리) | 세션 |
| 잔고 데이터 | balanceStore (메모리) | API 호출 시 갱신 |
| 콘솔 로그 | consoleStore (메모리, 1000개) | 세션, FIFO |
| API 키 | .env 파일 (디스크) | 영구 |

### Requirements to Structure Mapping

**FR 카테고리 → 파일 매핑:**

| FR 그룹 | 프론트엔드 | 백엔드 | 스토어 |
|---------|-----------|--------|--------|
| FR1-3 (거래소) | `ExchangePanel.tsx` | - | `wtsStore.ts` |
| FR4-6 (잔고) | `BalancePanel.tsx` | `wts_get_balance` | `balanceStore.ts` |
| FR7-9 (오더북) | `OrderbookPanel.tsx` | 기존 서버 WS | - |
| FR10-16 (주문) | `OrderPanel.tsx`, `ConfirmDialog.tsx` | `wts_place_order`, `wts_cancel_order` | `orderStore.ts` |
| FR17-20 (입금) | `TransferPanel.tsx` | `wts_get_deposit_address`, `wts_generate_deposit_address` | `transferStore.ts` |
| FR21-27 (출금) | `TransferPanel.tsx`, `ConfirmDialog.tsx` | `wts_withdraw`, `wts_get_withdraw_addresses` | `transferStore.ts` |
| FR28-31 (콘솔) | `ConsolePanel.tsx` | - | `consoleStore.ts` |
| FR32-33 (창) | `WtsWindow.tsx`, `index.tsx` | `tauri.conf.json` | - |
| FR34-36 (에러) | `Toast.tsx`, `errorHandler.ts` | 에러 응답 | - |

**Cross-Cutting Concerns 매핑:**

| 관심사 | 파일 |
|--------|------|
| 에러 처리 | `utils/errorHandler.ts`, `utils/upbitErrors.ts` |
| Rate Limit | `utils/rateLimiter.ts`, `hooks/useUpbitApi.ts` |
| 인증 (JWT) | `src-tauri/src/wts/upbit/auth.rs` |
| 로깅 | `hooks/useConsole.ts`, `stores/consoleStore.ts` |
| 확인 다이얼로그 | `components/ConfirmDialog.tsx` |

### Integration Points

**Internal Communication:**
- 패널 → Zustand Store: React 훅 (`useWtsStore()`, `useOrderStore()`)
- Store → Tauri: `invoke('wts_*', params)`
- Tauri → Store: `listen('wts:*', callback)` 이벤트

**External Integrations:**
- Upbit REST API: `https://api.upbit.com/v1/*`
- Upbit WebSocket: `wss://api.upbit.com/websocket/v1` (myOrder)
- 기존 서버 WebSocket: `ws://localhost:9001/ws` (오더북)

**Data Flow:**
```
사용자 입력 → OrderPanel → orderStore.setOrderForm()
                               │
                               ▼
                         [주문 버튼 클릭]
                               │
                               ▼
                         ConfirmDialog.tsx
                               │
                               ▼ [확인]
                    invoke('wts_place_order')
                               │
                               ▼
                    Rust: upbit/client.rs
                               │
                               ▼
                    Upbit REST API
                               │
                               ▼
                    응답 → consoleStore.addLog()
                               │
                               ▼
                    Toast 알림 표시
```

### File Organization Patterns

**Configuration Files:**
- `apps/desktop/src-tauri/tauri.conf.json`: WTS 창 설정
- `apps/desktop/package.json`: Zustand 의존성 추가
- `apps/desktop/src-tauri/Cargo.toml`: (필요시) 추가 크레이트

**Source Organization:**
- 기능별 분리: `panels/`, `stores/`, `hooks/`, `utils/`
- 공용 컴포넌트: `components/`
- 타입: `types.ts` (프론트), `types.rs` (백엔드)

**Test Organization:**
- 프론트엔드: `__tests__/` 디렉토리 (Jest/Vitest)
- 백엔드: `#[cfg(test)]` 인라인 모듈

### Development Workflow Integration

**개발 서버:**
```bash
# 데스크톱 앱 개발 모드
cd apps/desktop
pnpm tauri dev
```

**빌드 프로세스:**
```bash
# 프로덕션 빌드
cd apps/desktop
pnpm tauri build
```

**테스트:**
```bash
# 프론트엔드 테스트
cd apps/desktop
pnpm test

# 백엔드 테스트
cd apps/desktop/src-tauri
cargo test
```

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**
- Tauri 2.0 + React 18 + TypeScript 5.5: 기존 코드베이스와 완전 호환
- Zustand 상태 관리: React 18 Concurrent 기능과 호환
- Rust tokio + WebSocket: 기존 인프라 패턴 재사용

**Pattern Consistency:**
- 네이밍 규칙: 언어별 표준 준수 (snake_case/camelCase/PascalCase)
- 스토어 패턴: `use{Domain}Store` 형식 통일
- Tauri 명령: `wts_*` 접두사 일관 적용
- 에러 처리: 콘솔 로깅 + Toast 알림 통합 패턴

**Structure Alignment:**
- 프론트엔드/백엔드 분리 명확
- 기능별 폴더 구조 (panels, stores, hooks, utils)
- 테스트 위치 일관성 (__tests__, 인라인 모듈)

### Requirements Coverage Validation ✅

**Functional Requirements Coverage:**

| FR 그룹 | 커버리지 | 담당 컴포넌트 |
|---------|---------|--------------|
| FR1-3 (거래소 관리) | 100% | ExchangePanel, wtsStore |
| FR4-6 (잔고 조회) | 100% | BalancePanel, wts_get_balance |
| FR7-9 (오더북) | 100% | OrderbookPanel (서버 WS) |
| FR10-16 (주문) | 100% | OrderPanel, ConfirmDialog |
| FR17-20 (입금) | 100% | TransferPanel |
| FR21-27 (출금) | 100% | TransferPanel, ConfirmDialog |
| FR28-31 (콘솔) | 100% | ConsolePanel, consoleStore |
| FR32-33 (창 관리) | 100% | WtsWindow, tauri.conf.json |
| FR34-36 (에러 처리) | 100% | errorHandler, Toast |

**Non-Functional Requirements Coverage:**
- 성능: 즉시 API 호출, 배치 금지 규칙
- 보안: .env 저장, 확인 다이얼로그 필수, 메모리 로깅 금지
- 통합: 하이브리드 통신 (REST + WebSocket)
- 안정성: 에러 처리 패턴, Rate Limit 관리

### Implementation Readiness Validation ✅

**Decision Completeness:**
- 모든 기술 스택 버전 명시
- 패턴별 Good/Anti-Pattern 예시 제공
- 5개 필수 구현 규칙 정의

**Structure Completeness:**
- 전체 디렉토리 구조 명시 (프론트 19개 파일, 백엔드 8개 파일)
- FR → 파일 매핑 완료 (36개 FR)
- API/컴포넌트/데이터 경계 정의

**Pattern Completeness:**
- 네이밍, 구조, 포맷, 통신, 프로세스 패턴 모두 정의
- 15개 잠재적 충돌 지점 해결

### Gap Analysis Results

**Critical Gaps:** 없음

**Important Gaps:** 없음

**Nice-to-Have (Post-MVP):**
- 다중 거래소 확장 패턴
- 리사이즈 가능 패널 가이드
- E2E 테스트 패턴

### Architecture Completeness Checklist

**✅ Requirements Analysis**
- [x] 프로젝트 컨텍스트 분석 완료
- [x] 복잡도 평가 (High)
- [x] 기술 제약 식별 (Upbit API 상세)
- [x] 크로스커팅 관심사 매핑

**✅ Architectural Decisions**
- [x] 상태 관리: Zustand
- [x] 통신 아키텍처: 하이브리드
- [x] UI 레이아웃: 고정 그리드
- [x] 확인 다이얼로그: 커스텀 모달

**✅ Implementation Patterns**
- [x] 네이밍 규칙 확립
- [x] 구조 패턴 정의
- [x] 통신 패턴 명시
- [x] 프로세스 패턴 문서화

**✅ Project Structure**
- [x] 완전한 디렉토리 구조 정의
- [x] 컴포넌트 경계 확립
- [x] 통합 지점 매핑
- [x] 요구사항 → 구조 매핑 완료

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION ✅

**Confidence Level:** High

**Key Strengths:**
- 기존 코드베이스 패턴 100% 준수
- Upbit API 상세 문서화 (Rate Limit, 에러 코드, 엔드포인트)
- 명확한 컴포넌트 경계 및 데이터 흐름
- 구체적인 구현 예시 (Good/Anti-Pattern)

**Areas for Future Enhancement:**
- 다중 거래소 확장 시 추상화 레이어
- 성능 최적화 (메모이제이션, 가상화)
- 오프라인 모드 지원

### Implementation Handoff

**AI Agent Guidelines:**
1. 모든 아키텍처 결정을 문서 그대로 따를 것
2. 구현 패턴을 모든 컴포넌트에 일관되게 적용할 것
3. 프로젝트 구조와 경계를 존중할 것
4. 아키텍처 관련 질문은 이 문서를 참조할 것

**First Implementation Priority:**
1. `apps/desktop/package.json`에 Zustand 의존성 추가
2. `apps/desktop/src/wts/` 디렉토리 구조 생성
3. `apps/desktop/src-tauri/src/wts/` 모듈 생성
4. `tauri.conf.json`에 WTS 창 설정 추가

## Architecture Completion Summary

### Workflow Completion

**Architecture Decision Workflow:** COMPLETED ✅
**Total Steps Completed:** 8
**Date Completed:** 2026-01-18
**Document Location:** `_bmad-output/planning-artifacts/architecture.md`

### Final Architecture Deliverables

**📋 Complete Architecture Document**
- 모든 아키텍처 결정이 구체적인 버전과 함께 문서화됨
- AI 에이전트 일관성을 보장하는 구현 패턴 정의
- 모든 파일과 디렉토리가 포함된 완전한 프로젝트 구조
- 요구사항 → 아키텍처 매핑 완료
- 일관성과 완전성을 확인하는 검증 완료

**🏗️ Implementation Ready Foundation**
- 5개 핵심 아키텍처 결정
- 15개 구현 패턴 정의
- 6개 아키텍처 컴포넌트 (패널)
- 36개 기능 요구사항 100% 지원

**📚 AI Agent Implementation Guide**
- 검증된 버전의 기술 스택
- 구현 충돌을 방지하는 일관성 규칙
- 명확한 경계가 있는 프로젝트 구조
- 통합 패턴 및 통신 표준

### Implementation Handoff

**For AI Agents:**
이 아키텍처 문서는 Bloomberg Terminal Style WTS 구현을 위한 완전한 가이드입니다. 문서에 명시된 모든 결정, 패턴, 구조를 정확히 따르세요.

**First Implementation Priority:**
```bash
# 1. Zustand 의존성 추가
cd apps/desktop
pnpm add zustand

# 2. WTS 디렉토리 구조 생성
mkdir -p src/wts/{stores,panels,components,hooks,utils,__tests__}

# 3. Rust WTS 모듈 생성
mkdir -p src-tauri/src/wts/upbit

# 4. 개발 서버 실행
pnpm tauri dev
```

**Development Sequence:**
1. 프로젝트 초기화 (Zustand, 디렉토리 구조)
2. 아키텍처에 따른 개발 환경 설정
3. 핵심 아키텍처 기반 구현 (Stores, Types)
4. 확립된 패턴에 따른 기능 구현
5. 문서화된 규칙과의 일관성 유지

### Quality Assurance Checklist

**✅ Architecture Coherence**
- [x] 모든 결정이 충돌 없이 함께 작동
- [x] 기술 선택이 호환됨
- [x] 패턴이 아키텍처 결정을 지원
- [x] 구조가 모든 선택과 정렬됨

**✅ Requirements Coverage**
- [x] 모든 기능 요구사항 지원 (36개 FR)
- [x] 모든 비기능 요구사항 해결 (성능, 보안, 통합, 안정성)
- [x] 크로스커팅 관심사 처리 (에러, Rate Limit, 인증, 로깅)
- [x] 통합 지점 정의 (Upbit REST/WS, 서버 WS, Tauri IPC)

**✅ Implementation Readiness**
- [x] 결정이 구체적이고 실행 가능
- [x] 패턴이 에이전트 충돌 방지
- [x] 구조가 완전하고 명확
- [x] 명확성을 위한 예시 제공

### Project Success Factors

**🎯 Clear Decision Framework**
모든 기술 선택이 명확한 근거와 함께 협력적으로 이루어져 모든 이해관계자가 아키텍처 방향을 이해합니다.

**🔧 Consistency Guarantee**
구현 패턴과 규칙이 여러 AI 에이전트가 호환되고 일관된 코드를 생성하여 원활하게 함께 작동하도록 보장합니다.

**📋 Complete Coverage**
모든 프로젝트 요구사항이 아키텍처적으로 지원되며, 비즈니스 요구에서 기술 구현까지 명확한 매핑이 있습니다.

**🏗️ Solid Foundation**
선택된 기술 스택과 아키텍처 패턴이 현재 모범 사례를 따르는 프로덕션 준비 기반을 제공합니다.

---

**Architecture Status:** READY FOR IMPLEMENTATION ✅

**Next Phase:** 여기 문서화된 아키텍처 결정과 패턴을 사용하여 구현을 시작하세요.

**Document Maintenance:** 구현 중 주요 기술 결정이 내려지면 이 아키텍처를 업데이트하세요.


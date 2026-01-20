# Story WTS-3.3: 시장가 매수/매도 주문 실행

Status: done

## Story

As a **트레이더**,
I want **시장가 매수/매도 주문을 즉시 실행하는 기능**,
So that **현재 시장 가격으로 빠르게 거래할 수 있다**.

## Acceptance Criteria

1. **Given** 시장가 모드가 선택되고 수량이 입력되어 있을 때 **When** 매수/매도 버튼을 클릭하면 **Then** 주문 확인 다이얼로그가 표시되어야 한다
2. **Given** 확인 다이얼로그가 표시되어 있을 때 **When** 확인을 클릭하면 **Then** 주문이 즉시 실행되어야 한다 (버튼 클릭 → API 호출 내부 지연 없음)
3. **Given** 주문이 실행되었을 때 **When** API 응답이 수신되면 **Then** 주문 결과가 콘솔에 즉시 표시되어야 한다
4. **Given** 주문이 성공했을 때 **When** 체결되면 **Then** 토스트 알림이 표시되어야 한다
5. **Given** 시장가 매수일 때 **When** 주문을 실행하면 **Then** Upbit API에 `ord_type: 'price'`, `side: 'bid'`, `price: 총액(KRW)` 파라미터가 전송되어야 한다
6. **Given** 시장가 매도일 때 **When** 주문을 실행하면 **Then** Upbit API에 `ord_type: 'market'`, `side: 'ask'`, `volume: 수량` 파라미터가 전송되어야 한다
7. **Given** 주문 버튼이 클릭되었을 때 **When** API 호출 중이면 **Then** 버튼이 로딩 상태로 변경되고 중복 클릭이 방지되어야 한다
8. **Given** 주문이 실패했을 때 **When** 에러 응답이 수신되면 **Then** 콘솔에 ERROR 레벨로 기록되고 토스트에 한국어 에러 메시지가 표시되어야 한다

## Tasks / Subtasks

- [x] Task 1: 확인 다이얼로그 컴포넌트 구현 (AC: #1)
  - [x] Subtask 1.1: ConfirmDialog.tsx 컴포넌트 생성 (apps/desktop/src/wts/components/)
  - [x] Subtask 1.2: 다이얼로그 props 정의 (isOpen, onConfirm, onCancel, orderInfo)
  - [x] Subtask 1.3: 주문 정보 요약 표시 UI (마켓, 방향, 유형, 수량/금액)
  - [x] Subtask 1.4: 확인/취소 버튼 (Enter=확인, Esc=취소)
  - [x] Subtask 1.5: 매수는 녹색 계열, 매도는 빨간색 계열 스타일링

- [x] Task 2: 주문 실행 로직 구현 (AC: #2, #5, #6, #7)
  - [x] Subtask 2.1: OrderPanel에 placeOrder 함수 추가
  - [x] Subtask 2.2: 시장가 매수 파라미터 빌드 (ord_type='price', side='bid', price=총액)
  - [x] Subtask 2.3: 시장가 매도 파라미터 빌드 (ord_type='market', side='ask', volume=수량)
  - [x] Subtask 2.4: toUpbitSide, toUpbitOrderType 헬퍼 함수 활용
  - [x] Subtask 2.5: invoke('wts_place_order', params) Tauri 명령 호출
  - [x] Subtask 2.6: isSubmitting 상태 관리 (버튼 로딩, 중복 방지)

- [x] Task 3: 콘솔 로깅 통합 (AC: #3, #8)
  - [x] Subtask 3.1: 주문 요청 시 INFO 로그 (예: "시장가 매수 주문 요청: KRW-BTC, 100,000원")
  - [x] Subtask 3.2: 주문 성공 시 SUCCESS 로그 (예: "주문 체결: 매수 0.002 BTC @ 시장가")
  - [x] Subtask 3.3: 주문 실패 시 ERROR 로그 (예: "주문 실패: 매수 가능 금액이 부족합니다")
  - [x] Subtask 3.4: useConsoleStore.addLog() 활용

- [x] Task 4: 토스트 알림 구현 (AC: #4, #8)
  - [x] Subtask 4.1: 토스트 표시 유틸리티 또는 컴포넌트 확인/구현
  - [x] Subtask 4.2: 성공 시 녹색 토스트 ("주문이 체결되었습니다")
  - [x] Subtask 4.3: 실패 시 빨간색 토스트 (한국어 에러 메시지)
  - [x] Subtask 4.4: getOrderErrorMessage() 활용하여 에러 코드 → 한국어 변환

- [x] Task 5: OrderPanel UI 통합 (AC: #1, #7)
  - [x] Subtask 5.1: 매수/매도 버튼 onClick에 다이얼로그 표시 로직 연결
  - [x] Subtask 5.2: 로딩 상태 시 버튼 비활성화 + 스피너 표시
  - [x] Subtask 5.3: ConfirmDialog 컴포넌트 렌더링 추가
  - [x] Subtask 5.4: 시장가 모드 전용 입력값 검증 (매수: price > 0, 매도: quantity > 0)

- [x] Task 6: 잔고 자동 갱신 (AC: #4 확장)
  - [x] Subtask 6.1: 주문 성공 후 balanceStore.refresh() 호출
  - [x] Subtask 6.2: 1초 이내 잔고 갱신 보장

- [x] Task 7: 단위 테스트 작성 (AC: #1-#8)
  - [x] Subtask 7.1: ConfirmDialog 렌더링 및 버튼 동작 테스트
  - [x] Subtask 7.2: 시장가 매수 파라미터 빌드 테스트
  - [x] Subtask 7.3: 시장가 매도 파라미터 빌드 테스트
  - [x] Subtask 7.4: 콘솔 로그 추가 테스트
  - [x] Subtask 7.5: 로딩 상태 및 중복 클릭 방지 테스트

## Dev Notes

### 시장가 주문 Upbit API 스펙

[Source: architecture.md#Upbit 주문 유형]

| UI 상태 | ord_type | side | 필수 파라미터 | 설명 |
|---------|----------|------|--------------|------|
| 시장가 + 매수 | `price` | `bid` | market, price(총액 KRW) | **volume 없음** |
| 시장가 + 매도 | `market` | `ask` | market, volume(수량) | **price 없음** |

**중요:** 시장가 매수는 수량이 아닌 **KRW 총액**을 price 파라미터로 전달해야 함.

### 기존 코드 활용

**OrderPanel.tsx 현황:**
```typescript
// apps/desktop/src/wts/panels/OrderPanel.tsx
// 이미 구현된 것:
// - orderType ('market' | 'limit'), side ('buy' | 'sell')
// - price, quantity 상태
// - 시장가 모드 시 가격 필드 비활성화
// - % 버튼으로 수량 계산
// - 예상 총액 표시
// - 매수/매도 버튼 UI
```

**types.ts 헬퍼 함수:**
```typescript
// apps/desktop/src/wts/types.ts
import { toUpbitSide, toUpbitOrderType, getOrderErrorMessage } from '../types';

// UI → Upbit API 변환
const upbitSide = toUpbitSide(side);       // 'buy' → 'bid', 'sell' → 'ask'
const upbitType = toUpbitOrderType(orderType, side);
// 시장가 매수: 'price', 시장가 매도: 'market', 지정가: 'limit'
```

**consoleStore 사용법:**
```typescript
// apps/desktop/src/wts/stores/consoleStore.ts
import { useConsoleStore } from '../stores/consoleStore';

// 로그 추가
useConsoleStore.getState().addLog('INFO', 'ORDER', '시장가 매수 주문 요청: KRW-BTC, ₩100,000');
useConsoleStore.getState().addLog('SUCCESS', 'ORDER', '주문 체결: 매수 0.002 BTC @ 시장가');
useConsoleStore.getState().addLog('ERROR', 'ORDER', '주문 실패: 매수 가능 금액이 부족합니다');
```

### 주문 파라미터 빌드 예시

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { OrderParams, OrderResponse, WtsApiResult } from '../types';

// 시장가 매수: KRW 총액으로 구매
const buildMarketBuyParams = (market: string, krwAmount: string): OrderParams => ({
  market,
  side: 'bid',
  ord_type: 'price',
  price: krwAmount,  // 예: "100000" (KRW 총액)
  // volume 없음
});

// 시장가 매도: 코인 수량 매도
const buildMarketSellParams = (market: string, volume: string): OrderParams => ({
  market,
  side: 'ask',
  ord_type: 'market',
  volume,  // 예: "0.001" (코인 수량)
  // price 없음
});

// Tauri 명령 호출
const result = await invoke<WtsApiResult<OrderResponse>>('wts_place_order', { params });
if (result.success && result.data) {
  // 성공 처리
} else if (result.error) {
  // 에러 처리: result.error.code, result.error.message
}
```

### ConfirmDialog 컴포넌트 설계

[Source: ux-design-specification.md#Modal & Dialog Patterns]

```typescript
// apps/desktop/src/wts/components/ConfirmDialog.tsx
interface OrderConfirmInfo {
  market: string;       // "KRW-BTC"
  side: 'buy' | 'sell';
  orderType: 'market' | 'limit';
  quantity?: string;    // 시장가 매도, 지정가
  price?: string;       // 시장가 매수 (총액), 지정가 (단가)
  total?: number;       // 예상 총액 (지정가)
}

interface ConfirmDialogProps {
  isOpen: boolean;
  orderInfo: OrderConfirmInfo;
  onConfirm: () => void;
  onCancel: () => void;
  isLoading?: boolean;
}
```

**다이얼로그 UI 예시:**
```
┌─────────────────────────────────┐
│         주문 확인               │
├─────────────────────────────────┤
│                                 │
│  마켓: KRW-BTC                  │
│  유형: 시장가 매수              │
│  주문금액: ₩100,000             │
│                                 │
│  ⚠️ 시장가 주문은 즉시 체결됩니다 │
│                                 │
├─────────────────────────────────┤
│     [취소]         [매수]       │  ← 매수: 녹색, 매도: 빨간색
└─────────────────────────────────┘
```

### 토스트 알림 패턴

기존 프로젝트에 토스트가 없다면 간단한 구현 필요:

```typescript
// 간단한 토스트 (window.alert 대체 또는 별도 컴포넌트)
// 옵션 1: 기존 shadcn/ui toast 사용 (설치 필요)
// 옵션 2: consoleStore 로그 + 일시적 UI 표시

// 최소 구현: 콘솔 로그가 주 피드백, 토스트는 보조
```

### 에러 처리 패턴

```typescript
import { getOrderErrorMessage } from '../types';

// 에러 응답 처리
if (result.error) {
  const errorMsg = getOrderErrorMessage(result.error.code, result.error.message);

  // 콘솔에 ERROR 로그
  useConsoleStore.getState().addLog('ERROR', 'ORDER', `주문 실패: ${errorMsg}`);

  // 토스트 표시
  showToast({ type: 'error', message: errorMsg });
}
```

### 시장가 매수 입력값 처리

**현재 OrderPanel 문제점:**
- 시장가 모드에서 `price` 필드가 비활성화됨
- 시장가 매수 시에는 KRW 총액 입력이 필요함

**해결 방안:**
```typescript
// OrderPanel.tsx 수정
// 시장가 매수: price 필드를 "주문 금액 (KRW)"으로 사용
// 시장가 매도: quantity 필드만 사용, price 비활성화

const isPriceDisabled = isMarket && side === 'sell';  // 시장가 매도만 비활성화
const priceLabel = isMarket && side === 'buy' ? '주문 금액' : '가격';
```

### Project Structure Notes

**수정 파일:**
- `apps/desktop/src/wts/panels/OrderPanel.tsx` - 주문 실행 로직, 다이얼로그 연동
- `apps/desktop/src/wts/__tests__/panels/OrderPanel.test.tsx` - 테스트 확장

**신규 파일:**
- `apps/desktop/src/wts/components/ConfirmDialog.tsx` - 주문 확인 다이얼로그
- `apps/desktop/src/wts/__tests__/components/ConfirmDialog.test.tsx` - 다이얼로그 테스트

**아키텍처 정합성:**
- Tauri 명령 `wts_place_order` 사용 (WTS-3.1에서 구현됨)
- WtsApiResult<OrderResponse> 응답 패턴 준수
- consoleStore 로깅 패턴 준수
- shadcn/ui + Tailwind CSS 스타일링

### 이전 스토리 학습사항

**WTS-3.1 (Order API Backend):**
- `wts_place_order` Tauri 명령 구현 완료
- OrderParams, OrderResponse 타입 정의됨
- Rate Limit 8회/초 클라이언트 스로틀링 구현됨
- 에러 코드 한국어 메시지 매핑 완료

**WTS-3.2 (Order Panel UI):**
- 주문 유형 탭 (지정가/시장가) 구현됨
- 가격/수량 입력 필드 구현됨
- % 버튼 (25%, 50%, 75%, MAX) 구현됨
- 예상 총액 계산 및 표시 구현됨
- 매수/매도 버튼 UI 구현됨 (아직 클릭 시 동작 없음)

### References

- [Architecture: Upbit 주문 API](/_bmad-output/planning-artifacts/architecture.md#Upbit 주문 API)
- [UX Design: ConfirmDialog](/_bmad-output/planning-artifacts/ux-design-specification.md#Modal & Dialog Patterns)
- [UX Design: OrderForm](/_bmad-output/planning-artifacts/ux-design-specification.md#OrderForm)
- [WTS Epics: Story 3.3](/_bmad-output/planning-artifacts/wts-epics.md#Story 3.3)
- [Previous Story: WTS-3.1](/_bmad-output/implementation-artifacts/wts-3-1-order-api-rust-backend.md)
- [Previous Story: WTS-3.2](/_bmad-output/implementation-artifacts/wts-3-2-order-panel-ui-qty-price-input.md)

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

- Task 1 완료: ConfirmDialog 컴포넌트 구현 (22개 테스트 통과)
  - OrderConfirmInfo 인터페이스: market, side, orderType, quantity, price, total
  - 키보드 지원: Enter=확인, Escape=취소
  - 매수 녹색(bg-green-600), 매도 빨간색(bg-red-600) 스타일링
  - 시장가 주문 경고 메시지 표시
  - isLoading 상태에서 버튼 비활성화 및 "처리중..." 표시

- Task 2-6 완료: 주문 실행 로직 통합
  - handleConfirmOrder: 시장가 매수(price 파라미터), 시장가 매도(volume 파라미터), 지정가 주문 지원
  - toUpbitSide, toUpbitOrderType 헬퍼 함수 활용
  - isSubmitting 상태로 중복 클릭 방지
  - invoke('wts_place_order', { params }) Tauri 명령 호출
  - 콘솔 로그: INFO(요청), SUCCESS(체결), ERROR(실패)
  - 토스트 알림: 성공(녹색), 실패(빨간색)
  - 주문 성공 후 500ms 후 잔고 자동 갱신

- Task 7 완료: 단위 테스트 작성
  - ConfirmDialog: 22개 테스트
  - OrderPanel 추가: 10개 테스트 (주문 제출 버튼, 다이얼로그 표시)
  - toastStore: 7개 테스트
  - 전체 WTS 테스트: 350개 통과

- 코드 리뷰 수정사항:
  - 시장가 매수 입력 필드 활성화 및 총액 계산 수정
  - ConfirmDialog에 지정가 상세(가격/수량/총액) 표시 추가
  - 한국어 에러 메시지 기본값 강화
  - 주문 파라미터 빌드 테스트 추가

### File List

- apps/desktop/src/wts/components/ConfirmDialog.tsx (신규)
- apps/desktop/src/wts/components/ToastContainer.tsx (신규)
- apps/desktop/src/wts/stores/toastStore.ts (신규)
- apps/desktop/src/wts/panels/OrderPanel.tsx (수정)
- apps/desktop/src/wts/WtsWindow.tsx (수정)
- apps/desktop/src/wts/__tests__/components/ConfirmDialog.test.tsx (신규)
- apps/desktop/src/wts/__tests__/stores/toastStore.test.ts (신규)
- apps/desktop/src/wts/__tests__/panels/OrderPanel.test.tsx (수정)
- apps/desktop/src/wts/types.ts (수정)

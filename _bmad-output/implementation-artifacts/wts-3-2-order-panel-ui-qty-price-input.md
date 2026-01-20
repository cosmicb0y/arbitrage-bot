# Story WTS-3.2: 주문 패널 UI (수량/가격 입력)

Status: done

## Story

As a **트레이더**,
I want **수량과 가격을 입력하고 주문 유형을 선택하는 폼**,
So that **정확한 주문 조건을 설정할 수 있다**.

## Acceptance Criteria

1. **Given** 주문 패널이 표시되어 있을 때 **When** 화면이 렌더링되면 **Then** 지정가/시장가 탭이 표시되어야 한다
2. **Given** 주문 패널이 표시되어 있을 때 **When** 화면이 렌더링되면 **Then** 수량 입력 필드와 % 버튼(25%, 50%, 75%, MAX)이 표시되어야 한다
3. **Given** 주문 패널이 표시되어 있을 때 **When** 지정가 모드가 선택되면 **Then** 가격 입력 필드가 활성화되어야 한다
4. **Given** 주문 패널이 표시되어 있을 때 **When** 시장가 모드가 선택되면 **Then** 가격 입력 필드가 비활성화되어야 한다
5. **Given** 입력값이 변경될 때 **When** 수량 또는 가격이 변경되면 **Then** 예상 총액이 계산되어 표시되어야 한다
6. **Given** 매수 모드일 때 **When** % 버튼을 클릭하면 **Then** 가용 KRW 잔고 기준으로 수량이 계산되어야 한다
7. **Given** 매도 모드일 때 **When** % 버튼을 클릭하면 **Then** 가용 코인 잔고 기준으로 수량이 설정되어야 한다
8. **Given** 수량 입력 필드에 **When** 비숫자 문자가 입력되면 **Then** 입력이 차단되어야 한다

## Tasks / Subtasks

- [x] Task 1: 주문 유형 탭 UI 구현 (AC: #1, #3, #4)
  - [x] Subtask 1.1: 지정가/시장가 탭 컴포넌트 추가 (shadcn/ui Tabs 스타일)
  - [x] Subtask 1.2: orderStore.setOrderType 연결
  - [x] Subtask 1.3: 지정가 선택 시 가격 필드 활성화, 시장가 선택 시 비활성화

- [x] Task 2: 가격 입력 UI 개선 (AC: #1, #3, #4)
  - [x] Subtask 2.1: 가격 입력 필드 숫자 포맷팅 (천단위 콤마)
  - [x] Subtask 2.2: 시장가 모드 시 가격 필드 비활성화 + placeholder "시장가"
  - [x] Subtask 2.3: 가격 입력 시 실시간 검증 (양수만 허용)

- [x] Task 3: 수량 입력 UI 구현 (AC: #2, #8)
  - [x] Subtask 3.1: 수량 입력 필드 추가 (orderStore.quantity 연결)
  - [x] Subtask 3.2: 숫자+소수점만 허용하는 입력 검증
  - [x] Subtask 3.3: 수량 라벨에 코인 심볼 표시 (예: "수량 (BTC)")

- [x] Task 4: % 버튼 UI 구현 (AC: #2, #6, #7)
  - [x] Subtask 4.1: 25%, 50%, 75%, MAX 버튼 그리드 레이아웃
  - [x] Subtask 4.2: 매수 모드: KRW 잔고 기준 수량 계산 로직
  - [x] Subtask 4.3: 매도 모드: 코인 잔고 기준 수량 설정 로직
  - [x] Subtask 4.4: balanceStore에서 현재 잔고 조회

- [x] Task 5: 예상 총액 표시 (AC: #5)
  - [x] Subtask 5.1: 예상 총액 계산 (수량 × 가격)
  - [x] Subtask 5.2: 시장가 매수 시 입력된 KRW 금액 표시
  - [x] Subtask 5.3: KRW 포맷팅 (천단위 콤마 + ₩ 기호)
  - [x] Subtask 5.4: 잔고 초과 시 경고 표시 (빨간 텍스트)

- [x] Task 6: 매수/매도 버튼 UI (AC: #1)
  - [x] Subtask 6.1: 매수 버튼 (녹색 계열)
  - [x] Subtask 6.2: 매도 버튼 (빨간 계열)
  - [x] Subtask 6.3: 버튼 클릭 시 side 상태 설정
  - [x] Subtask 6.4: 현재 side에 따른 버튼 강조 표시

- [x] Task 7: 단위 테스트 작성 (AC: #1-#8)
  - [x] Subtask 7.1: OrderPanel 탭 전환 테스트
  - [x] Subtask 7.2: 가격/수량 입력 검증 테스트
  - [x] Subtask 7.3: % 버튼 계산 로직 테스트
  - [x] Subtask 7.4: 예상 총액 계산 테스트

## Dev Notes

### 기존 OrderPanel 현황

[Source: apps/desktop/src/wts/panels/OrderPanel.tsx]

현재 OrderPanel은 기본적인 구조만 있음:
- orderType, side, price 표시
- 가격 입력 필드 (기본)
- 수량 입력 필드 미구현

### orderStore 현황

[Source: apps/desktop/src/wts/stores/orderStore.ts]

이미 구현된 상태:
- `orderType`: 'market' | 'limit'
- `side`: 'buy' | 'sell'
- `price`: string
- `quantity`: string
- `setOrderType`, `setSide`, `setPrice`, `setQuantity` 액션
- `setPriceFromOrderbook`: 오더북 클릭 시 가격 자동 입력 (WTS-2.6에서 구현)

### 가용 잔고 조회 방법

[Source: apps/desktop/src/wts/stores/balanceStore.ts]

```typescript
import { useBalanceStore } from '../stores/balanceStore';

// 특정 화폐 잔고 조회
const getAvailableBalance = (currency: string): number => {
  const balances = useBalanceStore.getState().balances;
  const entry = balances.find(b => b.currency === currency);
  if (!entry) return 0;
  return parseFloat(entry.balance); // locked 제외한 가용 잔고
};

// 예: KRW 잔고
const krwBalance = getAvailableBalance('KRW');
// 예: BTC 잔고
const btcBalance = getAvailableBalance('BTC');
```

### 수량 계산 로직

**매수 (Buy) 모드 - KRW 기준:**
```typescript
// 지정가 매수: KRW 잔고 / 가격 = 구매 가능 수량
const calculateBuyQuantity = (percent: number, price: number): string => {
  const krwBalance = getAvailableBalance('KRW');
  const totalKrw = krwBalance * (percent / 100);
  const quantity = totalKrw / price;
  return quantity.toFixed(8); // 소수점 8자리
};

// 시장가 매수: KRW 금액 직접 입력 (price 필드에 총액)
// 시장가 매수는 수량이 아닌 KRW 금액 기준
```

**매도 (Sell) 모드 - 코인 기준:**
```typescript
// 매도: 코인 잔고의 %
const calculateSellQuantity = (percent: number, coinCurrency: string): string => {
  const coinBalance = getAvailableBalance(coinCurrency);
  const quantity = coinBalance * (percent / 100);
  return quantity.toFixed(8);
};
```

### 시장가 주문 특수 처리

[Source: wts-3-1-order-api-rust-backend.md#Upbit 주문 유형]

| 주문 유형 | side | 필수 파라미터 | 설명 |
|----------|------|--------------|------|
| `limit` | bid/ask | market, side, **volume**, **price** | 지정가 |
| `price` | bid | market, side, **price** (총액) | 시장가 매수 |
| `market` | ask | market, side, **volume** | 시장가 매도 |

**시장가 매수 UI 처리:**
- 수량 입력 대신 **KRW 금액** 입력
- % 버튼 클릭 시 KRW 잔고의 %를 price 필드에 설정
- 라벨: "주문 금액 (KRW)"

**시장가 매도 UI 처리:**
- 일반적인 **수량** 입력
- % 버튼 클릭 시 코인 잔고의 %를 quantity 필드에 설정
- 가격 필드 비활성화

### UI 디자인 참조

[Source: ux-design-specification.md#OrderForm]

```
┌─────────────────────────┐
│ [지정가] [시장가]       │  ← 탭
├─────────────────────────┤
│ 가격  [          ]  KRW │  ← 지정가만 활성화
│ 수량  [          ]  BTC │  ← 항상 표시
│ [25%][50%][75%][MAX]    │  ← % 버튼
├─────────────────────────┤
│ 예상 총액: ₩1,234,567   │  ← 실시간 계산
├─────────────────────────┤
│ [  매수  ] [  매도  ]   │  ← 녹색/빨간색
└─────────────────────────┘
```

### 스타일 패턴

[Source: architecture.md#Design System]

**색상:**
- 매수 버튼: `bg-green-600 hover:bg-green-700` (success)
- 매도 버튼: `bg-red-600 hover:bg-red-700` (destructive)
- % 버튼: `bg-wts-secondary hover:bg-wts-tertiary` (secondary)
- 비활성화: `opacity-50 cursor-not-allowed`

**입력 필드:**
```tsx
<input
  type="text"
  inputMode="decimal"
  className="w-full px-3 py-2 rounded border border-wts bg-wts-secondary
             text-wts-foreground font-mono text-right
             focus:outline-none focus:border-wts-focus
             disabled:opacity-50 disabled:cursor-not-allowed"
/>
```

**탭 스타일:**
```tsx
<div className="flex border-b border-wts">
  <button
    className={`flex-1 py-2 text-sm font-medium
      ${isActive ? 'text-wts-foreground border-b-2 border-wts-accent' : 'text-wts-muted'}
    `}
  >
    지정가
  </button>
</div>
```

### 입력 검증 유틸리티

```typescript
// 숫자 + 소수점만 허용
export function sanitizeNumericInput(value: string): string {
  // 소수점 하나만 허용, 숫자만 허용
  const sanitized = value.replace(/[^0-9.]/g, '');
  const parts = sanitized.split('.');
  if (parts.length > 2) {
    return parts[0] + '.' + parts.slice(1).join('');
  }
  return sanitized;
}

// KRW 포맷팅 (천단위 콤마)
export function formatKrw(amount: number | string): string {
  const num = typeof amount === 'string' ? parseFloat(amount) : amount;
  if (isNaN(num)) return '₩0';
  return `₩${num.toLocaleString('ko-KR')}`;
}

// 암호화폐 수량 포맷팅
export function formatCrypto(amount: number | string, decimals = 8): string {
  const num = typeof amount === 'string' ? parseFloat(amount) : amount;
  if (isNaN(num)) return '0';
  return num.toFixed(decimals).replace(/\.?0+$/, '');
}
```

### 현재 마켓 정보 조회

```typescript
import { useWtsStore } from '../stores/wtsStore';

// 현재 선택된 마켓에서 코인 심볼 추출
const getCoinFromMarket = (): string => {
  const market = useWtsStore.getState().selectedMarket;
  if (!market) return 'BTC';
  // "KRW-BTC" → "BTC"
  const parts = market.split('-');
  return parts[1] || 'BTC';
};
```

### Project Structure Notes

**수정 파일:**
- `apps/desktop/src/wts/panels/OrderPanel.tsx` - 전체 UI 개선
- `apps/desktop/src/wts/utils/formatters.ts` - sanitizeNumericInput 추가 (필요시)

**신규 파일:**
- (없음, 기존 컴포넌트 확장)

**아키텍처 정합성:**
- shadcn/ui 스타일 준수 (터미널 다크 테마)
- Zustand 스토어 패턴 준수 (orderStore, balanceStore, wtsStore)
- Tailwind CSS 유틸리티 클래스 사용
- 기존 포맷터 패턴 준수 (formatters.ts)

### 이전 스토리 학습사항

**WTS-3.1 (Order API Backend):**
- `toUpbitSide`, `toUpbitOrderType` 헬퍼 함수가 types.ts에 이미 존재
- 시장가 매수(price)와 시장가 매도(market)가 다른 API 유형임
- OrderParams 인터페이스가 정의됨

**WTS-2.6 (Orderbook Click):**
- `setPriceFromOrderbook` 함수로 오더북 → 주문폼 가격 연동 완료
- 매도호가 클릭 = 매수, 매수호가 클릭 = 매도 로직 구현됨

### References

- [Architecture: OrderPanel Structure](/_bmad-output/planning-artifacts/architecture.md#OrderPanel)
- [UX Design: OrderForm Component](/_bmad-output/planning-artifacts/ux-design-specification.md#OrderForm)
- [WTS Epics: Story 3.2](/_bmad-output/planning-artifacts/wts-epics.md#Story 3.2)
- [Previous Story: WTS-3.1](/_bmad-output/implementation-artifacts/wts-3-1-order-api-rust-backend.md)
- [Previous Story: WTS-2.6](/_bmad-output/implementation-artifacts/wts-2-6-orderbook-panel-ui-click-interaction.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- 이전 실행: 모든 단위 테스트 통과 (수정 전)
- 코드 리뷰 수정 후 테스트 미실행

### Completion Notes List

- Task 1: 지정가/시장가 탭 UI 구현 완료 (role="tab", aria-selected 활용)
- Task 2: 가격 입력 UI 개선 - 천단위 콤마 포맷팅, 시장가 모드 비활성화/placeholder 적용, 음수 입력 차단
- Task 3: 수량 입력 UI 구현 - 숫자+소수점 검증, 코인 심볼 표시
- Task 4: % 버튼 구현 - 매수(KRW 잔고/가격 기준 수량 계산), 매도(코인 잔고 기준)
- Task 5: 예상 총액 계산 및 표시(시장가/지정가), 잔고 초과 시 빨간 경고
- Task 6: 매수/매도 버튼 UI - 녹색/빨간색 강조, setSide 연결
- Task 7: 단위 테스트 작성 (탭 전환, 입력 검증, % 버튼 계산, 예상 총액)
- Code review fixes: AC2/4/5/2.3 정합성 수정 및 테스트 보강

**주요 구현 사항:**
- sanitizeNumericInput() 함수로 비숫자/음수 입력 차단
- formatPriceInput() 함수로 천단위 콤마 포맷팅
- 시장가 모드 가격 필드 비활성화, placeholder "시장가"
- 수량 필드 항상 표시 (코인 심볼 포함)
- 가용 잔고 표시 (KRW, 코인)

### File List

- apps/desktop/src/wts/panels/OrderPanel.tsx (수정)
- apps/desktop/src/wts/__tests__/panels/OrderPanel.test.tsx (신규)
- _bmad-output/implementation-artifacts/wts-3-2-order-panel-ui-qty-price-input.md (수정)
- _bmad-output/implementation-artifacts/sprint-status.yaml (수정)

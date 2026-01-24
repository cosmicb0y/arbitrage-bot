# Story WTS-4.2: 입금 탭 UI (자산/네트워크 선택)

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **입금할 자산과 네트워크를 선택하는 UI**,
So that **원하는 방식으로 입금을 준비할 수 있다**.

## Acceptance Criteria

1. **Given** Transfer 패널의 입금 탭이 선택되어 있을 때 **When** 화면이 렌더링되면 **Then** 자산 선택 드롭다운이 표시되어야 한다
2. **Given** 자산이 선택되어 있을 때 **When** 자산 드롭다운에서 자산을 선택하면 **Then** 해당 자산의 네트워크 목록이 표시되어야 한다
3. **Given** 네트워크 목록이 표시되어 있을 때 **When** 네트워크를 확인하면 **Then** 네트워크별 특징(입금 상태, 확인 횟수, 최소 입금 수량)이 안내되어야 한다
4. **Given** 자산 또는 네트워크가 선택될 때 **When** 선택 상태가 변경되면 **Then** transferStore에 선택 상태가 저장되어야 한다
5. **Given** Transfer 패널이 표시되어 있을 때 **When** 입금/출금 탭을 전환하면 **Then** 해당 탭 콘텐츠가 표시되어야 한다
6. **Given** 입금 탭이 활성화되어 있을 때 **When** 자산을 선택하면 **Then** 콘솔에 자산 선택 로그가 기록되어야 한다
7. **Given** 입금이 일시 중단된 네트워크일 때 **When** 네트워크 목록을 표시하면 **Then** 해당 네트워크는 중단 상태로 표시되어야 한다

## Tasks / Subtasks

- [x] Task 1: transferStore Zustand 스토어 생성 (AC: #4)
  - [x] Subtask 1.1: `stores/transferStore.ts` 파일 생성
  - [x] Subtask 1.2: TransferState 인터페이스 정의 (activeTab, selectedCurrency, selectedNetwork 등)
  - [x] Subtask 1.3: Zustand 스토어 create() 구현
  - [x] Subtask 1.4: 액션 함수 구현 (setActiveTab, setSelectedCurrency, setSelectedNetwork)
  - [x] Subtask 1.5: 스토어 index.ts에 export 추가

- [x] Task 2: TransferPanel 컴포넌트 기본 구조 생성 (AC: #1, #5)
  - [x] Subtask 2.1: `panels/TransferPanel.tsx` 파일 생성
  - [x] Subtask 2.2: 기존 패널 구조 패턴 적용 (wts-panel, wts-panel-header, wts-panel-content)
  - [x] Subtask 2.3: 입금/출금 탭 UI 구현 (role="tablist", aria-selected)
  - [x] Subtask 2.4: DepositTab / WithdrawTab 조건부 렌더링

- [x] Task 3: 자산 선택 드롭다운 구현 (AC: #1, #4, #6)
  - [x] Subtask 3.1: 자산 목록 상수 정의 (DEPOSIT_CURRENCIES)
  - [x] Subtask 3.2: Select 컴포넌트 (shadcn/ui 스타일) 구현
  - [x] Subtask 3.3: 자산 선택 시 transferStore 상태 업데이트
  - [x] Subtask 3.4: 자산 선택 시 콘솔 로그 기록 (addLog)
  - [x] Subtask 3.5: 자산별 아이콘 또는 심볼 표시

- [x] Task 4: 네트워크 목록 조회 및 표시 (AC: #2, #3, #7)
  - [x] Subtask 4.1: `wts_get_deposit_chance` API 호출 훅 또는 함수 구현
  - [x] Subtask 4.2: 자산 선택 시 네트워크 정보 조회 트리거
  - [x] Subtask 4.3: 네트워크 목록 UI 렌더링 (RadioGroup 또는 Select)
  - [x] Subtask 4.4: 네트워크 정보 표시 (name, deposit_state, confirm_count, minimum)
  - [x] Subtask 4.5: 입금 중단 상태 시각적 표시 (deposit_state !== 'normal')
  - [x] Subtask 4.6: 로딩 상태 및 에러 처리

- [x] Task 5: WtsWindow에 TransferPanel 통합 (AC: #1)
  - [x] Subtask 5.1: WtsWindow.tsx에서 TransferPanel import
  - [x] Subtask 5.2: 레이아웃 그리드에 TransferPanel 배치
  - [x] Subtask 5.3: className 전달 및 스타일 적용

- [x] Task 6: 테스트 작성 (AC: #1-#7)
  - [x] Subtask 6.1: transferStore 단위 테스트 (`__tests__/stores/transferStore.test.ts`)
  - [x] Subtask 6.2: TransferPanel 컴포넌트 테스트 (`__tests__/panels/TransferPanel.test.tsx`)
  - [x] Subtask 6.3: 탭 전환 테스트
  - [x] Subtask 6.4: 자산 선택 테스트
  - [x] Subtask 6.5: 네트워크 정보 표시 테스트

## Dev Notes

### 프로젝트 구조 요구사항

[Source: architecture.md#WTS Frontend Structure]

**신규 파일:**
- `apps/desktop/src/wts/panels/TransferPanel.tsx` - Transfer 패널 컴포넌트 (입금/출금 탭)
- `apps/desktop/src/wts/stores/transferStore.ts` - Transfer 상태 관리 스토어
- `apps/desktop/src/wts/__tests__/stores/transferStore.test.ts` - 스토어 테스트
- `apps/desktop/src/wts/__tests__/panels/TransferPanel.test.tsx` - 패널 테스트

**수정 파일:**
- `apps/desktop/src/wts/WtsWindow.tsx` - TransferPanel 배치
- `apps/desktop/src/wts/stores/index.ts` - transferStore export 추가

### 기존 코드 패턴 참조

**패널 컴포넌트 구조 (OrderPanel.tsx 참조):**

```tsx
export function TransferPanel({ className = '' }: TransferPanelProps) {
  return (
    <div
      data-testid="transfer-panel"
      className={`wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Transfer</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        {/* 컨텐츠 */}
      </div>
    </div>
  );
}
```

**Zustand 스토어 패턴 (orderStore.ts 참조):**

```typescript
import { create } from 'zustand';

export interface TransferState {
  /** 활성 탭: deposit(입금) | withdraw(출금) */
  activeTab: 'deposit' | 'withdraw';
  /** 선택된 자산 코드 */
  selectedCurrency: string | null;
  /** 선택된 네트워크 타입 */
  selectedNetwork: string | null;
  /** 네트워크 정보 (deposit chance 응답) */
  networkInfo: DepositChanceResponse | null;
  /** 로딩 상태 */
  isLoading: boolean;
  /** 에러 메시지 */
  error: string | null;

  // Actions
  setActiveTab: (tab: 'deposit' | 'withdraw') => void;
  setSelectedCurrency: (currency: string | null) => void;
  setSelectedNetwork: (network: string | null) => void;
  setNetworkInfo: (info: DepositChanceResponse | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

export const useTransferStore = create<TransferState>()((set) => ({
  activeTab: 'deposit',
  selectedCurrency: null,
  selectedNetwork: null,
  networkInfo: null,
  isLoading: false,
  error: null,

  setActiveTab: (activeTab) => set({ activeTab }),
  setSelectedCurrency: (selectedCurrency) => set({ selectedCurrency, selectedNetwork: null, networkInfo: null }),
  setSelectedNetwork: (selectedNetwork) => set({ selectedNetwork }),
  setNetworkInfo: (networkInfo) => set({ networkInfo }),
  setLoading: (isLoading) => set({ isLoading }),
  setError: (error) => set({ error }),
  reset: () => set({
    activeTab: 'deposit',
    selectedCurrency: null,
    selectedNetwork: null,
    networkInfo: null,
    isLoading: false,
    error: null,
  }),
}));
```

### WTS-4.1 백엔드 API 사용법

[Source: wts-4-1-deposit-api-rust-backend.md]

**입금 가능 정보 조회 (자산/네트워크 선택 시):**

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { DepositChanceParams, DepositChanceResponse, WtsApiResult } from '../types';

async function fetchDepositChance(currency: string, netType: string) {
  const params: DepositChanceParams = { currency, net_type: netType };
  const result = await invoke<WtsApiResult<DepositChanceResponse>>(
    'wts_get_deposit_chance',
    { params }
  );

  if (result.success && result.data) {
    return result.data;
  } else {
    throw new Error(result.error?.message || '입금 가능 정보 조회 실패');
  }
}
```

**DepositChanceResponse 구조:**

```typescript
interface DepositChanceResponse {
  currency: string;           // 자산 코드
  net_type: string;           // 네트워크 타입
  network: {
    name: string;             // 네트워크 이름 (예: "Bitcoin")
    net_type: string;         // 네트워크 타입
    priority: number;         // 우선순위
    deposit_state: string;    // 입금 상태 (normal, paused, suspended)
    confirm_count: number;    // 확인 횟수
  };
  deposit_state: string;      // 입금 상태
  minimum: string;            // 최소 입금 수량
}
```

### 입금 자산 목록 (MVP)

[Source: Upbit API 분석]

Upbit에서 지원하는 주요 입금 자산 및 네트워크:

```typescript
/** 입금 가능 자산 목록 (MVP) */
export const DEPOSIT_CURRENCIES = [
  { code: 'BTC', name: '비트코인', networks: ['BTC'] },
  { code: 'ETH', name: '이더리움', networks: ['ETH'] },
  { code: 'XRP', name: '리플', networks: ['XRP'] },
  { code: 'SOL', name: '솔라나', networks: ['SOL'] },
  { code: 'DOGE', name: '도지코인', networks: ['DOGE'] },
  { code: 'ADA', name: '에이다', networks: ['ADA'] },
  { code: 'USDT', name: '테더', networks: ['TRX', 'ETH'] }, // 다중 네트워크
  { code: 'USDC', name: 'USD 코인', networks: ['ETH', 'SOL', 'ARB'] },
] as const;
```

### 탭 UI 패턴

[Source: OrderPanel.tsx]

```tsx
{/* 탭 UI */}
<div className="flex border-b border-wts" role="tablist">
  <button
    role="tab"
    aria-selected={activeTab === 'deposit'}
    onClick={() => setActiveTab('deposit')}
    className={`flex-1 py-2 text-sm font-medium transition-colors
      ${activeTab === 'deposit'
        ? 'text-wts-foreground border-b-2 border-wts-accent'
        : 'text-wts-muted hover:text-wts-foreground'
      }
    `}
  >
    입금
  </button>
  <button
    role="tab"
    aria-selected={activeTab === 'withdraw'}
    onClick={() => setActiveTab('withdraw')}
    className={`flex-1 py-2 text-sm font-medium transition-colors
      ${activeTab === 'withdraw'
        ? 'text-wts-foreground border-b-2 border-wts-accent'
        : 'text-wts-muted hover:text-wts-foreground'
      }
    `}
  >
    출금
  </button>
</div>
```

### Select 컴포넌트 스타일링

[Source: OrderPanel.tsx 입력 스타일]

```tsx
{/* 자산 선택 드롭다운 */}
<label className="block text-xs">
  <span className="text-wts-muted mb-1 block">자산</span>
  <select
    value={selectedCurrency || ''}
    onChange={(e) => handleCurrencyChange(e.target.value)}
    className="w-full px-3 py-2 rounded border border-wts bg-wts-secondary
               text-wts-foreground text-sm
               focus:outline-none focus:border-wts-focus"
  >
    <option value="">자산 선택</option>
    {DEPOSIT_CURRENCIES.map((c) => (
      <option key={c.code} value={c.code}>
        {c.code} - {c.name}
      </option>
    ))}
  </select>
</label>
```

### 네트워크 정보 표시 UI

```tsx
{/* 네트워크 정보 표시 */}
{networkInfo && (
  <div className="mt-3 p-3 rounded bg-wts-tertiary text-xs space-y-2">
    <div className="flex justify-between">
      <span className="text-wts-muted">네트워크</span>
      <span className="text-wts-foreground">{networkInfo.network.name}</span>
    </div>
    <div className="flex justify-between">
      <span className="text-wts-muted">입금 상태</span>
      <span className={isDepositAvailable(networkInfo.deposit_state)
        ? 'text-green-500'
        : 'text-red-500'}>
        {networkInfo.deposit_state === 'normal' ? '정상' : '중단'}
      </span>
    </div>
    <div className="flex justify-between">
      <span className="text-wts-muted">확인 횟수</span>
      <span className="text-wts-foreground">{networkInfo.network.confirm_count}회</span>
    </div>
    <div className="flex justify-between">
      <span className="text-wts-muted">최소 입금</span>
      <span className="text-wts-foreground font-mono">
        {networkInfo.minimum} {networkInfo.currency}
      </span>
    </div>
  </div>
)}
```

### 에러 처리 패턴

[Source: errorHandler.ts, OrderPanel.tsx]

```typescript
import { handleApiError } from '../utils/errorHandler';
import { useConsoleStore } from '../stores/consoleStore';

// 컴포넌트 내부
const addLog = useConsoleStore((state) => state.addLog);

try {
  const result = await invoke<WtsApiResult<DepositChanceResponse>>('wts_get_deposit_chance', { params });
  if (result.success && result.data) {
    setNetworkInfo(result.data);
    addLog('INFO', 'DEPOSIT', `입금 정보 조회: ${currency}/${netType}`);
  } else {
    handleApiError(result.error, 'DEPOSIT', '입금 정보 조회 실패');
  }
} catch (err) {
  handleApiError(err, 'DEPOSIT', '입금 정보 조회 실패');
}
```

### 출금 탭 (플레이스홀더)

출금 기능은 WTS Epic 5에서 구현되므로, 이 스토리에서는 플레이스홀더만 표시:

```tsx
{activeTab === 'withdraw' && (
  <div className="flex items-center justify-center h-32 text-wts-muted text-sm">
    출금 기능은 준비 중입니다
  </div>
)}
```

### Project Structure Notes

**아키텍처 정합성:**
- Zustand 스토어: `use{Domain}Store` 패턴 준수 (`useTransferStore`)
- 패널 구조: `wts-panel`, `wts-panel-header`, `wts-panel-content` 클래스 사용
- Tauri 명령: `wts_get_deposit_chance` 호출
- 에러 처리: `handleApiError` + 콘솔 로깅 + 토스트 알림
- 콘솔 로그: `LogCategory = 'DEPOSIT'` 사용

**WtsWindow 레이아웃 배치:**

현재 WtsWindow.tsx 레이아웃 구조 확인 후 TransferPanel 배치 필요.
아키텍처에 따르면 TransferPanel은 중앙 하단 또는 별도 위치에 배치.

### 이전 스토리 인텔리전스

**WTS-4.1 (입금 API Rust 백엔드) 핵심 학습:**

1. **타입 정의 완료**: TypeScript 입금 타입이 이미 `types.ts`에 정의됨
   - `DepositAddressParams`, `DepositAddressResponse`
   - `DepositChanceParams`, `DepositChanceResponse`
   - `GenerateAddressResponse` (union type)
   - `isDepositAvailable()`, `isAddressGenerating()` 헬퍼

2. **Tauri 명령 사용 가능**:
   - `wts_get_deposit_chance` - 입금 가능 정보 조회
   - `wts_get_deposit_address` - 입금 주소 조회
   - `wts_generate_deposit_address` - 입금 주소 생성

3. **에러 코드 매핑 완료**: `UPBIT_ORDER_ERROR_MESSAGES`에 입금 관련 에러 추가됨
   - `deposit_address_not_found`, `invalid_currency`, `invalid_net_type`
   - `deposit_paused`, `deposit_suspended`, `address_generation_failed`

4. **Rate Limit**: 입금 API는 30회/초 (Exchange Default)

### Git 인텔리전스

**최근 커밋 패턴:**

| 커밋 | 패턴 |
|------|------|
| `53747fe feat(wts): implement deposit API Rust backend (WTS-4.1)` | feat(wts): 접두사 |
| `aa32cc7 feat(wts): complete console log for order results` | 기능별 명확한 메시지 |
| `b7f9c92 feat(wts): enhance order confirm dialog` | enhance 키워드 사용 |

**파일 명명 규칙:**
- 패널: `{Feature}Panel.tsx` (PascalCase)
- 스토어: `{feature}Store.ts` (camelCase)
- 테스트: `{filename}.test.ts(x)`

### 테스트 패턴

**스토어 테스트 (vitest):**

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import { useTransferStore } from '../transferStore';

describe('transferStore', () => {
  beforeEach(() => {
    useTransferStore.getState().reset();
  });

  it('should set active tab', () => {
    useTransferStore.getState().setActiveTab('withdraw');
    expect(useTransferStore.getState().activeTab).toBe('withdraw');
  });

  it('should reset network info when currency changes', () => {
    useTransferStore.getState().setNetworkInfo({ /* mock */ } as any);
    useTransferStore.getState().setSelectedCurrency('BTC');
    expect(useTransferStore.getState().networkInfo).toBeNull();
  });
});
```

**컴포넌트 테스트 (@testing-library/react):**

```typescript
import { render, screen, fireEvent } from '@testing-library/react';
import { TransferPanel } from '../TransferPanel';

describe('TransferPanel', () => {
  it('renders deposit tab by default', () => {
    render(<TransferPanel />);
    expect(screen.getByRole('tab', { name: /입금/i })).toHaveAttribute('aria-selected', 'true');
  });

  it('shows asset dropdown', () => {
    render(<TransferPanel />);
    expect(screen.getByLabelText(/자산/i)).toBeInTheDocument();
  });
});
```

### References

- [Architecture: WTS Frontend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Frontend Structure)
- [Architecture: Implementation Patterns](/_bmad-output/planning-artifacts/architecture.md#Implementation Patterns)
- [UX Design: Component Strategy](/_bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy)
- [WTS Epics: Epic 4 Story 4.2](/_bmad-output/planning-artifacts/wts-epics.md#Story 4.2)
- [Previous Story: WTS-4.1 입금 API 백엔드](/_bmad-output/implementation-artifacts/wts-4-1-deposit-api-rust-backend.md)
- [Existing Pattern: OrderPanel.tsx](apps/desktop/src/wts/panels/OrderPanel.tsx)
- [Existing Pattern: orderStore.ts](apps/desktop/src/wts/stores/orderStore.ts)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A - No debugging issues encountered

### Completion Notes List

1. **Task 1 완료**: transferStore Zustand 스토어 생성
   - TransferState 인터페이스 정의 (activeTab, selectedCurrency, selectedNetwork, networkInfo, isLoading, error)
   - 액션 함수 구현 (setActiveTab, setSelectedCurrency, setSelectedNetwork, setNetworkInfo, setLoading, setError, reset)
   - 자산 변경 시 네트워크 정보 자동 초기화 로직 포함
   - 20개 단위 테스트 통과

2. **Task 2 완료**: TransferPanel 컴포넌트 기본 구조
   - wts-panel, wts-panel-header, wts-panel-content 패턴 적용
   - 입금/출금 탭 UI (role="tablist", aria-selected) 구현
   - 탭 전환 시 조건부 렌더링

3. **Task 3 완료**: 자산 선택 드롭다운
   - DEPOSIT_CURRENCIES 상수 정의 (BTC, ETH, XRP, SOL, DOGE, ADA, USDT, USDC)
   - 자산 선택 시 transferStore 상태 업데이트
   - 자산 선택 시 콘솔 로그 기록 (DEPOSIT 카테고리)

4. **Task 4 완료**: 네트워크 목록 조회 및 표시
   - wts_get_deposit_chance API 호출 구현
   - 네트워크 정보 표시 (name, deposit_state, confirm_count, minimum)
   - 입금 중단 상태 시각적 표시 (정상: 녹색, 중단: 빨간색)
   - 로딩 상태 및 에러 처리

5. **Task 5 완료**: WtsWindow에 TransferPanel 통합
   - 그리드 레이아웃 4열로 확장 (25% 35% 20% 20%)
   - wts-area-transfer 그리드 영역 추가
   - TransferPanel 배치 완료

6. **Task 6 완료**: 테스트 작성
   - transferStore.test.ts: 20개 테스트
   - TransferPanel.test.tsx: 25개 테스트
   - 모든 AC (#1-#7) 커버리지 확인

### Code Review Fixes (2026-01-24)
- **Fix Critical Issue**: Implemented missing Network Selection UI (Radio Buttons) in TransferPanel.
- **Fix Test Configuration**: Added `// @vitest-environment jsdom` to TransferPanel.test.tsx and installed jsdom dependency to fix ReferenceError.

### File List

**Created:**
- `apps/desktop/src/wts/panels/TransferPanel.tsx`
- `apps/desktop/src/wts/stores/transferStore.ts`
- `apps/desktop/src/wts/utils/errorHandler.ts`
- `apps/desktop/src/wts/__tests__/stores/transferStore.test.ts`
- `apps/desktop/src/wts/__tests__/panels/TransferPanel.test.tsx`
- `apps/desktop/src/wts/__tests__/utils/errorHandler.test.ts`

**Modified:**
- `apps/desktop/src/wts/WtsWindow.tsx` - TransferPanel import 및 배치 추가
- `apps/desktop/src/wts/stores/index.ts` - useTransferStore export 추가
- `apps/desktop/src/wts/panels/index.ts` - TransferPanel export 추가
- `apps/desktop/src/index.css` - wts-grid 레이아웃 및 wts-area-transfer 추가

## Change Log

- **2026-01-24**: WTS-4.2 입금 탭 UI 구현 완료
  - transferStore Zustand 스토어 생성 (activeTab, selectedCurrency, selectedNetwork, networkInfo 상태 관리)
  - TransferPanel 컴포넌트 구현 (입금/출금 탭, 자산 선택 드롭다운, 네트워크 정보 표시)
  - 45개 테스트 작성 및 통과 (transferStore: 20개, TransferPanel: 25개)
  - WtsWindow 그리드 레이아웃 확장 (4열) 및 TransferPanel 배치
  - AC #1-#7 모두 충족

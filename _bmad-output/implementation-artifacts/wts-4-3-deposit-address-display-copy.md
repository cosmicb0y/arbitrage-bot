# Story WTS-4.3: 입금 주소 표시 및 복사

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **생성된 입금 주소를 복사하는 기능**,
So that **외부 지갑에서 쉽게 송금할 수 있다**.

## Acceptance Criteria

1. **Given** 자산과 네트워크가 선택되어 있을 때 **When** 입금 주소가 조회/생성되면 **Then** 입금 주소가 화면에 표시되어야 한다
2. **Given** 입금 주소가 표시되어 있을 때 **When** 주소 영역을 확인하면 **Then** 주소 옆에 복사 버튼이 있어야 한다
3. **Given** 복사 버튼이 표시되어 있을 때 **When** 복사 버튼을 클릭하면 **Then** 클립보드에 주소가 복사되어야 한다
4. **Given** 복사가 성공했을 때 **When** 클립보드 복사가 완료되면 **Then** 토스트 알림이 표시되어야 한다
5. **Given** 입금 주소 조회 요청이 발생할 때 **When** API 응답을 받으면 **Then** 콘솔에 입금 주소 조회/생성 로그가 기록되어야 한다
6. **Given** 보조 주소(tag/memo)가 있는 자산일 때 **When** 입금 주소가 표시되면 **Then** 보조 주소도 함께 표시되고 복사 가능해야 한다
7. **Given** 입금 주소가 아직 생성되지 않았을 때 **When** null 응답을 받으면 **Then** "주소 생성" 버튼이 표시되어야 한다

## Tasks / Subtasks

- [x] Task 1: transferStore 확장 - 입금 주소 상태 관리 (AC: #1, #7)
  - [x] Subtask 1.1: `depositAddress` 상태 추가 (DepositAddressResponse | null)
  - [x] Subtask 1.2: `isAddressLoading` 상태 추가
  - [x] Subtask 1.3: `addressError` 상태 추가
  - [x] Subtask 1.4: `setDepositAddress`, `setAddressLoading`, `setAddressError` 액션 추가
  - [x] Subtask 1.5: `fetchDepositAddress` 비동기 액션 구현 (컴포넌트에서 직접 호출)
  - [x] Subtask 1.6: 스토어 테스트 업데이트

- [x] Task 2: 입금 주소 조회 API 호출 구현 (AC: #1, #5)
  - [x] Subtask 2.1: 네트워크 선택 시 `wts_get_deposit_address` 호출
  - [x] Subtask 2.2: 성공 시 `depositAddress` 상태 업데이트
  - [x] Subtask 2.3: 실패 시 에러 처리 및 콘솔 로그
  - [x] Subtask 2.4: 콘솔에 INFO 레벨로 조회 로그 기록

- [x] Task 3: 입금 주소 표시 UI 구현 (AC: #1, #6)
  - [x] Subtask 3.1: AddressDisplay 컴포넌트 또는 섹션 구현
  - [x] Subtask 3.2: 주소 텍스트 표시 (모노스페이스 폰트, 워드브레이크)
  - [x] Subtask 3.3: 보조 주소(secondary_address) 표시 조건부 렌더링
  - [x] Subtask 3.4: 주소 없을 때 "주소 생성" 버튼 표시 (AC: #7)

- [x] Task 4: 복사 기능 구현 (AC: #2, #3, #4)
  - [x] Subtask 4.1: 복사 버튼 UI (아이콘 + 툴팁)
  - [x] Subtask 4.2: `navigator.clipboard.writeText()` 복사 로직
  - [x] Subtask 4.3: 복사 성공 시 토스트 알림 표시
  - [x] Subtask 4.4: 복사 성공 시 버튼 아이콘 임시 변경 (체크 아이콘, 2초)
  - [x] Subtask 4.5: 보조 주소 별도 복사 버튼

- [x] Task 5: 주소 생성 요청 연결 (AC: #7 - WTS-4.4 준비)
  - [x] Subtask 5.1: "주소 생성" 버튼 클릭 시 `wts_generate_deposit_address` 호출
  - [x] Subtask 5.2: 생성 중 로딩 상태 표시
  - [x] Subtask 5.3: 생성 완료 후 자동 재조회 (WTS-4.4에서 상세 구현)

- [x] Task 6: 테스트 작성 (AC: #1-#7)
  - [x] Subtask 6.1: transferStore 입금 주소 관련 테스트 추가
  - [x] Subtask 6.2: TransferPanel 입금 주소 표시 테스트
  - [x] Subtask 6.3: 복사 기능 테스트 (clipboard mock)
  - [x] Subtask 6.4: 토스트 알림 표시 테스트
  - [x] Subtask 6.5: 보조 주소 표시 테스트

## Dev Notes

### 프로젝트 구조 요구사항

[Source: architecture.md#WTS Frontend Structure]

**수정 파일:**
- `apps/desktop/src/wts/stores/transferStore.ts` - 입금 주소 상태 추가
- `apps/desktop/src/wts/panels/TransferPanel.tsx` - 주소 표시 및 복사 UI 추가
- `apps/desktop/src/wts/__tests__/stores/transferStore.test.ts` - 스토어 테스트 확장
- `apps/desktop/src/wts/__tests__/panels/TransferPanel.test.tsx` - 컴포넌트 테스트 확장

### 기존 코드 패턴 참조

**TransferPanel 현재 구조 (apps/desktop/src/wts/panels/TransferPanel.tsx):**

현재 네트워크 정보 표시 섹션 아래에 입금 주소 표시 섹션을 추가해야 함:

```tsx
{/* 네트워크 정보 표시 - 기존 */}
{networkInfo && !isLoading && (
  <div className="mt-3 p-3 rounded bg-wts-tertiary text-xs space-y-2">
    {/* ... 네트워크 정보 ... */}
  </div>
)}

{/* 입금 주소 표시 - 신규 추가 */}
{networkInfo && depositAddress && !isLoading && (
  <div className="mt-3 p-3 rounded bg-wts-tertiary text-xs space-y-2">
    <div className="flex justify-between items-start">
      <span className="text-wts-muted">입금 주소</span>
      <button
        onClick={() => handleCopyAddress(depositAddress.deposit_address)}
        className="ml-2 p-1 rounded hover:bg-wts-secondary"
        title="주소 복사"
      >
        {copied ? <CheckIcon /> : <CopyIcon />}
      </button>
    </div>
    <div className="font-mono text-wts-foreground break-all text-[11px]">
      {depositAddress.deposit_address}
    </div>

    {/* 보조 주소 (XRP tag, EOS memo 등) */}
    {depositAddress.secondary_address && (
      <>
        <div className="flex justify-between items-start mt-2">
          <span className="text-wts-muted">Memo/Tag</span>
          <button
            onClick={() => handleCopyAddress(depositAddress.secondary_address)}
            className="ml-2 p-1 rounded hover:bg-wts-secondary"
            title="Tag 복사"
          >
            <CopyIcon className="w-3 h-3" />
          </button>
        </div>
        <div className="font-mono text-wts-foreground">
          {depositAddress.secondary_address}
        </div>
      </>
    )}
  </div>
)}
```

### WTS-4.1 백엔드 API 사용법

[Source: wts-4-1-deposit-api-rust-backend.md]

**입금 주소 조회:**

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { DepositAddressParams, DepositAddressResponse, WtsApiResult } from '../types';

async function fetchDepositAddress(currency: string, netType: string): Promise<DepositAddressResponse | null> {
  const params: DepositAddressParams = { currency, net_type: netType };
  const result = await invoke<WtsApiResult<DepositAddressResponse>>(
    'wts_get_deposit_address',
    { params }
  );

  if (result.success && result.data) {
    return result.data;
  } else if (result.error?.code === 'deposit_address_not_found') {
    // 주소가 아직 생성되지 않음 - null 반환
    return null;
  } else {
    throw new Error(result.error?.message || '입금 주소 조회 실패');
  }
}
```

**DepositAddressResponse 구조:**

```typescript
interface DepositAddressResponse {
  currency: string;           // 자산 코드
  net_type: string;           // 네트워크 타입
  deposit_address: string | null;  // 입금 주소 (null = 생성 중)
  secondary_address: string | null; // 보조 주소 (XRP tag, EOS memo 등)
}
```

### transferStore 확장 패턴

[Source: transferStore.ts]

```typescript
export interface TransferState {
  // ... 기존 상태 ...

  /** 입금 주소 정보 */
  depositAddress: DepositAddressResponse | null;
  /** 주소 로딩 상태 */
  isAddressLoading: boolean;
  /** 주소 조회 에러 */
  addressError: string | null;

  // Actions
  setDepositAddress: (address: DepositAddressResponse | null) => void;
  setAddressLoading: (loading: boolean) => void;
  setAddressError: (error: string | null) => void;
}

// 초기값
depositAddress: null,
isAddressLoading: false,
addressError: null,

// 액션
setDepositAddress: (depositAddress) => set({ depositAddress }),
setAddressLoading: (isAddressLoading) => set({ isAddressLoading }),
setAddressError: (addressError) => set({ addressError }),
```

### 복사 기능 구현 패턴

```typescript
import { useState, useCallback } from 'react';

const [copiedField, setCopiedField] = useState<string | null>(null);

const handleCopyAddress = useCallback(async (text: string | null, field: string) => {
  if (!text) return;

  try {
    await navigator.clipboard.writeText(text);
    setCopiedField(field);

    // 토스트 알림
    addLog('SUCCESS', 'DEPOSIT', `${field === 'address' ? '입금 주소' : 'Tag'}가 클립보드에 복사되었습니다`);

    // Toast 알림 (react-hot-toast 또는 커스텀)
    toast.success('클립보드에 복사되었습니다');

    // 2초 후 복사 상태 초기화
    setTimeout(() => setCopiedField(null), 2000);
  } catch (err) {
    addLog('ERROR', 'DEPOSIT', '클립보드 복사 실패');
    toast.error('복사에 실패했습니다');
  }
}, [addLog]);
```

### 아이콘 컴포넌트

프로젝트에 아이콘 라이브러리가 없다면 인라인 SVG 사용:

```tsx
// 복사 아이콘
const CopyIcon = ({ className = 'w-4 h-4' }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
      d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
  </svg>
);

// 체크 아이콘 (복사 성공)
const CheckIcon = ({ className = 'w-4 h-4' }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
      d="M5 13l4 4L19 7" />
  </svg>
);
```

### 토스트 알림 패턴

현재 프로젝트에서 토스트 구현 확인 필요. 없다면 콘솔 로그 + 간단한 인라인 메시지로 대체:

```tsx
// 복사 성공 시 임시 메시지 표시
const [showCopySuccess, setShowCopySuccess] = useState(false);

{showCopySuccess && (
  <div className="absolute bottom-2 right-2 bg-green-600 text-white text-xs px-2 py-1 rounded">
    복사됨!
  </div>
)}
```

또는 `react-hot-toast` 설치하여 사용:

```typescript
import toast from 'react-hot-toast';

// 복사 성공
toast.success('클립보드에 복사되었습니다');
```

### 보조 주소가 필요한 자산

[Source: Upbit API 분석]

| 자산 | 보조 주소 타입 | 필수 여부 |
|------|---------------|----------|
| XRP | Destination Tag | 필수 |
| EOS | Memo | 필수 |
| XLM | Memo | 선택 |
| ATOM | Memo | 선택 |

보조 주소가 있는 경우 **반드시 함께 표시**하고, 사용자에게 경고:

```tsx
{depositAddress.secondary_address && (
  <div className="mt-2 p-2 rounded bg-yellow-900/20 border border-yellow-500/30">
    <div className="flex items-center gap-1 text-yellow-400 text-xs mb-1">
      <WarningIcon className="w-3 h-3" />
      <span>Memo/Tag 필수</span>
    </div>
    <div className="text-yellow-200 text-[10px]">
      입금 시 반드시 아래 Tag를 포함해야 합니다
    </div>
  </div>
)}
```

### Project Structure Notes

**아키텍처 정합성:**
- Zustand 스토어: `useTransferStore` 확장 (기존 패턴 유지)
- API 호출: `invoke<WtsApiResult<T>>` 패턴 준수
- 에러 처리: `handleApiError` + 콘솔 로깅
- 콘솔 로그: `LogCategory = 'DEPOSIT'` 사용
- 클립보드: Web API `navigator.clipboard` 사용 (Tauri 앱에서 지원)

**WtsWindow 레이아웃:**
TransferPanel 위치 변경 없음 - 기존 위치 유지

### 이전 스토리 인텔리전스

**WTS-4.2 (입금 탭 UI) 핵심 학습:**

1. **자산/네트워크 선택 완료**:
   - `selectedCurrency`, `selectedNetwork`, `networkInfo` 상태 이미 구현
   - 네트워크 선택 시 `fetchDepositChance` 호출됨

2. **입금 정보 조회 패턴**:
   - `wts_get_deposit_chance` 호출 후 `networkInfo` 상태 업데이트
   - 같은 패턴으로 `wts_get_deposit_address` 호출 구현

3. **UI 패턴**:
   - `bg-wts-tertiary` 배경의 정보 표시 카드
   - `text-wts-muted` / `text-wts-foreground` 색상 구분
   - `font-mono` 클래스로 주소 표시

4. **에러 처리**:
   - `handleApiError(error, 'DEPOSIT', message)` 패턴 사용
   - `isLoading`, `error` 상태로 UI 피드백

**WTS-4.1 (입금 API 백엔드) 참조:**
- `wts_get_deposit_address` Tauri 명령 사용 가능
- `wts_generate_deposit_address` Tauri 명령 사용 가능
- `DepositAddressResponse` 타입 정의됨

### Git 인텔리전스

**최근 커밋 패턴:**

| 커밋 | 패턴 |
|------|------|
| `6323801 feat(wts): implement deposit tab UI with asset/network selection (WTS-4.2)` | feat(wts): 접두사 + 스토리 번호 |
| `53747fe feat(wts): implement deposit API Rust backend (WTS-4.1)` | 백엔드/프론트엔드 분리 명확 |

**권장 커밋 메시지:**
```
feat(wts): implement deposit address display and copy (WTS-4.3)
```

**파일 명명 규칙:**
- 스토어 확장: 기존 `transferStore.ts` 수정
- 테스트 확장: 기존 테스트 파일에 케이스 추가

### 테스트 패턴

**스토어 테스트 추가 (vitest):**

```typescript
describe('deposit address state', () => {
  it('should set deposit address', () => {
    const mockAddress: DepositAddressResponse = {
      currency: 'BTC',
      net_type: 'BTC',
      deposit_address: '1A2b3C4d5E6f...',
      secondary_address: null,
    };
    useTransferStore.getState().setDepositAddress(mockAddress);
    expect(useTransferStore.getState().depositAddress).toEqual(mockAddress);
  });

  it('should handle address with secondary_address (XRP tag)', () => {
    const mockAddress: DepositAddressResponse = {
      currency: 'XRP',
      net_type: 'XRP',
      deposit_address: 'rXXXXXXXX...',
      secondary_address: '123456789',
    };
    useTransferStore.getState().setDepositAddress(mockAddress);
    expect(useTransferStore.getState().depositAddress?.secondary_address).toBe('123456789');
  });
});
```

**컴포넌트 테스트 (@testing-library/react):**

```typescript
describe('deposit address display', () => {
  it('displays deposit address when available', async () => {
    // transferStore에 주소 설정
    useTransferStore.setState({
      depositAddress: {
        currency: 'BTC',
        net_type: 'BTC',
        deposit_address: '1A2b3C4d5E6f7g8H9i0J',
        secondary_address: null,
      },
    });

    render(<TransferPanel />);
    expect(screen.getByText('1A2b3C4d5E6f7g8H9i0J')).toBeInTheDocument();
  });

  it('displays copy button next to address', () => {
    useTransferStore.setState({
      depositAddress: { /* ... */ },
    });

    render(<TransferPanel />);
    expect(screen.getByTitle('주소 복사')).toBeInTheDocument();
  });

  it('copies address to clipboard on button click', async () => {
    const mockWriteText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: { writeText: mockWriteText },
    });

    useTransferStore.setState({
      depositAddress: {
        deposit_address: '1A2b3C4d5E6f7g8H9i0J',
        // ...
      },
    });

    render(<TransferPanel />);
    fireEvent.click(screen.getByTitle('주소 복사'));

    await waitFor(() => {
      expect(mockWriteText).toHaveBeenCalledWith('1A2b3C4d5E6f7g8H9i0J');
    });
  });

  it('displays secondary address for XRP', () => {
    useTransferStore.setState({
      depositAddress: {
        currency: 'XRP',
        deposit_address: 'rXXX...',
        secondary_address: '123456',
      },
    });

    render(<TransferPanel />);
    expect(screen.getByText('Memo/Tag')).toBeInTheDocument();
    expect(screen.getByText('123456')).toBeInTheDocument();
  });

  it('shows generate button when address is null', () => {
    useTransferStore.setState({
      networkInfo: { /* valid network info */ },
      depositAddress: null,
    });

    render(<TransferPanel />);
    expect(screen.getByRole('button', { name: /주소 생성/i })).toBeInTheDocument();
  });
});
```

### 주의 사항

1. **주소 표시 시 워드브레이크**: 긴 주소가 UI를 깨뜨리지 않도록 `break-all` 클래스 사용
2. **보조 주소 경고**: XRP, EOS 등 tag/memo 필수 자산은 경고 메시지 표시
3. **클립보드 권한**: Tauri 앱에서는 기본적으로 클립보드 접근 가능
4. **복사 피드백**: 사용자에게 복사 성공 여부를 명확히 전달 (아이콘 변경 + 토스트)
5. **null 주소 처리**: deposit_address가 null인 경우 WTS-4.4(비동기 생성)로 연결

### References

- [Architecture: WTS Frontend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Frontend Structure)
- [WTS Epics: Epic 4 Story 4.3](/_bmad-output/planning-artifacts/wts-epics.md#Story 4.3)
- [Previous Story: WTS-4.2 입금 탭 UI](/_bmad-output/implementation-artifacts/wts-4-2-deposit-tab-ui-asset-network.md)
- [Previous Story: WTS-4.1 입금 API 백엔드](/_bmad-output/implementation-artifacts/wts-4-1-deposit-api-rust-backend.md)
- [Existing: TransferPanel.tsx](apps/desktop/src/wts/panels/TransferPanel.tsx)
- [Existing: transferStore.ts](apps/desktop/src/wts/stores/transferStore.ts)
- [Types: DepositAddressResponse](apps/desktop/src/wts/types.ts)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A - 구현 중 특별한 디버그 이슈 없음

### Completion Notes List

1. **transferStore 확장 완료**
   - `depositAddress`, `isAddressLoading`, `addressError` 상태 추가
   - `setDepositAddress`, `setAddressLoading`, `setAddressError` 액션 추가
   - `setSelectedCurrency` 호출 시 입금 주소 관련 상태 자동 초기화
   - `reset()` 호출 시 입금 주소 상태 포함하여 초기화

2. **입금 주소 조회 API 연동 완료**
   - `fetchDepositAddress()` 함수 구현
   - 네트워크 선택 시 자동으로 입금 주소 조회
   - 주소 미생성 시 `deposit_address: null` 상태로 처리

3. **입금 주소 표시 UI 완료**
   - 모노스페이스 폰트, break-all로 긴 주소 표시
   - 보조 주소(XRP tag, EOS memo 등) 조건부 렌더링
   - 보조 주소 필수 자산에 경고 메시지 표시

4. **복사 기능 완료**
   - `navigator.clipboard.writeText()` 사용
   - 복사 성공 시 체크 아이콘으로 2초간 변경
   - 토스트 알림 및 콘솔 로그 기록

5. **주소 생성 버튼 완료**
   - `deposit_address`가 null인 경우 "주소 생성" 버튼 표시
   - `wts_generate_deposit_address` API 호출
   - 생성 요청 후 1초 대기 후 자동 재조회

6. **테스트 작성 완료**
   - transferStore: 14개 새 테스트 추가 (총 34개)
   - TransferPanel: 8개 새 테스트 추가 (총 33개)

### Change Log

- 2026-01-24: WTS-4.3 입금 주소 표시 및 복사 기능 구현 완료

### File List

**Modified:**
- `apps/desktop/src/wts/stores/transferStore.ts` - 입금 주소 상태 및 액션 추가
- `apps/desktop/src/wts/panels/TransferPanel.tsx` - 입금 주소 표시, 복사, 생성 UI 추가
- `apps/desktop/src/wts/__tests__/stores/transferStore.test.ts` - 입금 주소 상태 테스트 14개 추가
- `apps/desktop/src/wts/__tests__/panels/TransferPanel.test.tsx` - 입금 주소 UI 테스트 8개 추가
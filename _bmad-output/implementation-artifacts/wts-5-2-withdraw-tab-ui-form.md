# Story WTS-5.2: 출금 탭 UI (자산/네트워크/주소/수량 입력)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **출금에 필요한 모든 정보를 입력하는 폼**,
So that **출금 조건을 정확하게 설정할 수 있다**.

## Acceptance Criteria

1. **Given** Transfer 패널의 출금 탭이 선택되어 있을 때 **When** 화면이 렌더링되면 **Then** 자산 선택 드롭다운이 표시되어야 한다
2. **Given** 자산이 선택되어 있을 때 **When** 화면이 렌더링되면 **Then** 해당 자산의 네트워크 선택 버튼이 표시되어야 한다
3. **Given** 네트워크가 선택되어 있을 때 **When** 출금 가능 정보를 조회하면 **Then** Upbit에 등록된 출금 주소 목록이 드롭다운으로 표시되어야 한다
4. **Given** 출금 주소가 선택되어 있을 때 **When** 화면이 렌더링되면 **Then** 출금 수량 입력 필드가 표시되어야 한다
5. **Given** 출금 가능 정보가 조회되었을 때 **When** 화면이 렌더링되면 **Then** 출금 가능 잔고(balance - locked)가 표시되어야 한다
6. **Given** 출금 가능 정보가 조회되었을 때 **When** 화면이 렌더링되면 **Then** 출금 수수료(withdraw_fee)가 표시되어야 한다
7. **Given** 출금 가능 정보가 조회되었을 때 **When** 화면이 렌더링되면 **Then** 최소 출금 수량(minimum)이 안내로 표시되어야 한다
8. **Given** 수량 입력 필드가 표시되어 있을 때 **When** % 버튼(25%, 50%, 75%, MAX)을 클릭하면 **Then** 출금 가능 잔고 기준으로 해당 비율의 수량이 자동 입력되어야 한다
9. **Given** 등록된 출금 주소가 없을 때 **When** 화면이 렌더링되면 **Then** "Upbit에서 출금 주소를 먼저 등록해주세요" 안내와 등록 가이드 링크가 표시되어야 한다
10. **Given** 출금 불가 상태일 때(can_withdraw=false) **When** 화면이 렌더링되면 **Then** 출금 버튼이 비활성화되고 사유가 표시되어야 한다

## Tasks / Subtasks

- [x] Task 1: transferStore 출금 상태 확장 (AC: #1-#10)
  - [x] Subtask 1.1: 출금 가능 정보 상태 추가 (withdrawChanceInfo: WithdrawChanceResponse | null)
  - [x] Subtask 1.2: 등록된 출금 주소 목록 상태 추가 (withdrawAddresses: WithdrawAddressResponse[])
  - [x] Subtask 1.3: 선택된 출금 주소 상태 추가 (selectedWithdrawAddress: WithdrawAddressResponse | null)
  - [x] Subtask 1.4: 출금 수량 상태 추가 (withdrawAmount: string)
  - [x] Subtask 1.5: 출금 관련 로딩/에러 상태 추가 (isWithdrawLoading, withdrawError)
  - [x] Subtask 1.6: 출금 상태 액션 함수 추가 (setWithdrawChanceInfo, setWithdrawAddresses, setSelectedWithdrawAddress, setWithdrawAmount, resetWithdrawState)
  - [x] Subtask 1.7: 기존 setSelectedCurrency, setSelectedNetwork에서 출금 상태도 초기화

- [x] Task 2: 출금 가능 자산 목록 정의 (AC: #1)
  - [x] Subtask 2.1: WITHDRAW_CURRENCIES 상수 정의 (입금과 동일 자산 목록 사용)
  - [x] Subtask 2.2: 자산별 네트워크 매핑 검증

- [x] Task 3: 출금 자산/네트워크 선택 UI 구현 (AC: #1, #2)
  - [x] Subtask 3.1: 출금 탭 자산 선택 드롭다운 구현 (입금 탭과 동일 패턴)
  - [x] Subtask 3.2: 네트워크 선택 버튼 그룹 구현 (입금 탭과 동일 패턴)
  - [x] Subtask 3.3: 자산 선택 시 handleWithdrawCurrencyChange 핸들러 구현
  - [x] Subtask 3.4: 네트워크 선택 시 fetchWithdrawChance 함수 호출

- [x] Task 4: 출금 가능 정보 API 연동 (AC: #5, #6, #7, #10)
  - [x] Subtask 4.1: fetchWithdrawChance 함수 구현 (wts_get_withdraw_chance 호출)
  - [x] Subtask 4.2: 응답 데이터 transferStore에 저장
  - [x] Subtask 4.3: 콘솔 로그 기록 (성공/실패)
  - [x] Subtask 4.4: 출금 가능 정보 조회 후 fetchWithdrawAddresses 호출

- [x] Task 5: 등록된 출금 주소 API 연동 (AC: #3, #9)
  - [x] Subtask 5.1: fetchWithdrawAddresses 함수 구현 (wts_get_withdraw_addresses 호출)
  - [x] Subtask 5.2: 주소 목록 transferStore에 저장
  - [x] Subtask 5.3: 주소가 없을 경우 빈 배열 처리 + 안내 메시지 플래그

- [x] Task 6: 출금 주소 선택 UI 구현 (AC: #3, #9)
  - [x] Subtask 6.1: 등록된 주소 드롭다운 구현 (주소 앞뒤 말줄임 표시)
  - [x] Subtask 6.2: 주소 선택 시 handleSelectWithdrawAddress 핸들러 구현
  - [x] Subtask 6.3: 보조 주소(secondary_address) 있는 경우 함께 표시
  - [x] Subtask 6.4: 주소 없음 안내 UI 구현 ("Upbit에서 출금 주소를 먼저 등록해주세요")
  - [x] Subtask 6.5: Upbit 출금 주소 등록 가이드 링크 추가 (https://upbit.com/mypage/address)

- [x] Task 7: 출금 수량 입력 UI 구현 (AC: #4, #8)
  - [x] Subtask 7.1: 수량 입력 필드 구현 (숫자 입력, 소수점 지원)
  - [x] Subtask 7.2: % 버튼 그룹 구현 (25%, 50%, 75%, MAX)
  - [x] Subtask 7.3: handlePercentClick 핸들러 구현 (가용 잔고 * 비율 계산)
  - [x] Subtask 7.4: 입력값 유효성 검사 (최소 수량, 최대 수량, 일일 한도)

- [x] Task 8: 출금 정보 표시 UI 구현 (AC: #5, #6, #7, #10)
  - [x] Subtask 8.1: 출금 가능 잔고 표시 (balance - locked)
  - [x] Subtask 8.2: 출금 수수료 표시 (currency_info.withdraw_fee)
  - [x] Subtask 8.3: 최소 출금 수량 표시 (withdraw_limit.minimum)
  - [x] Subtask 8.4: 1회 최대 출금 표시 (withdraw_limit.onetime)
  - [x] Subtask 8.5: 일일 잔여 한도 표시 (withdraw_limit.remaining_daily)
  - [x] Subtask 8.6: 지갑 상태 표시 (currency_info.wallet_state - working/paused)
  - [x] Subtask 8.7: 출금 불가 시 사유 표시 + 버튼 비활성화

- [x] Task 9: 실수령액 계산 및 표시 (AC: #6)
  - [x] Subtask 9.1: 실수령액 계산 로직 구현 (입력 수량 - 수수료)
  - [x] Subtask 9.2: 실수령액 실시간 표시 UI 구현
  - [x] Subtask 9.3: 수수료 차감 후 마이너스 되는 경우 경고 표시

- [x] Task 10: 출금 버튼 구현 (AC: #10)
  - [x] Subtask 10.1: 출금 버튼 UI 구현 (wts-accent 스타일)
  - [x] Subtask 10.2: 버튼 활성화 조건 검증 (주소 선택됨, 수량 입력됨, can_withdraw=true, 최소 수량 이상)
  - [x] Subtask 10.3: 버튼 클릭 시 onWithdrawClick 콜백 호출 (WTS-5.3 확인 다이얼로그 연결용)

- [x] Task 11: 단위 테스트 작성 (AC: #1-#10)
  - [x] Subtask 11.1: transferStore 출금 상태 테스트
  - [x] Subtask 11.2: TransferPanel 출금 탭 렌더링 테스트
  - [x] Subtask 11.3: % 버튼 수량 계산 테스트
  - [x] Subtask 11.4: 버튼 활성화 조건 테스트
  - [x] Subtask 11.5: 주소 없음 안내 메시지 테스트

## Dev Notes

### 기존 TransferPanel.tsx 구조

[Source: apps/desktop/src/wts/panels/TransferPanel.tsx]

현재 출금 탭은 "출금 기능은 준비 중입니다" 플레이스홀더로 구현됨. 입금 탭 패턴을 참조하여 동일한 스타일로 출금 UI 구현 필요.

**입금 탭 패턴:**
1. 자산 선택 드롭다운 (`DEPOSIT_CURRENCIES` 상수 사용)
2. 네트워크 선택 버튼 그룹
3. 네트워크 정보 표시 (조회 결과)
4. 입금 주소 표시/생성

**출금 탭 구현 순서:**
1. 자산 선택 드롭다운 (입금과 동일)
2. 네트워크 선택 버튼 그룹 (입금과 동일)
3. 출금 가능 정보 표시 (잔고, 수수료, 한도)
4. 등록된 출금 주소 드롭다운 (API 조회)
5. 수량 입력 + % 버튼
6. 실수령액 계산 표시
7. 출금 버튼

### 출금 가능 정보 응답 구조 (WithdrawChanceResponse)

[Source: apps/desktop/src/wts/types.ts]

```typescript
interface WithdrawChanceResponse {
  currency: string;
  net_type: string;
  member_level: WithdrawMemberLevel;
  currency_info: WithdrawCurrencyInfo;   // withdraw_fee, wallet_state
  account_info: WithdrawAccountInfo;     // balance, locked
  withdraw_limit: WithdrawLimitInfo;     // minimum, onetime, daily, remaining_daily, can_withdraw
}
```

**주요 필드:**
- `account_info.balance`: 총 잔고
- `account_info.locked`: 잠금 잔고 (미체결 주문)
- 가용 잔고 = balance - locked
- `currency_info.withdraw_fee`: 출금 수수료
- `currency_info.wallet_state`: "working" | "paused" | "suspended"
- `withdraw_limit.minimum`: 최소 출금 수량
- `withdraw_limit.onetime`: 1회 최대 출금
- `withdraw_limit.daily`: 일일 한도
- `withdraw_limit.remaining_daily`: 일일 잔여 한도
- `withdraw_limit.can_withdraw`: 출금 가능 여부

### 등록된 출금 주소 응답 구조 (WithdrawAddressResponse)

```typescript
interface WithdrawAddressResponse {
  currency: string;
  net_type: string;
  network_name: string;
  withdraw_address: string;
  secondary_address: string | null;  // XRP tag, EOS memo 등
}
```

**중요:** Upbit은 출금 주소 사전 등록 필수. API로는 등록된 주소만 조회 가능.

### Tauri 명령 호출 패턴

[Source: apps/desktop/src/wts/panels/TransferPanel.tsx]

```typescript
// 출금 가능 정보 조회
const result = await invoke<WtsApiResult<WithdrawChanceResponse>>(
  'wts_get_withdraw_chance',
  { params: { currency, net_type } }
);

// 등록된 출금 주소 조회
const result = await invoke<WtsApiResult<WithdrawAddressResponse[]>>(
  'wts_get_withdraw_addresses',
  { params: { currency, net_type } }
);
```

### TransferStore 확장 패턴

[Source: apps/desktop/src/wts/stores/transferStore.ts]

```typescript
// 추가할 출금 관련 상태
interface TransferState {
  // ... 기존 입금 상태 ...

  // 출금 상태 추가
  withdrawChanceInfo: WithdrawChanceResponse | null;
  withdrawAddresses: WithdrawAddressResponse[];
  selectedWithdrawAddress: WithdrawAddressResponse | null;
  withdrawAmount: string;
  isWithdrawLoading: boolean;
  withdrawError: string | null;

  // 출금 액션 추가
  setWithdrawChanceInfo: (info: WithdrawChanceResponse | null) => void;
  setWithdrawAddresses: (addresses: WithdrawAddressResponse[]) => void;
  setSelectedWithdrawAddress: (address: WithdrawAddressResponse | null) => void;
  setWithdrawAmount: (amount: string) => void;
  setWithdrawLoading: (loading: boolean) => void;
  setWithdrawError: (error: string | null) => void;
  resetWithdrawState: () => void;
}
```

### % 버튼 수량 계산 로직

```typescript
const handlePercentClick = (percent: number) => {
  if (!withdrawChanceInfo) return;

  const balance = parseFloat(withdrawChanceInfo.account_info.balance);
  const locked = parseFloat(withdrawChanceInfo.account_info.locked);
  const available = balance - locked;

  if (available <= 0) {
    addLog('WARN', 'WITHDRAW', '출금 가능 잔고가 없습니다');
    return;
  }

  const amount = percent === 100
    ? available
    : available * (percent / 100);

  // 소수점 정밀도 적용 (withdraw_limit.fixed)
  const fixed = withdrawChanceInfo.withdraw_limit.fixed;
  const formattedAmount = amount.toFixed(fixed);

  setWithdrawAmount(formattedAmount);
};
```

### 출금 버튼 활성화 조건

```typescript
const isWithdrawButtonEnabled = useMemo(() => {
  if (!withdrawChanceInfo) return false;
  if (!selectedWithdrawAddress) return false;
  if (!withdrawAmount || parseFloat(withdrawAmount) <= 0) return false;
  if (!withdrawChanceInfo.withdraw_limit.can_withdraw) return false;

  const amount = parseFloat(withdrawAmount);
  const minimum = parseFloat(withdrawChanceInfo.withdraw_limit.minimum);
  const available = parseFloat(withdrawChanceInfo.account_info.balance) -
                    parseFloat(withdrawChanceInfo.account_info.locked);

  if (amount < minimum) return false;
  if (amount > available) return false;

  return true;
}, [withdrawChanceInfo, selectedWithdrawAddress, withdrawAmount]);
```

### 실수령액 계산

```typescript
const receivableAmount = useMemo(() => {
  if (!withdrawChanceInfo || !withdrawAmount) return null;

  const amount = parseFloat(withdrawAmount);
  const fee = parseFloat(withdrawChanceInfo.currency_info.withdraw_fee);
  const result = amount - fee;

  return result > 0 ? result.toFixed(withdrawChanceInfo.withdraw_limit.fixed) : '0';
}, [withdrawChanceInfo, withdrawAmount]);
```

### UI 스타일 패턴

[Source: apps/desktop/src/wts/panels/TransferPanel.tsx]

입금 탭과 동일한 Tailwind 클래스 사용:
- 드롭다운: `w-full px-3 py-2 rounded border border-wts bg-wts-secondary text-wts-foreground text-sm focus:outline-none focus:border-wts-focus`
- 네트워크 버튼: `px-3 py-1 text-xs rounded border transition-colors` + 선택 상태 스타일
- 정보 카드: `p-3 rounded bg-wts-tertiary text-xs space-y-2`
- % 버튼: `px-2 py-1 text-xs rounded bg-wts-secondary hover:bg-wts-tertiary`
- 출금 버튼: `w-full py-2 rounded font-medium bg-wts-accent text-white disabled:opacity-50`

### 에러 처리 패턴

[Source: apps/desktop/src/wts/utils/errorHandler.ts]

```typescript
import { handleApiError } from '../utils/errorHandler';

// API 호출 실패 시
handleApiError(result.error, 'WITHDRAW', '출금 정보 조회 실패');
```

### 주소 등록 가이드 링크

Upbit 출금 주소 등록 페이지: `https://upbit.com/mypage/address`

### Project Structure Notes

**수정 파일:**
- `apps/desktop/src/wts/stores/transferStore.ts` - 출금 상태 추가
- `apps/desktop/src/wts/panels/TransferPanel.tsx` - 출금 탭 UI 구현

**테스트 파일:**
- `apps/desktop/src/wts/__tests__/stores/transferStore.test.ts` - 출금 상태 테스트 추가
- `apps/desktop/src/wts/__tests__/panels/TransferPanel.test.tsx` - 출금 탭 테스트 추가

**아키텍처 정합성:**
- WTS 패널 구조 준수 (`wts-panel`, `wts-panel-header`, `wts-panel-content`)
- Zustand 스토어 패턴 준수 (`useTransferStore`)
- 콘솔 로깅 패턴 준수 (`addLog('WITHDRAW', ...)`)
- 에러 처리 패턴 준수 (`handleApiError`)
- Tauri invoke 패턴 준수 (`WtsApiResult<T>`)

### 이전 스토리 참조

**WTS-5.1 (출금 API Rust 백엔드):**
- Tauri 명령 구현 완료: `wts_withdraw`, `wts_get_withdraw_chance`, `wts_get_withdraw_addresses`, `wts_get_withdraw`
- TypeScript 타입 동기화 완료
- 에러 코드 한국어 매핑 완료

**WTS-4.2 (입금 탭 UI):**
- 입금 탭 UI 패턴 참조
- TransferPanel 탭 구조 참조
- 자산/네트워크 선택 UI 패턴 참조

### WTS-5.3 연결 포인트

출금 버튼 클릭 시 확인 다이얼로그 표시는 WTS-5.3에서 구현.
이 스토리에서는 버튼 클릭 시 호출할 `onWithdrawClick` 콜백 준비만 해두면 됨.

```typescript
// TransferPanel props에 추가
interface TransferPanelProps {
  className?: string;
  onWithdrawClick?: (params: WithdrawParams) => void;  // WTS-5.3 연결용
}
```

### References

- [Architecture: WTS Frontend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Frontend Structure)
- [Architecture: Transfer Panel](/_bmad-output/planning-artifacts/architecture.md#TransferPanel)
- [PRD: FR21-24 출금 UI](/_bmad-output/planning-artifacts/prd.md)
- [WTS Epics: Epic 5 Story 5.2](/_bmad-output/planning-artifacts/wts-epics.md#Story 5.2)
- [Previous Story: WTS-5.1 출금 API 백엔드](/_bmad-output/implementation-artifacts/wts-5-1-withdraw-api-rust-backend.md)
- [Previous Story: WTS-4.2 입금 탭 UI](/_bmad-output/implementation-artifacts/wts-4-2-deposit-tab-ui-asset-network.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

### Completion Notes List

- transferStore에 출금 관련 상태 6개 추가 (withdrawChanceInfo, withdrawAddresses, selectedWithdrawAddress, withdrawAmount, isWithdrawLoading, withdrawError)
- transferStore에 출금 관련 액션 7개 추가 (setWithdrawChanceInfo, setWithdrawAddresses, setSelectedWithdrawAddress, setWithdrawAmount, setWithdrawLoading, setWithdrawError, resetWithdrawState)
- setSelectedCurrency, setSelectedNetwork에서 출금 상태도 초기화하도록 수정
- WITHDRAW_CURRENCIES 상수 정의 (DEPOSIT_CURRENCIES와 동일)
- TransferPanel에 출금 탭 전체 UI 구현:
  - 자산 선택 드롭다운
  - 네트워크 선택 버튼 그룹
  - 출금 가능 정보 표시 (잔고, 수수료, 한도, 지갑 상태)
  - 등록된 출금 주소 드롭다운 (주소 없을 시 안내 메시지 + Upbit 링크)
  - 수량 입력 필드 + % 버튼 (25%, 50%, 75%, MAX)
  - 실수령액 실시간 표시
  - 출금 버튼 (활성화 조건 검증)
- API 연동 함수 구현: fetchWithdrawChance, fetchWithdrawAddresses
- 출금 버튼 클릭 시 onWithdrawClick 콜백 호출 (WTS-5.3 연결용)
- transferStore 테스트 23개 추가 (총 64개)
- TransferPanel 테스트 10개 추가 (총 28개)

### File List

- apps/desktop/src/wts/stores/transferStore.ts (수정)
- apps/desktop/src/wts/panels/TransferPanel.tsx (수정)
- apps/desktop/src/wts/__tests__/stores/transferStore.test.ts (수정)
- apps/desktop/src/wts/__tests__/panels/TransferPanel.test.tsx (수정)

### Senior Developer Review (AI)

- **Zustand 상태 초기화 최적화**: `setSelectedCurrency`, `setSelectedNetwork` 내 중복된 출금 상태 초기화 로직을 `resetWithdrawState` 패턴으로 통일하여 코드 응집도를 높임.
- **수량 계산 안정성 강화**: `%` 버튼 클릭 시 부동 소수점 오차로 인해 가용 잔고를 초과하는 수치가 입력되지 않도록 `Math.min` 보정 로직 추가.
- **UI 피드백 개선**: 출금 가능 정보 및 주소 조회 중 네트워크 선택 버튼을 비활성화하여 중복 요청 방지.
- **테스트 커버리지 보완**: 실수령액이 0 이하인 경우의 UI 렌더링 및 경고 메시지 표시 여부를 확인하는 테스트 케이스 추가.

### Change Log

- 2026-01-25: WTS-5.2 출금 탭 UI 구현 완료
- 2026-01-25: 코드 리뷰 지적 사항(Zustand 중복, 계산 정밀도, UI 피드백) 수정 완료

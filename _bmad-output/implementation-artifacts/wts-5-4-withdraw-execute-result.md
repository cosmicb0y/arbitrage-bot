# Story WTS-5.4: 출금 실행 및 결과 처리

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **출금을 실행하고 결과를 확인하는 기능**,
So that **출금 상태를 추적할 수 있다**.

## Acceptance Criteria

1. **Given** 출금 확인 다이얼로그에서 확인을 클릭했을 때 **When** 출금 API가 호출되면 **Then** 출금 요청이 전송되고 로딩 상태가 표시되어야 한다
2. **Given** 출금 API 호출이 성공했을 때 **When** 응답이 수신되면 **Then** 출금 UUID와 상태(state)가 콘솔에 기록되어야 한다
3. **Given** 출금 API 호출이 성공했을 때 **When** 응답이 수신되면 **Then** 성공 토스트 알림이 표시되어야 한다
4. **Given** 출금 API 호출이 성공했을 때 **When** 응답이 수신되면 **Then** 잔고가 자동으로 갱신되어야 한다
5. **Given** 출금이 성공적으로 제출되었을 때 **When** 결과 화면이 표시되면 **Then** 출금 상태(submitting/submitted/processing)와 예상 완료 안내가 표시되어야 한다
6. **Given** 출금 결과가 표시되었을 때 **When** TXID가 아직 없으면(null) **Then** "블록체인 전송 대기 중" 메시지가 표시되어야 한다
7. **Given** 출금 상태를 확인하고 싶을 때 **When** 상태 조회 버튼을 클릭하면 **Then** wts_get_withdraw API를 통해 최신 상태를 조회해야 한다
8. **Given** 출금 상태 조회가 완료되었을 때 **When** TXID가 생성되었으면 **Then** TXID가 콘솔에 기록되고 복사 가능해야 한다

## Tasks / Subtasks

- [x] Task 1: 출금 결과 다이얼로그 상태 추가 (AC: #1, #5-#6)
  - [x] Subtask 1.1: WithdrawResultInfo 타입 정의 (uuid, state, currency, amount, txid 등)
  - [x] Subtask 1.2: WtsWindow에 출금 결과 다이얼로그 상태 추가 (isWithdrawResultOpen, withdrawResult)
  - [x] Subtask 1.3: handleWithdrawConfirm에서 성공 시 결과 다이얼로그 표시 로직 추가

- [x] Task 2: WithdrawResultDialog 컴포넌트 구현 (AC: #5-#6, #8)
  - [x] Subtask 2.1: WithdrawResultDialogProps 인터페이스 정의
  - [x] Subtask 2.2: 다이얼로그 기본 레이아웃 구현 (성공 상태 표시)
  - [x] Subtask 2.3: 출금 상태별 UI 표시 (submitting→processing→done)
  - [x] Subtask 2.4: TXID 표시 영역 구현 (없으면 "블록체인 전송 대기 중")
  - [x] Subtask 2.5: TXID 복사 버튼 구현
  - [x] Subtask 2.6: "상태 확인" 버튼 구현 (wts_get_withdraw 호출)
  - [x] Subtask 2.7: 닫기 버튼 구현

- [x] Task 3: 출금 상태 조회 기능 구현 (AC: #7-#8)
  - [x] Subtask 3.1: handleCheckWithdrawStatus 핸들러 구현 (WtsWindow에서 직접 관리)
  - [x] Subtask 3.2: wts_get_withdraw Tauri invoke 호출 구현
  - [x] Subtask 3.3: 상태 조회 결과로 withdrawResult 업데이트
  - [x] Subtask 3.4: TXID 생성 시 콘솔 로그 기록

- [x] Task 4: 콘솔 로그 개선 (AC: #2, #8)
  - [x] Subtask 4.1: 출금 성공 로그에 상태 정보 추가
  - [x] Subtask 4.2: TXID 생성 시 별도 INFO 로그 추가
  - [x] Subtask 4.3: 출금 상태별 한국어 메시지 매핑 추가

- [x] Task 5: 단위 테스트 작성 (AC: #1-#8)
  - [x] Subtask 5.1: WithdrawResultDialog 렌더링 테스트
  - [x] Subtask 5.2: 출금 상태별 UI 표시 테스트
  - [x] Subtask 5.3: TXID 복사 기능 테스트
  - [x] Subtask 5.4: 상태 조회 버튼 테스트
  - [x] Subtask 5.5: WithdrawResultInfo 타입 및 WITHDRAW_STATE_MESSAGES 테스트

## Dev Notes

### WTS-5.3에서 이미 구현된 내용

[Source: apps/desktop/src/wts/WtsWindow.tsx:77-112]

WTS-5.3에서 출금 확인 다이얼로그의 확인 버튼 클릭 시:
- `wts_withdraw` API 호출
- 성공 시 UUID만 로그에 기록 (`출금 요청 완료: ${result.data.uuid}`)
- 토스트 알림 표시
- 잔고 갱신 (`fetchBalance()`)
- 다이얼로그 닫기

**이 스토리에서 추가할 내용:**
- 출금 결과 다이얼로그 (성공 후 상세 정보 표시)
- 출금 상태(state) 로그 기록
- TXID 표시 및 복사 기능
- 상태 조회 기능

### 출금 상태 (WithdrawState) 한국어 매핑

[Source: apps/desktop/src/wts/types.ts:599-607]

```typescript
export type WithdrawState =
  | 'submitting'    // 제출 중
  | 'submitted'     // 제출됨
  | 'almost_accepted' // 거의 승인됨
  | 'rejected'      // 거부됨
  | 'accepted'      // 승인됨
  | 'processing'    // 처리 중
  | 'done'          // 완료
  | 'canceled';     // 취소됨
```

```typescript
// utils/withdrawStatus.ts에 추가
export const WITHDRAW_STATE_MESSAGES: Record<WithdrawState, string> = {
  submitting: '출금 요청 제출 중...',
  submitted: '출금 요청이 제출되었습니다',
  almost_accepted: '출금 요청이 곧 승인됩니다',
  accepted: '출금 요청이 승인되었습니다',
  processing: '블록체인 전송 처리 중...',
  done: '출금이 완료되었습니다',
  rejected: '출금 요청이 거부되었습니다',
  canceled: '출금이 취소되었습니다',
};
```

### WithdrawResultInfo 타입 정의

```typescript
// types.ts에 추가
export interface WithdrawResultInfo {
  /** 출금 고유 식별자 */
  uuid: string;
  /** 자산 코드 */
  currency: string;
  /** 네트워크 타입 */
  net_type: string;
  /** 출금 상태 */
  state: WithdrawState;
  /** 출금 수량 */
  amount: string;
  /** 출금 수수료 */
  fee: string;
  /** 트랜잭션 ID (블록체인 TXID, 처리 전에는 null) */
  txid: string | null;
  /** 출금 생성 시각 */
  created_at: string;
}
```

### WithdrawResultDialog 컴포넌트 구조

```typescript
// components/WithdrawResultDialog.tsx
interface WithdrawResultDialogProps {
  isOpen: boolean;
  result: WithdrawResultInfo;
  onClose: () => void;
  onCheckStatus: () => Promise<void>;
  isCheckingStatus: boolean;
}

function WithdrawResultDialog({
  isOpen,
  result,
  onClose,
  onCheckStatus,
  isCheckingStatus,
}: WithdrawResultDialogProps) {
  // ...
}
```

**UI 구조:**
```
┌─────────────────────────────────────┐
│  ✅ 출금 요청 완료                  │
├─────────────────────────────────────┤
│  자산: BTC                          │
│  네트워크: BTC                      │
│  수량: 0.01 BTC                     │
│  수수료: 0.0005 BTC                 │
│  ─────────────────────────────────  │
│  상태: 처리 중 🔄                   │
│  ─────────────────────────────────  │
│  TXID: 블록체인 전송 대기 중...     │
│        (또는 TXID 값 + 복사 버튼)   │
├─────────────────────────────────────┤
│  [상태 확인]           [닫기]       │
└─────────────────────────────────────┘
```

### WtsWindow 상태 관리 추가

[Source: apps/desktop/src/wts/WtsWindow.tsx]

```typescript
// 출금 결과 다이얼로그 상태 추가
const [isWithdrawResultOpen, setIsWithdrawResultOpen] = useState(false);
const [withdrawResult, setWithdrawResult] = useState<WithdrawResultInfo | null>(null);
const [isCheckingWithdrawStatus, setIsCheckingWithdrawStatus] = useState(false);

// handleWithdrawConfirm 수정 (성공 시)
if (result.success && result.data) {
  const stateMessage = WITHDRAW_STATE_MESSAGES[result.data.state as WithdrawState] || result.data.state;
  addLog(
    'SUCCESS',
    'WITHDRAW',
    `출금 요청 완료: ${result.data.uuid} (${stateMessage})`
  );
  showToast('success', '출금 요청이 완료되었습니다');
  fetchBalance();

  // 확인 다이얼로그 닫고 결과 다이얼로그 표시
  setIsWithdrawDialogOpen(false);
  setWithdrawConfirmInfo(null);
  setWithdrawResult({
    uuid: result.data.uuid,
    currency: withdrawConfirmInfo.currency,
    net_type: withdrawConfirmInfo.net_type,
    state: result.data.state,
    amount: result.data.amount,
    fee: result.data.fee,
    txid: result.data.txid,
    created_at: result.data.created_at,
  });
  setIsWithdrawResultOpen(true);
}
```

### 출금 상태 조회 핸들러

```typescript
// 출금 상태 조회
const handleCheckWithdrawStatus = useCallback(async () => {
  if (!withdrawResult) return;

  setIsCheckingWithdrawStatus(true);
  try {
    const result = await invoke<WtsApiResult<WithdrawResponse>>('wts_get_withdraw', {
      params: { uuid: withdrawResult.uuid }
    });

    if (result.success && result.data) {
      const prevTxid = withdrawResult.txid;
      const newTxid = result.data.txid;

      // TXID가 새로 생성된 경우 로그 기록
      if (!prevTxid && newTxid) {
        addLog('INFO', 'WITHDRAW', `TXID 생성됨: ${newTxid}`);
      }

      setWithdrawResult(prev => prev ? {
        ...prev,
        state: result.data.state,
        txid: result.data.txid,
      } : null);

      const stateMessage = WITHDRAW_STATE_MESSAGES[result.data.state as WithdrawState];
      addLog('INFO', 'WITHDRAW', `출금 상태: ${stateMessage}`);
    } else {
      handleApiError(result.error, 'WITHDRAW', '상태 조회 실패');
    }
  } catch (error) {
    addLog('ERROR', 'WITHDRAW', `상태 조회 실패: ${error}`);
  } finally {
    setIsCheckingWithdrawStatus(false);
  }
}, [withdrawResult, addLog]);
```

### 기존 wts_get_withdraw Tauri 명령

[Source: apps/desktop/src-tauri/src/wts/mod.rs:228-234]

백엔드에 이미 구현되어 있음:
```rust
#[tauri::command]
pub async fn wts_get_withdraw(params: GetWithdrawParams) -> WtsApiResult<WithdrawResponse> {
    match upbit::get_withdraw(params).await {
        Ok(withdraw) => WtsApiResult::ok(withdraw),
        Err(e) => WtsApiResult::err(e),
    }
}
```

[Source: apps/desktop/src/wts/types.ts:682-688]

프론트엔드 타입도 이미 정의되어 있음:
```typescript
export interface GetWithdrawParams {
  uuid?: string;
  txid?: string;
}
```

### TXID 복사 기능

```typescript
const handleCopyTxid = useCallback(async () => {
  if (!withdrawResult?.txid) return;

  try {
    await navigator.clipboard.writeText(withdrawResult.txid);
    showToast('success', 'TXID가 복사되었습니다');
  } catch {
    showToast('error', '복사에 실패했습니다');
  }
}, [withdrawResult?.txid, showToast]);
```

### UI 스타일 패턴

[Source: apps/desktop/src/wts/components/WithdrawConfirmDialog.tsx]

기존 출금 확인 다이얼로그 스타일 재사용:
- 오버레이: `fixed inset-0 z-50 flex items-center justify-center bg-black/60`
- 다이얼로그: `bg-wts-secondary border border-wts rounded-lg shadow-xl`
- 헤더: `px-4 py-3 border-b border-wts-accent/50`
- 버튼: `bg-wts-accent hover:bg-wts-accent/80`

**출금 상태별 색상:**
- submitting/submitted/processing: `text-yellow-400` (진행 중)
- done: `text-green-400` (완료)
- rejected/canceled: `text-red-400` (실패)

### Project Structure Notes

**수정 파일:**
- `apps/desktop/src/wts/types.ts` - WithdrawResultInfo 타입 추가
- `apps/desktop/src/wts/WtsWindow.tsx` - 출금 결과 다이얼로그 상태 및 핸들러 추가
- `apps/desktop/src/wts/components/WithdrawConfirmDialog.tsx` - (선택) 성공 후 결과 다이얼로그로 전환

**신규 생성 파일:**
- `apps/desktop/src/wts/components/WithdrawResultDialog.tsx` - 출금 결과 다이얼로그 컴포넌트
- `apps/desktop/src/wts/utils/withdrawStatus.ts` - 출금 상태 유틸리티 (한국어 매핑)
- `apps/desktop/src/wts/__tests__/components/WithdrawResultDialog.test.tsx` - 테스트

**아키텍처 정합성:**
- WTS 컴포넌트 구조 준수 (`wts/components/`)
- 기존 다이얼로그 패턴 확장
- 콘솔 로깅 패턴 준수 (`addLog('WITHDRAW', ...)`)
- Tauri invoke 패턴 준수 (`WtsApiResult<T>`)

### 이전 스토리 참조

**WTS-5.1 (출금 API Rust 백엔드):**
- `wts_withdraw` Tauri 명령 구현 완료
- `wts_get_withdraw` Tauri 명령 구현 완료
- WithdrawParams, WithdrawResponse, GetWithdrawParams 타입 정의 완료

**WTS-5.2 (출금 탭 UI):**
- TransferPanel에 출금 폼 구현 완료
- onWithdrawClick 핸들러 연결 완료

**WTS-5.3 (출금 확인 다이얼로그):**
- WithdrawConfirmDialog 컴포넌트 구현 완료
- handleWithdrawConfirm에서 wts_withdraw 호출 완료
- 성공/실패 시 콘솔 로그 및 토스트 표시 완료

### 다음 스토리 연결 (WTS-5.5, WTS-5.6)

**WTS-5.5 (2FA 및 출금 제한 안내):**
- 출금 실패 시 2FA 필요 에러 처리
- 이 스토리에서는 기본 에러 처리만 구현, 5.5에서 상세 안내 추가

**WTS-5.6 (출금 에러 처리 및 네트워크 오류 대응):**
- 네트워크 오류 시 재시도 로직
- 이 스토리에서는 기본 에러 표시만 구현, 5.6에서 재시도 로직 추가

### References

- [Architecture: WTS Frontend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Frontend Structure)
- [Architecture: Error Handling Flow](/_bmad-output/planning-artifacts/architecture.md#Error Handling Flow)
- [PRD: FR26 출금 실행](/_bmad-output/planning-artifacts/prd.md)
- [WTS Epics: Epic 5 Story 5.4](/_bmad-output/planning-artifacts/wts-epics.md#Story 5.4)
- [Previous Story: WTS-5.3 출금 확인 다이얼로그](/_bmad-output/implementation-artifacts/wts-5-3-withdraw-confirm-dialog.md)
- [Rust Backend: wts_get_withdraw](apps/desktop/src-tauri/src/wts/mod.rs:228-234)
- [TypeScript Types: WithdrawResponse](apps/desktop/src/wts/types.ts:573-596)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- 기존 errorHandler.test.ts 테스트 불일치 수정 (마침표 유무)

### Completion Notes List

1. **Task 1 완료**: WithdrawResultInfo 타입을 types.ts에 추가, WtsWindow에 출금 결과 다이얼로그 상태 추가 (isWithdrawResultOpen, withdrawResult, isCheckingWithdrawStatus)
2. **Task 2 완료**: WithdrawResultDialog 컴포넌트 구현 - 출금 상태별 색상 표시, TXID 표시 및 복사 기능, 상태 확인 버튼, 닫기 버튼
3. **Task 3 완료**: handleCheckWithdrawStatus 핸들러 구현 - wts_get_withdraw Tauri invoke 호출, TXID 생성 시 로그 기록
4. **Task 4 완료**: WITHDRAW_STATE_MESSAGES 상수 추가, 출금 성공 로그에 상태 정보 포함
5. **Task 5 완료**: WithdrawResultDialog 테스트 26개, types.ts 테스트 24개 추가 (총 101개 WTS-5.4 관련 테스트)
6. **리뷰 수정**: 출금 결과 다이얼로그에 예상 완료 안내 추가
7. **리뷰 수정**: 출금 성공/상태 조회 플로우 WtsWindow 테스트 추가

### File List

**신규 생성:**
- apps/desktop/src/wts/components/WithdrawConfirmDialog.tsx
- apps/desktop/src/wts/components/WithdrawResultDialog.tsx
- apps/desktop/src/wts/__tests__/components/WithdrawConfirmDialog.test.tsx
- apps/desktop/src/wts/__tests__/components/WithdrawResultDialog.test.tsx
- apps/desktop/src/wts/__tests__/WtsWindow.withdraw.test.tsx
- _bmad-output/implementation-artifacts/wts-5-3-withdraw-confirm-dialog.md
- _bmad-output/implementation-artifacts/wts-5-4-withdraw-execute-result.md

**수정:**
- apps/desktop/src/wts/types.ts (WithdrawResultInfo 타입, WITHDRAW_STATE_MESSAGES 상수 추가)
- apps/desktop/src/wts/WtsWindow.tsx (결과 다이얼로그 상태 및 핸들러 추가)
- apps/desktop/src/wts/components/WithdrawResultDialog.tsx (예상 완료 안내 추가)
- apps/desktop/src/wts/__tests__/types.test.ts (WithdrawResultInfo, WITHDRAW_STATE_MESSAGES 테스트 추가)
- apps/desktop/src/wts/__tests__/components/WithdrawResultDialog.test.tsx (예상 완료 안내 테스트 추가)
- apps/desktop/src/wts/__tests__/utils/errorHandler.test.ts (기존 테스트 불일치 수정)
- apps/desktop/src/wts/utils/errorHandler.ts (출금 Rate Limit 메시지 분리)
- _bmad-output/implementation-artifacts/sprint-status.yaml (스프린트 상태 동기화)

## Change Log

- 2026-01-25: WTS-5.4 출금 실행 및 결과 처리 구현 완료

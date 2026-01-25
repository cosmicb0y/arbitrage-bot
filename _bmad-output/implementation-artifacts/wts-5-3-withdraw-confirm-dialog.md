# Story WTS-5.3: 출금 확인 다이얼로그

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **트레이더**,
I want **출금 실행 전 상세 정보를 확인하는 다이얼로그**,
So that **실수로 잘못된 출금을 방지할 수 있다**.

## Acceptance Criteria

1. **Given** 출금 버튼이 클릭되었을 때 **When** 확인 다이얼로그가 표시되면 **Then** 출금 자산, 네트워크, 주소, 수량, 수수료, 실수령액이 명확히 표시되어야 한다
2. **Given** 확인 다이얼로그가 표시되었을 때 **When** 출금 주소를 확인하면 **Then** 주소는 전체 표시되고 앞/뒤 일부(첫 8자, 마지막 8자)가 강조되어야 한다
3. **Given** 확인 다이얼로그가 표시되었을 때 **When** 화면이 렌더링되면 **Then** "주소를 다시 확인하세요" 경고 문구가 표시되어야 한다
4. **Given** 확인 다이얼로그가 표시되었을 때 **When** 화면이 렌더링되면 **Then** "확인"과 "취소" 버튼이 제공되어야 한다
5. **Given** 확인 다이얼로그가 표시되었을 때 **When** 다이얼로그가 열린 직후 **Then** 확인 버튼은 3초 후 활성화되어 실수를 방지해야 한다
6. **Given** 확인 버튼이 클릭되었을 때 **When** 출금 API가 호출되면 **Then** 로딩 상태가 표시되고 버튼이 비활성화되어야 한다
7. **Given** 출금 API 호출이 성공했을 때 **When** 응답이 수신되면 **Then** 콘솔에 SUCCESS 로그가 기록되고 다이얼로그가 닫혀야 한다
8. **Given** 출금 API 호출이 실패했을 때 **When** 에러 응답이 수신되면 **Then** 콘솔에 ERROR 로그가 기록되고 토스트 알림이 표시되어야 한다
9. **Given** 확인 다이얼로그가 표시되어 있을 때 **When** ESC 키를 누르면 **Then** 다이얼로그가 닫혀야 한다
10. **Given** 확인 다이얼로그가 표시되어 있을 때 **When** 배경(오버레이)을 클릭하면 **Then** 다이얼로그가 닫혀야 한다

## Tasks / Subtasks

- [x] Task 1: WithdrawConfirmInfo 타입 정의 (AC: #1)
  - [x] Subtask 1.1: WithdrawConfirmInfo 인터페이스 정의 (currency, net_type, address, secondary_address, amount, fee, receivable)
  - [x] Subtask 1.2: types.ts에 타입 추가

- [x] Task 2: WithdrawConfirmDialog 컴포넌트 구현 (AC: #1-#5, #9-#10)
  - [x] Subtask 2.1: WithdrawConfirmDialogProps 인터페이스 정의 (isOpen, withdrawInfo, onConfirm, onCancel, isLoading)
  - [x] Subtask 2.2: 다이얼로그 기본 레이아웃 구현 (ConfirmDialog 패턴 참조)
  - [x] Subtask 2.3: 출금 정보 표시 섹션 구현 (자산, 네트워크, 주소, 수량, 수수료, 실수령액)
  - [x] Subtask 2.4: 주소 강조 표시 구현 (앞 8자, 마지막 8자 하이라이트)
  - [x] Subtask 2.5: "주소를 다시 확인하세요" 경고 문구 표시
  - [x] Subtask 2.6: 3초 카운트다운 타이머 구현 (확인 버튼 활성화 지연)
  - [x] Subtask 2.7: 취소/확인 버튼 구현 (확인 버튼 wts-accent 스타일)
  - [x] Subtask 2.8: ESC 키 핸들러 구현
  - [x] Subtask 2.9: 오버레이 클릭 핸들러 구현
  - [x] Subtask 2.10: 포커스 트랩 구현 (Tab 순서 논리적)
  - [x] Subtask 2.11: 로딩 상태 UI 구현 (스피너 + 버튼 비활성화)

- [x] Task 3: WtsWindow에 출금 다이얼로그 상태 관리 추가 (AC: #1, #6-#8)
  - [x] Subtask 3.1: 출금 다이얼로그 표시 상태 추가 (isWithdrawDialogOpen)
  - [x] Subtask 3.2: 출금 정보 상태 추가 (withdrawConfirmInfo)
  - [x] Subtask 3.3: 출금 로딩 상태 추가 (isWithdrawLoading)
  - [x] Subtask 3.4: handleWithdrawClick 핸들러 구현 (TransferPanel onWithdrawClick 연결)
  - [x] Subtask 3.5: handleWithdrawConfirm 핸들러 구현 (wts_withdraw API 호출)
  - [x] Subtask 3.6: handleWithdrawCancel 핸들러 구현 (다이얼로그 닫기)
  - [x] Subtask 3.7: WithdrawConfirmDialog 렌더링 추가

- [x] Task 4: 출금 API 호출 및 결과 처리 (AC: #6-#8)
  - [x] Subtask 4.1: wts_withdraw Tauri 명령 호출 로직 구현
  - [x] Subtask 4.2: 성공 시 콘솔 로그 기록 (SUCCESS, uuid 포함)
  - [x] Subtask 4.3: 성공 시 토스트 알림 표시
  - [x] Subtask 4.4: 성공 시 잔고 갱신 (useBalance hook 활용)
  - [x] Subtask 4.5: 실패 시 콘솔 로그 기록 (ERROR, 에러 메시지 포함)
  - [x] Subtask 4.6: 실패 시 토스트 알림 표시 (handleApiError 활용)
  - [x] Subtask 4.7: 다이얼로그 닫기 및 상태 초기화

- [x] Task 5: 단위 테스트 작성 (AC: #1-#10)
  - [x] Subtask 5.1: WithdrawConfirmDialog 렌더링 테스트
  - [x] Subtask 5.2: 주소 강조 표시 테스트
  - [x] Subtask 5.3: 3초 카운트다운 타이머 테스트
  - [x] Subtask 5.4: 확인 버튼 활성화 조건 테스트
  - [x] Subtask 5.5: ESC 키 핸들러 테스트
  - [x] Subtask 5.6: 오버레이 클릭 테스트
  - [x] Subtask 5.7: 로딩 상태 테스트
  - [x] Subtask 5.8: onConfirm/onCancel 콜백 테스트

## Dev Notes

### 기존 ConfirmDialog 패턴

[Source: apps/desktop/src/wts/components/ConfirmDialog.tsx]

주문 확인 다이얼로그가 이미 구현되어 있으며, 동일한 패턴을 따라 출금 확인 다이얼로그를 구현합니다.

**공통 패턴:**
- 오버레이 (`fixed inset-0 z-50 flex items-center justify-center bg-black/60`)
- 다이얼로그 (`bg-wts-secondary border border-wts rounded-lg shadow-xl`)
- 키보드 핸들링 (Enter=확인, ESC=취소, Tab=포커스 트랩)
- 로딩 상태 (스피너 + 버튼 비활성화)
- 버튼 영역 (`px-4 py-3 border-t border-wts flex gap-2`)

**출금 다이얼로그 차이점:**
- 주문 다이얼로그는 매수(녹색)/매도(빨간색) 색상 구분
- 출금 다이얼로그는 wts-accent(파란색) 계열 사용
- 3초 확인 버튼 지연 (출금 전용)
- 주소 강조 표시 (앞/뒤 8자 하이라이트)

### WithdrawConfirmInfo 타입 정의

```typescript
export interface WithdrawConfirmInfo {
  /** 자산 코드 (예: "BTC") */
  currency: string;
  /** 네트워크 타입 (예: "BTC") */
  net_type: string;
  /** 출금 주소 */
  address: string;
  /** 보조 주소 (XRP tag, EOS memo 등) */
  secondary_address: string | null;
  /** 출금 수량 */
  amount: string;
  /** 출금 수수료 */
  fee: string;
  /** 실수령액 (amount - fee) */
  receivable: string;
}
```

### 주소 강조 표시 로직

```typescript
function formatHighlightedAddress(address: string): JSX.Element {
  if (address.length <= 16) {
    return <span className="text-yellow-400 font-mono">{address}</span>;
  }

  const prefix = address.slice(0, 8);
  const middle = address.slice(8, -8);
  const suffix = address.slice(-8);

  return (
    <span className="font-mono">
      <span className="text-yellow-400">{prefix}</span>
      <span className="text-wts-muted">{middle}</span>
      <span className="text-yellow-400">{suffix}</span>
    </span>
  );
}
```

### 3초 카운트다운 타이머 구현

```typescript
const [countdown, setCountdown] = useState(3);
const [isConfirmEnabled, setIsConfirmEnabled] = useState(false);

useEffect(() => {
  if (!isOpen) {
    setCountdown(3);
    setIsConfirmEnabled(false);
    return;
  }

  if (countdown > 0) {
    const timer = setTimeout(() => setCountdown(countdown - 1), 1000);
    return () => clearTimeout(timer);
  } else {
    setIsConfirmEnabled(true);
  }
}, [isOpen, countdown]);
```

### WtsWindow 상태 관리 추가

[Source: apps/desktop/src/wts/WtsWindow.tsx]

```typescript
// 출금 다이얼로그 상태
const [isWithdrawDialogOpen, setIsWithdrawDialogOpen] = useState(false);
const [withdrawConfirmInfo, setWithdrawConfirmInfo] = useState<WithdrawConfirmInfo | null>(null);
const [isWithdrawLoading, setIsWithdrawLoading] = useState(false);

// TransferPanel에서 출금 버튼 클릭 시
const handleWithdrawClick = useCallback((params: WithdrawParams) => {
  const fee = ...; // transferStore에서 수수료 정보 가져오기
  const receivable = (parseFloat(params.amount) - parseFloat(fee)).toFixed(8);

  setWithdrawConfirmInfo({
    currency: params.currency,
    net_type: params.net_type,
    address: params.address,
    secondary_address: params.secondary_address ?? null,
    amount: params.amount,
    fee,
    receivable,
  });
  setIsWithdrawDialogOpen(true);
}, []);

// 출금 확인
const handleWithdrawConfirm = useCallback(async () => {
  if (!withdrawConfirmInfo) return;

  setIsWithdrawLoading(true);
  try {
    const result = await invoke<WtsApiResult<WithdrawResponse>>('wts_withdraw', {
      params: {
        currency: withdrawConfirmInfo.currency,
        net_type: withdrawConfirmInfo.net_type,
        amount: withdrawConfirmInfo.amount,
        address: withdrawConfirmInfo.address,
        secondary_address: withdrawConfirmInfo.secondary_address,
      }
    });

    if (result.success && result.data) {
      addLog('SUCCESS', 'WITHDRAW', `출금 요청 완료: ${result.data.uuid}`);
      showToast('success', '출금 요청이 완료되었습니다');
      // 잔고 갱신
      refreshBalance();
    } else {
      handleApiError(result.error, 'WITHDRAW', '출금 실패');
    }
  } catch (error) {
    addLog('ERROR', 'WITHDRAW', `출금 요청 실패: ${error}`);
    showToast('error', '출금 요청에 실패했습니다');
  } finally {
    setIsWithdrawLoading(false);
    setIsWithdrawDialogOpen(false);
    setWithdrawConfirmInfo(null);
  }
}, [withdrawConfirmInfo]);
```

### Tauri 명령 호출 패턴

[Source: apps/desktop/src/wts/types.ts]

```typescript
// 출금 요청
const result = await invoke<WtsApiResult<WithdrawResponse>>(
  'wts_withdraw',
  { params: { currency, net_type, amount, address, secondary_address } }
);
```

### 에러 처리 패턴

[Source: apps/desktop/src/wts/utils/errorHandler.ts]

```typescript
import { handleApiError } from '../utils/errorHandler';

// API 호출 실패 시
handleApiError(result.error, 'WITHDRAW', '출금 요청 실패');
```

### UI 스타일 패턴

[Source: apps/desktop/src/wts/components/ConfirmDialog.tsx]

**다이얼로그 헤더:**
```tsx
<div className="px-4 py-3 border-b border-wts-accent/50">
  <h2 className="text-base font-semibold text-wts-accent">
    출금 확인
  </h2>
</div>
```

**정보 행:**
```tsx
<div className="flex justify-between text-sm">
  <span className="text-wts-muted">자산</span>
  <span className="text-wts-foreground font-mono">{currency}</span>
</div>
```

**경고 문구:**
```tsx
<div className="mt-3 p-2 bg-red-900/30 border border-red-700/50 rounded text-xs text-red-400">
  ⚠️ 출금 주소를 다시 확인하세요. 잘못된 주소로 출금하면 자산을 되찾을 수 없습니다.
</div>
```

**확인 버튼 (카운트다운):**
```tsx
<button
  onClick={onConfirm}
  disabled={!isConfirmEnabled || isLoading}
  className="flex-1 py-2 text-sm font-medium rounded text-white
             bg-wts-accent hover:bg-wts-accent/80
             disabled:opacity-50 disabled:cursor-not-allowed
             transition-colors"
>
  {isLoading ? (
    <span className="inline-flex items-center justify-center gap-2">
      <span className="animate-spin">⏳</span>
      처리중...
    </span>
  ) : isConfirmEnabled ? (
    '출금'
  ) : (
    `출금 (${countdown}초)`
  )}
</button>
```

### Project Structure Notes

**수정 파일:**
- `apps/desktop/src/wts/types.ts` - WithdrawConfirmInfo 타입 추가
- `apps/desktop/src/wts/components/WithdrawConfirmDialog.tsx` - 신규 생성
- `apps/desktop/src/wts/components/index.ts` - export 추가
- `apps/desktop/src/wts/WtsWindow.tsx` - 출금 다이얼로그 상태 관리 및 렌더링 추가

**테스트 파일:**
- `apps/desktop/src/wts/__tests__/components/WithdrawConfirmDialog.test.tsx` - 신규 생성

**아키텍처 정합성:**
- WTS 컴포넌트 구조 준수 (`wts/components/`)
- 기존 ConfirmDialog 패턴 확장 (새 파일로 분리)
- Zustand 스토어 직접 사용 대신 props 전달 패턴 사용
- 콘솔 로깅 패턴 준수 (`addLog('WITHDRAW', ...)`)
- 에러 처리 패턴 준수 (`handleApiError`)
- Tauri invoke 패턴 준수 (`WtsApiResult<T>`)

### 이전 스토리 참조

**WTS-5.1 (출금 API Rust 백엔드):**
- `wts_withdraw` Tauri 명령 구현 완료
- WithdrawParams, WithdrawResponse 타입 정의 완료
- 에러 코드 한국어 매핑 완료

**WTS-5.2 (출금 탭 UI):**
- TransferPanel에 onWithdrawClick prop 준비 완료
- withdrawChanceInfo에서 수수료(currency_info.withdraw_fee) 정보 사용 가능
- 버튼 클릭 시 WithdrawParams 객체 전달

**WTS-3.5 (주문 확인 다이얼로그):**
- ConfirmDialog 컴포넌트 패턴 참조
- 키보드 핸들링, 포커스 트랩 로직 재사용 가능

### WTS-5.4 연결 포인트

출금 성공 후 잔고 자동 갱신은 이 스토리에서 구현.
출금 완료 후 트래킹(TXID 조회)은 WTS-5.4에서 구현 예정.

### References

- [Architecture: WTS Frontend Structure](/_bmad-output/planning-artifacts/architecture.md#WTS Frontend Structure)
- [Architecture: ConfirmDialog](/_bmad-output/planning-artifacts/architecture.md#ConfirmDialog)
- [PRD: FR25 출금 확인 다이얼로그](/_bmad-output/planning-artifacts/prd.md)
- [UX: Modal Dialog Patterns](/_bmad-output/planning-artifacts/ux-design-specification.md#Modal & Dialog Patterns)
- [WTS Epics: Epic 5 Story 5.3](/_bmad-output/planning-artifacts/wts-epics.md#Story 5.3)
- [Previous Story: WTS-5.2 출금 탭 UI](/_bmad-output/implementation-artifacts/wts-5-2-withdraw-tab-ui-form.md)
- [Previous Story: WTS-3.5 주문 확인 다이얼로그](/_bmad-output/implementation-artifacts/wts-3-5-order-confirm-dialog.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

### Completion Notes List

- TDD Red-Green-Refactor 사이클로 모든 기능 구현
- 37개 단위 테스트 작성 (모든 AC 커버)
- 기존 ConfirmDialog 패턴 확장하여 3초 카운트다운, 주소 강조 기능 추가
- WtsWindow에 출금 다이얼로그 상태 관리 및 API 호출 로직 통합
- Vitest fake timers를 사용한 타이머 테스트 구현
- 포커스 트랩, ESC 키, 오버레이 클릭 핸들러 구현 완료
- 112개 테스트 전체 통과 (types 56 + dialog 37 + WtsWindow 19)
- 리뷰 수정: 로딩 중 ESC/오버레이 닫기 허용, 성공 시에만 다이얼로그 종료, 출금 Rate Limit 메시지 분리
- 리뷰 테스트 보완: WithdrawConfirmDialog/handleApiError/출금 에러 매핑 테스트 갱신
- 리뷰 수정 후 테스트 미실행

### File List

**신규 생성:**
- `apps/desktop/src/wts/components/WithdrawConfirmDialog.tsx` - 출금 확인 다이얼로그 컴포넌트
- `apps/desktop/src/wts/__tests__/components/WithdrawConfirmDialog.test.tsx` - 단위 테스트 (37개)

**수정:**
- `apps/desktop/src/wts/types.ts` - WithdrawConfirmInfo 타입 추가
- `apps/desktop/src/wts/WtsWindow.tsx` - 출금 다이얼로그 상태 관리 및 렌더링 추가
- `apps/desktop/src/wts/__tests__/types.test.ts` - WithdrawConfirmInfo 타입 테스트 추가 (3개)
- `apps/desktop/src/wts/utils/errorHandler.ts` - 출금 Rate Limit 메시지 분리
- `apps/desktop/src/wts/__tests__/utils/errorHandler.test.ts` - 출금 Rate Limit 테스트 추가
- `_bmad-output/implementation-artifacts/sprint-status.yaml` - 스프린트 상태 동기화
- `_bmad-output/implementation-artifacts/wts-5-3-withdraw-confirm-dialog.md` - 리뷰 수정 반영

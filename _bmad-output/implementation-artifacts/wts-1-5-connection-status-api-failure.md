# Story WTS-1.5: 연결 상태 표시 및 API 장애 감지

Status: done

## Story

As a **트레이더**,
I want **거래소 API 연결 상태를 실시간으로 확인할 수 있는 기능**,
So that **거래 전에 시스템 상태를 파악할 수 있다**.

## Acceptance Criteria

1. **Given** 거래소가 선택되어 있을 때 **When** API 연결 상태가 변경되면 **Then** 헤더에 연결 상태 인디케이터(녹색=연결됨, 빨강=끊김, 노랑=연결중)가 표시되어야 한다
2. **Given** 거래소가 선택되어 있을 때 **When** 연결 상태가 변경되면 **Then** 콘솔에 연결 상태 변경 로그가 기록되어야 한다
3. **Given** 거래소가 선택되어 있을 때 **When** API 장애가 감지되면 **Then** 사용자에게 장애 상태가 표시되어야 한다
4. **Given** WTS 창이 열렸을 때 **When** Upbit API에 연결을 시도하면 **Then** 헬스 체크를 통해 연결 상태가 자동 확인되어야 한다
5. **Given** 연결이 끊긴 상태일 때 **When** 재연결을 시도하면 **Then** 자동 재연결 로직이 동작하고 상태가 갱신되어야 한다

## Tasks / Subtasks

- [x] Task 1: ConnectionStatus 상태 확장 (AC: #1)
  - [x] Subtask 1.1: `connecting` 상태 추가 (현재 `connected` | `disconnected` → `connected` | `connecting` | `disconnected`)
  - [x] Subtask 1.2: wtsStore에 lastConnectionError 필드 추가 (에러 메시지 저장용)
  - [x] Subtask 1.3: 연결 상태 변경 시 콘솔 로그 자동 기록 로직 추가

- [x] Task 2: ExchangePanel 연결 상태 UI 개선 (AC: #1, #3)
  - [x] Subtask 2.1: Badge 컴포넌트를 3상태 지원으로 확장 (녹색/노랑/빨강)
  - [x] Subtask 2.2: `connecting` 상태에서 펄스 애니메이션 추가
  - [x] Subtask 2.3: 장애 상태 시 에러 메시지 툴팁 표시

- [x] Task 3: Upbit API 헬스 체크 구현 (AC: #4)
  - [x] Subtask 3.1: Tauri 명령 `wts_check_connection` 생성 (Rust 백엔드)
  - [x] Subtask 3.2: Upbit `/v1/market/all` API 호출로 연결 확인
  - [x] Subtask 3.3: 프론트엔드 useConnectionCheck 훅 구현

- [x] Task 4: 자동 연결 및 재연결 로직 (AC: #4, #5)
  - [x] Subtask 4.1: WTS 창 마운트 시 자동 연결 체크 실행
  - [x] Subtask 4.2: 연결 실패 시 exponential backoff 재시도 (최대 5회)
  - [x] Subtask 4.3: 재연결 시도 중 상태 표시 (`connecting`)

- [x] Task 5: 콘솔 로그 연동 (AC: #2)
  - [x] Subtask 5.1: 연결 성공 시 `[SUCCESS] Upbit API 연결됨` 로그
  - [x] Subtask 5.2: 연결 실패 시 `[ERROR] Upbit API 연결 실패: {reason}` 로그
  - [x] Subtask 5.3: 재연결 시도 시 `[INFO] Upbit API 재연결 시도 중...` 로그

- [x] Task 6: 테스트 작성 (AC: #1, #2, #3, #4)
  - [x] Subtask 6.1: ConnectionStatus 3상태 렌더링 테스트
  - [x] Subtask 6.2: 연결 상태 변경 시 콘솔 로그 기록 테스트
  - [x] Subtask 6.3: useConnectionCheck 훅 동작 테스트 (mock Tauri invoke)
  - [x] Subtask 6.4: 재연결 로직 테스트

## Dev Notes

### Architecture 준수사항

**연결 상태 패턴:**
[Source: _bmad-output/planning-artifacts/architecture.md#Communication Patterns]

```typescript
// wtsStore.ts 확장
export interface WtsState {
  // ... 기존 상태
  connectionStatus: ConnectionStatus;
  setConnectionStatus: (status: ConnectionStatus) => void;
  lastConnectionError: string | null;
  setConnectionError: (error: string | null) => void;
}
```

**Tauri 명령 네이밍:**
[Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]

```rust
// apps/desktop/src-tauri/src/wts/commands.rs
#[tauri::command]
pub async fn wts_check_connection(exchange: &str) -> Result<ConnectionResult, String>
```

### UX 디자인 요구사항

**연결 상태 인디케이터:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Visual Feedback & Animation]

| 상태 | 색상 | 아이콘/효과 | 텍스트 |
|------|------|------------|--------|
| connected | 녹색 (#22c55e) | 원형 점 | 연결됨 |
| connecting | 노랑 (#f59e0b) | 펄스 애니메이션 | 연결중... |
| disconnected | 빨강 (#ef4444) | 원형 점 | 연결 안됨 |

**에러 표시:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Feedback Patterns]

```
- 에러 발생 시 Badge에 hover하면 툴팁으로 에러 상세 표시
- 콘솔에 ERROR 레벨로 상세 메시지 기록
```

### 이전 스토리 (WTS-1.4)에서 학습한 사항

**완료된 작업:**
- ExchangePanel.tsx에 기본 연결 상태 Badge 구현됨 (현재 2상태: connected/disconnected)
- wtsStore에 connectionStatus, setConnectionStatus 구현됨
- ENABLED_EXCHANGES로 MVP에서 Upbit만 활성화됨

**현재 제한사항:**
- `connecting` 상태 없음 - 이번 스토리에서 추가 필요
- API 헬스 체크 로직 없음 - Tauri 명령 필요
- 자동 재연결 로직 없음

**재사용 가능한 패턴:**
- `useWtsStore` 훅으로 연결 상태 접근
- `useConsoleStore` 훅으로 콘솔 로그 추가
- Badge 컴포넌트 (shadcn/ui)

### Git 최근 커밋 패턴

**최근 작업:**
- `5118bb7 fix(wts): align layout with shadcn theme`
- `510d8d4 feat: extend subscription flow and WTS UI`
- `4c75242 feat(wts): scaffold stores and tests`

**커밋 메시지 형식:** `feat(wts): 설명` 또는 `fix(wts): 설명`

### 구현 가이드

**1. types.ts 확장:**

```typescript
// ConnectionStatus는 이미 정의됨: 'connected' | 'connecting' | 'disconnected'
// 추가 불필요

/** 연결 체크 결과 (Tauri 명령 응답) */
export interface ConnectionCheckResult {
  success: boolean;
  latency?: number; // ms
  error?: string;
}
```

**2. wtsStore 확장:**

```typescript
// apps/desktop/src/wts/stores/wtsStore.ts
export interface WtsState {
  // ... 기존 상태
  lastConnectionError: string | null;
  setConnectionError: (error: string | null) => void;
}

export const useWtsStore = create<WtsState>()((set) => ({
  // ... 기존 코드
  lastConnectionError: null,
  setConnectionError: (error: string | null) => set({ lastConnectionError: error }),
}));
```

**3. useConnectionCheck 훅:**

```typescript
// apps/desktop/src/wts/hooks/useConnectionCheck.ts
import { useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWtsStore } from '../stores';
import { useConsoleStore } from '../stores/consoleStore';

const MAX_RETRIES = 5;
const INITIAL_DELAY = 1000; // 1초

export function useConnectionCheck() {
  const { selectedExchange, setConnectionStatus, setConnectionError } = useWtsStore();
  const { addLog } = useConsoleStore();
  const retryCount = useRef(0);
  const timeoutRef = useRef<number | null>(null);

  const checkConnection = useCallback(async () => {
    setConnectionStatus('connecting');
    addLog('INFO', 'SYSTEM', `${selectedExchange} API 연결 확인 중...`);

    try {
      const result = await invoke<ConnectionCheckResult>('wts_check_connection', {
        exchange: selectedExchange,
      });

      if (result.success) {
        setConnectionStatus('connected');
        setConnectionError(null);
        retryCount.current = 0;
        addLog('SUCCESS', 'SYSTEM', `${selectedExchange} API 연결됨 (${result.latency}ms)`);
      } else {
        throw new Error(result.error || 'Unknown error');
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      setConnectionStatus('disconnected');
      setConnectionError(errorMessage);
      addLog('ERROR', 'SYSTEM', `${selectedExchange} API 연결 실패: ${errorMessage}`);

      // 재시도 로직
      if (retryCount.current < MAX_RETRIES) {
        const delay = INITIAL_DELAY * Math.pow(2, retryCount.current);
        retryCount.current++;
        addLog('INFO', 'SYSTEM', `${delay / 1000}초 후 재연결 시도 (${retryCount.current}/${MAX_RETRIES})`);
        timeoutRef.current = window.setTimeout(checkConnection, delay);
      }
    }
  }, [selectedExchange, setConnectionStatus, setConnectionError, addLog]);

  // 마운트 시 자동 연결 체크
  useEffect(() => {
    checkConnection();
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, [selectedExchange]); // 거래소 변경 시 재체크

  return { checkConnection };
}
```

**4. ExchangePanel 개선:**

```typescript
// Badge 색상 매핑 개선
const getStatusVariant = (status: ConnectionStatus) => {
  switch (status) {
    case 'connected': return 'success';
    case 'connecting': return 'warning';
    case 'disconnected': return 'destructive';
  }
};

const getStatusText = (status: ConnectionStatus) => {
  switch (status) {
    case 'connected': return '연결됨';
    case 'connecting': return '연결중...';
    case 'disconnected': return '연결 안됨';
  }
};

// Badge 렌더링
<Badge
  variant={getStatusVariant(connectionStatus)}
  className={connectionStatus === 'connecting' ? 'animate-pulse' : ''}
  title={lastConnectionError || undefined}
>
  {getStatusText(connectionStatus)}
</Badge>
```

**5. Rust 백엔드 (wts_check_connection):**

```rust
// apps/desktop/src-tauri/src/wts/commands.rs

#[derive(Debug, Serialize)]
pub struct ConnectionCheckResult {
    pub success: bool,
    pub latency: Option<u64>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn wts_check_connection(exchange: &str) -> Result<ConnectionCheckResult, String> {
    use std::time::Instant;

    match exchange {
        "upbit" => {
            let start = Instant::now();
            // Upbit API 서버 상태 확인 (공개 API)
            let client = reqwest::Client::new();
            match client
                .get("https://api.upbit.com/v1/market/all")
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    let latency = start.elapsed().as_millis() as u64;
                    Ok(ConnectionCheckResult {
                        success: true,
                        latency: Some(latency),
                        error: None,
                    })
                }
                Ok(response) => Ok(ConnectionCheckResult {
                    success: false,
                    latency: None,
                    error: Some(format!("HTTP {}", response.status())),
                }),
                Err(e) => Ok(ConnectionCheckResult {
                    success: false,
                    latency: None,
                    error: Some(e.to_string()),
                }),
            }
        }
        _ => Ok(ConnectionCheckResult {
            success: false,
            latency: None,
            error: Some(format!("Unsupported exchange: {}", exchange)),
        }),
    }
}
```

**6. 테스트 예시:**

```typescript
// apps/desktop/src/wts/__tests__/panels/ExchangePanel.connection.test.tsx
import { render, screen } from '@testing-library/react';
import { ExchangePanel } from '../../panels/ExchangePanel';
import { useWtsStore } from '../../stores';

describe('ExchangePanel Connection Status', () => {
  beforeEach(() => {
    useWtsStore.setState({
      connectionStatus: 'disconnected',
      lastConnectionError: null,
    });
  });

  it('should display connected status with green badge', () => {
    useWtsStore.setState({ connectionStatus: 'connected' });
    render(<ExchangePanel />);
    expect(screen.getByText('연결됨')).toBeInTheDocument();
    // Badge에 success variant 확인
  });

  it('should display connecting status with pulse animation', () => {
    useWtsStore.setState({ connectionStatus: 'connecting' });
    render(<ExchangePanel />);
    expect(screen.getByText('연결중...')).toBeInTheDocument();
    // animate-pulse 클래스 확인
  });

  it('should display disconnected status with error tooltip', () => {
    useWtsStore.setState({
      connectionStatus: 'disconnected',
      lastConnectionError: 'Network timeout',
    });
    render(<ExchangePanel />);
    expect(screen.getByText('연결 안됨')).toBeInTheDocument();
    // title 속성에 에러 메시지 확인
  });
});
```

### Project Structure Notes

**기존 파일 (변경):**
- `apps/desktop/src/wts/types.ts` - ConnectionCheckResult 타입 추가
- `apps/desktop/src/wts/stores/wtsStore.ts` - lastConnectionError 필드 추가
- `apps/desktop/src/wts/panels/ExchangePanel.tsx` - 3상태 Badge, 펄스 애니메이션

**신규 파일:**
- `apps/desktop/src/wts/hooks/useConnectionCheck.ts` - 연결 체크 훅
- `apps/desktop/src-tauri/src/wts/commands.rs` - wts_check_connection 명령
- `apps/desktop/src/wts/__tests__/hooks/useConnectionCheck.test.ts` - 훅 테스트

**디렉토리 구조:**
```
apps/desktop/src/wts/
├── types.ts            # ConnectionCheckResult 추가
├── stores/
│   └── wtsStore.ts     # lastConnectionError 추가
├── panels/
│   └── ExchangePanel.tsx  # 3상태 Badge (변경)
├── hooks/
│   └── useConnectionCheck.ts  # 연결 체크 훅 (신규)
└── __tests__/
    ├── panels/
    │   └── ExchangePanel.connection.test.tsx (신규)
    └── hooks/
        └── useConnectionCheck.test.ts (신규)

apps/desktop/src-tauri/src/wts/
├── mod.rs              # commands 모듈 등록
└── commands.rs         # wts_check_connection (신규)
```

### 성능 고려사항

**재연결 로직:**
- Exponential backoff: 1초 → 2초 → 4초 → 8초 → 16초
- 최대 5회 재시도 후 중단
- 거래소 변경 시 기존 타이머 취소

**API 호출 최적화:**
- 헬스 체크 타임아웃: 5초
- 공개 API 사용 (인증 불필요)
- Rate Limit 영향 없음 (별도 카운트)

### Upbit API 참고

**헬스 체크 대상 API:**
[Source: _bmad-output/planning-artifacts/architecture.md#Technical Constraints & Dependencies]

- `GET /v1/market/all` - 공개 API, 인증 불필요
- 응답 시간으로 레이턴시 측정
- Rate Limit: Quotation 10회/초 (IP) - 영향 최소

### References

- [Architecture Document: Communication Patterns](_bmad-output/planning-artifacts/architecture.md#Communication Patterns)
- [Architecture Document: Naming Patterns](_bmad-output/planning-artifacts/architecture.md#Naming Patterns)
- [UX Design: Visual Feedback & Animation](_bmad-output/planning-artifacts/ux-design-specification.md#Visual Feedback & Animation)
- [UX Design: Feedback Patterns](_bmad-output/planning-artifacts/ux-design-specification.md#Feedback Patterns)
- [WTS Epics: Story 1.5](_bmad-output/planning-artifacts/wts-epics.md#Story 1.5)
- [Previous Story: WTS-1.4](_bmad-output/implementation-artifacts/wts-1-4-exchange-tab-selection.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- 모든 테스트 통과 (60/60)
- Rust 코드 컴파일 성공 (7 warnings - 미사용 타입)

### Completion Notes List

1. **Task 1 완료**: `connecting` 상태는 이미 types.ts에 정의되어 있었음. wtsStore에 `lastConnectionError` 필드와 `setConnectionError` 액션 추가.

2. **Task 2 완료**: Badge 컴포넌트에 `warning` variant 추가 (amber-500). ExchangePanel에서 3상태 Badge 렌더링, `connecting` 시 animate-pulse 클래스 적용, `title` 속성으로 에러 메시지 툴팁 표시.

3. **Task 3 완료**: Rust 백엔드에 `wts_check_connection` Tauri 명령 추가. Upbit `/v1/market/all` API 호출로 연결 상태 확인, 레이턴시 측정 (5초 타임아웃).

4. **Task 4 완료**: `useConnectionCheck` 훅 구현. WTS 창 마운트 시 자동 연결 체크, 실패 시 exponential backoff 재시도 (1초→2초→4초→8초→16초, 최대 5회), 거래소 변경 시 재연결.

5. **Task 5 완료**: useConnectionCheck 훅에서 콘솔 로그 연동 구현. INFO (연결 확인 중), SUCCESS (연결됨 + 레이턴시), ERROR (연결 실패 + 에러 메시지).

6. **Task 6 완료**: useConnectionCheck 훅 테스트 8개 추가, ExchangePanel 연결 상태 테스트 3개 추가. 기존 테스트 mock 업데이트.
7. **Review Fixes**: 연결 로그 포맷 정합화, 재연결 시 에러 초기화, 재시도(backoff) 테스트 보강.

### File List

**변경된 파일:**
- `apps/desktop/src/wts/types.ts` - ConnectionCheckResult 타입 추가
- `apps/desktop/src/wts/stores/wtsStore.ts` - lastConnectionError, setConnectionError 추가
- `apps/desktop/src/wts/panels/ExchangePanel.tsx` - 3상태 Badge, 펄스 애니메이션, 에러 툴팁
- `apps/desktop/src/wts/WtsWindow.tsx` - useConnectionCheck 훅 사용
- `apps/desktop/src/components/ui/badge.tsx` - warning variant 추가
- `apps/desktop/src-tauri/src/wts/mod.rs` - wts_check_connection 명령 추가
- `apps/desktop/src-tauri/src/wts/types.rs` - ConnectionCheckResult 타입 추가
- `apps/desktop/src-tauri/src/main.rs` - wts_check_connection 명령 등록
- `apps/desktop/src/wts/__tests__/stores/wtsStore.test.ts` - lastConnectionError 테스트 추가
- `apps/desktop/src/wts/__tests__/panels/ExchangePanel.test.tsx` - 연결 상태 테스트 추가
- `apps/desktop/src/wts/__tests__/WtsWindow.test.tsx` - mock 업데이트
- `apps/desktop/src/wts/__tests__/index.test.tsx` - mock 업데이트

**신규 파일:**
- `apps/desktop/src/wts/hooks/useConnectionCheck.ts` - 연결 체크 훅
- `apps/desktop/src/wts/hooks/index.ts` - hooks 모듈 export
- `apps/desktop/src/wts/__tests__/hooks/useConnectionCheck.test.ts` - 훅 테스트

**기타 변경(스토리 범위 외, git 상태 기준):**
- `crates/engine/src/detector.rs`
- `crates/feeds/src/adapter/binance.rs`
- `crates/feeds/src/adapter/upbit.rs`
- `crates/feeds/src/aggregator.rs`
- `crates/feeds/src/subscription.rs`
- `_bmad-output/implementation-artifacts/5-1-new-market-price-data-verification.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/wts-1-4-exchange-tab-selection.md`
- `_bmad-output/implementation-artifacts/wts-1-6-console-panel-basic-structure.md`

## Change Log

- 2026-01-19: WTS-1.5 구현 완료 - 연결 상태 표시 및 API 장애 감지 기능 (AC #1-#5 모두 충족)
- 2026-01-19: 리뷰 수정 - 로그 포맷/재연결 테스트 보강 및 스토리 메타 업데이트

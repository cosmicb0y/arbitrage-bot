# Story WTS-1.6: 콘솔 패널 기본 구조

Status: done

## Story

As a **트레이더**,
I want **VS Code 스타일의 콘솔 로그 패널**,
So that **시스템 이벤트와 거래 결과를 확인할 수 있다**.

## Acceptance Criteria

1. **Given** WTS 창이 열렸을 때 **When** 시스템 이벤트가 발생하면 **Then** 콘솔에 타임스탬프(HH:mm:ss.SSS)와 함께 로그가 표시되어야 한다
2. **Given** WTS 창이 열렸을 때 **When** 로그가 표시되면 **Then** 로그 레벨별 색상 구분(INFO=회색, SUCCESS=녹색, ERROR=빨강, WARN=노랑)이 되어야 한다
3. **Given** WTS 창이 열렸을 때 **When** 로그가 1000개를 초과하면 **Then** FIFO 방식으로 오래된 로그가 삭제되어야 한다
4. **Given** WTS 창이 열렸을 때 **When** 로그가 많아지면 **Then** 스크롤하여 이전 로그를 볼 수 있어야 한다
5. **Given** WTS 창이 열렸을 때 **When** 로그가 추가되면 **Then** consoleStore에 ConsoleLogEntry 형식으로 저장되어야 한다
6. **Given** 콘솔 패널이 열렸을 때 **When** 새 로그가 추가되면 **Then** 자동 스크롤로 최신 로그가 보여야 한다 (단, 사용자가 스크롤 중이면 자동 스크롤 비활성화)

## Tasks / Subtasks

- [x] Task 1: ConsolePanel UI 구현 (AC: #1, #2, #4, #6)
  - [x] Subtask 1.1: consoleStore의 logs 배열 렌더링
  - [x] Subtask 1.2: 타임스탬프 포맷터 구현 (HH:mm:ss.SSS)
  - [x] Subtask 1.3: 로그 레벨별 색상 스타일 적용 (INFO=text-wts-muted, SUCCESS=text-green-500, ERROR=text-red-500, WARN=text-yellow-500)
  - [x] Subtask 1.4: 스크롤 가능한 로그 목록 구현 (overflow-y-auto)
  - [x] Subtask 1.5: 자동 스크롤 로직 구현 (새 로그 시 하단으로 스크롤, 사용자 스크롤 시 비활성화)
  - [x] Subtask 1.6: 로그 엔트리 컴포넌트 분리 (ConsoleLogItem)

- [x] Task 2: 로그 표시 형식 구현 (AC: #1, #2)
  - [x] Subtask 2.1: 로그 포맷: `HH:mm:ss.SSS [CATEGORY] message` 형식 적용
  - [x] Subtask 2.2: 카테고리별 배지 스타일 (ORDER, BALANCE, DEPOSIT, WITHDRAW, SYSTEM)
  - [x] Subtask 2.3: 에러 로그 시 상세 정보 접기/펼치기 (detail 필드)

- [x] Task 3: 콘솔 헤더 및 컨트롤 (AC: #4)
  - [x] Subtask 3.1: 콘솔 헤더에 "Console" 제목 표시
  - [x] Subtask 3.2: 로그 초기화(Clear) 버튼 추가
  - [x] Subtask 3.3: 로그 카운트 표시 (예: "123 logs")

- [x] Task 4: 성능 최적화 (AC: #3, #4)
  - [x] Subtask 4.1: React.memo로 ConsoleLogItem 최적화
  - [x] Subtask 4.2: 가상화 고려 (1000개 로그 성능) - MVP에서는 선택적
  - [x] Subtask 4.3: useRef로 스크롤 위치 추적

- [x] Task 5: 테스트 작성 (AC: #1, #2, #3, #4, #5, #6)
  - [x] Subtask 5.1: 로그 렌더링 테스트 (타임스탬프, 메시지, 카테고리)
  - [x] Subtask 5.2: 로그 레벨별 색상 테스트
  - [x] Subtask 5.3: FIFO 로그 관리 테스트 (1000개 초과 시)
  - [x] Subtask 5.4: 스크롤 동작 테스트
  - [x] Subtask 5.5: 로그 초기화 버튼 테스트

## Dev Notes

### Architecture 준수사항

**콘솔 로그 형식:**
[Source: _bmad-output/planning-artifacts/architecture.md#Format Patterns]

```typescript
interface ConsoleLogEntry {
  id: string;           // 고유 ID (randomUUID 또는 fallback)
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

**로그 저장 제한:**
[Source: _bmad-output/planning-artifacts/architecture.md#Error Handling & Logging]

- 저장 방식: 메모리만 (Zustand)
- 최대 개수: 1000개 (FIFO)
- 이유: 보안 (민감 정보), MVP 단순성

### UX 디자인 요구사항

**콘솔 스타일:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Design]

```
콘솔 패널 (VS Code 스타일):
- 헤더: "Console" 제목 + 로그 카운트 + Clear 버튼
- 로그 영역: 스크롤 가능, 고정 높이
- 폰트: JetBrains Mono (또는 기본 mono 폰트)
- 배경: 다크 (터미널 스타일)
```

**색상 시스템:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Color System]

| 로그 레벨 | 색상 | Tailwind 클래스 |
|---------|------|----------------|
| INFO | 회색 (muted) | `text-wts-muted` |
| SUCCESS | 녹색 (#22c55e) | `text-green-500` |
| ERROR | 빨강 (#ef4444) | `text-red-500` |
| WARN | 노랑 (#f59e0b) | `text-yellow-500` |

**카테고리 배지:**

| 카테고리 | 색상 |
|---------|------|
| ORDER | 보라 |
| BALANCE | 파랑 |
| DEPOSIT | 청록 |
| WITHDRAW | 주황 |
| SYSTEM | 회색 |

### 이전 스토리에서 학습한 사항

**WTS-1.4 (거래소 탭 선택) 완료:**
- ExchangePanel에서 `useConsoleStore.addLog()` 호출로 거래소 전환 로그 기록
- 로그 형식: `[INFO] 거래소 전환: {exchange}`
- consoleStore에 logs 배열 관리 중

**WTS-1.5 (연결 상태 표시) - ready-for-dev:**
- 연결 상태 변경 시 콘솔 로그 기록 예정
- 로그 형식: `[SUCCESS] Upbit API 연결됨`, `[ERROR] Upbit API 연결 실패`

**현재 ConsolePanel 상태:**
- 플레이스홀더 UI만 존재
- `consoleStore`와 연결 안됨
- 실제 로그 렌더링 없음

**재사용 가능한 패턴:**
- `useConsoleStore` 훅으로 로그 상태 접근
- WTS CSS 변수 시스템 (`--wts-*`)
- wts-panel, wts-panel-header, wts-panel-content 클래스

### Git 최근 커밋 패턴

**최근 작업:**
- `5118bb7 fix(wts): align layout with shadcn theme`
- `510d8d4 feat: extend subscription flow and WTS UI`
- `4c75242 feat(wts): scaffold stores and tests`

**커밋 메시지 형식:** `feat(wts): 설명` 또는 `fix(wts): 설명`

### 구현 가이드

**1. 타임스탬프 포맷터:**

```typescript
// apps/desktop/src/wts/utils/formatters.ts (신규 또는 기존 확장)

/**
 * Unix timestamp를 HH:mm:ss.SSS 형식으로 변환
 */
export function formatLogTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  const ms = date.getMilliseconds().toString().padStart(3, '0');
  return `${hours}:${minutes}:${seconds}.${ms}`;
}
```

**2. 로그 레벨 색상 유틸:**

```typescript
// apps/desktop/src/wts/utils/consoleStyles.ts (신규)
import type { LogLevel, LogCategory } from '../types';

export const LOG_LEVEL_STYLES: Record<LogLevel, string> = {
  INFO: 'text-wts-muted',
  SUCCESS: 'text-green-500',
  ERROR: 'text-red-500',
  WARN: 'text-yellow-500',
};

export const LOG_CATEGORY_STYLES: Record<LogCategory, string> = {
  ORDER: 'bg-purple-500/20 text-purple-400',
  BALANCE: 'bg-blue-500/20 text-blue-400',
  DEPOSIT: 'bg-cyan-500/20 text-cyan-400',
  WITHDRAW: 'bg-orange-500/20 text-orange-400',
  SYSTEM: 'bg-gray-500/20 text-gray-400',
};
```

**3. ConsoleLogItem 컴포넌트:**

```typescript
// apps/desktop/src/wts/components/ConsoleLogItem.tsx
import { memo } from 'react';
import type { ConsoleLogEntry } from '../types';
import { formatLogTimestamp } from '../utils/formatters';
import { LOG_LEVEL_STYLES, LOG_CATEGORY_STYLES } from '../utils/consoleStyles';

interface ConsoleLogItemProps {
  log: ConsoleLogEntry;
}

export const ConsoleLogItem = memo(function ConsoleLogItem({ log }: ConsoleLogItemProps) {
  return (
    <div className={`flex items-start gap-2 py-0.5 font-mono text-xs ${LOG_LEVEL_STYLES[log.level]}`}>
      <span className="text-wts-muted shrink-0">
        {formatLogTimestamp(log.timestamp)}
      </span>
      <span className={`px-1 rounded text-[10px] shrink-0 ${LOG_CATEGORY_STYLES[log.category]}`}>
        {log.category}
      </span>
      <span className="break-words">{log.message}</span>
    </div>
  );
});
```

**4. ConsolePanel 구현:**

```typescript
// apps/desktop/src/wts/panels/ConsolePanel.tsx
import { useRef, useEffect, useCallback, useState } from 'react';
import { useConsoleStore } from '../stores';
import { ConsoleLogItem } from '../components/ConsoleLogItem';
import { Button } from '@/components/ui/button';
import { Trash2 } from 'lucide-react';

interface ConsolePanelProps {
  className?: string;
}

export function ConsolePanel({ className = '' }: ConsolePanelProps) {
  const { logs, clearLogs } = useConsoleStore();
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  // 자동 스크롤
  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  // 사용자 스크롤 감지
  const handleScroll = useCallback(() => {
    if (!scrollRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
    setAutoScroll(isAtBottom);
  }, []);

  return (
    <div
      data-testid="console-panel"
      className={`wts-area-console wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header flex items-center justify-between">
        <span>Console</span>
        <div className="flex items-center gap-2">
          <span className="text-wts-muted text-xs">{logs.length} logs</span>
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5"
            onClick={clearLogs}
            title="Clear logs"
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>
      </div>
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="wts-panel-content flex-1 overflow-y-auto p-2"
      >
        {logs.length === 0 ? (
          <p className="text-wts-muted text-xs italic">No logs yet</p>
        ) : (
          logs.map((log) => <ConsoleLogItem key={log.id} log={log} />)
        )}
      </div>
    </div>
  );
}
```

**5. 테스트 예시:**

```typescript
// apps/desktop/src/wts/__tests__/panels/ConsolePanel.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { ConsolePanel } from '../../panels/ConsolePanel';
import { useConsoleStore } from '../../stores/consoleStore';

describe('ConsolePanel', () => {
  beforeEach(() => {
    useConsoleStore.setState({ logs: [] });
  });

  it('should display "No logs yet" when empty', () => {
    render(<ConsolePanel />);
    expect(screen.getByText('No logs yet')).toBeInTheDocument();
  });

  it('should display log count', () => {
    useConsoleStore.getState().addLog('INFO', 'SYSTEM', 'Test message');
    useConsoleStore.getState().addLog('SUCCESS', 'ORDER', 'Order placed');
    render(<ConsolePanel />);
    expect(screen.getByText('2 logs')).toBeInTheDocument();
  });

  it('should display logs with timestamp format HH:mm:ss.SSS', () => {
    useConsoleStore.getState().addLog('INFO', 'SYSTEM', 'Test message');
    render(<ConsolePanel />);
    // 타임스탬프 형식 확인 (정규식)
    expect(screen.getByText(/\d{2}:\d{2}:\d{2}\.\d{3}/)).toBeInTheDocument();
  });

  it('should apply correct color for ERROR level', () => {
    useConsoleStore.getState().addLog('ERROR', 'ORDER', 'Order failed');
    render(<ConsolePanel />);
    const errorLog = screen.getByText('Order failed');
    expect(errorLog.closest('div')).toHaveClass('text-red-500');
  });

  it('should apply correct color for SUCCESS level', () => {
    useConsoleStore.getState().addLog('SUCCESS', 'SYSTEM', 'Connected');
    render(<ConsolePanel />);
    const successLog = screen.getByText('Connected');
    expect(successLog.closest('div')).toHaveClass('text-green-500');
  });

  it('should clear logs when clear button is clicked', () => {
    useConsoleStore.getState().addLog('INFO', 'SYSTEM', 'Test');
    render(<ConsolePanel />);
    const clearButton = screen.getByTitle('Clear logs');
    fireEvent.click(clearButton);
    expect(screen.getByText('No logs yet')).toBeInTheDocument();
  });

  it('should display category badge', () => {
    useConsoleStore.getState().addLog('INFO', 'ORDER', 'Test');
    render(<ConsolePanel />);
    expect(screen.getByText('ORDER')).toBeInTheDocument();
  });
});
```

**6. FIFO 로그 관리 테스트:**

```typescript
// apps/desktop/src/wts/__tests__/stores/consoleStore.test.ts (확장)
import { useConsoleStore } from '../../stores/consoleStore';
import { MAX_CONSOLE_LOGS } from '../../types';

describe('consoleStore FIFO', () => {
  it('should trim logs when exceeding MAX_CONSOLE_LOGS', () => {
    const store = useConsoleStore.getState();

    // MAX_CONSOLE_LOGS + 10개 로그 추가
    for (let i = 0; i < MAX_CONSOLE_LOGS + 10; i++) {
      store.addLog('INFO', 'SYSTEM', `Log ${i}`);
    }

    const logs = useConsoleStore.getState().logs;
    expect(logs.length).toBe(MAX_CONSOLE_LOGS);
    // 첫 번째 로그는 "Log 10"이어야 함 (처음 10개 삭제됨)
    expect(logs[0].message).toBe('Log 10');
    // 마지막 로그는 "Log 1009"
    expect(logs[logs.length - 1].message).toBe(`Log ${MAX_CONSOLE_LOGS + 9}`);
  });
});
```

### Project Structure Notes

**기존 파일 (변경):**
- `apps/desktop/src/wts/panels/ConsolePanel.tsx` - 완전 재구현

**신규 파일:**
- `apps/desktop/src/wts/utils/formatters.ts` - 타임스탬프 포맷터
- `apps/desktop/src/wts/utils/consoleStyles.ts` - 로그 스타일 상수
- `apps/desktop/src/wts/components/ConsoleLogItem.tsx` - 로그 엔트리 컴포넌트
- `apps/desktop/src/wts/__tests__/panels/ConsolePanel.test.tsx` - ConsolePanel 테스트

**디렉토리 구조:**
```
apps/desktop/src/wts/
├── types.ts              # (기존) ConsoleLogEntry 타입
├── stores/
│   └── consoleStore.ts   # (기존) 로그 관리 스토어
├── panels/
│   └── ConsolePanel.tsx  # (변경) 콘솔 패널 UI
├── components/
│   └── ConsoleLogItem.tsx  # (신규) 로그 엔트리 컴포넌트
├── utils/
│   ├── formatters.ts     # (신규) 타임스탬프 포맷터
│   └── consoleStyles.ts  # (신규) 로그 스타일 상수
└── __tests__/
    ├── panels/
    │   └── ConsolePanel.test.tsx  # (신규)
    └── stores/
        └── consoleStore.test.ts   # (확장) FIFO 테스트
```

### 성능 고려사항

**렌더링 최적화:**
- `React.memo`로 ConsoleLogItem 메모이제이션
- 스토어 selector로 필요한 상태만 구독
- 스크롤 이벤트 throttling 고려 (MVP에서는 선택적)

**1000개 로그 성능:**
- MVP: 단순 map 렌더링으로 시작
- 성능 이슈 발생 시: react-window 또는 react-virtualized 도입 고려
- consoleStore의 FIFO 로직이 이미 구현되어 있음

**자동 스크롤:**
- useEffect로 logs 변경 시 스크롤
- 사용자 스크롤 중에는 자동 스크롤 비활성화 (UX 개선)

### 기존 코드 현황

**consoleStore (완료):**
```typescript
// apps/desktop/src/wts/stores/consoleStore.ts
export const useConsoleStore = create<ConsoleState>()((set) => ({
  logs: [],
  addLog: (level, category, message, detail?) => set((state) => { ... }),
  clearLogs: () => set({ logs: [] }),
}));
```
- 이미 완전히 구현됨
- MAX_CONSOLE_LOGS (1000) 제한 로직 포함
- randomUUID 또는 fallback ID 생성

**ConsolePanel (플레이스홀더):**
```typescript
// 현재 상태 - 완전 재구현 필요
export function ConsolePanel({ className = '' }: ConsolePanelProps) {
  return (
    <div data-testid="console-panel" className={`wts-area-console wts-panel flex flex-col ${className}`}>
      <div className="wts-panel-header">Console</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        <p className="text-wts-muted text-xs">콘솔 로그가 여기에 표시됩니다 (Story 1.6에서 구현)</p>
      </div>
    </div>
  );
}
```

### UI/UX 구현 세부사항

**폰트:**
- 콘솔 로그: `font-mono` (기본 monospace)
- 타임스탬프/메시지: `text-xs` (12px)

**레이아웃:**
- 각 로그 라인: flex, items-start, gap-2
- 타임스탬프: shrink-0 (줄바꿈 방지)
- 카테고리 배지: shrink-0, rounded, px-1
- 메시지: break-words (긴 메시지 줄바꿈)

**스크롤:**
- 콘솔 영역: overflow-y-auto
- 자동 스크롤: 새 로그 추가 시 하단으로
- 수동 스크롤 감지: scrollTop + clientHeight >= scrollHeight - 50

### Upbit API 참고 (향후 연동)

이 스토리는 콘솔 UI 구현에 집중합니다. API 로그는 이후 스토리(Epic 2, 3)에서 연동됩니다:
- 잔고 조회: `[BALANCE] 잔고 조회 완료: 5개 자산`
- 주문 생성: `[ORDER] 매수 주문 생성: KRW-BTC, 시장가, 100,000원`
- 주문 실패: `[ERROR] 주문 실패: insufficient_funds_bid`

### References

- [Architecture Document: Format Patterns](_bmad-output/planning-artifacts/architecture.md#Format Patterns)
- [Architecture Document: Error Handling & Logging](_bmad-output/planning-artifacts/architecture.md#Error Handling & Logging)
- [UX Design: Component Design](_bmad-output/planning-artifacts/ux-design-specification.md#Component Design)
- [UX Design: Color System](_bmad-output/planning-artifacts/ux-design-specification.md#Color System)
- [WTS Epics: Story 1.6](_bmad-output/planning-artifacts/wts-epics.md#Story 1.6)
- [Previous Story: WTS-1.5](_bmad-output/implementation-artifacts/wts-1-5-connection-status-api-failure.md)
- [Previous Story: WTS-1.4](_bmad-output/implementation-artifacts/wts-1-4-exchange-tab-selection.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A - 구현 중 특별한 디버깅 이슈 없음

### Completion Notes List

- ConsolePanel UI를 플레이스홀더에서 완전한 기능으로 재구현
- ConsoleLogItem 컴포넌트를 React.memo로 메모이제이션하여 성능 최적화
- 타임스탬프 포맷터 (HH:mm:ss.SSS) 유틸리티 구현
- 로그 레벨별 색상 및 카테고리별 배지 스타일 상수 정의
- 자동 스크롤 로직 구현 (새 로그 추가 시 하단으로 스크롤, 사용자 스크롤 중 비활성화)
- detail 필드 접기/펼치기 기능 추가 (에러 로그 상세 정보)
- 자동 스크롤 동작 테스트 보강 및 로그 포맷 표기 개선
- FIFO 로그 관리는 기존 consoleStore에 이미 구현됨 (MAX_CONSOLE_LOGS=1000)

### File List

**신규 파일:**
- apps/desktop/src/wts/utils/formatters.ts
- apps/desktop/src/wts/utils/consoleStyles.ts
- apps/desktop/src/wts/components/ConsoleLogItem.tsx
- apps/desktop/src/wts/__tests__/utils/formatters.test.ts
- apps/desktop/src/wts/__tests__/utils/consoleStyles.test.ts
- apps/desktop/src/wts/__tests__/components/ConsoleLogItem.test.tsx
- apps/desktop/src/wts/__tests__/panels/ConsolePanel.test.tsx

**변경된 파일:**
- apps/desktop/src/wts/panels/ConsolePanel.tsx (완전 재구현)

**작업 트리 상이점 (스토리 범위 외 변경 감지):**
- _bmad-output/implementation-artifacts/5-1-new-market-price-data-verification.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/wts-1-4-exchange-tab-selection.md
- apps/desktop/src-tauri/src/main.rs
- apps/desktop/src-tauri/src/wts/mod.rs
- apps/desktop/src-tauri/src/wts/types.rs
- apps/desktop/src/components/ui/badge.tsx
- apps/desktop/src/wts/WtsWindow.tsx
- apps/desktop/src/wts/__tests__/WtsWindow.test.tsx
- apps/desktop/src/wts/__tests__/index.test.tsx
- apps/desktop/src/wts/__tests__/stores/wtsStore.test.ts
- apps/desktop/src/wts/panels/ExchangePanel.tsx
- apps/desktop/src/wts/stores/wtsStore.ts
- apps/desktop/src/wts/types.ts
- crates/engine/src/detector.rs
- crates/feeds/src/adapter/binance.rs
- crates/feeds/src/adapter/upbit.rs
- crates/feeds/src/aggregator.rs
- crates/feeds/src/subscription.rs

## Change Log

- 2026-01-19: Story WTS-1.6 구현 완료 - 콘솔 패널 기본 구조 및 테스트
- 2026-01-19: 코드 리뷰 지적사항 수정 - 로그 포맷 표기 및 자동 스크롤 테스트 보강

# Story WTS-1.3: 6패널 그리드 레이아웃 구현

Status: done

## Story

As a **트레이더**,
I want **Bloomberg 터미널 스타일의 6패널 레이아웃**,
So that **필요한 모든 정보를 한 화면에서 볼 수 있다**.

## Acceptance Criteria

1. **Given** WTS 창이 열렸을 때 **When** 화면이 렌더링되면 **Then** 콘솔(좌측), 오더북(중앙 상단), 잔고(중앙 하단), 주문(우측 상단), 미체결(우측 하단), 헤더(상단)가 배치되어야 한다
2. **Given** WTS 창이 열렸을 때 **When** 화면이 렌더링되면 **Then** CSS Grid 기반 고정 레이아웃이 적용되어야 한다
3. **Given** WTS 창이 열렸을 때 **When** 화면이 렌더링되면 **Then** 다크 테마(터미널 스타일)가 기본 적용되어야 한다
4. **Given** WTS 창이 열렸을 때 **When** 화면이 렌더링되면 **Then** shadcn/ui 컴포넌트 기반 스타일링이 적용되어야 한다

## Tasks / Subtasks

- [x] Task 1: CSS Grid 기반 레이아웃 구조 구현 (AC: #1, #2)
  - [x] Subtask 1.1: WtsWindow.tsx에 CSS Grid 레이아웃 정의 (헤더 + 3컬럼)
  - [x] Subtask 1.2: 그리드 영역 정의: header, console, orderbook, balances, order, openOrders
  - [x] Subtask 1.3: 패널 비율 설정 (좌측 25%, 중앙 35%, 우측 40%)

- [x] Task 2: 플레이스홀더 패널 컴포넌트 생성 (AC: #1)
  - [x] Subtask 2.1: panels/ExchangePanel.tsx - 거래소 탭 + 연결 상태 (헤더 영역)
  - [x] Subtask 2.2: panels/ConsolePanel.tsx - 콘솔 로그 영역
  - [x] Subtask 2.3: panels/OrderbookPanel.tsx - 오더북 영역
  - [x] Subtask 2.4: panels/BalancePanel.tsx - 잔고 영역
  - [x] Subtask 2.5: panels/OrderPanel.tsx - 주문 폼 영역
  - [x] Subtask 2.6: panels/OpenOrdersPanel.tsx - 미체결 주문 영역

- [x] Task 3: 다크 테마 스타일링 적용 (AC: #3)
  - [x] Subtask 3.1: 배경색 적용 (--background: #0a0a0f, --background-secondary: #111118)
  - [x] Subtask 3.2: 텍스트 색상 적용 (--foreground: #e4e4e7, --foreground-muted: #71717a)
  - [x] Subtask 3.3: 테두리 색상 적용 (--border: #27272a)
  - [x] Subtask 3.4: 패널 간 구분선 또는 간격 (4px)

- [x] Task 4: shadcn/ui 컴포넌트 통합 (AC: #4)
  - [x] Subtask 4.1: shadcn/ui 초기화 (npx shadcn@latest init) - 이미 설정된 경우 생략
  - [x] Subtask 4.2: 필요 컴포넌트 설치: Tabs, Button, ScrollArea - 후속 스토리에서 필요시 설치
  - [x] Subtask 4.3: WTS 전용 다크 테마 CSS 변수 추가 (tailwind.config.ts 또는 globals.css)

- [x] Task 5: 레이아웃 테스트 작성 (AC: #1, #2)
  - [x] Subtask 5.1: WtsWindow 그리드 레이아웃 렌더링 테스트
  - [x] Subtask 5.2: 6개 패널 존재 여부 테스트
  - [x] Subtask 5.3: 다크 테마 클래스 적용 테스트

## Dev Notes

### Architecture 준수사항

**레이아웃 패턴 (UX 설계서 기준):**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design Direction Decision]

```
┌─────────────────────────────────────────────────────────────┐
│  WTS                    [UP][BN][HB][BT][...]     ● 연결됨  │
├───────────────┬─────────────────────┬───────────────────────┤
│               │   ORDERBOOK         │   ORDER ENTRY         │
│               │   ┌───────────────┐ │   ┌─────────────────┐ │
│   CONSOLE     │   │ ASK (매도호가)│ │   │ [Limit][Market] │ │
│               │   │ ───────────── │ │   │ Price: [     ]  │ │
│   (좌측 고정) │   │ BID (매수호가)│ │   │ Qty:   [     ]  │ │
│               │   └───────────────┘ │   │ [25%][50%][MAX] │ │
│               ├─────────────────────┤   │ [BUY]   [SELL]  │ │
│               │   BALANCES          │   └─────────────────┘ │
│               │   ┌───────────────┐ │                       │
│               │   │ BTC: 0.5      │ │   OPEN ORDERS         │
│               │   │ KRW: 1,000,000│ │   ┌─────────────────┐ │
│               │   └───────────────┘ │   │ (미체결 목록)   │ │
└───────────────┴─────────────────────┴───────────────────────┘
```

**CSS Grid 구조:**

```css
.wts-grid {
  display: grid;
  grid-template-columns: 25% 35% 40%;
  grid-template-rows: 40px 60% 40%;
  grid-template-areas:
    "header  header     header"
    "console orderbook  order"
    "console balances   openOrders";
  height: 100vh;
  gap: 4px;
}
```

**색상 시스템 (다크 테마):**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Color System]

| 변수 | 값 | 용도 |
|------|-----|------|
| --background | #0a0a0f | 메인 배경 |
| --background-secondary | #111118 | 패널 배경 |
| --background-tertiary | #1a1a24 | 호버, 선택 상태 |
| --foreground | #e4e4e7 | 메인 텍스트 |
| --foreground-muted | #71717a | 보조 텍스트 |
| --border | #27272a | 기본 테두리 |
| --success | #22c55e | 매수, 성공 |
| --destructive | #ef4444 | 매도, 에러 |
| --warning | #f59e0b | 경고 |
| --accent | #3b82f6 | 강조 |

**타이포그래피:**
[Source: _bmad-output/planning-artifacts/ux-design-specification.md#Typography System]

| 용도 | 폰트 | 크기 |
|------|------|------|
| 가격/숫자 | JetBrains Mono | 14px |
| UI 텍스트 | Inter | 13px |
| 제목 | Inter Semi-Bold | 14px |
| 콘솔 | JetBrains Mono | 12px |

### 네이밍 규칙

**React 컴포넌트:**
[Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]

- 파일명: `{ComponentName}.tsx` (PascalCase)
- 패널 컴포넌트: `panels/` 디렉토리에 위치
- Props 인터페이스: `{Component}Props`

**디렉토리 구조:**
```
apps/desktop/src/wts/
├── WtsWindow.tsx       # 6패널 그리드 레이아웃 (변경)
├── panels/             # 패널 컴포넌트 (신규)
│   ├── ExchangePanel.tsx
│   ├── ConsolePanel.tsx
│   ├── OrderbookPanel.tsx
│   ├── BalancePanel.tsx
│   ├── OrderPanel.tsx
│   └── OpenOrdersPanel.tsx
└── __tests__/
    └── WtsWindow.test.tsx (변경)
```

### 이전 스토리 (WTS-1.2)에서 학습한 사항

**완료된 작업:**
- WtsWindow.tsx 기본 구조 생성 (플레이스홀더)
- Zustand 스토어 설정 (wtsStore, consoleStore)
- React Router /wts 라우트 설정
- Tauri 다중 창 동적 생성

**재사용 가능한 패턴:**
- `useWtsStore` 훅으로 거래소/마켓/연결상태 접근
- `useConsoleStore` 훅으로 콘솔 로그 관리
- 다크 테마 배경: `bg-dark-900` (또는 새 CSS 변수)

**주의사항:**
- 현재 WtsWindow는 중앙 정렬 플레이스홀더 상태
- 기존 스타일 클래스 (`bg-dark-900`, `text-gray-400`) 유지하거나 통합 필요

### Git 최근 커밋 패턴

**최근 작업:**
- `feat(wts): scaffold stores and tests` - WTS 스토어 및 테스트 스캐폴딩
- 커밋 메시지 형식: `feat(wts): 설명`

### 플레이스홀더 패널 구조

각 패널은 최소 기능으로 구현 (후속 스토리에서 확장):

```typescript
// panels/ConsolePanel.tsx
export function ConsolePanel() {
  return (
    <div className="h-full bg-background-secondary p-4 border border-border rounded">
      <h3 className="text-sm font-semibold text-foreground-muted mb-2">Console</h3>
      <div className="text-xs text-foreground-muted">
        콘솔 로그가 여기에 표시됩니다 (Story 1.6에서 구현)
      </div>
    </div>
  );
}
```

### 테스트 전략

**Vitest + Testing Library:**
- jsdom 환경 (vite.config.ts에 설정됨)
- 렌더링 테스트: 6개 패널 존재 확인
- 스타일 테스트: 그리드 클래스 적용 확인

```typescript
import { render, screen } from '@testing-library/react';
import { WtsWindow } from '../WtsWindow';

describe('WtsWindow 6-Panel Layout', () => {
  it('should render all 6 panels', () => {
    render(<WtsWindow />);
    expect(screen.getByTestId('console-panel')).toBeInTheDocument();
    expect(screen.getByTestId('orderbook-panel')).toBeInTheDocument();
    // ...
  });
});
```

### Project Structure Notes

**기존 파일:**
- `apps/desktop/src/wts/WtsWindow.tsx` - 변경 (그리드 레이아웃으로 교체)
- `apps/desktop/src/wts/__tests__/WtsWindow.test.tsx` - 변경 (레이아웃 테스트 추가)

**신규 파일:**
- `apps/desktop/src/wts/panels/ExchangePanel.tsx`
- `apps/desktop/src/wts/panels/ConsolePanel.tsx`
- `apps/desktop/src/wts/panels/OrderbookPanel.tsx`
- `apps/desktop/src/wts/panels/BalancePanel.tsx`
- `apps/desktop/src/wts/panels/OrderPanel.tsx`
- `apps/desktop/src/wts/panels/OpenOrdersPanel.tsx`
- `apps/desktop/src/wts/panels/index.ts` (배럴 파일)

**CSS/스타일 변경:**
- `apps/desktop/src/index.css` 또는 `globals.css` - WTS 다크 테마 변수 추가
- `apps/desktop/tailwind.config.ts` - WTS 색상 확장 (선택적)

### References

- [Architecture Document: Project Structure](_bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure)
- [Architecture Document: Naming Patterns](_bmad-output/planning-artifacts/architecture.md#Naming Patterns)
- [UX Design: Design Direction Decision](_bmad-output/planning-artifacts/ux-design-specification.md#Design Direction Decision)
- [UX Design: Color System](_bmad-output/planning-artifacts/ux-design-specification.md#Color System)
- [UX Design: Typography System](_bmad-output/planning-artifacts/ux-design-specification.md#Typography System)
- [WTS Epics: Story 1.3](_bmad-output/planning-artifacts/wts-epics.md#Story 1.3)
- [Previous Story: WTS-1.2](_bmad-output/implementation-artifacts/wts-1-2-tauri-multiwindow-wts-open.md)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- RED phase: 모든 새 테스트 실패 확인
- GREEN phase: 26/26 테스트 통과
- Build: 성공 (dist 생성)

### Completion Notes List

- Bloomberg 터미널 스타일의 6패널 CSS Grid 레이아웃 구현
- WTS 전용 다크 테마 CSS 변수 시스템 추가 (--wts-* 변수)
- 6개 플레이스홀더 패널 컴포넌트 생성 (ExchangePanel, ConsolePanel, OrderbookPanel, BalancePanel, OrderPanel, OpenOrdersPanel)
- Inter/JetBrains Mono 웹폰트 추가 (타이포그래피 시스템)
- 그리드 영역 클래스 (wts-area-*) 및 패널 베이스 스타일 (wts-panel) 정의
- 연결 상태 표시기 구현 (연결됨/연결 안됨)
- 모든 Acceptance Criteria 충족 확인
- 리뷰 수정: shadcn Badge 적용 및 테마 토큰 정합화
- 리뷰 수정: 그리드 행 비율 40px/60%/40% 반영

### File List

**신규 파일:**
- apps/desktop/src/wts/panels/ExchangePanel.tsx
- apps/desktop/src/wts/panels/ConsolePanel.tsx
- apps/desktop/src/wts/panels/OrderbookPanel.tsx
- apps/desktop/src/wts/panels/BalancePanel.tsx
- apps/desktop/src/wts/panels/OrderPanel.tsx
- apps/desktop/src/wts/panels/OpenOrdersPanel.tsx
- apps/desktop/src/wts/panels/index.ts
- apps/desktop/src/components/ui/badge.tsx
- apps/desktop/src/lib/utils.ts

**변경 파일:**
- apps/desktop/src/wts/WtsWindow.tsx - 6패널 그리드 레이아웃으로 교체
- apps/desktop/src/wts/__tests__/WtsWindow.test.tsx - 레이아웃 테스트 추가
- apps/desktop/src/wts/__tests__/index.test.tsx - 새 레이아웃에 맞게 테스트 수정
- apps/desktop/src/index.css - WTS 다크 테마 CSS 변수 및 그리드 스타일 추가
- apps/desktop/src/wts/panels/ExchangePanel.tsx - 연결 상태 Badge 적용

## Change Log

- 2026-01-18: Story WTS-1.3 구현 완료 - 6패널 그리드 레이아웃, 다크 테마, 플레이스홀더 패널
- 2026-01-18: 리뷰 수정 - shadcn Badge 적용, 테마 토큰/그리드 비율 정합화

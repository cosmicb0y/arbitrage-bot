# Story WTS-1.1: WTS 프로젝트 구조 및 Zustand 설정

Status: done

## Story

As a **개발자**,
I want **WTS 전용 디렉토리 구조와 Zustand 상태 관리가 설정된 환경**,
So that **WTS 기능을 체계적으로 개발할 수 있다**.

## Acceptance Criteria

1. **Given** 기존 arbitrage-bot 프로젝트가 있을 때 **When** WTS 프로젝트 구조를 생성하면 **Then** `apps/desktop/src/wts/` 디렉토리 구조가 생성되어야 한다
2. **Given** WTS 프론트엔드 구조가 생성될 때 **When** Rust 백엔드 모듈이 필요하면 **Then** `apps/desktop/src-tauri/src/wts/` Rust 모듈 구조가 생성되어야 한다
3. **Given** WTS 상태 관리가 필요할 때 **When** Zustand 패키지를 설치하면 **Then** 기본 스토어(wtsStore, consoleStore)가 정의되어야 한다
4. **Given** WTS 컴포넌트 개발이 시작될 때 **When** TypeScript 타입이 필요하면 **Then** WTS 전용 TypeScript 타입 파일(types.ts)이 생성되어야 한다

## Tasks / Subtasks

- [x] Task 1: Zustand 패키지 설치 (AC: #3)
  - [x] `pnpm add zustand` 실행
  - [x] package.json에 zustand 의존성 확인

- [x] Task 2: WTS 프론트엔드 디렉토리 구조 생성 (AC: #1)
  - [x] `apps/desktop/src/wts/` 루트 디렉토리
  - [x] `apps/desktop/src/wts/stores/` 스토어 디렉토리
  - [x] `apps/desktop/src/wts/panels/` 패널 컴포넌트 디렉토리
  - [x] `apps/desktop/src/wts/components/` 공통 컴포넌트 디렉토리
  - [x] `apps/desktop/src/wts/hooks/` 커스텀 훅 디렉토리
  - [x] `apps/desktop/src/wts/utils/` 유틸리티 디렉토리

- [x] Task 3: WTS TypeScript 타입 정의 (AC: #4)
  - [x] `apps/desktop/src/wts/types.ts` 파일 생성
  - [x] Exchange 타입 정의 (Upbit MVP)
  - [x] ConsoleLogEntry 인터페이스 정의
  - [x] WtsState 인터페이스 정의
  - [x] OrderFormState 인터페이스 정의

- [x] Task 4: Zustand 스토어 구현 (AC: #3)
  - [x] `wtsStore.ts` - 거래소/마켓 선택, 연결 상태
  - [x] `consoleStore.ts` - 콘솔 로그 (최대 1000개 FIFO)

- [x] Task 5: WTS Rust 백엔드 모듈 구조 생성 (AC: #2)
  - [x] `apps/desktop/src-tauri/src/wts/mod.rs`
  - [x] `apps/desktop/src-tauri/src/wts/types.rs`
  - [x] main.rs에 wts 모듈 선언 추가

- [x] Task 6: WTS 스토어 테스트 추가
  - [x] `apps/desktop/src/wts/__tests__/stores/wtsStore.test.ts`
  - [x] `apps/desktop/src/wts/__tests__/stores/consoleStore.test.ts`

## Dev Notes

### Architecture 준수사항

**프로젝트 구조 패턴:**
[Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]

```
apps/desktop/src/wts/
├── index.tsx                    # WTS 앱 진입점
├── WtsWindow.tsx                # 6패널 그리드 레이아웃
├── types.ts                     # WTS 전용 TypeScript 타입
├── stores/
│   ├── index.ts                 # Store 내보내기
│   ├── wtsStore.ts              # 거래소/마켓 선택, 연결 상태
│   └── consoleStore.ts          # 콘솔 로그 (최대 1000개)
├── panels/                      # 추후 스토리에서 구현
├── components/                  # 추후 스토리에서 구현
├── hooks/                       # 추후 스토리에서 구현
└── utils/                       # 추후 스토리에서 구현
```

```
apps/desktop/src-tauri/src/wts/
├── mod.rs                       # 모듈 선언
└── types.rs                     # Rust 타입 정의
```

### 네이밍 규칙

**Zustand Store:**
[Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]

- 파일명: `{도메인}Store.ts` (예: `wtsStore.ts`)
- 훅 export: `use{Domain}Store` (예: `useWtsStore`)
- 내부 상태: camelCase (예: `selectedExchange`)
- 액션: camelCase 동사형 (예: `setExchange`)

```typescript
// 올바른 예시
export const useWtsStore = create<WtsState>()((set) => ({
  selectedExchange: 'upbit',
  setExchange: (exchange) => set({ selectedExchange: exchange }),
}));
```

**Rust 모듈:**
- snake_case 파일명
- pub mod 선언

### 기술 스택 버전

[Source: _bmad-output/planning-artifacts/architecture.md#Architectural Decisions Inherited]

- Zustand: 최신 stable (^5.0.0 권장)
- TypeScript: 5.5+
- React: 18.3.1 (기존)
- Rust: 기존 프로젝트 버전 유지

### ConsoleLogEntry 형식

[Source: _bmad-output/planning-artifacts/architecture.md#Format Patterns]

```typescript
interface ConsoleLogEntry {
  id: string;           // 고유 ID (nanoid 또는 crypto.randomUUID)
  timestamp: number;    // Unix ms
  level: 'INFO' | 'SUCCESS' | 'ERROR' | 'WARN';
  category: 'ORDER' | 'BALANCE' | 'DEPOSIT' | 'WITHDRAW' | 'SYSTEM';
  message: string;      // 사용자 친화적 메시지
  detail?: unknown;     // API 응답 원본 (디버깅용)
}
```

### WtsStore 초기 상태

```typescript
interface WtsState {
  // 거래소 선택
  selectedExchange: Exchange;
  setExchange: (exchange: Exchange) => void;

  // 마켓 선택
  selectedMarket: string | null;
  setMarket: (market: string | null) => void;

  // 연결 상태
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  setConnectionStatus: (status: ConnectionStatus) => void;
}
```

### ConsoleStore 초기 상태

```typescript
interface ConsoleState {
  logs: ConsoleLogEntry[];
  addLog: (level: LogLevel, category: Category, message: string, detail?: unknown) => void;
  clearLogs: () => void;
}

// 최대 1000개 로그 유지 (FIFO)
const MAX_LOGS = 1000;
```

### Project Structure Notes

**기존 프로젝트 구조와의 정렬:**
- `apps/desktop/src/` - 기존 모니터링 앱 컴포넌트
- `apps/desktop/src/wts/` - WTS 전용 컴포넌트 (새로 생성)
- `apps/desktop/src-tauri/src/` - 기존 Rust 백엔드
- `apps/desktop/src-tauri/src/wts/` - WTS Rust 모듈 (새로 생성)

**충돌 방지:**
- WTS 컴포넌트는 모두 `src/wts/` 하위에 배치
- 기존 `src/types.ts`와 별도로 `src/wts/types.ts` 사용
- main.rs 수정 시 기존 코드 영향 최소화

### References

- [Architecture Document: Project Structure & Boundaries](_bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries)
- [Architecture Document: Naming Patterns](_bmad-output/planning-artifacts/architecture.md#Naming Patterns)
- [Architecture Document: Format Patterns](_bmad-output/planning-artifacts/architecture.md#Format Patterns)
- [UX Design: Design System Foundation](_bmad-output/planning-artifacts/ux-design-specification.md#Design System Foundation)
- [WTS Epics: Story 1.1](_bmad-output/planning-artifacts/wts-epics.md#Story 1.1)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- `pnpm add zustand` - 성공 (zustand 5.0.10 설치)
- `pnpm build` - 성공 (TypeScript 빌드)
- `pnpm install --lockfile-only` - 성공 (vitest lockfile 갱신)
- `cargo build` - 성공 (8 warnings - 미사용 타입, 추후 스토리에서 사용 예정)

### Completion Notes List

- Zustand 5.0.10 패키지 설치 완료
- WTS 프론트엔드 디렉토리 구조 생성: stores, panels, components, hooks, utils
- WTS TypeScript 타입 정의 (types.ts): Exchange, ConnectionStatus, ConsoleLogEntry, WtsState, OrderFormState
- Zustand 스토어 구현: wtsStore (거래소/마켓 선택, 연결 상태), consoleStore (FIFO 1000개 로그)
- WTS Rust 백엔드 모듈 구조 생성: mod.rs, types.rs (TypeScript와 1:1 매칭 타입)
- main.rs에 wts 모듈 선언 추가
- 콘솔 로그 FIFO 순서 정렬 및 안전한 ID 생성 적용
- WTS 스토어 테스트 추가 (wtsStore, consoleStore)
- Vitest 도입 및 lockfile 갱신
- TypeScript 빌드 및 Rust 빌드 검증 완료

### File List

**새로 생성:**
- `apps/desktop/src/wts/types.ts`
- `apps/desktop/src/wts/stores/wtsStore.ts`
- `apps/desktop/src/wts/stores/consoleStore.ts`
- `apps/desktop/src/wts/stores/index.ts`
- `apps/desktop/src/wts/__tests__/stores/wtsStore.test.ts`
- `apps/desktop/src/wts/__tests__/stores/consoleStore.test.ts`
- `apps/desktop/src-tauri/src/wts/mod.rs`
- `apps/desktop/src-tauri/src/wts/types.rs`
- `docs/api-contracts.md`
- `docs/development-guide.md`
- `docs/index.md`
- `docs/project-overview.md`
- `docs/source-tree-analysis.md`

**수정:**
- `apps/desktop/package.json` (zustand 의존성 추가)
- `apps/desktop/pnpm-lock.yaml` (vitest lockfile 갱신)
- `apps/desktop/src-tauri/src/main.rs` (wts 모듈 선언 추가)
- `docs/ARCHITECTURE.md`
- `docs/DATA_MODEL.md` (삭제)
- `symbol_mappings.json`

**디렉토리 생성:**
- `apps/desktop/src/wts/`
- `apps/desktop/src/wts/stores/`
- `apps/desktop/src/wts/panels/`
- `apps/desktop/src/wts/components/`
- `apps/desktop/src/wts/hooks/`
- `apps/desktop/src/wts/utils/`
- `apps/desktop/src/wts/__tests__/`
- `apps/desktop/src/wts/__tests__/stores/`
- `apps/desktop/src-tauri/src/wts/`

## Change Log

- 2026-01-18: 초기 구현 완료 - WTS 프로젝트 구조 및 Zustand 설정
- 2026-01-18: 리뷰 수정 - 콘솔 FIFO 정렬, 스토어 테스트 추가, vitest lockfile 갱신


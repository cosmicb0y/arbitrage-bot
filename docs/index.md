# Arbitrage Bot - 문서 인덱스

## 프로젝트 개요

| 항목 | 내용 |
|------|------|
| **프로젝트 유형** | Monorepo (3개 파트) |
| **도메인** | 암호화폐 차익거래 탐지 시스템 |
| **주요 언어** | Rust (백엔드), TypeScript (프론트엔드) |
| **아키텍처** | 마이크로서비스 + 이벤트 드리븐 |

---

## 빠른 참조

### 기술 스택

| 범주 | 기술 |
|------|------|
| **백엔드 런타임** | Tokio (Rust) |
| **웹 프레임워크** | Axum 0.7 |
| **데스크톱 프레임워크** | Tauri 2.0 |
| **프론트엔드** | React 18 + TypeScript 5.5 |
| **스타일링** | Tailwind CSS 3.4 |
| **WebSocket** | tokio-tungstenite 0.24 |
| **동시성** | DashMap, Crossbeam |

### 지원 거래소

| 거래소 | 호가 통화 | 상태 |
|--------|----------|------|
| Binance | USDT, USDC | ✅ Active |
| Coinbase | USD, USDT, USDC | ✅ Active |
| Bybit | USDT, USDC | ✅ Active |
| GateIO | USDT, USDC | ✅ Active |
| Upbit | KRW | ✅ Active |
| Bithumb | KRW | ✅ Active |

### 프로젝트 구조

| 파트 | 유형 | 경로 |
|------|------|------|
| server | Backend CLI Server | `apps/server` |
| desktop | Tauri Desktop App | `apps/desktop` |
| libraries | Rust Library Crates | `crates/` |

---

## 생성된 문서

### 핵심 문서

- [프로젝트 개요](./project-overview.md) - 프로젝트 소개, 기능, 기술 스택
- [시스템 아키텍처](./architecture.md) - 상세 아키텍처, 크레이트 구조, 동시성 모델
- [API 계약](./api-contracts.md) - WebSocket 메시지 포맷, 데이터 구조
- [개발 가이드](./development-guide.md) - 환경 설정, 빌드, 테스트, 기여
- [소스 트리 분석](./source-tree-analysis.md) - 디렉토리 구조, 파일별 역할

### 기존 문서

- [README.md](../README.md) - 프로젝트 README

---

## 빠른 시작

### 1. CLI 서버 실행

```bash
# 시뮬레이션 모드
cargo run -p arbitrage-server

# 라이브 모드
cargo run -p arbitrage-server -- --live
```

### 2. 데스크톱 앱 실행

```bash
cd apps/desktop
pnpm install
pnpm tauri dev
```

### 3. WebSocket 연결

```bash
wscat -c ws://localhost:9001/ws
```

---

## 주요 명령어

| 명령어 | 설명 |
|--------|------|
| `cargo build --release` | 릴리스 빌드 |
| `cargo test --workspace` | 전체 테스트 |
| `cargo fmt` | 코드 포맷팅 |
| `cargo clippy --workspace` | 린트 검사 |
| `pnpm tauri dev` | 데스크톱 앱 개발 모드 |
| `pnpm tauri build` | 데스크톱 앱 빌드 |

---

## 환경 변수

| 변수 | 용도 |
|------|------|
| `TELEGRAM_BOT_TOKEN` | 차익거래 알림 |
| `BINANCE_API_KEY` / `SECRET_KEY` | Binance API |
| `COINBASE_API_KEY_ID` / `SECRET_KEY` | Coinbase API |
| `UPBIT_ACCESS_KEY` / `SECRET_KEY` | Upbit API |
| `BITHUMB_API_KEY` / `SECRET_KEY` | Bithumb API |
| `BYBIT_API_KEY` / `SECRET_KEY` | Bybit API |
| `GATEIO_API_KEY` / `SECRET_KEY` | Gate.io API |

---

## 다음 단계

### Brownfield PRD 작성 시

새 기능 계획시 이 문서를 PRD 워크플로우 입력으로 제공하세요:

```
docs/index.md
```

### UI 기능 개발 시

참조 문서:
- `docs/architecture.md` (Desktop Layer 섹션)
- `apps/desktop/src/` (React 컴포넌트)

### API 기능 개발 시

참조 문서:
- `docs/api-contracts.md`
- `apps/server/src/ws_server.rs`

### 새 거래소 추가 시

참조 문서:
- `docs/development-guide.md` (새 거래소 추가 섹션)
- `crates/feeds/src/adapter/`
- `crates/feeds/src/runner/`

---

## 문서 메타데이터

| 항목 | 값 |
|------|-----|
| **생성일** | 2026-01-11 |
| **스캔 모드** | Exhaustive (전체 분석) |
| **생성 문서 수** | 6개 |
| **프로젝트 버전** | 0.1.0 |

---

*이 문서는 AI 기반 개발을 위한 주요 진입점입니다.*

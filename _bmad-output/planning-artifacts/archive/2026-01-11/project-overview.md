# Arbitrage Bot - 프로젝트 개요

**생성일**: 2026-01-11
**프로젝트 타입**: Rust Monorepo (Cargo Workspace)
**도메인**: 암호화폐 차익거래 탐지 시스템

---

## 요약

Arbitrage Bot은 다중 암호화폐 거래소 간의 가격 차이를 실시간으로 모니터링하고 차익거래 기회를 탐지하는 고성능 Rust 기반 시스템입니다.

### 핵심 기능

| 기능 | 설명 |
|------|------|
| **실시간 가격 피드** | 6개 거래소 WebSocket 연결 (Binance, Coinbase, Bybit, GateIO, Upbit, Bithumb) |
| **차익거래 탐지** | 거래소 간 프리미엄 계산 및 기회 탐지 (김치 프리미엄 포함) |
| **가격 정규화** | USDT/USDC 디페깅 대응, 다중 호가통화 USD 변환 |
| **데스크톱 GUI** | Tauri + React 기반 실시간 모니터링 대시보드 |
| **텔레그램 알림** | 실시간 차익거래 기회 알림 |
| **CLI 서버** | 헤드리스 모드 지원 |

---

## 기술 스택

### Backend (Rust)

| 카테고리 | 기술 | 버전 |
|----------|------|------|
| 언어 | Rust | 2021 Edition |
| 런타임 | Tokio | 1.x |
| 웹 프레임워크 | Axum | 0.7 |
| CLI | Clap | 4 |
| 동시성 | DashMap, Crossbeam | 6, 0.8 |
| 직렬화 | Serde, Serde JSON | 1.x |
| 암호화 | HMAC, SHA2, P256 | 0.12, 0.10, 0.13 |
| WebSocket | tokio-tungstenite | 0.24 |

### Frontend (Desktop App)

| 카테고리 | 기술 | 버전 |
|----------|------|------|
| 프레임워크 | React | 18.3 |
| 언어 | TypeScript | 5.5 |
| 빌드 도구 | Vite | 5.3 |
| 스타일링 | Tailwind CSS | 3.4 |
| 차트 | Recharts | 2.12 |
| 데스크톱 | Tauri | 2.0 |

### 인프라

| 카테고리 | 기술 |
|----------|------|
| 데이터베이스 | SQLite (알림 설정 저장) |
| 컨테이너 | Docker, Docker Compose |
| 알림 | Telegram Bot API |

---

## 저장소 구조

```
arbitrage-bot/                    # Cargo Workspace Root
├── crates/                       # 라이브러리 크레이트
│   ├── core/                     # 핵심 데이터 타입
│   ├── feeds/                    # WebSocket 가격 피드
│   ├── engine/                   # 차익거래 탐지 엔진
│   ├── executor/                 # 거래 실행 (개발 중)
│   └── alerts/                   # 텔레그램 알림
├── apps/                         # 애플리케이션
│   ├── server/                   # CLI 서버
│   └── desktop/                  # Tauri 데스크톱 앱
│       ├── src/                  # React 프론트엔드
│       └── src-tauri/            # Rust 백엔드
├── docs/                         # 문서
├── data/                         # 런타임 데이터 (SQLite)
└── Cargo.toml                    # Workspace 설정
```

---

## 파트 구성

### 1. Server (Backend CLI)
- **경로**: `apps/server/`
- **타입**: Backend
- **진입점**: `cargo run -p arbitrage-server`
- **기능**: 헤드리스 서버, WebSocket 브로드캐스트, REST 헬스체크

### 2. Desktop (Tauri App)
- **경로**: `apps/desktop/`
- **타입**: Desktop
- **진입점**: `pnpm tauri dev`
- **기능**: 실시간 대시보드, 차익거래 모니터링, 설정 관리

### 3. Libraries (Rust Crates)
- **경로**: `crates/`
- **타입**: Library
- **구성**: core, feeds, engine, executor, alerts
- **기능**: 재사용 가능한 도메인 로직

---

## 지원 거래소

| 거래소 | 호가 통화 | 데이터 소스 | 상태 |
|--------|----------|-------------|------|
| Binance | USDT, USDC | WebSocket + REST | Active |
| Coinbase | USD, USDT, USDC | WebSocket (L2) | Active |
| Bybit | USDT, USDC | WebSocket | Active |
| Gate.io | USDT, USDC | WebSocket | Active |
| Upbit | KRW | WebSocket | Active |
| Bithumb | KRW | WebSocket | Active |
| Kraken | USD | - | Planned |
| OKX | USDT | - | Planned |

---

## 빠른 시작

### 요구 사항
- Rust 1.75+
- Node.js 18+
- pnpm

### CLI 서버 실행
```bash
# 빌드
cargo build

# 시뮬레이션 모드
cargo run -p arbitrage-server

# 라이브 피드
cargo run -p arbitrage-server -- --live
```

### 데스크톱 앱 실행
```bash
cd apps/desktop
pnpm install
pnpm tauri dev
```

---

## 관련 문서

- [README.md](../README.md) - 프로젝트 소개
- [ARCHITECTURE.md](ARCHITECTURE.md) - 상세 아키텍처
- [DATA_MODEL.md](DATA_MODEL.md) - 데이터 모델 및 ERD
- [api-contracts.md](api-contracts.md) - API 및 WebSocket 메시지 명세
- [component-inventory.md](component-inventory.md) - UI 컴포넌트 인벤토리
- [development-guide.md](development-guide.md) - 개발 가이드
- [source-tree-analysis.md](source-tree-analysis.md) - 소스 트리 분석

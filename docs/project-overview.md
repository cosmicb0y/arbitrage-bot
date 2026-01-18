# Arbitrage Bot - 프로젝트 개요

## 요약

**Arbitrage Bot**은 다중 암호화폐 거래소 간 가격 차이를 실시간으로 모니터링하고 차익거래 기회를 탐지하는 고성능 Rust 기반 시스템입니다.

| 항목 | 내용 |
|------|------|
| **프로젝트 유형** | Monorepo (3개 파트) |
| **도메인** | 암호화폐 차익거래 탐지 시스템 |
| **주요 언어** | Rust (백엔드), TypeScript (프론트엔드) |
| **아키텍처** | 마이크로서비스 + 이벤트 드리븐 |
| **라이선스** | MIT |

---

## 핵심 기능

### 1. 실시간 가격 피드
- 6개 거래소 WebSocket 연결 (Binance, Coinbase, Bybit, GateIO, Upbit, Bithumb)
- 자동 재연결 및 Circuit Breaker 패턴
- 멀티 호가통화 지원 (USD, USDT, USDC, KRW)

### 2. 차익거래 탐지
- **Raw Premium**: 직접 가격 비교
- **USDlike Premium**: 동일 스테이블코인 기준 비교 (USDT↔USDT)
- **Kimchi Premium**: USD/KRW 환율 기반 한국 프리미엄

### 3. 최적 거래량 계산
- 오더북 깊이 분석 (Depth Walking Algorithm)
- 수수료 및 출금 비용 고려
- 수익성 있는 최대 거래량 자동 계산

### 4. 데스크톱 GUI
- Tauri 2.0 + React 18 기반
- 실시간 대시보드 및 프리미엄 매트릭스
- 지갑 상태 및 입출금 가능 여부 표시

### 5. 텔레그램 알림
- 차익거래 기회 실시간 알림
- 연결 상태 모니터링

---

## 기술 스택

### 백엔드 (Rust)

| 범주 | 기술 | 버전 |
|------|------|------|
| **런타임** | Tokio | 1.x |
| **웹 프레임워크** | Axum | 0.7 |
| **WebSocket** | tokio-tungstenite | 0.24 |
| **CLI** | Clap | 4.x |
| **동시성** | DashMap, Crossbeam | 6.x, 0.8 |
| **직렬화** | Serde, serde_json | 1.x |
| **데이터베이스** | SQLite (SQLx) | 0.8 |
| **암호화** | HMAC, SHA2, P256 | 0.12, 0.10, 0.13 |

### 프론트엔드 (TypeScript)

| 범주 | 기술 | 버전 |
|------|------|------|
| **프레임워크** | React | 18.3 |
| **데스크톱** | Tauri | 2.0 |
| **빌드 도구** | Vite | 5.3 |
| **스타일링** | Tailwind CSS | 3.4 |
| **차트** | Recharts | 2.12 |
| **언어** | TypeScript | 5.5 |

---

## 프로젝트 구조

```
arbitrage-bot/
├── crates/                    # Rust 라이브러리 크레이트
│   ├── core/                  # 핵심 데이터 타입
│   ├── feeds/                 # WebSocket 가격 피드
│   ├── engine/                # 차익거래 탐지 엔진
│   ├── executor/              # 거래 실행 (개발 중)
│   └── alerts/                # 텔레그램 알림
├── apps/
│   ├── server/                # CLI 헤드리스 서버
│   └── desktop/               # Tauri 데스크톱 앱
│       ├── src/               # React 프론트엔드
│       └── src-tauri/         # Rust 백엔드
├── docs/                      # 프로젝트 문서
└── Cargo.toml                 # 워크스페이스 설정
```

---

## 지원 거래소

| 거래소 | 호가 통화 | WebSocket | 지갑 API | 상태 |
|--------|----------|-----------|----------|------|
| Binance | USDT, USDC | ✅ | ✅ (HMAC) | Active |
| Coinbase | USD, USDT, USDC | ✅ | ✅ (ES256) | Active |
| Bybit | USDT, USDC | ✅ | ✅ (HMAC) | Active |
| GateIO | USDT, USDC | ✅ | ✅ (HMAC) | Active |
| Upbit | KRW | ✅ | ✅ (JWT) | Active |
| Bithumb | KRW | ✅ | ✅ (JWT) | Active |

---

## 데이터 흐름

```
┌─────────────────────────────────────────────────────────────┐
│                      거래소 WebSocket                        │
│         Binance, Coinbase, Bybit, GateIO, Upbit, Bithumb    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    CLI Server (Rust)                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │  Feeds   │→ │  Engine  │→ │ Detector │→ │ Broadcast│     │
│  │(WebSocket)│  │(Premium) │  │(Optimal) │  │(WebSocket)│    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
└─────────────────────────────────────────────────────────────┘
                              │
                    WebSocket (:9001)
                              │
          ┌───────────────────┴───────────────────┐
          ▼                                       ▼
┌──────────────────┐                   ┌──────────────────┐
│  Desktop App     │                   │  Telegram Bot    │
│  (Tauri + React) │                   │  (Alerts)        │
└──────────────────┘                   └──────────────────┘
```

---

## 실행 모드

| 모드 | 설명 |
|------|------|
| **AlertOnly** (기본) | 기회 탐지, 클라이언트 브로드캐스트, 실행 없음 |
| **ManualApproval** | 수동 승인 후 거래 실행 |
| **Auto** | 설정된 임계값에 따라 자동 실행 |

---

## 빠른 시작

### 1. 의존성 설치

```bash
# Rust 크레이트 빌드
cargo build --release

# 프론트엔드 의존성
cd apps/desktop && pnpm install
```

### 2. CLI 서버 실행

```bash
# 시뮬레이션 모드
cargo run -p arbitrage-server

# 라이브 WebSocket 피드
cargo run -p arbitrage-server -- --live
```

### 3. 데스크톱 앱 실행

```bash
cd apps/desktop && pnpm tauri dev
```

---

## 환경 변수

| 변수 | 용도 |
|------|------|
| `TELEGRAM_BOT_TOKEN` | 차익거래 알림 봇 토큰 |
| `TELEGRAM_STATUS_BOT_TOKEN` | 연결 상태 알림 봇 토큰 |
| `BINANCE_API_KEY` / `SECRET_KEY` | Binance 지갑 API |
| `COINBASE_API_KEY_ID` / `SECRET_KEY` | Coinbase CDP API |
| `UPBIT_ACCESS_KEY` / `SECRET_KEY` | Upbit API |
| `BITHUMB_API_KEY` / `SECRET_KEY` | Bithumb API |
| `BYBIT_API_KEY` / `SECRET_KEY` | Bybit API |
| `GATEIO_API_KEY` / `SECRET_KEY` | Gate.io API |

---

## 관련 문서

- [아키텍처](./architecture.md) - 시스템 아키텍처 상세
- [API 계약](./api-contracts.md) - WebSocket 메시지 포맷
- [개발 가이드](./development-guide.md) - 개발 환경 설정
- [소스 트리 분석](./source-tree-analysis.md) - 디렉토리 구조 상세

---

*문서 생성일: 2026-01-11 | 버전: 0.1.0*

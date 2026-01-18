# Source Tree Analysis - 소스 트리 분석

**생성일**: 2026-01-11

이 문서는 arbitrage-bot 프로젝트의 전체 디렉토리 구조와 주요 파일들을 설명합니다.

---

## 프로젝트 루트

```
arbitrage-bot/
├── Cargo.toml                    # Workspace 설정
├── Cargo.lock                    # 의존성 잠금 파일
├── Dockerfile                    # Docker 이미지 빌드
├── docker-compose.yml            # Docker Compose 설정
├── README.md                     # 프로젝트 소개
├── .gitignore                    # Git 무시 파일
├── .env                          # 환경 변수 (비밀 정보)
│
├── crates/                       # 📦 라이브러리 크레이트
├── apps/                         # 🚀 애플리케이션
├── docs/                         # 📚 문서
├── data/                         # 💾 런타임 데이터
├── target/                       # 🔧 빌드 산출물 (Git 무시)
│
├── network_name_mapping.json     # 거래소 간 네트워크명 매핑
├── symbol_mappings.json          # 심볼 disambiguation 매핑
└── config.json                   # 런타임 설정 (선택적)
```

---

## crates/ - 라이브러리 크레이트

### crates/core/ - 핵심 데이터 타입

```
crates/core/
├── Cargo.toml
└── src/
    ├── lib.rs                    # 모듈 export (38 LOC)
    ├── exchange.rs               # Exchange, ExchangeType 열거형
    ├── chain.rs                  # Chain, BridgeProtocol 열거형
    ├── quote_currency.rs         # QuoteCurrency 열거형
    ├── fixed_point.rs            # FixedPoint 고정소수점 타입
    ├── price_tick.rs             # PriceTick 가격 틱 (71 bytes)
    ├── orderbook.rs              # OrderbookSnapshot 호가창
    ├── asset.rs                  # Asset 자산 정의
    ├── trading_pair.rs           # TradingPair 거래쌍
    ├── bridge.rs                 # BridgeRoute 브릿지 경로
    ├── route_step.rs             # RouteStep 거래 경로 단계
    ├── opportunity.rs            # ArbitrageOpportunity 차익거래 기회
    ├── execution.rs              # ExecutionState, ExecutionConfig
    └── error.rs                  # CoreError 에러 타입
```

**주요 타입**:
- `Exchange` (u16): 거래소 식별자 (Binance=100, Upbit=105, ...)
- `FixedPoint` (u64): 8자리 고정소수점 가격
- `PriceTick`: 71 bytes packed 가격 데이터
- `ArbitrageOpportunity`: 차익거래 기회 완전 정의

---

### crates/feeds/ - WebSocket 가격 피드

```
crates/feeds/
├── Cargo.toml
└── src/
    ├── lib.rs                    # 모듈 export
    ├── config.rs                 # FeedConfig 피드 설정
    ├── message.rs                # FeedMessage, ParsedTick, Orderbook
    ├── aggregator.rs             # PriceAggregator 가격 집계
    ├── manager.rs                # FeedManager 연결 관리
    ├── runner.rs                 # FeedRunner 피드 실행기
    ├── symbol_mapping.rs         # SymbolMapping 심볼 정규화
    │
    └── adapter/                  # 📡 거래소별 어댑터
        ├── mod.rs
        ├── binance.rs            # Binance 어댑터
        ├── coinbase.rs           # Coinbase 어댑터 (JWT 인증)
        ├── bybit.rs              # Bybit 어댑터
        ├── gateio.rs             # GateIO 어댑터
        ├── upbit.rs              # Upbit 어댑터 (KRW 마켓)
        └── bithumb.rs            # Bithumb 어댑터 (KRW 마켓)
```

**주요 타입**:
- `ParsedTick`: 정규화된 가격 틱 (Price 또는 StablecoinRate)
- `Orderbook`: 호가창 스냅샷/델타
- `ConnectionEvent`: 연결 상태 이벤트

---

### crates/engine/ - 차익거래 탐지 엔진

```
crates/engine/
├── Cargo.toml
└── src/
    ├── lib.rs                    # 모듈 export
    ├── detector.rs               # OpportunityDetector 기회 탐지기
    ├── premium.rs                # PremiumMatrix 프리미엄 계산
    ├── denominated_prices.rs     # DenominatedPrices 다중 통화 가격
    ├── orderbook.rs              # Orderbook 관리
    ├── depth.rs                  # DepthAnalyzer 깊이 분석
    ├── fee.rs                    # FeeCalculator 수수료 계산
    └── route.rs                  # RouteFinder 경로 탐색 (플레이스홀더)
```

**주요 타입**:
- `OpportunityDetector`: DashMap 기반 lock-free 탐지기
- `PremiumMatrix`: 거래소 쌍별 프리미엄 계산
- `DenominatedPrices`: raw/usd/usdlike 다중 통화 가격

---

### crates/executor/ - 거래 실행

```
crates/executor/
├── Cargo.toml
└── src/
    ├── lib.rs                    # 모듈 export
    ├── order.rs                  # Order, OrderType, OrderStatus
    ├── cex.rs                    # CexExecutor, CexClient 트레이트
    ├── dex.rs                    # DexExecutor, DexClient 트레이트
    └── error.rs                  # ExecutorError 에러 타입
```

**주요 타입**:
- `Order`: 주문 관리 (상태 머신)
- `CexClient`: CEX 거래소 클라이언트 트레이트
- `DexClient`: DEX 스왑 클라이언트 트레이트
- `ExecutionResult`: 실행 결과

**상태**: 개발 중 (실제 거래 미구현)

---

### crates/alerts/ - 텔레그램 알림

```
crates/alerts/
├── Cargo.toml
└── src/
    ├── lib.rs                    # 모듈 export
    ├── config.rs                 # AlertConfig 사용자 설정
    ├── db.rs                     # Database SQLite 저장소
    ├── telegram.rs               # TelegramBot 봇 핸들러
    └── notifier.rs               # Notifier 알림 발송기
```

**주요 타입**:
- `AlertConfig`: 사용자별 알림 설정
- `Notifier`: 중복 제거, 쿨다운 처리
- `TransferPathChecker`: 전송 경로 확인 함수

---

## apps/ - 애플리케이션

### apps/server/ - CLI 서버

```
apps/server/
├── Cargo.toml
└── src/
    ├── main.rs                   # 📍 진입점 (1,313 LOC)
    │   ├── CLI 파싱 (Clap)
    │   ├── 피드 스포닝
    │   ├── 탐지 루프
    │   └── 백그라운드 태스크
    │
    ├── ws_server.rs              # 🌐 WebSocket 서버 (890 LOC)
    │   ├── /ws 라우트
    │   ├── /health 엔드포인트
    │   └── 브로드캐스트 함수들
    │
    ├── state.rs                  # 📊 공유 상태 (1,033 LOC)
    │   ├── SharedState
    │   └── 가격/기회 저장소
    │
    ├── config.rs                 # ⚙️ 설정 (162 LOC)
    ├── exchange_rate.rs          # 💱 환율 업데이터 (128 LOC)
    ├── wallet_status.rs          # 👛 지갑 상태 (1,193 LOC)
    ├── status_notifier.rs        # 📬 텔레그램 알림 (231 LOC)
    │
    └── feeds/                    # 📡 피드 핸들러
        ├── mod.rs                # FeedContext
        ├── handler.rs            # FeedHandler (267 LOC)
        └── common.rs             # 공통 유틸리티
```

**진입점**: `cargo run -p arbitrage-server`

---

### apps/desktop/ - Tauri 데스크톱 앱

```
apps/desktop/
├── package.json                  # Node.js 의존성
├── pnpm-lock.yaml
├── tsconfig.json                 # TypeScript 설정
├── tsconfig.node.json
├── vite.config.ts                # Vite 빌드 설정
├── tailwind.config.js            # Tailwind 설정
├── postcss.config.js
├── index.html                    # HTML 템플릿
│
├── src/                          # 🎨 React 프론트엔드
│   ├── App.tsx                   # 루트 컴포넌트
│   ├── main.tsx                  # React 진입점
│   ├── App.css                   # 전역 스타일
│   ├── types.ts                  # TypeScript 타입 정의
│   │
│   ├── hooks/
│   │   └── useTauri.ts           # 🔌 Tauri IPC 훅들
│   │
│   └── components/
│       ├── Header.tsx            # 헤더 & 통계
│       ├── Dashboard.tsx         # 대시보드 & 프리미엄 매트릭스
│       ├── Opportunities.tsx     # 기회 테이블
│       ├── Markets.tsx           # 마켓 목록
│       ├── Wallets.tsx           # 지갑 현황
│       └── Settings.tsx          # 설정 폼
│
└── src-tauri/                    # ⚙️ Rust 백엔드
    ├── Cargo.toml
    ├── tauri.conf.json           # Tauri 설정
    ├── capabilities/             # 권한 설정
    ├── icons/                    # 앱 아이콘
    │
    └── src/
        ├── main.rs               # Tauri 진입점
        ├── lib.rs                # 라이브러리 설정
        ├── commands.rs           # 🔧 IPC 커맨드 (21개)
        ├── state.rs              # AppState
        ├── credentials.rs        # API 자격증명 관리
        ├── exchange_client.rs    # 거래소 API 클라이언트
        └── symbol_mapping.rs     # 심볼 매핑
```

**진입점**: `pnpm tauri dev`

---

## docs/ - 문서

```
docs/
├── index.md                      # 문서 인덱스 (AI 검색용)
├── project-overview.md           # 프로젝트 개요
├── ARCHITECTURE.md               # 상세 아키텍처 (기존)
├── DATA_MODEL.md                 # 데이터 모델 ERD (기존)
├── api-contracts.md              # API/WebSocket 명세
├── component-inventory.md        # UI 컴포넌트 인벤토리
├── source-tree-analysis.md       # 소스 트리 분석 (이 문서)
├── development-guide.md          # 개발 가이드
└── project-scan-report.json      # 스캔 상태 파일
```

---

## data/ - 런타임 데이터

```
data/
└── alerts.db                     # SQLite 데이터베이스
                                  # - AlertConfig 사용자 설정
                                  # - AlertHistory 알림 이력
                                  # - ActiveOpportunity 활성 기회
```

---

## 주요 설정 파일

| 파일 | 설명 |
|------|------|
| `Cargo.toml` | Workspace 루트 설정, 공통 의존성 |
| `.env` | 환경 변수 (API 키, 시크릿) |
| `config.json` | 런타임 설정 (선택적) |
| `network_name_mapping.json` | 거래소 간 네트워크명 매핑 |
| `symbol_mappings.json` | 심볼 disambiguation |
| `tauri.conf.json` | Tauri 앱 설정 |
| `vite.config.ts` | Vite 빌드 설정 |
| `tailwind.config.js` | Tailwind CSS 설정 |

---

## 코드 통계

| 파트 | 언어 | LOC (추정) |
|------|------|-----------|
| crates/ | Rust | ~5,000 |
| apps/server/ | Rust | ~4,800 |
| apps/desktop/src-tauri/ | Rust | ~3,100 |
| apps/desktop/src/ | TypeScript/TSX | ~2,500 |
| **Total** | - | **~15,400** |

---

## 빌드 산출물

```
target/
├── debug/                        # 디버그 빌드
│   ├── arbitrage-bot             # CLI 서버 바이너리
│   └── ...
├── release/                      # 릴리스 빌드
│   ├── arbitrage-bot
│   └── ...
└── deps/                         # 의존성 캐시

apps/desktop/
├── dist/                         # Vite 빌드 결과
└── src-tauri/target/
    └── release/bundle/           # Tauri 앱 번들
        ├── macos/                # .app 파일
        ├── dmg/                  # DMG 이미지
        └── ...
```

---

## Git 무시 패턴

```gitignore
# Rust
/target/
Cargo.lock (워크스페이스에서는 포함)

# Node.js
node_modules/
dist/

# IDE
.idea/
.vscode/

# 환경 변수
.env
.env.local

# 데이터
data/*.db

# macOS
.DS_Store
```

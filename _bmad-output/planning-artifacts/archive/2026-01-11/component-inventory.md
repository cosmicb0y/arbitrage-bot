# Component Inventory - UI 컴포넌트 인벤토리

**생성일**: 2026-01-11

이 문서는 arbitrage-bot 데스크톱 애플리케이션의 UI 컴포넌트와 상태 관리 패턴을 정의합니다.

---

## 컴포넌트 목록

### 1. App.tsx

**경로**: `apps/desktop/src/App.tsx`
**타입**: Root Component

| 속성 | 값 |
|------|------|
| Props | 없음 |
| State | `activeTab: Tab` |
| Hooks | `useCommonMarkets()` |
| 자식 컴포넌트 | Dashboard, Opportunities, Markets, Wallets, Settings, Header |

**역할**: 메인 애플리케이션 셸, 탭 기반 네비게이션

---

### 2. Header.tsx

**경로**: `apps/desktop/src/components/Header.tsx`
**타입**: Navigation Component

| 속성 | 값 |
|------|------|
| Props | `activeTab`, `onTabChange` |
| State | 없음 (Stateless) |
| Tauri Commands | `start_bot()`, `stop_bot()` |
| Hooks | `useStats()`, `useBotControl()` |

**역할**: 상단 네비게이션 바, 봇 상태 표시, 시작/정지 버튼

**표시 정보**:
- Uptime (HH:MM:SS)
- Price updates 카운트
- Opportunities detected 카운트
- Trades executed 카운트
- Bot running 상태 인디케이터

---

### 3. Dashboard.tsx

**경로**: `apps/desktop/src/components/Dashboard.tsx`
**타입**: Main View Component

| 속성 | 값 |
|------|------|
| Props | 없음 |
| State | `selectedSymbol`, `searchQuery`, `minVolume`, `premiumMode`, `quoteFilter` |
| Hooks | `usePrices()`, `useOpportunities()`, `useExchangeRate()`, `useCommonMarkets()`, `usePremiumMatrix()` |

**역할**: 마켓 개요, 프리미엄 매트릭스 시각화

**서브 컴포넌트**:
- **PremiumMatrix**: 거래소 간 프리미엄 표 (매수/매도 방향)
- **StablecoinPrices**: USDT/USD, USDC/USD 가격 표시

**기능**:
- 환율 배너 (Upbit USDT/KRW, 김치 프리미엄)
- 프리미엄 매트릭스 그리드
- 마켓 검색 및 필터링
- 최근 기회 섹션

---

### 4. Opportunities.tsx

**경로**: `apps/desktop/src/components/Opportunities.tsx`
**타입**: Data Table Component

| 속성 | 값 |
|------|------|
| Props | 없음 |
| State | `executing`, `message`, `priceChanges`, `minVolume`, `searchQuery`, `premiumMode`, `pathOnly`, `tick` |
| Tauri Commands | `execute_opportunity(id, amount)` |
| Hooks | `useOpportunities()`, `usePrices()`, `useWalletStatus()`, `useExchangeRate()` |

**역할**: 차익거래 기회 테이블, 실행 기능

**테이블 컬럼**:
| 컬럼 | 설명 |
|------|------|
| Asset | 자산 심볼 |
| Route | 거래 경로 (source → target + 호가 통화) |
| Transfer Path | W/D 배지 (출금/입금 가능 여부) |
| Buy/Sell Price | 매수/매도 가격 |
| Optimal Size | 최적 거래 수량 |
| Spread | 스프레드 % (Kimchi/USDlike 모드) |
| Age | 데이터 나이 (색상 코딩) |
| Execute | 실행 버튼 |

**서브 컴포넌트**:
- **ExchangeLink**: 거래소 트레이딩 URL 링크 생성

---

### 5. Markets.tsx

**경로**: `apps/desktop/src/components/Markets.tsx`
**타입**: Data Table Component

| 속성 | 값 |
|------|------|
| Props | 없음 |
| State | `searchTerm`, `filterMode`, `expandedRows`, `selectedExchanges`, `showExchangeFilter` |
| Hooks | `useCommonMarkets()`, `useWalletStatus()` |

**역할**: 거래 가능 마켓 및 네트워크 상태 표시

**기능**:
- 자산별 거래소 커버리지
- 네트워크별 입출금 상태
- 확장 가능한 행 (네트워크 상세)
- 거래소 필터

**서브 컴포넌트**:
- **StatusBadge**: 상태 표시 배지 (normal/partial/suspended/unknown)

---

### 6. Wallets.tsx

**경로**: `apps/desktop/src/components/Wallets.tsx`
**타입**: Data Display Component

| 속성 | 값 |
|------|------|
| Props | 없음 |
| State | `selectedExchange`, `searchQuery`, `showOnlyWithBalance` |
| Tauri Commands | `get_wallet_info(exchange)`, `get_all_wallets()` |
| Hooks | `useWalletInfo(exchange?)` |

**역할**: 계정 잔액 및 입출금 상태 표시

**테이블 컬럼**:
| 컬럼 | 설명 |
|------|------|
| Asset | 자산명 |
| Available | 가용 잔액 |
| Locked | 주문 잠금 잔액 |
| Total | 총 잔액 |
| Deposit | 입금 상태 |
| Withdraw | 출금 상태 |
| Status | 전체 상태 배지 |

---

### 7. Settings.tsx

**경로**: `apps/desktop/src/components/Settings.tsx`
**타입**: Form Component

| 속성 | 값 |
|------|------|
| Props | 없음 |
| State | `localConfig`, `saved`, `credentialsSaved`, `mappingSaved`, `activeExchange`, `editingCredentials`, `newMapping`, `showAddMapping` |
| Tauri Commands | `update_config()`, `get_credentials()`, `save_credentials()`, `get_symbol_mappings()`, `upsert_symbol_mapping()`, `remove_symbol_mapping()`, `save_symbol_mappings()` |
| Hooks | `useConfig()`, `useCredentials()`, `useSymbolMappings()`, `useCommonMarkets()` |

**역할**: 봇 설정, API 자격증명, 심볼 매핑 관리

**설정 섹션**:

1. **Execution Settings**
   - Mode: Alert Only / Manual Approval / Auto Execute
   - Min Premium (bps)
   - Max Slippage (bps)
   - Dry Run 토글

2. **API Credentials** (거래소별)
   - Binance: API key + Secret
   - Coinbase: API Key ID + PEM
   - Upbit: Access + Secret
   - Bithumb: API + Secret
   - Bybit: API + Secret

3. **Symbol Mappings**
   - 거래소, 심볼, Canonical Name, Exclude, Notes

---

## Custom Hooks

**경로**: `apps/desktop/src/hooks/useTauri.ts`

### 가격 & 마켓 데이터

| Hook | 반환값 | 이벤트 | 특징 |
|------|--------|--------|------|
| `usePrices()` | `PriceData[]` | `price`, `prices` | 10 FPS 배치 처리, 전역 캐시 |
| `useOpportunities()` | `{ opportunities, executeOpportunity }` | `new_opportunity` | 전역 캐시, 1초 age 업데이트 |
| `useExchangeRate()` | `ExchangeRate | null` | `exchange_rate` | 5분마다 업데이트 |
| `useCommonMarkets()` | `CommonMarkets | null` | `common_markets` | 모듈 레벨 캐시 |
| `usePremiumMatrix()` | `Map<string, PremiumMatrixData>` | `premium_matrix` | 10 FPS 배치 처리 |

### 상태 & 제어

| Hook | 반환값 | Commands |
|------|--------|----------|
| `useStats()` | `BotStats` | `get_stats()` |
| `useBotControl()` | `{ start, stop }` | `start_bot()`, `stop_bot()` |
| `useConfig()` | `{ config, updateConfig }` | `get_config()`, `update_config()` |

### 지갑 & 자격증명

| Hook | 반환값 | Commands |
|------|--------|----------|
| `useWalletStatus()` | `ExchangeWalletStatus[]` | `get_wallet_status()` |
| `useWalletInfo(exchange?)` | `{ wallets, loading, error, fetchWallets }` | `get_wallet_info()`, `get_all_wallets()` |
| `useCredentials()` | `{ credentials, saveCredentials, loading }` | `get_credentials()`, `save_credentials()` |
| `useSymbolMappings()` | `{ mappings, upsertMapping, removeMapping, saveMappings }` | `get_symbol_mappings()`, 기타 |

---

## Tauri IPC Commands

### 데이터 조회

| Command | 반환값 | 설명 |
|---------|--------|------|
| `get_prices()` | `Vec<PriceData>` | 모든 현재 가격 |
| `get_opportunities()` | `Vec<OpportunityData>` | 모든 기회 |
| `get_stats()` | `BotStats` | 봇 통계 |
| `get_config()` | `ExecutionConfig` | 실행 설정 |
| `get_exchange_rate()` | `Option<ExchangeRateData>` | 현재 환율 |
| `get_common_markets()` | `Option<CommonMarketsData>` | 공통 마켓 |
| `get_wallet_status()` | `Option<WalletStatusData>` | 지갑 상태 |
| `get_credentials()` | `Credentials` (masked) | API 자격증명 |
| `get_wallet_info(exchange)` | `Result<ExchangeWalletInfo>` | 단일 지갑 정보 |
| `get_all_wallets()` | `Vec<ExchangeWalletInfo>` | 모든 지갑 정보 |
| `get_symbol_mappings()` | `SymbolMappings` | 심볼 매핑 |

### 제어 & 설정

| Command | 반환값 | 설명 |
|---------|--------|------|
| `start_bot()` | `bool` | 봇 시작 |
| `stop_bot()` | `bool` | 봇 정지 |
| `update_config(config)` | `bool` | 설정 업데이트 |
| `execute_opportunity(id, amount)` | `Result<String>` | 기회 실행 |
| `save_credentials(creds)` | `Result<bool>` | 자격증명 저장 |
| `upsert_symbol_mapping(mapping)` | `Result<bool>` | 심볼 매핑 추가/수정 |
| `remove_symbol_mapping(exchange, symbol)` | `Result<bool>` | 심볼 매핑 삭제 |

---

## 스타일링

**프레임워크**: Tailwind CSS (Dark Theme)

**커스텀 색상**:
- `dark-900`: 가장 어두운 배경
- `dark-800`: 메인 컨테이너
- `dark-750`: 약간 밝은
- `dark-700`: 테두리/구분선
- `primary-*`: 브랜드 색상
- `success-*`: 녹색 (성공)
- `danger-*`: 빨간색 (위험)
- `yellow-*`: 경고/프리미엄

---

## 아키텍처 패턴

1. **10 FPS Batching**: 가격, 기회, 프리미엄 매트릭스 → 100ms 플러시 간격으로 리렌더 최소화
2. **Global Caches**: CommonMarkets, opportunities, wallet status → 모듈 레벨 캐시로 마운트 간 유지
3. **Dual-Mode**: Tauri IPC (데스크톱) + WebSocket (브라우저 폴백)
4. **Event-Driven**: Tauri 이벤트 리스너로 서버 푸시 알림 수신
5. **Lazy Loading**: 지갑 정보는 사용자 요청 시에만 조회

---

## 파일 위치

| 컴포넌트 | 경로 |
|----------|------|
| Components | `apps/desktop/src/components/` |
| Hooks | `apps/desktop/src/hooks/useTauri.ts` |
| Types | `apps/desktop/src/types.ts` |
| App Entry | `apps/desktop/src/App.tsx` |
| Tauri Commands | `apps/desktop/src-tauri/src/commands.rs` |
| Tauri State | `apps/desktop/src-tauri/src/state.rs` |
| Credentials | `apps/desktop/src-tauri/src/credentials.rs` |
| Exchange Client | `apps/desktop/src-tauri/src/exchange_client.rs` |

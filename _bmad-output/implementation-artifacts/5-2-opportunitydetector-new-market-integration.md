# Story 5.2: OpportunityDetector 새 마켓 통합

Status: done

## Story

As a **트레이더**,
I want **새로 구독된 마켓에 대해 차익거래 기회가 탐지**,
So that **새 상장 코인에서도 수익 기회를 포착할 수 있다**.

## Acceptance Criteria

1. **AC1:** Given 새 마켓의 가격 데이터가 수신되고 있을 때, When 가격 차이가 임계값을 초과하면, Then OpportunityDetector가 해당 마켓의 차익거래 기회를 탐지해야 한다
2. **AC2:** Given 차익거래 기회가 탐지되었을 때, When 브로드캐스트 로직이 실행되면, Then 기회가 WebSocket 클라이언트에 브로드캐스트되어야 한다
3. **AC3:** Given 동적으로 구독된 마켓일 때, When detect_all() 또는 detect(pair_id)가 호출되면, Then 기존 이벤트 드리븐 탐지 로직과 동일하게 동작해야 한다
4. **AC4:** Given 새 마켓이 다수 거래소에서 구독 완료되었을 때, When 가격 업데이트가 발생하면, Then PremiumMatrix에 올바른 거래소간 프리미엄이 계산되어야 한다
5. **AC5:** Given 새 마켓 기회 탐지가 완료되었을 때, When 탐지 결과를 로깅하면, Then 기존 로깅 패턴과 일관되게 기회 정보가 출력되어야 한다

## Tasks / Subtasks

- [x] Task 1: 동적 마켓 기회 탐지 검증 (AC: #1, #3)
  - [x] Subtask 1.1: 동적으로 등록된 마켓(DOGE, XRP 등)에서 detect(pair_id)가 기회를 반환하는지 테스트
  - [x] Subtask 1.2: detect_all()이 동적 마켓을 포함하여 모든 마켓 기회를 반환하는지 테스트
  - [x] Subtask 1.3: min_premium_bps 임계값 초과 시에만 기회가 탐지되는지 검증

- [x] Task 2: WebSocket 브로드캐스트 통합 (AC: #2)
  - [x] Subtask 2.1: `apps/server/src/state.rs`에서 새 마켓 기회가 클라이언트에 브로드캐스트되는지 확인
  - [x] Subtask 2.2: ArbitrageOpportunity 직렬화에 동적 마켓 정보(symbol, pair_id)가 포함되는지 검증
  - [x] Subtask 2.3: 브로드캐스트 메시지 포맷이 기존 패턴과 동일한지 확인

- [x] Task 3: 다중 거래소 프리미엄 계산 검증 (AC: #4)
  - [x] Subtask 3.1: 새 마켓에서 2개 이상 거래소 가격이 있을 때 프리미엄 계산 테스트
  - [x] Subtask 3.2: KRW/USD 환율 적용 시 김치 프리미엄이 올바르게 계산되는지 검증
  - [x] Subtask 3.3: 서로 다른 QuoteCurrency(USD, USDT, KRW) 간 변환이 정확한지 테스트

- [x] Task 4: 기회 로깅 검증 (AC: #5)
  - [x] Subtask 4.1: 기회 탐지 시 기존 로깅 패턴 확인 (tracing 매크로 사용)
  - [x] Subtask 4.2: 동적 마켓 기회에 대한 로그 메시지에 symbol 정보 포함 확인
  - [x] Subtask 4.3: 로그 레벨(INFO/DEBUG)이 기존 패턴과 일관되는지 검증

- [x] Task 5: 통합 테스트 작성 (AC: #1~5)
  - [x] Subtask 5.1: 동적 구독 → 가격 업데이트 → 기회 탐지 → 브로드캐스트 전체 흐름 테스트
  - [x] Subtask 5.2: 에지 케이스 테스트 (단일 거래소, 가격 없음, 스테일 가격 등)

## Dev Notes

### 핵심 구현 포인트

**이미 구현된 기능들 (Story 5.1 완료):**
- `OpportunityDetector.register_symbol(symbol)` → pair_id 반환
- `OpportunityDetector.get_or_register_pair_id(symbol)` → 등록 또는 기존 반환
- `OpportunityDetector.update_price_with_bid_ask(...)` → PremiumMatrix 자동 생성
- `OpportunityDetector.get_matrix(pair_id)` → 매트릭스 조회
- `PriceAggregator.update(tick)` → 가격 저장
- 구독 타이밍 검증 (SubscriptionEvent.elapsed_ms)

**Story 5.2 핵심 작업:**
1. 동적 마켓이 `detect()` / `detect_all()`에서 기존 로직과 동일하게 처리되는지 검증
2. 서버의 기회 브로드캐스트 로직이 동적 마켓 기회도 전송하는지 확인
3. 통합 테스트로 전체 흐름 검증

### 기존 코드 패턴 분석

**OpportunityDetector.detect(pair_id) 흐름:**
```rust
// crates/engine/src/detector.rs:239-372
pub fn detect(&self, pair_id: u32) -> Vec<ArbitrageOpportunity> {
    // 1. matrices에서 pair_id로 PremiumMatrix 조회
    let Some(matrix) = self.matrices.get(&pair_id) else {
        return Vec::new();
    };

    // 2. all_premiums_multi_denomination(rates) 호출
    // 3. 각 (buy_ex, sell_ex, ...) 조합에 대해 ArbitrageOpportunity 생성
    // 4. usdlike_premium.bps로 정렬 후 반환
}
```

**ArbitrageOpportunity 생성 패턴:**
```rust
let asset = asset_for_pair_id_dashmap(pair_id, &self.symbol_registry);
// symbol_registry에서 심볼 조회 → Asset 생성
// 동적 마켓도 register_symbol() 호출 시 자동 등록됨
```

**서버 브로드캐스트 흐름 (apps/server):**
```rust
// apps/server/src/state.rs 또는 main.rs
// 1. detect_all_with_rates() 호출 → Vec<ArbitrageOpportunity>
// 2. 각 기회를 JSON 직렬화
// 3. WebSocket 클라이언트에 브로드캐스트
```

### 기존 테스트 패턴

**detector.rs 테스트 구조:**
```rust
#[test]
fn test_dynamic_market_premium_matrix_auto_creation() {
    let pair_id = detector.register_symbol("SHIB");
    detector.update_price_with_bid_ask(Exchange::Binance, pair_id, ...);
    assert!(detector.has_matrix(pair_id));
}
```

**Story 5.1에서 추가된 테스트:**
- `test_dynamic_market_premium_matrix_auto_creation`
- `test_dynamic_market_get_matrix_retrieval`
- `test_dynamic_market_registered_pair_ids_includes_new_markets`
- `test_dynamic_market_pair_id_to_symbol`

### 핵심 파일 및 역할

| 파일 | 역할 | 수정 필요 |
|------|------|----------|
| `crates/engine/src/detector.rs` | OpportunityDetector - 기회 탐지 | 테스트 추가 |
| `apps/server/src/state.rs` | SharedState - 브로드캐스트 로직 | 검증 필요 |
| `apps/server/src/main.rs` | 서버 초기화, 탐지 루프 | 검증 필요 |
| `crates/core/src/opportunity.rs` | ArbitrageOpportunity 구조체 | 검증 필요 |

### 주요 함수/메서드

**OpportunityDetector (검증 대상):**
```rust
pub fn detect(&self, pair_id: u32) -> Vec<ArbitrageOpportunity>
pub fn detect_all(&self) -> Vec<ArbitrageOpportunity>
pub fn detect_all_with_rates(...) -> Vec<ArbitrageOpportunity>
pub fn detect_all_with_all_rates(...) -> Vec<ArbitrageOpportunity>
```

**asset_for_pair_id_dashmap (중요):**
```rust
fn asset_for_pair_id_dashmap(pair_id: u32, symbol_registry: &DashMap<u32, String>) -> Asset {
    if let Some(symbol) = symbol_registry.get(&pair_id) {
        return Asset::from_symbol(&symbol);
    }
    // 레거시 pair_id 폴백 (1=BTC, 2=ETH, 3=SOL)
}
```

### Project Structure Notes

**테스트 위치:**
- 단위 테스트: `crates/engine/src/detector.rs` (기존 #[cfg(test)] 모듈 확장)
- 통합 테스트: `apps/server/tests/` (필요시 신규 작성)

**확인 필요 파일:**
- `apps/server/src/state.rs`: 브로드캐스트 로직
- `apps/server/src/main.rs`: 탐지 주기 및 호출 패턴

### 이전 Story 학습 사항

**Story 5.1 완료 내용 (직접 활용):**
- FeedHandler가 동적 마켓 파싱 지원 (테스트 완료)
- PriceAggregator가 동적 pair_id 저장/조회 지원 (테스트 완료)
- OpportunityDetector.register_symbol() 및 get_or_register_pair_id() 동작 확인
- update_price_with_bid_ask() 호출 시 PremiumMatrix 자동 생성 확인
- 구독 타이밍 측정 및 NFR3 (10초) 검증 로직 추가

**Epic 1-4 구현 패턴 (참조):**
- mpsc 채널 기반 구독 통신 (SubscriptionChange)
- tracing 매크로 로깅 (`tracing::info!`, `tracing::debug!`)
- DashMap lock-free 상태 관리

### Git Intelligence

**참조할 커밋:**
- `510d8d4`: feat: extend subscription flow and WTS UI
- `17e9c8c`: feat(server): integrate dynamic subscription system with WebSocket feeds
- 서버의 detect_all() 호출 패턴 및 브로드캐스트 로직 확인 가능

### References

- [Source: crates/engine/src/detector.rs#OpportunityDetector.detect]
- [Source: crates/engine/src/detector.rs#asset_for_pair_id_dashmap]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.2]
- [Source: _bmad-output/implementation-artifacts/5-1-new-market-price-data-verification.md]

### Technical Requirements

**Rust 버전:** 1.75+
**관련 크레이트:**
- `arbitrage-engine`: OpportunityDetector, PremiumMatrix
- `arbitrage-core`: ArbitrageOpportunity, Asset, Exchange
- `dashmap`: Lock-free concurrent HashMap
- `tracing`: 로깅 인프라
- `serde`: JSON 직렬화 (브로드캐스트)

**테스트 요구사항:**
- 단위 테스트: `cargo test -p arbitrage-engine`
- 통합 테스트: `cargo test -p arbitrage-server` (필요시)
- 동적 마켓 기회 탐지 전체 흐름 검증

### Anti-Pattern Prevention

**❌ 피해야 할 것:**
- 새로운 detect_dynamic() 같은 별도 메서드 생성 → 기존 detect() 재사용
- ArbitrageOpportunity 구조체 수정 → 기존 구조체로 충분
- 브로드캐스트 로직 변경 → 기존 로직이 동적 마켓도 처리

**✅ 해야 할 것:**
- 기존 detect(pair_id) 로직이 동적 마켓에서도 동작하는지 테스트로 검증
- asset_for_pair_id_dashmap 함수가 symbol_registry에서 심볼 조회하는지 확인
- 서버의 기존 브로드캐스트 코드가 수정 없이 동적 마켓 기회도 전송하는지 확인

### 구현 전략

1. **검증 중심 접근**: 새 코드 작성보다 기존 코드가 동적 마켓을 처리하는지 테스트
2. **테스트 우선**: 동적 마켓 시나리오에 대한 테스트 케이스 추가
3. **통합 확인**: 서버의 detect → broadcast 흐름이 동적 마켓에서도 동작하는지 검증
4. **로그 검증**: 기존 로깅 패턴이 동적 마켓 기회에도 적용되는지 확인

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5

### Debug Log References

- All 60 tests passing in arbitrage-engine
- `cargo test -p arbitrage-engine`
- `cargo test -p arbitrage-server`

### Completion Notes List

- **Task 1**: 동적 마켓 기회 탐지 기능이 기존 코드에서 정상 동작함을 테스트로 검증
  - `test_dynamic_market_detect_returns_opportunities` - 동적 마켓에서 detect(pair_id)가 기회 반환
  - `test_detect_all_includes_dynamic_markets` - detect_all()이 동적 마켓 포함
  - `test_dynamic_market_respects_min_premium_threshold` - 임계값 검증

- **Task 2**: WebSocket 브로드캐스트 통합 검증
  - `test_dynamic_market_opportunity_contains_symbol_info` - ArbitrageOpportunity JSON 직렬화 검증
  - `test_dynamic_market_asset_conversion` - Asset 심볼 변환 검증
  - 기존 서버 코드(state.rs, main.rs)가 동적 마켓 기회도 브로드캐스트함을 확인

- **Task 3**: 다중 거래소 프리미엄 계산 검증
  - `test_dynamic_market_multi_exchange_premium_calculation` - 3개 거래소 프리미엄
  - `test_dynamic_market_krw_premium_with_exchange_rates` - KRW 환율 적용
  - `test_dynamic_market_multi_quote_currency_handling` - USD/USDT/USDC 변환

- **Task 4**: 기회 로깅 검증
  - `test_dynamic_market_logging_consistency` - 기존 로깅 경로와 동일
  - `test_dynamic_market_opportunity_has_loggable_info` - 로깅 필드 설정 확인

- **Task 5**: 통합 테스트 작성
  - `test_full_flow_dynamic_subscription_to_opportunity_detection` - 전체 흐름
  - `test_edge_case_single_exchange` - 단일 거래소
  - `test_edge_case_no_prices` - 가격 없음
  - `test_edge_case_zero_depth` - 제로 깊이
  - `test_edge_case_same_price` - 동일 가격
  - `test_multiple_dynamic_markets_concurrent_detection` - 10개 마켓 병렬 처리
- **Review Fixes**:
  - min_premium_bps 임계값 필터 적용 및 임계값 테스트 보강
  - ArbitrageOpportunity에 pair_id 추가, WS 브로드캐스트 payload 반영
  - 브로드캐스트/로깅/환율 프리미엄 테스트 강화
  - exchange_rate 테스트 경쟁 조건 정리, wallet_status 클라이언트 no_proxy 적용

### File List

- `crates/engine/src/detector.rs` - 14개 새 테스트 추가 (line 770-1612)
- `crates/engine/Cargo.toml` - serde_json/tracing-subscriber dev-dependency 추가
- `crates/core/src/opportunity.rs` - pair_id 필드/빌더 추가
- `apps/server/src/ws_server.rs` - WsOpportunityData에 pair_id 추가 + 브로드캐스트 테스트
- `apps/server/src/exchange_rate.rs` - 테스트 동기화(경쟁 조건 완화)
- `apps/server/src/wallet_status.rs` - no_proxy 적용
- `apps/desktop/src/types.ts` - ArbitrageOpportunity에 pair_id 추가

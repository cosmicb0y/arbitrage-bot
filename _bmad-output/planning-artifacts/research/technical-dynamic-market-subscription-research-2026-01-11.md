---
stepsCompleted: ['discovery', 'exchange-api-research', 'architecture-design']
inputDocuments: ['docs/ARCHITECTURE.md']
workflowType: 'research'
lastStep: 3
research_type: 'technical'
research_topic: 'dynamic-market-subscription'
research_goals: '기존 거래소에 새로 상장된 코인을 런타임 중 자동 구독 (공통 마켓 한정)'
user_name: 'Hyowon'
date: '2026-01-11'
web_research_enabled: true
source_verification: true
---

# 기술 리서치 보고서: 런타임 동적 마켓 구독

**Date:** 2026-01-11
**Author:** Hyowon
**Research Type:** Technical Research

---

## Executive Summary

본 리서치는 arbitrage-bot 프로젝트에서 **기존 거래소에 새로 상장된 코인을 런타임 중 자동으로 WebSocket 구독에 추가**하는 기능의 기술적 구현 가능성과 전략을 조사합니다.

### 핵심 발견사항

1. **모든 6개 거래소가 런타임 동적 구독을 지원**
2. **현재 아키텍처는 5분마다 마켓을 발견하지만, WebSocket 재구독 로직이 없음**
3. **구현 복잡도: 중간** - 기존 아키텍처 확장으로 해결 가능

### 권장 전략

**Hybrid Approach**: 마켓 디스커버리 루프와 WebSocket 클라이언트 간 채널 기반 통신으로 동적 구독 지원

---

## 목차

1. [현재 아키텍처 분석](#1-현재-아키텍처-분석)
2. [거래소별 동적 구독 API](#2-거래소별-동적-구독-api)
3. [Rust/Tokio 동적 구독 패턴](#3-rusttokio-동적-구독-패턴)
4. [제안 아키텍처](#4-제안-아키텍처)
5. [구현 전략](#5-구현-전략)
6. [리스크 및 고려사항](#6-리스크-및-고려사항)
7. [결론 및 권장사항](#7-결론-및-권장사항)

---

## 1. 현재 아키텍처 분석

### 1.1 마켓 디스커버리 흐름

```
시작시:
  spawn_live_feeds()
    ├─ MarketDiscovery::fetch_all() [1회]
    ├─ find_markets_on_n_exchanges_with_mappings()
    ├─ state.register_common_markets()
    └─ 거래소별 WsClient 스폰 (고정된 심볼 목록)

백그라운드 (5분마다):
  run_market_discovery()
    ├─ MarketDiscovery::fetch_all()
    ├─ state.update_common_markets()
    └─ broadcast_common_markets() → 클라이언트만 갱신
       ⚠️ WebSocket 구독은 갱신되지 않음!
```

**핵심 파일 위치:**
- 마켓 디스커버리: [crates/feeds/src/discovery.rs](crates/feeds/src/discovery.rs) (565-630줄)
- 백그라운드 태스크: [apps/server/src/main.rs](apps/server/src/main.rs) (994-1038줄)
- 피드 스폰: [apps/server/src/main.rs](apps/server/src/main.rs) (646-990줄)

### 1.2 현재 제한사항

| 항목 | 현재 상태 | 문제점 |
|------|----------|--------|
| 마켓 발견 | ✅ 5분마다 갱신 | - |
| 공통 마켓 브로드캐스트 | ✅ 클라이언트에 전송 | - |
| WebSocket 재구독 | ❌ 없음 | 서버 재시작 필요 |
| 새 심볼 가격 수신 | ❌ 불가 | 차익거래 기회 손실 |

### 1.3 WebSocket 클라이언트 구조

현재 `WsClient`는 연결 시 고정된 심볼 목록으로 구독하며, 런타임 중 구독 변경 메커니즘이 없습니다:

```rust
// crates/feeds/src/websocket.rs - 현재 구조
pub struct WsClient {
    url: String,
    subscribe_messages: Vec<String>,  // 시작 시 고정
    // ... 동적 구독 채널 없음
}
```

---

## 2. 거래소별 동적 구독 API

### 2.1 API 지원 현황 요약

| 거래소 | 동적 Subscribe | 동적 Unsubscribe | Rate Limit | 최대 스트림 |
|--------|---------------|-----------------|------------|------------|
| **Binance** | ✅ | ✅ | 5 msg/sec | 1024/연결 |
| **Coinbase** | ✅ | ✅ | - | - |
| **Bybit** | ✅ | ✅ | - | - |
| **Upbit** | ✅ | ✅ | - | - |
| **Bithumb** | ⚠️ 제한적 | ⚠️ 제한적 | - | - |
| **Gate.io** | ✅ | ✅ | - | - |

### 2.2 Binance

**출처:** [Binance WebSocket Streams](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md)

```json
// 런타임 구독 추가
{
  "method": "SUBSCRIBE",
  "params": ["btcusdt@trade", "ethusdt@depth20@100ms"],
  "id": 1
}

// 런타임 구독 해제
{
  "method": "UNSUBSCRIBE",
  "params": ["btcusdt@trade"],
  "id": 2
}
```

**제한사항:**
- 5 incoming messages per second (초과 시 연결 해제)
- 최대 1024 스트림/연결
- 2025-07-08부터 WebSocket 업그레이드 예정

### 2.3 Coinbase

**출처:** [Coinbase Advanced Trade WebSocket](https://docs.cdp.coinbase.com/coinbase-app/advanced-trade-apis/guides/websocket)

```json
// 구독 추가
{
  "type": "subscribe",
  "product_ids": ["BTC-USD", "ETH-USD"],
  "channel": "ticker"
}

// 구독 해제
{
  "type": "unsubscribe",
  "product_ids": ["BTC-USD"],
  "channel": "ticker"
}
```

**특징:**
- JWT 토큰 인증 필요
- 실시간 시장 데이터 및 사용자별 주문 정보 지원

### 2.4 Bybit

**출처:** [Bybit WebSocket Connect](https://bybit-exchange.github.io/docs/v5/ws/connect)

```json
// 구독 추가
{
  "op": "subscribe",
  "args": ["orderbook.50.BTCUSDT", "tickers.ETHUSDT"]
}

// 구독 해제
{
  "op": "unsubscribe",
  "args": ["orderbook.50.BTCUSDT"]
}
```

**Endpoint:**
- Spot: `wss://stream.bybit.com/v5/public/spot`
- Linear: `wss://stream.bybit.com/v5/public/linear`

**주의사항:**
- 500 연결/5분 제한
- heartbeat 필수 (ping-pong)

### 2.5 Upbit

**출처:** [Upbit WebSocket Guide](https://global-docs.upbit.com/reference/websocket-guide)

```json
// 구독 (ticket + type + codes)
[
  {"ticket": "unique-ticket-id"},
  {"type": "ticker", "codes": ["KRW-BTC", "KRW-ETH"]},
  {"format": "SIMPLE"}
]
```

**특징:**
- TLS 1.2+ 필수 (1.3 권장)
- 스냅샷 + 실시간 스트림 구분
- 새 구독 메시지 전송 시 이전 구독 대체 (누적 아님)

### 2.6 Bithumb

**특징:**
- 공식 WebSocket 문서 제한적
- 구독 변경 시 재연결 권장

### 2.7 Gate.io

**출처:** [Gate.io Spot WebSocket API v4](https://www.gate.com/docs/developers/apiv4/ws/en/)

```json
// 구독 추가
{
  "time": 123456,
  "channel": "spot.trades",
  "event": "subscribe",
  "payload": ["BTC_USDT", "ETH_USDT"]
}

// 구독 해제
{
  "time": 123456,
  "channel": "spot.trades",
  "event": "unsubscribe",
  "payload": ["BTC_USDT"]
}
```

**특징:**
- 이전 구독 유지 (명시적 unsubscribe 전까지)
- v4 권장 (`wss://api.gateio.ws/ws/v4/`)

---

## 3. Rust/Tokio 동적 구독 패턴

### 3.1 핵심 라이브러리

**출처:** [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite), [tokio-websockets](https://crates.io/crates/tokio-websockets)

```rust
// tokio-tungstenite의 Stream/Sink 트레잇
// 런타임 중 언제든 메시지 전송 가능
let (write, read) = ws_stream.split();

// 구독 변경을 위한 채널 패턴
let (sub_tx, sub_rx) = mpsc::channel::<SubscriptionChange>(100);
```

### 3.2 권장 아키텍처 패턴

```rust
// 구독 변경 타입
enum SubscriptionChange {
    Subscribe(Vec<String>),
    Unsubscribe(Vec<String>),
}

// WsClient에 구독 변경 채널 추가
struct WsClient {
    url: String,
    subscribe_messages: Vec<String>,
    subscription_rx: mpsc::Receiver<SubscriptionChange>,  // 새로 추가
}

// 메인 루프에서 select!로 처리
loop {
    tokio::select! {
        // 기존 메시지 수신
        msg = read.next() => { /* 처리 */ }

        // 구독 변경 요청 수신
        Some(change) = subscription_rx.recv() => {
            match change {
                Subscribe(symbols) => {
                    let msg = build_subscribe_message(&symbols);
                    write.send(msg).await?;
                }
                Unsubscribe(symbols) => {
                    let msg = build_unsubscribe_message(&symbols);
                    write.send(msg).await?;
                }
            }
        }
    }
}
```

---

## 4. 제안 아키텍처

### 4.1 전체 흐름

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Market Discovery Loop (5분)                          │
│                                                                              │
│   1. fetch_all() → 모든 거래소 마켓 조회                                    │
│   2. find_common_markets() → 2개+ 거래소 공통 마켓 필터                     │
│   3. diff_with_current() → 새로 추가된 마켓 감지 ← NEW!                     │
│   4. subscription_tx.send(new_markets) → 각 거래소 WsClient에 전송          │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Subscription Manager (NEW)                          │
│                                                                              │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│   │ Binance Sub │  │ Coinbase Sub│  │ Bybit Sub   │  │ Upbit Sub   │ ...   │
│   │   Channel   │  │   Channel   │  │   Channel   │  │   Channel   │       │
│   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘       │
│          │                │                │                │               │
└──────────┼────────────────┼────────────────┼────────────────┼───────────────┘
           │                │                │                │
           ▼                ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        WebSocket Clients (수정)                              │
│                                                                              │
│   WsClient::run() 루프에서:                                                  │
│   - 기존 메시지 수신 처리                                                   │
│   - subscription_rx로 새 심볼 수신 시 subscribe 메시지 전송                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 핵심 컴포넌트

#### 4.2.1 SubscriptionManager (신규)

```rust
pub struct SubscriptionManager {
    /// 거래소별 구독 변경 전송 채널
    senders: HashMap<Exchange, mpsc::Sender<SubscriptionChange>>,

    /// 현재 구독 중인 심볼 (diff 계산용)
    current_subscriptions: DashMap<Exchange, HashSet<String>>,
}

impl SubscriptionManager {
    /// 새로운 공통 마켓 발견 시 호출
    pub async fn update_subscriptions(&self, new_common: &CommonMarkets) {
        for (exchange, markets) in &new_common.by_quote {
            let current = self.current_subscriptions.get(exchange);
            let new_symbols: Vec<_> = markets
                .iter()
                .filter(|m| !current.contains(&m.symbol))
                .collect();

            if !new_symbols.is_empty() {
                if let Some(tx) = self.senders.get(exchange) {
                    tx.send(SubscriptionChange::Subscribe(new_symbols)).await?;
                }
            }
        }
    }
}
```

#### 4.2.2 WsClient 수정

```rust
pub struct WsClient {
    // 기존 필드...

    /// 구독 변경 수신 채널 (NEW)
    subscription_rx: Option<mpsc::Receiver<SubscriptionChange>>,

    /// 거래소 타입 (구독 메시지 포맷 결정)
    exchange: Exchange,
}

impl WsClient {
    pub async fn run(mut self, tx: Sender<WsMessage>) -> Result<()> {
        loop {
            tokio::select! {
                // 기존: WebSocket 메시지 수신
                msg = self.ws.next() => { /* ... */ }

                // 신규: 구독 변경 요청 처리
                Some(change) = async {
                    match &mut self.subscription_rx {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    self.handle_subscription_change(change).await?;
                }
            }
        }
    }

    async fn handle_subscription_change(&mut self, change: SubscriptionChange) -> Result<()> {
        let msg = match self.exchange {
            Exchange::Binance => self.build_binance_subscribe(&change),
            Exchange::Coinbase => self.build_coinbase_subscribe(&change),
            Exchange::Bybit => self.build_bybit_subscribe(&change),
            // ...
        };
        self.ws.send(msg).await
    }
}
```

### 4.3 거래소별 구독 메시지 빌더

```rust
impl WsClient {
    fn build_binance_subscribe(&self, symbols: &[String]) -> String {
        let streams: Vec<String> = symbols
            .iter()
            .flat_map(|s| vec![
                format!("{}@trade", s.to_lowercase()),
                format!("{}@depth20@100ms", s.to_lowercase()),
            ])
            .collect();

        json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": self.next_request_id()
        }).to_string()
    }

    fn build_bybit_subscribe(&self, symbols: &[String]) -> String {
        let args: Vec<String> = symbols
            .iter()
            .flat_map(|s| vec![
                format!("orderbook.50.{}", s),
                format!("tickers.{}", s),
            ])
            .collect();

        json!({
            "op": "subscribe",
            "args": args
        }).to_string()
    }

    // Coinbase, Upbit, Gate.io 등...
}
```

---

## 5. 구현 전략

### 5.1 단계별 구현 계획

#### Phase 1: 기반 구조 (예상 복잡도: 낮음)

1. **SubscriptionChange enum 정의**
   - 파일: `crates/feeds/src/subscription.rs` (신규)

2. **SubscriptionManager 구현**
   - 파일: `crates/feeds/src/subscription.rs`
   - 기능: 거래소별 채널 관리, diff 계산

3. **WsClient에 subscription_rx 추가**
   - 파일: `crates/feeds/src/websocket.rs`
   - 기존 구조 확장

#### Phase 2: 거래소별 구독 로직 (예상 복잡도: 중간)

4. **거래소별 subscribe 메시지 빌더 구현**
   - Binance, Coinbase, Bybit: 간단
   - Upbit: 전체 구독 목록 재전송 필요
   - Bithumb: 재연결 방식 고려

5. **Rate Limit 핸들링**
   - Binance: 5 msg/sec 제한 → 배치 처리
   - 다른 거래소: 필요시 추가

#### Phase 3: 통합 및 테스트 (예상 복잡도: 낮음)

6. **run_market_discovery 수정**
   - SubscriptionManager 호출 추가
   - 새 마켓 발견 시 로깅

7. **텔레그램 알림**
   - 새 마켓 상장 알림 (선택적)

### 5.2 파일 변경 요약

| 파일 | 변경 유형 | 설명 |
|------|----------|------|
| `crates/feeds/src/subscription.rs` | 신규 | SubscriptionManager, SubscriptionChange |
| `crates/feeds/src/websocket.rs` | 수정 | subscription_rx 추가, select! 확장 |
| `crates/feeds/src/lib.rs` | 수정 | subscription 모듈 export |
| `apps/server/src/main.rs` | 수정 | SubscriptionManager 초기화, 채널 연결 |

---

## 6. 리스크 및 고려사항

### 6.1 기술적 리스크

| 리스크 | 영향도 | 대응 방안 |
|--------|-------|----------|
| **Binance Rate Limit** | 중간 | 배치 처리 (1초에 최대 5개 심볼) |
| **Upbit 구독 덮어쓰기** | 중간 | 전체 구독 목록 재전송 로직 |
| **Bithumb 제한적 API** | 낮음 | 재연결 기반 갱신 또는 제외 |
| **오더북 초기화** | 중간 | REST API로 초기 스냅샷 fetch |

### 6.2 운영 고려사항

1. **새 마켓 발견 로깅**
   ```
   [INFO] New market discovered: SOL/USDT on [Binance, Coinbase, Bybit]
   [INFO] Subscribing to 3 exchanges for SOL/USDT
   ```

2. **구독 실패 처리**
   - 재시도 로직 (지수 백오프)
   - 알림 (텔레그램)

3. **메모리 관리**
   - 심볼 수 증가에 따른 DashMap 크기 모니터링

### 6.3 엣지 케이스

1. **상장 폐지 (Delisting)**
   - 현재 요구사항 범위 외
   - 향후 unsubscribe 로직 추가 가능

2. **동시 다발적 상장**
   - Binance Rate Limit 고려하여 큐잉 처리

3. **거래소 연결 끊김 중 새 마켓**
   - 재연결 시 현재 공통 마켓 전체 재구독

---

## 7. 결론 및 권장사항

### 7.1 핵심 결론

1. **기술적으로 완전히 실현 가능**
   - 모든 주요 거래소가 런타임 동적 구독 API 지원
   - Rust/Tokio의 채널 + select! 패턴으로 깔끔하게 구현 가능

2. **기존 아키텍처 자연스럽게 확장**
   - 마켓 디스커버리 루프 활용 (이미 5분마다 실행 중)
   - WsClient 구조에 채널만 추가

3. **구현 복잡도: 중간**
   - 핵심 변경: 4개 파일
   - 예상 코드량: 300-500줄

### 7.2 권장 구현 순서

1. **Binance 먼저** - 가장 명확한 API, 테스트 용이
2. **Coinbase, Bybit** - 유사한 패턴
3. **Gate.io** - 약간 다른 포맷이지만 지원 양호
4. **Upbit** - 전체 재구독 방식
5. **Bithumb** - 필요시 재연결 방식

### 7.3 추가 권장사항

- **텔레그램 알림**: 새 마켓 상장 시 알림 (차익거래 기회 선점)
- **대시보드 표시**: 새로 추가된 마켓 하이라이트
- **로깅 강화**: 구독 변경 이벤트 추적

---

## Sources

- [Binance WebSocket Streams Documentation](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md)
- [Coinbase Advanced Trade WebSocket API](https://docs.cdp.coinbase.com/coinbase-app/advanced-trade-apis/guides/websocket)
- [Bybit WebSocket Connect](https://bybit-exchange.github.io/docs/v5/ws/connect)
- [Upbit WebSocket Guide](https://global-docs.upbit.com/reference/websocket-guide)
- [Gate.io Spot WebSocket API v4](https://www.gate.com/docs/developers/apiv4/ws/en/)
- [tokio-tungstenite GitHub](https://github.com/snapview/tokio-tungstenite)
- [Rust WebSocket Development 2025](https://www.videosdk.live/developer-hub/websocket/rust-websocket)

---

*문서 생성일: 2026-01-11 | 리서치 타입: Technical*

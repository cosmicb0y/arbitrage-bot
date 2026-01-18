# Arbitrage Bot - API 계약

## 개요

이 문서는 CLI 서버와 클라이언트 간의 WebSocket 메시지 포맷 및 데이터 구조를 정의합니다.

---

## WebSocket 연결

| 항목 | 값 |
|------|-----|
| **엔드포인트** | `ws://localhost:9001/ws` |
| **프로토콜** | WebSocket (RFC 6455) |
| **포맷** | JSON |
| **인증** | 없음 (로컬 전용) |

### 연결 수명주기

1. **연결 시 초기 동기화**
   - 모든 캐시된 가격 (batch)
   - 현재 통계
   - 환율 데이터
   - 탐지된 기회 (batch)
   - 공통 마켓 목록
   - 지갑 상태

2. **연속 업데이트**
   - 가격 업데이트 (실시간)
   - 기회 탐지 (즉시)
   - 프리미엄 매트릭스 (가격 업데이트마다)
   - 통계 (10초마다)
   - 환율 (5분마다)
   - 지갑 상태 (5분마다)

---

## 메시지 타입

### 메시지 포맷

모든 메시지는 `type` 필드로 구분됩니다:

```json
{
  "type": "MessageType",
  // ... type-specific fields
}
```

### 메시지 타입 목록

| 타입 | 설명 | 방향 |
|------|------|------|
| `Price` | 단일 가격 업데이트 | Server → Client |
| `Prices` | 가격 배치 (초기 동기화) | Server → Client |
| `Stats` | 봇 통계 | Server → Client |
| `Opportunity` | 단일 기회 탐지 | Server → Client |
| `Opportunities` | 기회 배치 (초기 동기화) | Server → Client |
| `ExchangeRate` | 환율 데이터 | Server → Client |
| `CommonMarkets` | 공통 마켓 목록 | Server → Client |
| `WalletStatus` | 지갑 입출금 상태 | Server → Client |
| `PremiumMatrix` | 프리미엄 매트릭스 | Server → Client |

---

## 데이터 구조 상세

### Price

단일 가격 업데이트.

```json
{
  "type": "Price",
  "exchange": "Binance",
  "symbol": "BTC",
  "pair_id": 12345678,
  "price": 50000.0,
  "bid": 49999.5,
  "ask": 50000.5,
  "volume_24h": 1234567.89,
  "timestamp": 1736582400000,
  "quote": "USDT",
  "price_usd": 50000.0,
  "bid_usd": 49999.5,
  "ask_usd": 50000.5
}
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `exchange` | string | 거래소 이름 |
| `symbol` | string | 심볼 (예: "BTC", "ETH") |
| `pair_id` | u32 | 심볼 해시 ID |
| `price` | f64 | 중간 가격 (원본 호가 통화) |
| `bid` | f64 | 최우선 매수 호가 |
| `ask` | f64 | 최우선 매도 호가 |
| `volume_24h` | f64 | 24시간 거래량 |
| `timestamp` | u64 | 밀리초 타임스탬프 |
| `quote` | string? | 호가 통화 ("USDT", "USDC", "KRW", "USD") |
| `price_usd` | f64? | USD 변환 가격 |
| `bid_usd` | f64? | USD 변환 매수 호가 |
| `ask_usd` | f64? | USD 변환 매도 호가 |

### Prices

가격 배치 (초기 동기화용).

```json
{
  "type": "Prices",
  "prices": [
    { /* Price 객체 */ },
    { /* Price 객체 */ }
  ]
}
```

---

### Stats

봇 실행 통계.

```json
{
  "type": "Stats",
  "uptime_secs": 3600,
  "price_updates": 125000,
  "opportunities_detected": 450,
  "trades_executed": 0,
  "is_running": true
}
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `uptime_secs` | u64 | 가동 시간 (초) |
| `price_updates` | u64 | 총 가격 업데이트 수 |
| `opportunities_detected` | u64 | 총 탐지된 기회 수 |
| `trades_executed` | u64 | 총 실행된 거래 수 |
| `is_running` | bool | 봇 실행 상태 |

---

### Opportunity

차익거래 기회.

```json
{
  "type": "Opportunity",
  "id": 1736582400001,
  "symbol": "BTC",
  "source_exchange": "Binance",
  "target_exchange": "Upbit",
  "source_quote": "USDT",
  "target_quote": "KRW",
  "premium_bps": 150,
  "usdlike_premium": {
    "bps": 145,
    "quote": "USDT"
  },
  "kimchi_premium_bps": 155,
  "source_price": 50000.0,
  "target_price": 50750.0,
  "net_profit_bps": 120,
  "confidence_score": 85,
  "timestamp": 1736582400000,
  "common_networks": ["Bitcoin", "Ethereum"],
  "has_transfer_path": true,
  "wallet_status_known": true,
  "source_depth": 5.5,
  "target_depth": 3.2,
  "optimal_size": 2.5,
  "optimal_profit": 375.0,
  "optimal_size_reason": "ok",
  "source_raw_price": 50000.0,
  "target_raw_price": 67500000.0
}
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `id` | u64 | 고유 ID |
| `symbol` | string | 자산 심볼 |
| `source_exchange` | string | 매수 거래소 |
| `target_exchange` | string | 매도 거래소 |
| `source_quote` | string | 매수측 호가 통화 |
| `target_quote` | string | 매도측 호가 통화 |
| `premium_bps` | i32 | Raw 프리미엄 (basis points) |
| `usdlike_premium` | object? | USDlike 프리미엄 |
| `kimchi_premium_bps` | i32 | Kimchi 프리미엄 (bps) |
| `source_price` | f64 | 매수 가격 (USD 환산) |
| `target_price` | f64 | 매도 가격 (USD 환산) |
| `net_profit_bps` | i32 | 순이익 (수수료 후, bps) |
| `confidence_score` | u8 | 신뢰도 점수 (0-100) |
| `timestamp` | u64 | 탐지 시간 (ms) |
| `common_networks` | string[] | 공통 전송 네트워크 |
| `has_transfer_path` | bool | 실행 가능 여부 |
| `wallet_status_known` | bool | 지갑 상태 확인됨 |
| `source_depth` | f64 | 매수측 오더북 깊이 |
| `target_depth` | f64 | 매도측 오더북 깊이 |
| `optimal_size` | f64 | 최적 거래량 |
| `optimal_profit` | f64 | 예상 수익 (USD) |
| `optimal_size_reason` | string? | 사이즈 계산 결과 |
| `source_raw_price` | f64 | 원본 매수 가격 |
| `target_raw_price` | f64 | 원본 매도 가격 |

#### UsdlikePremium 객체

```json
{
  "bps": 145,
  "quote": "USDT"
}
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `bps` | i32 | 프리미엄 (basis points) |
| `quote` | string | 기준 스테이블코인 ("USDT", "USDC", "BUSD") |

#### optimal_size_reason 값

| 값 | 설명 |
|-----|------|
| `"ok"` | 정상 계산됨 |
| `"no_orderbook"` | 오더북 데이터 없음 |
| `"not_profitable"` | 수익성 없음 |
| `"no_conversion_rate"` | 환율 데이터 없음 |

---

### ExchangeRate

환율 데이터.

```json
{
  "type": "ExchangeRate",
  "usd_krw": 1350.0,
  "upbit_usdt_krw": 1352.5,
  "bithumb_usdt_krw": 1351.8,
  "api_rate": 1349.5,
  "usdt_usd": 1.0,
  "usdc_usd": 1.0,
  "timestamp": 1736582400000
}
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `usd_krw` | f64 | USD/KRW 환율 (Upbit 기준) |
| `upbit_usdt_krw` | f64 | Upbit USDT/KRW 시장가 |
| `bithumb_usdt_krw` | f64 | Bithumb USDT/KRW 시장가 |
| `api_rate` | f64? | 외부 API 환율 (하나은행) |
| `usdt_usd` | f64 | USDT/USD 가격 |
| `usdc_usd` | f64 | USDC/USD 가격 |
| `timestamp` | u64 | 업데이트 시간 (ms) |

---

### CommonMarkets

거래소별 공통 마켓 목록.

```json
{
  "type": "CommonMarkets",
  "bases": ["BTC", "ETH", "SOL", "XRP"],
  "exchanges": ["Binance", "Coinbase", "Upbit", "Bithumb"],
  "pairs": {
    "BTC": {
      "Binance": ["USDT", "USDC"],
      "Coinbase": ["USD", "USDT"],
      "Upbit": ["KRW"],
      "Bithumb": ["KRW"]
    },
    "ETH": {
      "Binance": ["USDT", "USDC"],
      "Coinbase": ["USD", "USDT"],
      "Upbit": ["KRW"],
      "Bithumb": ["KRW"]
    }
  }
}
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `bases` | string[] | 기초 자산 목록 |
| `exchanges` | string[] | 거래소 목록 |
| `pairs` | object | 자산 → 거래소 → 호가통화 매핑 |

---

### WalletStatus

거래소별 지갑 입출금 상태.

```json
{
  "type": "WalletStatus",
  "exchanges": [
    {
      "exchange": "Binance",
      "assets": [
        {
          "asset": "BTC",
          "name": "Bitcoin",
          "can_deposit": true,
          "can_withdraw": true,
          "networks": [
            {
              "network": "BTC",
              "name": "Bitcoin",
              "deposit_enabled": true,
              "withdraw_enabled": true,
              "min_withdraw": 0.0001,
              "withdraw_fee": 0.00005,
              "confirms_required": 2
            }
          ]
        }
      ],
      "last_updated": 1736582400000
    }
  ]
}
```

#### ExchangeWalletStatus

| 필드 | 타입 | 설명 |
|------|------|------|
| `exchange` | string | 거래소 이름 |
| `assets` | AssetWalletStatus[] | 자산별 상태 |
| `last_updated` | u64 | 마지막 업데이트 (ms) |

#### AssetWalletStatus

| 필드 | 타입 | 설명 |
|------|------|------|
| `asset` | string | 자산 심볼 |
| `name` | string | 자산 이름 |
| `can_deposit` | bool | 입금 가능 여부 (any network) |
| `can_withdraw` | bool | 출금 가능 여부 (any network) |
| `networks` | NetworkStatus[] | 네트워크별 상세 |

#### NetworkStatus

| 필드 | 타입 | 설명 |
|------|------|------|
| `network` | string | 네트워크 ID |
| `name` | string | 네트워크 이름 |
| `deposit_enabled` | bool | 입금 가능 |
| `withdraw_enabled` | bool | 출금 가능 |
| `min_withdraw` | f64 | 최소 출금량 |
| `withdraw_fee` | f64 | 출금 수수료 |
| `confirms_required` | u32 | 필요 확인 수 |

---

### PremiumMatrix

심볼별 거래소 쌍 프리미엄 매트릭스.

```json
{
  "type": "PremiumMatrix",
  "symbol": "BTC",
  "pair_id": 12345678,
  "entries": [
    {
      "buy_exchange": "Binance",
      "sell_exchange": "Upbit",
      "buy_quote": "USDT",
      "sell_quote": "KRW",
      "tether_premium_bps": 145,
      "kimchi_premium_bps": 155
    },
    {
      "buy_exchange": "Coinbase",
      "sell_exchange": "Upbit",
      "buy_quote": "USD",
      "sell_quote": "KRW",
      "tether_premium_bps": 150,
      "kimchi_premium_bps": 155
    }
  ],
  "timestamp": 1736582400000
}
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `symbol` | string | 자산 심볼 |
| `pair_id` | u32 | 페어 ID |
| `entries` | PremiumEntry[] | 거래소 쌍별 프리미엄 |
| `timestamp` | u64 | 계산 시간 (ms) |

#### PremiumEntry

| 필드 | 타입 | 설명 |
|------|------|------|
| `buy_exchange` | string | 매수 거래소 |
| `sell_exchange` | string | 매도 거래소 |
| `buy_quote` | string | 매수측 호가 통화 |
| `sell_quote` | string | 매도측 호가 통화 |
| `tether_premium_bps` | i32 | USDlike 프리미엄 (bps) |
| `kimchi_premium_bps` | i32 | Kimchi 프리미엄 (bps) |

---

## HTTP 엔드포인트

| 엔드포인트 | 메서드 | 설명 |
|------------|--------|------|
| `/ws` | GET | WebSocket 업그레이드 |
| `/health` | GET | 헬스체크 (응답: "OK") |

---

## 오류 처리

### 연결 오류

- **재연결**: 2초 후 자동 재시도
- **Circuit Breaker**: 10회 연속 실패 시 5분 차단

### 메시지 파싱 오류

- 잘못된 JSON: 로그 후 무시
- 알 수 없는 타입: 로그 후 무시

---

## 클라이언트 구현 가이드

### TypeScript 예제

```typescript
interface WsMessage {
  type: string;
  [key: string]: any;
}

const ws = new WebSocket('ws://localhost:9001/ws');

ws.onmessage = (event) => {
  const msg: WsMessage = JSON.parse(event.data);

  switch (msg.type) {
    case 'Price':
      handlePrice(msg as PriceData);
      break;
    case 'Prices':
      handlePrices(msg.prices as PriceData[]);
      break;
    case 'Opportunity':
      handleOpportunity(msg as OpportunityData);
      break;
    case 'Stats':
      handleStats(msg as StatsData);
      break;
    case 'ExchangeRate':
      handleExchangeRate(msg as ExchangeRateData);
      break;
    case 'CommonMarkets':
      handleCommonMarkets(msg as CommonMarketsData);
      break;
    case 'WalletStatus':
      handleWalletStatus(msg as WalletStatusData);
      break;
    case 'PremiumMatrix':
      handlePremiumMatrix(msg as PremiumMatrixData);
      break;
  }
};
```

### Rust 예제

```rust
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum WsServerMessage {
    Price(PriceData),
    Prices { prices: Vec<PriceData> },
    Stats(StatsData),
    Opportunity(OpportunityData),
    Opportunities { opportunities: Vec<OpportunityData> },
    ExchangeRate(ExchangeRateData),
    CommonMarkets(CommonMarketsData),
    WalletStatus(WalletStatusData),
    PremiumMatrix(PremiumMatrixData),
}
```

---

## 버전 호환성

| 버전 | 변경 사항 |
|------|----------|
| 0.1.0 | 초기 API 정의 |

---

*문서 생성일: 2026-01-11 | 버전: 0.1.0*

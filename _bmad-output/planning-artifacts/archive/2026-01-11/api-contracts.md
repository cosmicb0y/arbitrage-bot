# API Contracts - WebSocket 메시지 명세

**생성일**: 2026-01-11

이 문서는 arbitrage-bot 서버의 WebSocket API 명세를 정의합니다.

---

## 개요

Arbitrage Bot 서버는 REST API가 아닌 **WebSocket 브로드캐스트 서버**로 동작합니다.

- **프로토콜**: WebSocket (RFC 6455)
- **기본 포트**: 9001
- **메시지 형식**: JSON (Serde tagged enum)
- **통신 방향**: 서버 → 클라이언트 (단방향 브로드캐스트)

### 엔드포인트

| 경로 | 메서드 | 설명 |
|------|--------|------|
| `/ws` | GET (Upgrade) | WebSocket 연결 |
| `/health` | GET | 헬스 체크 ("OK" 반환) |

---

## WebSocket 메시지 타입

### 메시지 구조

모든 메시지는 다음 구조를 따릅니다:

```json
{
  "type": "<message_type>",
  "data": { /* 메시지별 데이터 */ }
}
```

---

### 1. Price (가격 업데이트)

**Type**: `"price"`
**트리거**: 거래소 WebSocket에서 가격 틱 수신 시

```json
{
  "type": "price",
  "data": {
    "exchange": "Binance",
    "symbol": "BTC",
    "pair_id": 1,
    "price": 50000.0,
    "bid": 49900.0,
    "ask": 50100.0,
    "volume_24h": 1234.5,
    "timestamp": 1673000000000,
    "quote": "USDT",
    "price_usd": 50000.0,
    "bid_usd": 49900.0,
    "ask_usd": 50100.0
  }
}
```

**필드 설명**:
- `exchange`: 거래소 이름
- `symbol`: 자산 심볼 (예: "BTC", "ETH")
- `pair_id`: 내부 페어 ID (u32)
- `price`, `bid`, `ask`: 원본 호가 통화 가격
- `quote`: 호가 통화 ("USDT", "USD", "KRW" 등)
- `*_usd`: USD 변환 가격 (선택적)

---

### 2. Prices (가격 배치)

**Type**: `"prices"`
**트리거**: 새 클라이언트 연결 시 초기 동기화

```json
{
  "type": "prices",
  "data": [
    { /* WsPriceData */ },
    { /* WsPriceData */ }
  ]
}
```

---

### 3. Stats (통계)

**Type**: `"stats"`
**트리거**: 10초마다 주기적 전송

```json
{
  "type": "stats",
  "data": {
    "uptime_secs": 3600,
    "price_updates": 45000,
    "opportunities_detected": 150,
    "trades_executed": 5,
    "is_running": true
  }
}
```

---

### 4. Opportunity (차익거래 기회)

**Type**: `"opportunity"` (단일) / `"opportunities"` (배치)
**트리거**: 새로운 차익거래 기회 탐지 시

```json
{
  "type": "opportunity",
  "data": {
    "id": 12345,
    "symbol": "BTC",
    "source_exchange": "Binance",
    "target_exchange": "Upbit",
    "source_quote": "USDT",
    "target_quote": "KRW",
    "premium_bps": 250,
    "usdlike_premium": {
      "bps": 240,
      "quote": "USDT"
    },
    "kimchi_premium_bps": 280,
    "source_price": 50000.0,
    "target_price": 51500.0,
    "net_profit_bps": 200,
    "confidence_score": 95,
    "timestamp": 1673000000000,
    "common_networks": ["Bitcoin", "Ethereum"],
    "has_transfer_path": true,
    "wallet_status_known": true,
    "source_depth": 5.2,
    "target_depth": 8.1,
    "optimal_size": 2.5,
    "optimal_profit": 15000.0,
    "optimal_size_reason": "ok",
    "source_raw_price": 50000.0,
    "target_raw_price": 65000000.0
  }
}
```

**필드 설명**:
- `premium_bps`: 직접 가격 비교 (basis points)
- `usdlike_premium`: 동일 스테이블코인 기준 프리미엄
- `kimchi_premium_bps`: USD/KRW 환율 기준 프리미엄
- `common_networks`: 사용 가능한 전송 경로
- `has_transfer_path`: 전송 가능 여부
- `optimal_size`: 호가창 기반 최적 거래 수량
- `optimal_size_reason`: "ok" | "no_orderbook" | "not_profitable" | "no_conversion_rate"

---

### 5. Exchange Rate (환율)

**Type**: `"exchange_rate"`
**트리거**: 5분마다 주기적 업데이트

```json
{
  "type": "exchange_rate",
  "data": {
    "usd_krw": 1350.25,
    "upbit_usdt_krw": 1352.15,
    "upbit_usdc_krw": 1351.80,
    "bithumb_usdt_krw": 1351.50,
    "bithumb_usdc_krw": 1351.10,
    "api_rate": 1350.25,
    "usdt_usd": 0.9995,
    "usdc_usd": 0.9998,
    "timestamp": 1673000000000
  }
}
```

**필드 설명**:
- `usd_krw`: 하나은행 API 기준 USD/KRW 환율
- `upbit_usdt_krw`: Upbit USDT/KRW 환율
- `usdt_usd`: 스테이블코인 디페깅 (USDT/USD)
- `api_rate`: API 기반 환율 (김치 프리미엄 계산용)

---

### 6. Common Markets (공통 마켓)

**Type**: `"common_markets"`
**트리거**: 5분마다 마켓 탐색

```json
{
  "type": "common_markets",
  "data": {
    "common_bases": ["BTC", "ETH", "SOL"],
    "markets": {
      "BTC": [
        {"base": "BTC", "symbol": "BTCUSDT", "exchange": "Binance"},
        {"base": "BTC", "symbol": "BTC-USD", "exchange": "Coinbase"},
        {"base": "BTC", "symbol": "KRW-BTC", "exchange": "Upbit"}
      ]
    },
    "exchanges": ["Binance", "Coinbase", "Upbit", "Bithumb", "Bybit", "GateIO"],
    "timestamp": 1673000000000
  }
}
```

---

### 7. Wallet Status (지갑 상태)

**Type**: `"wallet_status"`
**트리거**: 주기적 지갑 상태 조회

```json
{
  "type": "wallet_status",
  "data": {
    "exchanges": [
      {
        "exchange": "Binance",
        "wallet_status": [
          {
            "asset": "BTC",
            "name": "Bitcoin",
            "networks": [
              {
                "network": "BTC",
                "name": "Bitcoin",
                "deposit_enabled": true,
                "withdraw_enabled": true,
                "min_withdraw": 0.0001,
                "withdraw_fee": 0.00005,
                "confirms_required": 1
              }
            ],
            "can_deposit": true,
            "can_withdraw": true
          }
        ],
        "last_updated": 1673000000000
      }
    ],
    "timestamp": 1673000000000
  }
}
```

---

### 8. Premium Matrix (프리미엄 매트릭스)

**Type**: `"premium_matrix"`
**트리거**: 가격 업데이트 시 (이벤트 기반)

```json
{
  "type": "premium_matrix",
  "data": {
    "symbol": "BTC",
    "pair_id": 1,
    "entries": [
      {
        "buy_exchange": "Binance",
        "sell_exchange": "Upbit",
        "buy_quote": "USDT",
        "sell_quote": "KRW",
        "tether_premium_bps": 240,
        "kimchi_premium_bps": 280
      }
    ],
    "timestamp": 1673000000000
  }
}
```

---

## 초기 연결 시 수신 데이터

클라이언트가 `/ws`에 연결하면 다음 순서로 데이터를 수신합니다:

1. `prices` - 모든 현재 가격
2. `stats` - 현재 통계
3. `exchange_rate` - 현재 환율 (로드된 경우)
4. `opportunities` - 현재 기회들
5. `common_markets` - 사용 가능한 마켓
6. `wallet_status` - 지갑 상태 (로드된 경우)

---

## CLI 명령어

```
arbitrage-bot [OPTIONS]

OPTIONS:
  -c, --config <CONFIG>           설정 파일 경로 [기본: config.json]
  -p, --min-premium <MIN_PREMIUM> 최소 프리미엄 (bps) [기본: 30]
  -m, --mode <MODE>               실행 모드: auto, manual, alert [기본: alert]
  -l, --log-level <LOG_LEVEL>     로그 레벨: trace, debug, info, warn, error [기본: info]
      --dry-run <DRY_RUN>         드라이런 모드 [기본: true]
      --live <LIVE>               라이브 피드 사용 [기본: false]
      --ws-port <WS_PORT>         WebSocket 포트 [기본: 9001]
      --telegram <TELEGRAM>       텔레그램 알림 활성화 [기본: false]
      --db-path <DB_PATH>         SQLite 경로 [기본: data/alerts.db]
```

---

## 백그라운드 태스크

| 태스크 | 주기 | 설명 |
|--------|------|------|
| Stats Reporter | 10초 | 통계 브로드캐스트 |
| Exchange Rate Updater | 5분 | 환율 업데이트 |
| Market Discovery | 5분 | 공통 마켓 탐색 |
| Wallet Status Updater | 주기적 | 지갑 상태 업데이트 |
| Stale Price Cleanup | 10초 | 오래된 가격 데이터 정리 |

---

## 파일 위치

| 컴포넌트 | 파일 |
|----------|------|
| WebSocket 서버 | `apps/server/src/ws_server.rs` |
| 메인 애플리케이션 | `apps/server/src/main.rs` |
| 피드 핸들러 | `apps/server/src/feeds/handler.rs` |
| 환율 업데이터 | `apps/server/src/exchange_rate.rs` |
| 지갑 상태 | `apps/server/src/wallet_status.rs` |
| 상태 관리 | `apps/server/src/state.rs` |

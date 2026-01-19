import { describe, expect, it } from 'vitest';
import {
  UPBIT_DEFAULT_MARKETS,
  type Market,
  type MarketCode,
  type OrderbookEntry,
  type OrderbookData,
  type UpbitOrderbookResponse,
  type UpbitOrderbookUnit,
} from '../types';

describe('Market Types', () => {
  describe('Market interface', () => {
    it('Market 타입은 code, base, quote 필수 속성을 가진다', () => {
      const market: Market = {
        code: 'KRW-BTC',
        base: 'BTC',
        quote: 'KRW',
      };

      expect(market.code).toBe('KRW-BTC');
      expect(market.base).toBe('BTC');
      expect(market.quote).toBe('KRW');
    });

    it('Market 타입은 displayName 옵션 속성을 가질 수 있다', () => {
      const market: Market = {
        code: 'KRW-BTC',
        base: 'BTC',
        quote: 'KRW',
        displayName: '비트코인',
      };

      expect(market.displayName).toBe('비트코인');
    });
  });

  describe('MarketCode type', () => {
    it('MarketCode는 string 타입이다', () => {
      const code: MarketCode = 'KRW-BTC';
      expect(typeof code).toBe('string');
    });
  });

  describe('UPBIT_DEFAULT_MARKETS', () => {
    it('UPBIT_DEFAULT_MARKETS는 readonly 배열이다', () => {
      expect(Array.isArray(UPBIT_DEFAULT_MARKETS)).toBe(true);
    });

    it('UPBIT_DEFAULT_MARKETS는 최소 5개 이상의 마켓을 포함한다', () => {
      expect(UPBIT_DEFAULT_MARKETS.length).toBeGreaterThanOrEqual(5);
    });

    it('모든 마켓은 code, base, quote를 가진다', () => {
      UPBIT_DEFAULT_MARKETS.forEach((market) => {
        expect(market.code).toBeDefined();
        expect(market.base).toBeDefined();
        expect(market.quote).toBeDefined();
        expect(typeof market.code).toBe('string');
        expect(typeof market.base).toBe('string');
        expect(typeof market.quote).toBe('string');
      });
    });

    it('모든 마켓의 code 형식은 {quote}-{base} 패턴이다', () => {
      UPBIT_DEFAULT_MARKETS.forEach((market) => {
        expect(market.code).toBe(`${market.quote}-${market.base}`);
      });
    });

    it('KRW-BTC 마켓이 포함되어 있다', () => {
      const btcMarket = UPBIT_DEFAULT_MARKETS.find((m) => m.code === 'KRW-BTC');
      expect(btcMarket).toBeDefined();
      expect(btcMarket?.base).toBe('BTC');
      expect(btcMarket?.quote).toBe('KRW');
      expect(btcMarket?.displayName).toBe('비트코인');
    });

    it('KRW-ETH 마켓이 포함되어 있다', () => {
      const ethMarket = UPBIT_DEFAULT_MARKETS.find((m) => m.code === 'KRW-ETH');
      expect(ethMarket).toBeDefined();
      expect(ethMarket?.base).toBe('ETH');
      expect(ethMarket?.quote).toBe('KRW');
      expect(ethMarket?.displayName).toBe('이더리움');
    });

    it('모든 마켓에 displayName이 정의되어 있다', () => {
      UPBIT_DEFAULT_MARKETS.forEach((market) => {
        expect(market.displayName).toBeDefined();
        expect(typeof market.displayName).toBe('string');
        expect(market.displayName!.length).toBeGreaterThan(0);
      });
    });
  });
});

describe('Orderbook Types', () => {
  describe('OrderbookEntry', () => {
    it('OrderbookEntry 타입은 price, size 필수 속성을 가진다', () => {
      const entry: OrderbookEntry = {
        price: 50100000,
        size: 0.5,
      };

      expect(entry.price).toBe(50100000);
      expect(entry.size).toBe(0.5);
    });
  });

  describe('OrderbookData', () => {
    it('OrderbookData 타입은 asks, bids, timestamp 속성을 가진다', () => {
      const orderbook: OrderbookData = {
        asks: [{ price: 50100000, size: 0.5 }],
        bids: [{ price: 50000000, size: 0.8 }],
        timestamp: 1704067200000,
      };

      expect(orderbook.asks).toHaveLength(1);
      expect(orderbook.bids).toHaveLength(1);
      expect(orderbook.timestamp).toBe(1704067200000);
    });

    it('OrderbookData의 timestamp는 null일 수 있다', () => {
      const orderbook: OrderbookData = {
        asks: [],
        bids: [],
        timestamp: null,
      };

      expect(orderbook.timestamp).toBeNull();
    });
  });

  describe('UpbitOrderbookUnit', () => {
    it('UpbitOrderbookUnit 타입은 ask_price, bid_price, ask_size, bid_size를 가진다', () => {
      const unit: UpbitOrderbookUnit = {
        ask_price: 50100000,
        bid_price: 50000000,
        ask_size: 0.5,
        bid_size: 0.8,
      };

      expect(unit.ask_price).toBe(50100000);
      expect(unit.bid_price).toBe(50000000);
      expect(unit.ask_size).toBe(0.5);
      expect(unit.bid_size).toBe(0.8);
    });
  });

  describe('UpbitOrderbookResponse', () => {
    it('UpbitOrderbookResponse 타입은 type, code, timestamp, total_ask_size, total_bid_size, orderbook_units를 가진다', () => {
      const response: UpbitOrderbookResponse = {
        type: 'orderbook',
        code: 'KRW-BTC',
        timestamp: 1704067200000,
        total_ask_size: 10.12345678,
        total_bid_size: 8.87654321,
        orderbook_units: [
          {
            ask_price: 50100000,
            bid_price: 50000000,
            ask_size: 0.5,
            bid_size: 0.8,
          },
        ],
      };

      expect(response.type).toBe('orderbook');
      expect(response.code).toBe('KRW-BTC');
      expect(response.timestamp).toBe(1704067200000);
      expect(response.total_ask_size).toBe(10.12345678);
      expect(response.total_bid_size).toBe(8.87654321);
      expect(response.orderbook_units).toHaveLength(1);
    });
  });
});

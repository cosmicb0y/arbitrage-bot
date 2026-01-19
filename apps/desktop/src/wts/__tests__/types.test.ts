import { describe, expect, it } from 'vitest';
import {
  UPBIT_DEFAULT_MARKETS,
  type Market,
  type MarketCode,
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

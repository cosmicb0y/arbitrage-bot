import { describe, expect, it } from 'vitest';
import {
  UPBIT_DEFAULT_MARKETS,
  UPBIT_ORDER_ERROR_MESSAGES,
  getOrderErrorMessage,
  isRateLimitError,
  isNetworkError,
  isDepositAvailable,
  isAddressGenerating,
  type Market,
  type MarketCode,
  type OrderbookEntry,
  type OrderbookData,
  type UpbitOrderbookResponse,
  type UpbitOrderbookUnit,
  type DepositAddressParams,
  type DepositAddressResponse,
  type DepositChanceParams,
  type DepositChanceResponse,
  type DepositNetwork,
  type GenerateAddressResponse,
  type GenerateAddressCreating,
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

describe('Error Message Types', () => {
  describe('UPBIT_ORDER_ERROR_MESSAGES', () => {
    it('rate_limit 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['rate_limit']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['rate_limit']).toBe(
        '주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.'
      );
    });

    it('too_many_requests 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['too_many_requests']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['too_many_requests']).toBe(
        '주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.'
      );
    });

    it('network_error 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['network_error']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['network_error']).toContain('네트워크');
    });

    it('timeout_error 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['timeout_error']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['timeout_error']).toContain('시간');
    });

    it('connection_error 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['connection_error']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['connection_error']).toContain('연결');
    });

    it('server_error 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['server_error']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['server_error']).toContain('서버');
    });
  });

  describe('getOrderErrorMessage', () => {
    it('알려진 에러 코드는 한국어 메시지를 반환한다', () => {
      expect(getOrderErrorMessage('rate_limit')).toBe(
        '주문 요청이 너무 빠릅니다. 잠시 후 다시 시도하세요.'
      );
      expect(getOrderErrorMessage('insufficient_funds_bid')).toContain('매수');
      expect(getOrderErrorMessage('under_min_total_bid')).toContain('5,000원');
    });

    it('알 수 없는 에러 코드는 fallback 메시지를 사용한다', () => {
      expect(getOrderErrorMessage('unknown_code', '커스텀 에러')).toBe('커스텀 에러');
    });

    it('알 수 없는 에러 코드 + 영어 fallback은 기본 메시지를 반환한다', () => {
      expect(getOrderErrorMessage('unknown_code', 'Some error')).toBe('알 수 없는 오류가 발생했습니다');
    });

    it('알 수 없는 에러 코드 + fallback 없으면 기본 메시지를 반환한다', () => {
      expect(getOrderErrorMessage('unknown_code')).toBe('알 수 없는 오류가 발생했습니다');
    });
  });

  describe('isRateLimitError', () => {
    it('rate_limit 에러 코드를 인식한다', () => {
      expect(isRateLimitError('rate_limit')).toBe(true);
    });

    it('too_many_requests 에러 코드를 인식한다', () => {
      expect(isRateLimitError('too_many_requests')).toBe(true);
    });

    it('다른 에러 코드는 false를 반환한다', () => {
      expect(isRateLimitError('network_error')).toBe(false);
      expect(isRateLimitError('insufficient_funds_bid')).toBe(false);
    });
  });

  describe('isNetworkError', () => {
    it('network_error 에러 코드를 인식한다', () => {
      expect(isNetworkError('network_error')).toBe(true);
    });

    it('timeout_error 에러 코드를 인식한다', () => {
      expect(isNetworkError('timeout_error')).toBe(true);
    });

    it('connection_error 에러 코드를 인식한다', () => {
      expect(isNetworkError('connection_error')).toBe(true);
    });

    it('다른 에러 코드는 false를 반환한다', () => {
      expect(isNetworkError('rate_limit')).toBe(false);
      expect(isNetworkError('insufficient_funds_bid')).toBe(false);
    });
  });

  describe('Deposit Error Messages (WTS-4.1)', () => {
    it('deposit_address_not_found 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['deposit_address_not_found']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['deposit_address_not_found']).toContain('입금 주소');
    });

    it('invalid_currency 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['invalid_currency']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['invalid_currency']).toContain('자산');
    });

    it('invalid_net_type 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['invalid_net_type']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['invalid_net_type']).toContain('네트워크');
    });

    it('deposit_paused 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['deposit_paused']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['deposit_paused']).toContain('중단');
    });

    it('deposit_suspended 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['deposit_suspended']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['deposit_suspended']).toContain('중단');
    });

    it('address_generation_failed 에러 코드에 대한 메시지가 정의되어 있다', () => {
      expect(UPBIT_ORDER_ERROR_MESSAGES['address_generation_failed']).toBeDefined();
      expect(UPBIT_ORDER_ERROR_MESSAGES['address_generation_failed']).toContain('생성');
    });
  });
});

describe('Deposit API Types (WTS-4.1)', () => {
  describe('DepositAddressParams', () => {
    it('DepositAddressParams 타입은 currency, net_type 필수 속성을 가진다', () => {
      const params: DepositAddressParams = {
        currency: 'BTC',
        net_type: 'BTC',
      };

      expect(params.currency).toBe('BTC');
      expect(params.net_type).toBe('BTC');
    });
  });

  describe('DepositAddressResponse', () => {
    it('DepositAddressResponse 타입은 currency, net_type, deposit_address, secondary_address 속성을 가진다', () => {
      const response: DepositAddressResponse = {
        currency: 'BTC',
        net_type: 'BTC',
        deposit_address: 'bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh',
        secondary_address: null,
      };

      expect(response.currency).toBe('BTC');
      expect(response.net_type).toBe('BTC');
      expect(response.deposit_address).toBe('bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh');
      expect(response.secondary_address).toBeNull();
    });

    it('deposit_address는 null일 수 있다 (생성 중)', () => {
      const response: DepositAddressResponse = {
        currency: 'BTC',
        net_type: 'BTC',
        deposit_address: null,
        secondary_address: null,
      };

      expect(response.deposit_address).toBeNull();
    });

    it('secondary_address는 XRP 태그 등에 사용된다', () => {
      const response: DepositAddressResponse = {
        currency: 'XRP',
        net_type: 'XRP',
        deposit_address: 'rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh',
        secondary_address: '123456789',
      };

      expect(response.secondary_address).toBe('123456789');
    });
  });

  describe('DepositChanceParams', () => {
    it('DepositChanceParams 타입은 currency, net_type 필수 속성을 가진다', () => {
      const params: DepositChanceParams = {
        currency: 'ETH',
        net_type: 'ETH',
      };

      expect(params.currency).toBe('ETH');
      expect(params.net_type).toBe('ETH');
    });
  });

  describe('DepositNetwork', () => {
    it('DepositNetwork 타입은 name, net_type, priority, deposit_state, confirm_count 속성을 가진다', () => {
      const network: DepositNetwork = {
        name: 'Bitcoin',
        net_type: 'BTC',
        priority: 1,
        deposit_state: 'normal',
        confirm_count: 3,
      };

      expect(network.name).toBe('Bitcoin');
      expect(network.net_type).toBe('BTC');
      expect(network.priority).toBe(1);
      expect(network.deposit_state).toBe('normal');
      expect(network.confirm_count).toBe(3);
    });
  });

  describe('DepositChanceResponse', () => {
    it('DepositChanceResponse 타입은 currency, net_type, network, deposit_state, minimum 속성을 가진다', () => {
      const response: DepositChanceResponse = {
        currency: 'BTC',
        net_type: 'BTC',
        network: {
          name: 'Bitcoin',
          net_type: 'BTC',
          priority: 1,
          deposit_state: 'normal',
          confirm_count: 3,
        },
        deposit_state: 'normal',
        minimum: '0.001',
      };

      expect(response.currency).toBe('BTC');
      expect(response.net_type).toBe('BTC');
      expect(response.network.name).toBe('Bitcoin');
      expect(response.deposit_state).toBe('normal');
      expect(response.minimum).toBe('0.001');
    });
  });

  describe('GenerateAddressResponse', () => {
    it('비동기 생성 중 응답', () => {
      const response: GenerateAddressCreating = {
        success: true,
        message: 'creating',
      };

      expect(response.success).toBe(true);
      expect(response.message).toBe('creating');
    });

    it('이미 존재하는 주소 응답', () => {
      const response: GenerateAddressResponse = {
        currency: 'BTC',
        net_type: 'BTC',
        deposit_address: 'bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh',
        secondary_address: null,
      };

      expect('currency' in response).toBe(true);
      if ('currency' in response) {
        expect(response.deposit_address).toBe('bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh');
      }
    });
  });

  describe('isDepositAvailable', () => {
    it('normal 상태는 입금 가능하다', () => {
      expect(isDepositAvailable('normal')).toBe(true);
    });

    it('paused 상태는 입금 불가능하다', () => {
      expect(isDepositAvailable('paused')).toBe(false);
    });

    it('suspended 상태는 입금 불가능하다', () => {
      expect(isDepositAvailable('suspended')).toBe(false);
    });
  });

  describe('isAddressGenerating', () => {
    it('Creating 응답을 인식한다', () => {
      const response: GenerateAddressCreating = {
        success: true,
        message: 'creating',
      };

      expect(isAddressGenerating(response)).toBe(true);
    });

    it('Existing 응답은 false를 반환한다', () => {
      const response: DepositAddressResponse = {
        currency: 'BTC',
        net_type: 'BTC',
        deposit_address: 'bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh',
        secondary_address: null,
      };

      expect(isAddressGenerating(response)).toBe(false);
    });
  });
});

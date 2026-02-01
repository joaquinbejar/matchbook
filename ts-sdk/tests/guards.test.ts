/**
 * Type guard tests.
 */

import { describe, it, expect } from 'vitest';
import {
  isSide,
  isOrderType,
  isTimeInForce,
  isOrderStatus,
  isSelfTradeBehavior,
  isNonEmptyString,
  isNumericString,
  isPositiveNumericString,
  isNonNegativeNumericString,
  isBookLevel,
  isMarket,
  isOrder,
  isTrade,
  isOrderBook,
  isBalance,
  assertMarket,
  assertOrder,
} from '../src/guards';

describe('Primitive type guards', () => {
  describe('isSide', () => {
    it('returns true for valid sides', () => {
      expect(isSide('bid')).toBe(true);
      expect(isSide('ask')).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isSide('buy')).toBe(false);
      expect(isSide('sell')).toBe(false);
      expect(isSide('')).toBe(false);
      expect(isSide(null)).toBe(false);
      expect(isSide(undefined)).toBe(false);
      expect(isSide(123)).toBe(false);
    });
  });

  describe('isOrderType', () => {
    it('returns true for valid order types', () => {
      expect(isOrderType('limit')).toBe(true);
      expect(isOrderType('postOnly')).toBe(true);
      expect(isOrderType('ioc')).toBe(true);
      expect(isOrderType('fok')).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isOrderType('market')).toBe(false);
      expect(isOrderType('')).toBe(false);
      expect(isOrderType(null)).toBe(false);
    });
  });

  describe('isTimeInForce', () => {
    it('returns true for valid time in force values', () => {
      expect(isTimeInForce('gtc')).toBe(true);
      expect(isTimeInForce('ioc')).toBe(true);
      expect(isTimeInForce('fok')).toBe(true);
      expect(isTimeInForce('postOnly')).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isTimeInForce('day')).toBe(false);
      expect(isTimeInForce('')).toBe(false);
    });
  });

  describe('isOrderStatus', () => {
    it('returns true for valid order statuses', () => {
      expect(isOrderStatus('open')).toBe(true);
      expect(isOrderStatus('partiallyFilled')).toBe(true);
      expect(isOrderStatus('filled')).toBe(true);
      expect(isOrderStatus('cancelled')).toBe(true);
      expect(isOrderStatus('expired')).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isOrderStatus('pending')).toBe(false);
      expect(isOrderStatus('')).toBe(false);
    });
  });

  describe('isSelfTradeBehavior', () => {
    it('returns true for valid self-trade behaviors', () => {
      expect(isSelfTradeBehavior('decrementTake')).toBe(true);
      expect(isSelfTradeBehavior('cancelProvide')).toBe(true);
      expect(isSelfTradeBehavior('abortTransaction')).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isSelfTradeBehavior('allow')).toBe(false);
      expect(isSelfTradeBehavior('')).toBe(false);
    });
  });
});

describe('String validators', () => {
  describe('isNonEmptyString', () => {
    it('returns true for non-empty strings', () => {
      expect(isNonEmptyString('hello')).toBe(true);
      expect(isNonEmptyString(' ')).toBe(true);
    });

    it('returns false for empty strings and non-strings', () => {
      expect(isNonEmptyString('')).toBe(false);
      expect(isNonEmptyString(null)).toBe(false);
      expect(isNonEmptyString(undefined)).toBe(false);
      expect(isNonEmptyString(123)).toBe(false);
    });
  });

  describe('isNumericString', () => {
    it('returns true for valid numeric strings', () => {
      expect(isNumericString('123')).toBe(true);
      expect(isNumericString('123.45')).toBe(true);
      expect(isNumericString('-123.45')).toBe(true);
      expect(isNumericString('0')).toBe(true);
      expect(isNumericString('0.0')).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isNumericString('abc')).toBe(false);
      expect(isNumericString('')).toBe(false);
      expect(isNumericString('12.34.56')).toBe(false);
      expect(isNumericString(null)).toBe(false);
      expect(isNumericString(123)).toBe(false);
    });
  });

  describe('isPositiveNumericString', () => {
    it('returns true for positive numeric strings', () => {
      expect(isPositiveNumericString('123')).toBe(true);
      expect(isPositiveNumericString('0.001')).toBe(true);
    });

    it('returns false for non-positive values', () => {
      expect(isPositiveNumericString('0')).toBe(false);
      expect(isPositiveNumericString('-1')).toBe(false);
      expect(isPositiveNumericString('abc')).toBe(false);
    });
  });

  describe('isNonNegativeNumericString', () => {
    it('returns true for non-negative numeric strings', () => {
      expect(isNonNegativeNumericString('123')).toBe(true);
      expect(isNonNegativeNumericString('0')).toBe(true);
      expect(isNonNegativeNumericString('0.0')).toBe(true);
    });

    it('returns false for negative values', () => {
      expect(isNonNegativeNumericString('-1')).toBe(false);
      expect(isNonNegativeNumericString('-0.001')).toBe(false);
    });
  });
});

describe('Entity type guards', () => {
  describe('isBookLevel', () => {
    it('returns true for valid book levels', () => {
      expect(isBookLevel({ price: '100.0', quantity: '50.0' })).toBe(true);
      expect(isBookLevel({ price: '100.0', quantity: '50.0', orders: 5 })).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isBookLevel({ price: '100.0' })).toBe(false);
      expect(isBookLevel({ quantity: '50.0' })).toBe(false);
      expect(isBookLevel(null)).toBe(false);
      expect(isBookLevel({})).toBe(false);
    });
  });

  describe('isMarket', () => {
    const validMarket = {
      address: '7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF',
      baseMint: 'So11111111111111111111111111111111111111112',
      quoteMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
      tickSize: '0.01',
      lotSize: '0.001',
      makerFee: -0.0002,
      takerFee: 0.0005,
    };

    it('returns true for valid markets', () => {
      expect(isMarket(validMarket)).toBe(true);
      expect(isMarket({ ...validMarket, baseSymbol: 'SOL', quoteSymbol: 'USDC' })).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isMarket({ ...validMarket, address: '' })).toBe(false);
      expect(isMarket({ ...validMarket, makerFee: 'invalid' })).toBe(false);
      expect(isMarket(null)).toBe(false);
      expect(isMarket({})).toBe(false);
    });
  });

  describe('isOrder', () => {
    const validOrder = {
      id: '340282366920938463463374607431768211455',
      market: '7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF',
      owner: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM',
      side: 'bid',
      price: '105.50',
      quantity: '100.0',
      filledQuantity: '25.0',
      orderType: 'limit',
      status: 'partiallyFilled',
      createdAt: '2026-01-30T12:00:00.000Z',
    };

    it('returns true for valid orders', () => {
      expect(isOrder(validOrder)).toBe(true);
      expect(isOrder({ ...validOrder, clientOrderId: 12345 })).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isOrder({ ...validOrder, side: 'buy' })).toBe(false);
      expect(isOrder({ ...validOrder, status: 'pending' })).toBe(false);
      expect(isOrder(null)).toBe(false);
      expect(isOrder({})).toBe(false);
    });
  });

  describe('isTrade', () => {
    const validTrade = {
      id: 'abc123',
      market: '7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF',
      price: '105.55',
      quantity: '5.0',
      side: 'bid',
      timestamp: '2026-01-30T12:00:01.234Z',
    };

    it('returns true for valid trades', () => {
      expect(isTrade(validTrade)).toBe(true);
      expect(isTrade({ ...validTrade, makerFee: '0.001' })).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isTrade({ ...validTrade, side: 'buy' })).toBe(false);
      expect(isTrade({ ...validTrade, price: '' })).toBe(false);
      expect(isTrade(null)).toBe(false);
    });
  });

  describe('isOrderBook', () => {
    const validOrderBook = {
      market: '7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF',
      bids: [{ price: '105.50', quantity: '100.0' }],
      asks: [{ price: '105.60', quantity: '75.0' }],
    };

    it('returns true for valid order books', () => {
      expect(isOrderBook(validOrderBook)).toBe(true);
      expect(isOrderBook({ ...validOrderBook, sequence: 12345 })).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isOrderBook({ ...validOrderBook, bids: 'invalid' })).toBe(false);
      expect(isOrderBook({ ...validOrderBook, bids: [{ price: '100' }] })).toBe(false);
      expect(isOrderBook(null)).toBe(false);
    });
  });

  describe('isBalance', () => {
    const validBalance = {
      market: '7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF',
      owner: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM',
      baseAvailable: '100.0',
      baseLocked: '25.0',
      quoteAvailable: '5000.0',
      quoteLocked: '1000.0',
    };

    it('returns true for valid balances', () => {
      expect(isBalance(validBalance)).toBe(true);
      expect(isBalance({ ...validBalance, baseUnsettled: '10.0' })).toBe(true);
    });

    it('returns false for invalid values', () => {
      expect(isBalance({ ...validBalance, baseAvailable: '' })).toBe(false);
      expect(isBalance(null)).toBe(false);
      expect(isBalance({})).toBe(false);
    });
  });
});

describe('Assert functions', () => {
  describe('assertMarket', () => {
    const validMarket = {
      address: '7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF',
      baseMint: 'So11111111111111111111111111111111111111112',
      quoteMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
      tickSize: '0.01',
      lotSize: '0.001',
      makerFee: -0.0002,
      takerFee: 0.0005,
    };

    it('returns the value for valid markets', () => {
      const result = assertMarket(validMarket);
      expect(result).toEqual(validMarket);
    });

    it('throws for invalid values', () => {
      expect(() => assertMarket({})).toThrow('Invalid Market');
      expect(() => assertMarket(null)).toThrow('Invalid Market');
    });
  });

  describe('assertOrder', () => {
    const validOrder = {
      id: '340282366920938463463374607431768211455',
      market: '7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF',
      owner: '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM',
      side: 'bid',
      price: '105.50',
      quantity: '100.0',
      filledQuantity: '25.0',
      orderType: 'limit',
      status: 'partiallyFilled',
      createdAt: '2026-01-30T12:00:00.000Z',
    };

    it('returns the value for valid orders', () => {
      const result = assertOrder(validOrder);
      expect(result).toEqual(validOrder);
    });

    it('throws for invalid values', () => {
      expect(() => assertOrder({})).toThrow('Invalid Order');
    });
  });
});

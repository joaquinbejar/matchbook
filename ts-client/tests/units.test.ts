import { describe, expect, it } from 'vitest';

import { Units, amountToAtoms, parseDecimal, parseDuration } from '../src/units.js';

// The demo market: 9-decimal base, 6-decimal quote, 0.001 base per lot,
// 1 quote atom per lot. A price of 150 is 150,000 lots.
const demo = () => new Units(1_000_000n, 1n, 9, 6);

describe('units', () => {
  it('converts prices both ways', () => {
    const u = demo();
    expect(u.priceToLots('150')).toBe(150_000n);
    expect(u.priceToLots('150.125')).toBe(150_125n);
    expect(u.priceToLots('150.1254')).toBe(150_125n);
    expect(u.priceToString(150_125n)).toBe('150.125');
    expect(u.priceNumberToLots(150.1)).toBe(150_100n);
    expect(() => u.priceToLots('0.0001')).toThrow();
  });

  it('converts sizes and amounts', () => {
    const u = demo();
    expect(u.sizeToLots('1.5')).toBe(1_500n);
    expect(u.lotsToString(1_500n)).toBe('1.5');
    expect(amountToAtoms('2.25', 6)).toBe(2_250_000n);
    expect(u.quoteToString(2_250_000n)).toBe('2.25');
    expect(() => u.sizeToLots('0.0001')).toThrow();
    expect(() => parseDecimal('1.2.3')).toThrow();
    expect(() => parseDecimal('-1')).toThrow();
  });

  it('parses durations', () => {
    expect(parseDuration('90')).toBe(90n);
    expect(parseDuration('30m')).toBe(1_800n);
    expect(parseDuration('7d')).toBe(604_800n);
    expect(() => parseDuration('1w')).toThrow();
  });
});

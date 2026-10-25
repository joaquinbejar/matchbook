import { describe, expect, it } from 'vitest';

import { buildLadder, checkRisk, defaultQuoteParams, ladderOrders, moved } from '../src/mm.js';
import { Units } from '../src/units.js';

const units = () => new Units(1_000_000n, 1n, 9, 6);
const rich = { base: 100, quote: 100_000 };

// The same cases as the Rust bot's tests, so both bots quote alike.
describe('ladder', () => {
  it('is symmetric around the reference', () => {
    const l = buildLadder(150, rich, { ...defaultQuoteParams, targetBase: 100 }, units());
    expect(l.bids.map((q) => q.price)).toEqual([149_850n, 149_700n, 149_550n]);
    expect(l.asks.map((q) => q.price)).toEqual([150_150n, 150_300n, 150_450n]);
    expect([...l.bids, ...l.asks].every((q) => q.baseLots === 1_000n)).toBe(true);
    expect(ladderOrders(l, 7n)).toHaveLength(6);
  });

  it('skews down and stops buying when long', () => {
    const l = buildLadder(150, rich, { ...defaultQuoteParams, targetBase: 50, maxPosition: 40, skewBps: 20 }, units());
    expect(l.bids).toHaveLength(0);
    expect(l.center).toBeLessThan(150);
    expect(l.asks).toHaveLength(3);
  });

  it('never quotes more than the seat holds or the notional cap', () => {
    const l = buildLadder(150, { base: 1.5, quote: 200 }, { ...defaultQuoteParams, targetBase: 1.5 }, units());
    expect(l.bids).toHaveLength(2);
    expect(l.asks.reduce((s, q) => s + q.baseLots, 0n)).toBe(1_500n);
    const u = units();
    const capped = buildLadder(150, rich, { ...defaultQuoteParams, targetBase: 100, maxNotional: 300 }, u);
    const notional = capped.asks.reduce((s, q) => s + u.lotsToNumber(q.baseLots) * u.priceToNumber(q.price), 0);
    expect(notional).toBeLessThanOrEqual(300 + 1e-9);
  });
});

describe('risk', () => {
  it('halts on loss and pauses on stale or jumping prices', () => {
    const l = { maxLoss: 10, maxPriceAgeSecs: 30, maxJumpBps: 100 };
    expect(checkRisk(l, 1000, 995, 1, 150, 150.1).kind).toBe('quote');
    expect(checkRisk(l, 1000, 989, 1, 150, 150).kind).toBe('halt');
    expect(checkRisk(l, 1000, 1000, 31, 150, 150).kind).toBe('pause');
    expect(checkRisk(l, 1000, 1000, 1, 150, 152).kind).toBe('pause');
    expect(moved(150, 150.1, 5)).toBe(true);
    expect(moved(150, 150.05, 5)).toBe(false);
  });
});

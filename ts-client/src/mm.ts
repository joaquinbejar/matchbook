/**
 * The market maker's quoting and risk logic, free of I/O. It matches the
 * Rust bot (`client/src/mm.rs`) rule for rule:
 *
 * - The first bid and ask sit `spreadBps` from the reference price, and each
 *   further level `stepBps` beyond the previous one.
 * - Inventory above its target skews the ladder down (and below, up).
 * - Past `maxPosition` from the target the side that grows it goes quiet.
 * - No side asks for more than the seat holds, nor more than `maxNotional`.
 */
import { OrderType, SelfTradeBehavior, Side, type OrderArgs } from '@matchbook/sdk';

import type { Units } from './units.js';

export interface QuoteParams {
  levels: number;
  spreadBps: number;
  stepBps: number;
  /** First level size in base tokens. */
  size: number;
  sizeGrowth: number;
  targetBase: number;
  maxPosition: number;
  skewBps: number;
  /** Quote tokens per side, 0 = no limit. */
  maxNotional: number;
  requoteBps: number;
}

export const defaultQuoteParams: QuoteParams = {
  levels: 3,
  spreadBps: 10,
  stepBps: 10,
  size: 1,
  sizeGrowth: 1,
  targetBase: 0,
  maxPosition: Infinity,
  skewBps: 0,
  maxNotional: 0,
  requoteBps: 5,
};

/** Tokens the seat holds: in total for risk, and what the next ladder can use. */
export interface Inventory {
  /** Base tokens, free plus locked in asks and stops. */
  base: number;
  /** Quote tokens, free plus locked in bids and stops. */
  quote: number;
  /**
   * Base tokens the ladder can sell: free plus what the requote's own cancels
   * release. Funds locked in other orders or stops stay out. Defaults to `base`.
   */
  baseAvailable?: number;
  /** Quote tokens the ladder can spend, counted the same way. Defaults to `quote`. */
  quoteAvailable?: number;
}

export const equity = (inv: Inventory, price: number) => inv.base * price + inv.quote;

export interface Quote {
  side: Side;
  /** Quote lots per base lot. */
  price: bigint;
  baseLots: bigint;
}

export interface Ladder {
  bids: Quote[];
  asks: Quote[];
  /** The skewed center price. */
  center: number;
}

/** The ladder as post-only-slide orders tagged with `clientOrderId`. */
export function ladderOrders(l: Ladder, clientOrderId: bigint): OrderArgs[] {
  return [...l.bids, ...l.asks].map((q) => ({
    side: q.side,
    price: q.price,
    baseLots: q.baseLots,
    orderType: OrderType.PostOnlySlide,
    selfTrade: SelfTradeBehavior.CancelProvide,
    clientOrderId,
  }));
}

/** Builds the ladder around `reference` quote tokens per base token. */
export function buildLadder(reference: number, inv: Inventory, p: QuoteParams, u: Units): Ladder {
  const out: Ladder = { bids: [], asks: [], center: 0 };
  if (!(Number.isFinite(reference) && reference > 0) || p.levels === 0) return out;
  const deviation = inv.base - p.targetBase;
  const ratio =
    Number.isFinite(p.maxPosition) && p.maxPosition > 0 ? Math.min(Math.max(deviation / p.maxPosition, -1), 1) : 0;
  const center = reference * (1 - (ratio * p.skewBps) / 10_000);
  out.center = center;
  const quoteBids = deviation < p.maxPosition;
  const quoteAsks = -deviation < p.maxPosition;

  let baseLeft = Math.max(inv.baseAvailable ?? inv.base, 0);
  let quoteLeft = Math.max(inv.quoteAvailable ?? inv.quote, 0);
  if (p.maxNotional > 0) quoteLeft = Math.min(quoteLeft, p.maxNotional);
  let askNotionalLeft = p.maxNotional > 0 ? p.maxNotional : Infinity;
  let buyRoom = Math.max(p.maxPosition - deviation, 0);
  let sellRoom = Math.max(p.maxPosition + deviation, 0);

  let size = p.size;
  for (let level = 0; level < p.levels; level++) {
    const off = (p.spreadBps + level * p.stepBps) / 10_000;
    let bidPx = u.priceNumberToLots(center * (1 - off));
    if (bidPx < 1n) bidPx = 1n;
    let askPx = u.priceNumberToLots(center * (1 + off));
    if (askPx <= bidPx) askPx = bidPx + 1n;
    if (quoteBids) {
      const px = u.priceToNumber(bidPx);
      const lots = u.numberToLots(Math.min(size, buyRoom, quoteLeft / px));
      const last = out.bids[out.bids.length - 1];
      if (lots > 0n && (!last || last.price > bidPx)) {
        const tokens = u.lotsToNumber(lots);
        quoteLeft -= tokens * px;
        buyRoom -= tokens;
        out.bids.push({ side: Side.Bid, price: bidPx, baseLots: lots });
      }
    }
    if (quoteAsks) {
      const px = u.priceToNumber(askPx);
      const lots = u.numberToLots(Math.min(size, sellRoom, baseLeft, askNotionalLeft / px));
      const last = out.asks[out.asks.length - 1];
      if (lots > 0n && (!last || last.price < askPx)) {
        const tokens = u.lotsToNumber(lots);
        baseLeft -= tokens;
        sellRoom -= tokens;
        askNotionalLeft -= tokens * px;
        out.asks.push({ side: Side.Ask, price: askPx, baseLots: lots });
      }
    }
    size *= p.sizeGrowth;
  }
  return out;
}

/** True when the price moved enough since the last quote to requote. */
export const moved = (last: number, current: number, requoteBps: number) =>
  last <= 0 || (Math.abs(current - last) / last) * 10_000 >= requoteBps;

export interface RiskLimits {
  /** Quote tokens of equity loss that halts the bot, 0 = off. */
  maxLoss: number;
  maxPriceAgeSecs: number;
  /** One-cycle reference move that pauses quoting (bps), 0 = off. */
  maxJumpBps: number;
}

export type RiskVerdict = { kind: 'quote' } | { kind: 'pause'; why: string } | { kind: 'halt'; why: string };

export function checkRisk(
  l: RiskLimits,
  startEquity: number,
  currentEquity: number,
  priceAgeSecs: number,
  lastReference: number,
  reference: number,
): RiskVerdict {
  if (l.maxLoss > 0 && startEquity - currentEquity >= l.maxLoss) {
    return { kind: 'halt', why: `loss ${(startEquity - currentEquity).toFixed(6)} reached the limit ${l.maxLoss}` };
  }
  if (priceAgeSecs > l.maxPriceAgeSecs) return { kind: 'pause', why: `reference price is ${priceAgeSecs}s old` };
  if (l.maxJumpBps > 0 && lastReference > 0 && (Math.abs(reference - lastReference) / lastReference) * 10_000 > l.maxJumpBps) {
    return { kind: 'pause', why: `reference jumped from ${lastReference} to ${reference}` };
  }
  return { kind: 'quote' };
}

/**
 * Reference prices for the market maker:
 *
 * - `fixed:150.5` a constant.
 * - `random:150:20` a random walk from 150 in 20 bps steps.
 * - `book` the market's last trade, else its mid.
 * - `http:<url>#<path>` a JSON number or numeric string at a dotted path,
 *   e.g. `http:https://api.coinbase.com/v2/prices/SOL-USD/spot#data.amount`.
 */
import type { MarketCtx } from './chain.js';

export type PriceSource = (m: MarketCtx) => Promise<number>;

export function parsePriceSource(spec: string): PriceSource {
  if (spec.startsWith('fixed:')) {
    const p = Number(spec.slice(6));
    if (!(p > 0)) throw new Error('fixed:<price> needs a positive price');
    return async () => p;
  }
  if (spec.startsWith('random:')) {
    const [start, step = '10'] = spec.slice(7).split(':');
    let price = Number(start);
    const stepBps = Number(step);
    if (!(price > 0) || !(stepBps >= 0)) throw new Error('random:<start>[:<step bps>]');
    return async () => {
      price *= 1 + ((Math.random() * 2 - 1) * stepBps) / 10_000;
      return price;
    };
  }
  if (spec === 'book') {
    return async (m) => {
      const h = m.view.header;
      if (h.lastPrice > 0n) return m.units.priceToNumber(h.lastPrice);
      const max = (1n << 64n) - 1n;
      if (h.bestBidPrice === 0n || h.bestAskPrice === 0n || h.bestAskPrice === max) {
        throw new Error('the book has no trades and no two-sided market to follow');
      }
      return m.units.priceToNumber((h.bestBidPrice + h.bestAskPrice) / 2n);
    };
  }
  if (spec.startsWith('http:')) {
    const rest = spec.slice(5);
    const hash = rest.lastIndexOf('#');
    if (hash < 0) throw new Error('http:<url>#<json.path>');
    const url = rest.slice(0, hash);
    const path = rest.slice(hash + 1).split('.');
    return async () => {
      const res = await fetch(url, { signal: AbortSignal.timeout(5_000) });
      if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
      let cur: unknown = await res.json();
      for (const k of path) cur = (cur as Record<string, unknown> | undefined)?.[k];
      const p = typeof cur === 'number' ? cur : Number(cur);
      if (!(p > 0)) throw new Error(`no positive number at ${path.join('.')} in ${url}`);
      return p;
    };
  }
  throw new Error(`unknown price source \`${spec}\` (fixed:, random:, book or http:)`);
}

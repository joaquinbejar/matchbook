import { getBase64Encoder, type Address } from '@solana/kit';

import { Reader } from './codec.js';
import { OutReason, Side } from './types.js';

/** A maker order traded. */
export interface FillEvent {
  kind: 'fill';
  maker: Address;
  makerOrderId: bigint;
  makerClientOrderId: bigint;
  /** Execution price (the maker's price). */
  price: bigint;
  baseLots: bigint;
  /** Lots left on the maker order (0 when it is gone). */
  makerRemaining: bigint;
}

/** An order or stop left the book other than by a fill or its owner's cancel. */
export interface OutEvent {
  kind: 'out';
  owner: Address;
  orderId: bigint;
  clientOrderId: bigint;
  side: Side;
  price: bigint;
  baseLots: bigint;
  reason: OutReason;
}

/** A reverse order traded and re-posted the filled size on the other side. */
export interface ReversedEvent {
  kind: 'reversed';
  owner: Address;
  /** The order that received the size (new, or the existing partner). */
  orderId: bigint;
  clientOrderId: bigint;
  side: Side;
  price: bigint;
  /** Lots added. */
  baseLots: bigint;
  expiresAt: bigint;
}

/** Any MatchBook event. */
export type MatchbookEvent = FillEvent | OutEvent | ReversedEvent;

const FILL_LEN = 73;
const OUT_LEN = 67;
const REVERSED_LEN = 74;

/** Decodes the events packed in one `sol_log_data` payload. */
export function decodeEvents(data: Uint8Array): MatchbookEvent[] {
  const r = new Reader(data);
  const out: MatchbookEvent[] = [];
  let i = 0;
  while (i < data.length) {
    const tag = r.u8(i);
    if (tag === 1 && i + FILL_LEN <= data.length) {
      out.push({
        kind: 'fill',
        maker: r.address(i + 1),
        makerOrderId: r.u64(i + 33),
        makerClientOrderId: r.u64(i + 41),
        price: r.u64(i + 49),
        baseLots: r.u64(i + 57),
        makerRemaining: r.u64(i + 65),
      });
      i += FILL_LEN;
    } else if (tag === 2 && i + OUT_LEN <= data.length) {
      out.push({
        kind: 'out',
        owner: r.address(i + 1),
        orderId: r.u64(i + 33),
        clientOrderId: r.u64(i + 41),
        side: r.u8(i + 49) as Side,
        price: r.u64(i + 50),
        baseLots: r.u64(i + 58),
        reason: r.u8(i + 66) as OutReason,
      });
      i += OUT_LEN;
    } else if (tag === 3 && i + REVERSED_LEN <= data.length) {
      out.push({
        kind: 'reversed',
        owner: r.address(i + 1),
        orderId: r.u64(i + 33),
        clientOrderId: r.u64(i + 41),
        side: r.u8(i + 49) as Side,
        price: r.u64(i + 50),
        baseLots: r.u64(i + 58),
        expiresAt: r.u64(i + 66),
      });
      i += REVERSED_LEN;
    } else {
      throw new Error(`unknown MatchBook event tag ${tag} at byte ${i}`);
    }
  }
  return out;
}

const base64 = getBase64Encoder();

/**
 * Decodes every MatchBook event in a transaction's log messages. Pass only
 * the logs of MatchBook instructions (or of transactions with no other
 * program that logs data).
 */
export function parseEventsFromLogs(logs: readonly string[]): MatchbookEvent[] {
  const out: MatchbookEvent[] = [];
  for (const l of logs) {
    if (!l.startsWith('Program data: ')) continue;
    const payload = l.slice('Program data: '.length).trim();
    out.push(...decodeEvents(new Uint8Array(base64.encode(payload))));
  }
  return out;
}

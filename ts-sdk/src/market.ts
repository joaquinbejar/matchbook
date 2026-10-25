import type { Address } from '@solana/kit';

import { Reader } from './codec.js';
import { HEADER_SIZE, MARKET_MAGIC, MARKET_FLAG_PERMISSIONED, NIL, NODE_SIZE, ORDER_INDEX_BITS } from './constants.js';
import { MarketStatus, OrderType, SelfTradeBehavior, Side, TriggerDirection } from './types.js';

/** Node tags. */
export enum NodeTag {
  Uninitialized = 0,
  Free = 1,
  Order = 2,
  Trader = 3,
  Delegate = 4,
  Stop = 5,
}

const FLAG_ASK = 1 << 1;
const FLAG_ABOVE = 1 << 2;

/** Market header fields. */
export interface MarketHeader {
  baseMint: Address;
  quoteMint: Address;
  baseVault: Address;
  quoteVault: Address;
  authority: Address;
  /** Base atoms per base lot. */
  baseLotSize: bigint;
  /** Quote atoms per quote lot. */
  quoteLotSize: bigint;
  minBaseLots: bigint;
  seqNum: bigint;
  /** Quote atoms of fees owed to the authority. */
  feesAccrued: bigint;
  totalBaseLotsTraded: bigint;
  totalQuoteAtomsTraded: bigint;
  takerFeeBps: number;
  makerRebateBps: number;
  orderCount: number;
  traderCount: number;
  status: MarketStatus;
  delegateCount: number;
  stopCount: number;
  /** Best bid price, or 0 when there is none. */
  bestBidPrice: bigint;
  /** Best ask price, or `2^64 - 1` when there is none. */
  bestAskPrice: bigint;
  /** Price of the last trade, or 0 before the first. */
  lastPrice: bigint;
  referralShareBps: number;
  permissioned: boolean;
  /** Nodes in the account. */
  capacity: number;
  /** Nodes free for new orders, seats or stops. */
  freeNodes: number;
}

/** A resting order. */
export interface Order {
  /** Public id: `seq << 24 | node index`. */
  orderId: bigint;
  seq: bigint;
  side: Side;
  /** Price in quote lots per base lot. */
  price: bigint;
  baseLots: bigint;
  clientOrderId: bigint;
  /** Unix time it dies, or 0. */
  expiresAt: bigint;
  /** Reverse spread in parts per million, or 0. */
  reverseSpread: number;
  /** Node index of the seat that owns it. */
  seat: number;
  owner: Address;
}

/** A stop waiting for its trigger. */
export interface Stop {
  orderId: bigint;
  seq: bigint;
  direction: TriggerDirection;
  triggerPrice: bigint;
  side: Side;
  price: bigint;
  baseLots: bigint;
  orderType: OrderType;
  selfTrade: SelfTradeBehavior;
  matchLimit: number;
  clientOrderId: bigint;
  expiresAt: bigint;
  /** Id of the one-cancels-other partner, or undefined. */
  ocoWith?: bigint;
  seat: number;
  owner: Address;
}

/** A trading account. */
export interface Seat {
  /** Node index: pass it as the `seat` hint. */
  index: number;
  owner: Address;
  /** Base atoms available to trade or withdraw. */
  baseFree: bigint;
  /** Quote atoms available to trade or withdraw. */
  quoteFree: bigint;
}

/** A key allowed to act for a seat. */
export interface Delegate {
  delegate: Address;
  seat: number;
  expiresAt: bigint;
  permissions: bigint;
}

/** One aggregated price level. */
export interface Level {
  price: bigint;
  baseLots: bigint;
  orders: number;
}

/** What a swap would do, computed from a market snapshot. */
export interface SwapQuote {
  /** Base lots traded. */
  baseLots: bigint;
  /** Base atoms traded. */
  baseAtoms: bigint;
  /** Quote atoms traded before fees. */
  quoteAtoms: bigint;
  /** Taker fee in quote atoms. */
  feeAtoms: bigint;
  /** Quote atoms paid (bid) or received (ask), fees applied. */
  quoteNet: bigint;
  /** Volume-weighted price in quote lots per base lot, or 0. */
  averagePrice: number;
  /** Resting orders touched. */
  makersTouched: number;
}

const U64_MAX = (1n << 64n) - 1n;
const BPS = 10_000n;

const ceilDiv = (a: bigint, b: bigint): bigint => (a + b - 1n) / b;

/**
 * A decoded snapshot of a market account. Everything a client needs to show
 * the book, find a seat or quote a swap, without RPC calls of its own.
 */
export class MarketView {
  readonly header: MarketHeader;
  private readonly r: Reader;
  private readonly nextUnused: number;

  constructor(
    readonly address: Address,
    data: Uint8Array,
  ) {
    if (data.length < HEADER_SIZE) throw new Error('not a MatchBook market: too short');
    const r = new Reader(data);
    if (r.u64(0) !== MARKET_MAGIC) throw new Error('not a MatchBook market: bad magic');
    this.r = r;
    const capacity = Math.min(Math.floor((data.length - HEADER_SIZE) / NODE_SIZE), NIL - 1);
    this.nextUnused = r.u32(252);
    const reserve = r.u32(312);
    const free = Math.max(capacity - this.nextUnused + r.u32(248) - reserve, 0);
    const flags = r.u32(352);
    this.header = {
      baseMint: r.address(8),
      quoteMint: r.address(40),
      baseVault: r.address(72),
      quoteVault: r.address(104),
      authority: r.address(136),
      baseLotSize: r.u64(168),
      quoteLotSize: r.u64(176),
      minBaseLots: r.u64(184),
      seqNum: r.u64(192),
      feesAccrued: r.u64(200),
      totalBaseLotsTraded: r.u64(208),
      totalQuoteAtomsTraded: r.u64(216),
      takerFeeBps: r.u32(224),
      makerRebateBps: r.u32(228),
      orderCount: r.u32(256),
      traderCount: r.u32(260),
      status: r.u32(264) as MarketStatus,
      delegateCount: r.u32(292),
      bestBidPrice: r.u64(296),
      bestAskPrice: r.u64(304),
      stopCount: r.u32(316),
      lastPrice: r.u64(320),
      referralShareBps: r.u32(356),
      permissioned: (flags & MARKET_FLAG_PERMISSIONED) !== 0,
      capacity,
      freeNodes: free,
    };
  }

  private base(i: number): number {
    return HEADER_SIZE + i * NODE_SIZE;
  }

  private tag(i: number): NodeTag {
    return this.r.u32(this.base(i)) as NodeTag;
  }

  private seatOwner(i: number): Address {
    return this.r.address(this.base(i) + 40);
  }

  private order(i: number): Order {
    const b = this.base(i);
    const r = this.r;
    const seq = r.u64(b + 80);
    const seat = r.u32(b + 40);
    return {
      orderId: (seq << ORDER_INDEX_BITS) | BigInt(i),
      seq,
      side: r.u32(b + 4) & FLAG_ASK ? Side.Ask : Side.Bid,
      price: r.u64(b + 32),
      baseLots: r.u64(b + 56),
      clientOrderId: r.u64(b + 64),
      expiresAt: r.u64(b + 72),
      reverseSpread: r.u32(b + 52),
      seat,
      owner: this.seatOwner(seat),
    };
  }

  private stop(i: number): Stop {
    const b = this.base(i);
    const r = this.r;
    const flags = r.u32(b + 4);
    const above = (flags & FLAG_ABOVE) !== 0;
    const keyLo = r.u64(b + 24);
    const seq = above ? keyLo : ~keyLo & U64_MAX;
    const partner = r.u32(b + 20);
    const seat = r.u32(b + 40);
    let ocoWith: bigint | undefined;
    if (partner !== 0) {
      const p = this.base(partner - 1);
      const pAbove = (r.u32(p + 4) & FLAG_ABOVE) !== 0;
      const pLo = r.u64(p + 24);
      const pSeq = pAbove ? pLo : ~pLo & U64_MAX;
      ocoWith = (pSeq << ORDER_INDEX_BITS) | BigInt(partner - 1);
    }
    return {
      orderId: (seq << ORDER_INDEX_BITS) | BigInt(i),
      seq,
      direction: above ? TriggerDirection.Above : TriggerDirection.Below,
      triggerPrice: r.u64(b + 32),
      side: flags & FLAG_ASK ? Side.Ask : Side.Bid,
      orderType: r.u8(b + 52) as OrderType,
      selfTrade: r.u8(b + 53) as SelfTradeBehavior,
      matchLimit: r.u16(b + 54),
      price: r.u64(b + 56),
      baseLots: r.u64(b + 64),
      clientOrderId: r.u64(b + 72),
      expiresAt: r.u64(b + 80),
      ocoWith,
      seat,
      owner: this.seatOwner(seat),
    };
  }

  private *indices(tag: NodeTag): Generator<number> {
    for (let i = 0; i < this.nextUnused; i++) {
      if (this.tag(i) === tag) yield i;
    }
  }

  /** Every seat. */
  seats(): Seat[] {
    return [...this.indices(NodeTag.Trader)].map((i) => {
      const b = this.base(i);
      return { index: i, owner: this.seatOwner(i), baseFree: this.r.u64(b + 72), quoteFree: this.r.u64(b + 80) };
    });
  }

  /** The seat of `owner`, if it has one. */
  findSeat(owner: Address): Seat | undefined {
    return this.seats().find((s) => s.owner === owner);
  }

  /** Every delegate. */
  delegates(): Delegate[] {
    return [...this.indices(NodeTag.Delegate)].map((i) => {
      const b = this.base(i);
      return {
        delegate: this.r.address(b + 40),
        seat: this.r.u32(b + 20),
        expiresAt: this.r.u64(b + 72),
        permissions: this.r.u64(b + 80),
      };
    });
  }

  /**
   * Resting orders on `side`, best first (earliest first at equal prices).
   * With `now`, orders expired at that unix time are left out.
   */
  orders(side: Side, now?: bigint): Order[] {
    const v = [...this.indices(NodeTag.Order)]
      .map((i) => this.order(i))
      .filter((o) => o.side === side && !(now !== undefined && o.expiresAt !== 0n && now >= o.expiresAt));
    v.sort((a, b) => {
      if (a.price !== b.price) {
        const better = side === Side.Bid ? a.price > b.price : a.price < b.price;
        return better ? -1 : 1;
      }
      return a.seq < b.seq ? -1 : 1;
    });
    return v;
  }

  /** Orders of the seat owned by `owner`. */
  ordersOf(owner: Address): Order[] {
    return [...this.indices(NodeTag.Order)].map((i) => this.order(i)).filter((o) => o.owner === owner);
  }

  /** Stops of the seat owned by `owner`, or every stop. */
  stops(owner?: Address): Stop[] {
    return [...this.indices(NodeTag.Stop)]
      .map((i) => this.stop(i))
      .filter((s) => owner === undefined || s.owner === owner);
  }

  /**
   * Base and quote atoms the seat of `owner` has locked in resting orders
   * and stops, as the program counts them.
   */
  locked(owner: Address): { base: bigint; quote: bigint } {
    const bls = this.header.baseLotSize;
    const qls = this.header.quoteLotSize;
    let base = 0n;
    let quote = 0n;
    for (const o of this.ordersOf(owner)) {
      if (o.side === Side.Ask) base += o.baseLots * bls;
      else quote += o.baseLots * o.price * qls;
    }
    for (const s of this.stops(owner)) {
      if (s.side === Side.Ask) {
        base += s.baseLots * bls;
      } else {
        const notional = s.baseLots * s.price * qls;
        // One atom a fill for fee rounding, and none without a fee.
        const limit = this.header.takerFeeBps === 0 ? 0n : BigInt(Math.max(s.matchLimit, 1));
        quote += notional + this.takerFee(notional) + (limit < s.baseLots ? limit : s.baseLots);
      }
    }
    return { base, quote };
  }

  /** Aggregated levels on `side`, best first, at most `depth` of them. */
  levels(side: Side, depth = Infinity, now?: bigint): Level[] {
    const out: Level[] = [];
    for (const o of this.orders(side, now)) {
      const last = out[out.length - 1];
      if (last && last.price === o.price) {
        last.baseLots += o.baseLots;
        last.orders += 1;
      } else {
        if (out.length >= depth) break;
        out.push({ price: o.price, baseLots: o.baseLots, orders: 1 });
      }
    }
    return out;
  }

  /** Taker fee for `quote` atoms, as the program charges it on each fill. */
  takerFee(quote: bigint): bigint {
    return ceilDiv(quote * BigInt(this.header.takerFeeBps), BPS);
  }

  /** Converts a price in quote lots per base lot to quote atoms per base atom. */
  priceToAtoms(price: bigint): number {
    return (Number(price) * Number(this.header.quoteLotSize)) / Number(this.header.baseLotSize);
  }

  /**
   * Simulates a seatless swap exactly as the program would run it now,
   * fill by fill, fees and rounding included.
   *
   * For a bid, `baseLots` caps the size and `quoteBudget` the spend; for an
   * ask, `baseLots` is the size to sell.
   */
  quoteSwap(input: {
    side: Side;
    baseLots: bigint;
    quoteBudget?: bigint;
    limitPrice?: bigint;
    matchLimit?: number;
    now?: bigint;
  }): SwapQuote {
    const { side } = input;
    const qls = this.header.quoteLotSize;
    const fee = BigInt(this.header.takerFeeBps);
    const limit = input.limitPrice ?? (side === Side.Bid ? U64_MAX : 1n);
    const matchLimit = input.matchLimit || 64;
    let lots = input.baseLots;
    let budget = side === Side.Bid ? (input.quoteBudget ?? U64_MAX) : undefined;
    let filled = 0n;
    let quote = 0n;
    let fees = 0n;
    let touched = 0;
    const affordable = (b: bigint, price: bigint): bigint => {
      const unit = price * qls;
      const cost = (l: bigint) => {
        const q = l * unit;
        return q + ceilDiv(q * fee, BPS);
      };
      let l = (b * BPS) / (unit * (BPS + fee));
      while (l > 0n && cost(l) > b) l -= 1n;
      while (cost(l + 1n) <= b) l += 1n;
      return l;
    };
    for (const o of this.orders(side === Side.Bid ? Side.Ask : Side.Bid)) {
      if (lots === 0n || touched >= matchLimit) break;
      const crosses = side === Side.Bid ? o.price <= limit : o.price >= limit;
      if (!crosses) break;
      touched += 1;
      if (input.now !== undefined && o.expiresAt !== 0n && input.now >= o.expiresAt) continue;
      let fill = lots < o.baseLots ? lots : o.baseLots;
      let q = fill * o.price * qls;
      let f = ceilDiv(q * fee, BPS);
      if (budget !== undefined && q + f > budget) {
        const a = affordable(budget, o.price);
        fill = fill < a ? fill : a;
        if (fill === 0n) {
          touched -= 1;
          break;
        }
        q = fill * o.price * qls;
        f = ceilDiv(q * fee, BPS);
      }
      lots -= fill;
      filled += fill;
      quote += q;
      fees += f;
      if (budget !== undefined) budget -= q + f;
    }
    return {
      baseLots: filled,
      baseAtoms: filled * this.header.baseLotSize,
      quoteAtoms: quote,
      feeAtoms: fees,
      quoteNet: side === Side.Bid ? quote + fees : quote - fees,
      averagePrice: filled === 0n ? 0 : Number(quote) / Number(filled * qls),
      makersTouched: touched,
    };
  }
}

/** Decodes a market account. */
export function decodeMarket(address: Address, data: Uint8Array): MarketView {
  return new MarketView(address, data);
}

/** Bytes a market account needs for `nodes` nodes. */
export function marketSize(nodes: number): number {
  return HEADER_SIZE + nodes * NODE_SIZE;
}

/** Splits an order id into its sequence number and node index. */
export function splitOrderId(id: bigint): { seq: bigint; index: number } {
  return { seq: id >> ORDER_INDEX_BITS, index: Number(id & ((1n << ORDER_INDEX_BITS) - 1n)) };
}

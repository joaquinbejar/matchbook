/**
 * Trade type definitions.
 *
 * @module types/trade
 */

import type { Price, Quantity, Side } from './primitives';

/**
 * Executed trade record.
 *
 * Represents a single trade execution between two orders.
 *
 * @example
 * ```typescript
 * const trade: Trade = {
 *   id: "abc123",
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   price: "105.55",
 *   quantity: "5.0",
 *   side: "bid",
 *   makerOrderId: "123456",
 *   takerOrderId: "789012",
 *   timestamp: "2026-01-30T12:00:01.234Z",
 * };
 * ```
 */
export interface Trade {
  /** Unique trade ID. */
  id: string;

  /** Market address. */
  market: string;

  /** Trade execution price. */
  price: Price;

  /** Trade quantity. */
  quantity: Quantity;

  /** Taker side (the aggressive order). */
  side: Side;

  /** Maker order ID. */
  makerOrderId?: string;

  /** Taker order ID. */
  takerOrderId?: string;

  /** Maker wallet address. */
  maker?: string;

  /** Taker wallet address. */
  taker?: string;

  /** Maker fee (negative for rebate). */
  makerFee?: string;

  /** Taker fee. */
  takerFee?: string;

  /** Trade timestamp (ISO 8601). */
  timestamp: string;

  /** Slot number when trade occurred. */
  slot?: number;
}

/**
 * Trade filter parameters for querying trades.
 *
 * @example
 * ```typescript
 * const filter: TradeFilter = {
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   limit: 100,
 *   before: "2026-01-30T12:00:00.000Z",
 * };
 * ```
 */
export interface TradeFilter {
  /** Filter by market address. */
  market?: string;

  /** Filter by user address. */
  user?: string;

  /** Maximum number of trades to return. */
  limit?: number;

  /** Return trades before this timestamp (ISO 8601). */
  before?: string;

  /** Return trades after this timestamp (ISO 8601). */
  after?: string;
}

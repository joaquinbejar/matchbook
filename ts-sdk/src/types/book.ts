/**
 * Order book type definitions.
 *
 * @module types/book
 */

import type { Price, Quantity } from './primitives';

/**
 * Aggregated price level in the order book.
 *
 * Represents a single price level with total quantity at that price.
 *
 * @example
 * ```typescript
 * const level: BookLevel = {
 *   price: "105.50",
 *   quantity: "1000.5",
 *   orders: 5,
 * };
 * ```
 */
export interface BookLevel {
  /** Price at this level. */
  price: Price;

  /** Total quantity at this price level. */
  quantity: Quantity;

  /** Number of orders at this level (optional). */
  orders?: number;
}

/**
 * Full order book with bids and asks.
 *
 * @example
 * ```typescript
 * const book: OrderBook = {
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   bids: [
 *     { price: "105.50", quantity: "100.5" },
 *     { price: "105.40", quantity: "250.0" },
 *   ],
 *   asks: [
 *     { price: "105.60", quantity: "75.0" },
 *     { price: "105.70", quantity: "200.0" },
 *   ],
 *   timestamp: "2026-01-30T12:00:00.000Z",
 * };
 * ```
 */
export interface OrderBook {
  /** Market address. */
  market: string;

  /** Bid levels (sorted by price descending). */
  bids: BookLevel[];

  /** Ask levels (sorted by price ascending). */
  asks: BookLevel[];

  /** Sequence number for ordering updates. */
  sequence?: number;

  /** Slot number when snapshot was taken. */
  slot?: number;

  /** Timestamp of the snapshot (ISO 8601). */
  timestamp?: string;
}

/**
 * Book level change for delta updates.
 *
 * Used in WebSocket updates to indicate changes to the order book.
 * A quantity of "0" indicates the level should be removed.
 *
 * @example
 * ```typescript
 * const change: BookChange = {
 *   price: "105.50",
 *   quantity: "150.0", // Updated quantity
 * };
 *
 * const removal: BookChange = {
 *   price: "105.60",
 *   quantity: "0", // Level removed
 * };
 * ```
 */
export interface BookChange {
  /** Price level. */
  price: Price;

  /** New quantity (0 means level removed). */
  quantity: Quantity;
}

/**
 * Order book update (delta or snapshot).
 *
 * @example
 * ```typescript
 * const update: OrderBookUpdate = {
 *   type: 'delta',
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   bids: [{ price: "105.50", quantity: "150.0" }],
 *   asks: [{ price: "105.60", quantity: "0" }],
 *   sequence: 12345679,
 * };
 * ```
 */
export interface OrderBookUpdate {
  /** Update type: 'snapshot' for full book, 'delta' for incremental. */
  type: 'snapshot' | 'delta';

  /** Market address. */
  market: string;

  /** Bid changes. */
  bids: BookChange[];

  /** Ask changes. */
  asks: BookChange[];

  /** Sequence number. */
  sequence?: number;

  /** Slot number. */
  slot?: number;

  /** Timestamp (ISO 8601). */
  timestamp?: string;
}

/**
 * Primitive types for Matchbook SDK.
 *
 * These types mirror the Rust SDK primitives and provide type safety
 * for JavaScript/TypeScript clients.
 *
 * @module types/primitives
 */

/**
 * Price value as a string for JSON safety and precision.
 *
 * Prices are represented as strings to avoid floating-point precision issues
 * and to maintain compatibility with JSON serialization.
 *
 * @example
 * ```typescript
 * const price: Price = "105.50";
 * ```
 */
export type Price = string;

/**
 * Quantity value as a string for JSON safety and precision.
 *
 * Quantities are represented as strings to handle large numbers
 * and maintain precision across serialization boundaries.
 *
 * @example
 * ```typescript
 * const quantity: Quantity = "1000.5";
 * ```
 */
export type Quantity = string;

/**
 * Order side - bid (buy) or ask (sell).
 *
 * @example
 * ```typescript
 * const side: Side = 'bid';
 * ```
 */
export type Side = 'bid' | 'ask';

/**
 * Order type determining execution behavior.
 *
 * - `limit` - Standard limit order, rests on book if not filled
 * - `postOnly` - Only rests on book, rejected if would cross
 * - `ioc` - Immediate or cancel, fills what it can immediately
 * - `fok` - Fill or kill, must fill entirely or is rejected
 *
 * @example
 * ```typescript
 * const orderType: OrderType = 'limit';
 * ```
 */
export type OrderType = 'limit' | 'postOnly' | 'ioc' | 'fok';

/**
 * Time in force for order validity.
 *
 * - `gtc` - Good till cancelled
 * - `ioc` - Immediate or cancel
 * - `fok` - Fill or kill
 * - `postOnly` - Post only (maker only)
 *
 * @example
 * ```typescript
 * const tif: TimeInForce = 'gtc';
 * ```
 */
export type TimeInForce = 'gtc' | 'ioc' | 'fok' | 'postOnly';

/**
 * Order status values.
 *
 * - `open` - Order is active on the book
 * - `partiallyFilled` - Order has been partially filled
 * - `filled` - Order has been completely filled
 * - `cancelled` - Order was cancelled
 * - `expired` - Order expired (time-based)
 *
 * @example
 * ```typescript
 * const status: OrderStatus = 'open';
 * ```
 */
export type OrderStatus = 'open' | 'partiallyFilled' | 'filled' | 'cancelled' | 'expired';

/**
 * Self-trade behavior options.
 *
 * - `decrementTake` - Decrement the taker order
 * - `cancelProvide` - Cancel the maker order
 * - `abortTransaction` - Abort the entire transaction
 *
 * @example
 * ```typescript
 * const stb: SelfTradeBehavior = 'decrementTake';
 * ```
 */
export type SelfTradeBehavior = 'decrementTake' | 'cancelProvide' | 'abortTransaction';

/**
 * All valid Side values.
 */
export const SIDES: readonly Side[] = ['bid', 'ask'] as const;

/**
 * All valid OrderType values.
 */
export const ORDER_TYPES: readonly OrderType[] = ['limit', 'postOnly', 'ioc', 'fok'] as const;

/**
 * All valid TimeInForce values.
 */
export const TIME_IN_FORCE_VALUES: readonly TimeInForce[] = ['gtc', 'ioc', 'fok', 'postOnly'] as const;

/**
 * All valid OrderStatus values.
 */
export const ORDER_STATUSES: readonly OrderStatus[] = [
  'open',
  'partiallyFilled',
  'filled',
  'cancelled',
  'expired',
] as const;

/**
 * All valid SelfTradeBehavior values.
 */
export const SELF_TRADE_BEHAVIORS: readonly SelfTradeBehavior[] = [
  'decrementTake',
  'cancelProvide',
  'abortTransaction',
] as const;

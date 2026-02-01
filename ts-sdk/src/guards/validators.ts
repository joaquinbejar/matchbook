/**
 * Type guards and validation functions.
 *
 * @module guards/validators
 */

import type {
  Side,
  OrderType,
  TimeInForce,
  OrderStatus,
  SelfTradeBehavior,
  Market,
  Order,
  Trade,
  BookLevel,
  OrderBook,
  Balance,
} from '../types';

import {
  SIDES,
  ORDER_TYPES,
  TIME_IN_FORCE_VALUES,
  ORDER_STATUSES,
  SELF_TRADE_BEHAVIORS,
} from '../types';

/**
 * Checks if a value is a valid Side.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Side.
 *
 * @example
 * ```typescript
 * if (isSide(value)) {
 *   // value is Side
 * }
 * ```
 */
export function isSide(value: unknown): value is Side {
  return typeof value === 'string' && SIDES.includes(value as Side);
}

/**
 * Checks if a value is a valid OrderType.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderType.
 */
export function isOrderType(value: unknown): value is OrderType {
  return typeof value === 'string' && ORDER_TYPES.includes(value as OrderType);
}

/**
 * Checks if a value is a valid TimeInForce.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid TimeInForce.
 */
export function isTimeInForce(value: unknown): value is TimeInForce {
  return typeof value === 'string' && TIME_IN_FORCE_VALUES.includes(value as TimeInForce);
}

/**
 * Checks if a value is a valid OrderStatus.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderStatus.
 */
export function isOrderStatus(value: unknown): value is OrderStatus {
  return typeof value === 'string' && ORDER_STATUSES.includes(value as OrderStatus);
}

/**
 * Checks if a value is a valid SelfTradeBehavior.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid SelfTradeBehavior.
 */
export function isSelfTradeBehavior(value: unknown): value is SelfTradeBehavior {
  return typeof value === 'string' && SELF_TRADE_BEHAVIORS.includes(value as SelfTradeBehavior);
}

/**
 * Checks if a value is a non-empty string.
 *
 * @param value - The value to check.
 * @returns True if the value is a non-empty string.
 */
export function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

/**
 * Checks if a value is a valid numeric string.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid numeric string.
 */
export function isNumericString(value: unknown): boolean {
  if (typeof value !== 'string' || value === '') return false;
  // Use regex to validate numeric format (optional sign, digits, optional decimal)
  const numericRegex = /^-?\d+(\.\d+)?$/;
  return numericRegex.test(value);
}

/**
 * Checks if a value is a valid positive numeric string.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid positive numeric string.
 */
export function isPositiveNumericString(value: unknown): boolean {
  if (!isNumericString(value)) return false;
  return parseFloat(value as string) > 0;
}

/**
 * Checks if a value is a valid non-negative numeric string.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid non-negative numeric string.
 */
export function isNonNegativeNumericString(value: unknown): boolean {
  if (!isNumericString(value)) return false;
  return parseFloat(value as string) >= 0;
}

/**
 * Checks if a value is a valid BookLevel.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid BookLevel.
 */
export function isBookLevel(value: unknown): value is BookLevel {
  if (typeof value !== 'object' || value === null) return false;
  const obj = value as Record<string, unknown>;
  return isNonEmptyString(obj.price) && isNonEmptyString(obj.quantity);
}

/**
 * Checks if a value is a valid Market.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Market.
 */
export function isMarket(value: unknown): value is Market {
  if (typeof value !== 'object' || value === null) return false;
  const obj = value as Record<string, unknown>;

  return (
    isNonEmptyString(obj.address) &&
    isNonEmptyString(obj.baseMint) &&
    isNonEmptyString(obj.quoteMint) &&
    isNonEmptyString(obj.tickSize) &&
    isNonEmptyString(obj.lotSize) &&
    typeof obj.makerFee === 'number' &&
    typeof obj.takerFee === 'number'
  );
}

/**
 * Checks if a value is a valid Order.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Order.
 */
export function isOrder(value: unknown): value is Order {
  if (typeof value !== 'object' || value === null) return false;
  const obj = value as Record<string, unknown>;

  return (
    isNonEmptyString(obj.id) &&
    isNonEmptyString(obj.market) &&
    isNonEmptyString(obj.owner) &&
    isSide(obj.side) &&
    isNonEmptyString(obj.price) &&
    isNonEmptyString(obj.quantity) &&
    isNonEmptyString(obj.filledQuantity) &&
    isOrderType(obj.orderType) &&
    isOrderStatus(obj.status) &&
    isNonEmptyString(obj.createdAt)
  );
}

/**
 * Checks if a value is a valid Trade.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Trade.
 */
export function isTrade(value: unknown): value is Trade {
  if (typeof value !== 'object' || value === null) return false;
  const obj = value as Record<string, unknown>;

  return (
    isNonEmptyString(obj.id) &&
    isNonEmptyString(obj.market) &&
    isNonEmptyString(obj.price) &&
    isNonEmptyString(obj.quantity) &&
    isSide(obj.side) &&
    isNonEmptyString(obj.timestamp)
  );
}

/**
 * Checks if a value is a valid OrderBook.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderBook.
 */
export function isOrderBook(value: unknown): value is OrderBook {
  if (typeof value !== 'object' || value === null) return false;
  const obj = value as Record<string, unknown>;

  if (!isNonEmptyString(obj.market)) return false;
  if (!Array.isArray(obj.bids) || !Array.isArray(obj.asks)) return false;

  return obj.bids.every(isBookLevel) && obj.asks.every(isBookLevel);
}

/**
 * Checks if a value is a valid Balance.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Balance.
 */
export function isBalance(value: unknown): value is Balance {
  if (typeof value !== 'object' || value === null) return false;
  const obj = value as Record<string, unknown>;

  return (
    isNonEmptyString(obj.market) &&
    isNonEmptyString(obj.owner) &&
    isNonEmptyString(obj.baseAvailable) &&
    isNonEmptyString(obj.baseLocked) &&
    isNonEmptyString(obj.quoteAvailable) &&
    isNonEmptyString(obj.quoteLocked)
  );
}

/**
 * Validates a value and throws if invalid.
 *
 * @param value - The value to validate.
 * @param guard - The type guard function.
 * @param typeName - The name of the type for error messages.
 * @returns The validated value.
 * @throws Error if validation fails.
 */
export function assertType<T>(
  value: unknown,
  guard: (v: unknown) => v is T,
  typeName: string
): T {
  if (!guard(value)) {
    throw new Error(`Invalid ${typeName}: ${JSON.stringify(value)}`);
  }
  return value;
}

/**
 * Validates a Market and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Market.
 * @throws Error if validation fails.
 */
export function assertMarket(value: unknown): Market {
  return assertType(value, isMarket, 'Market');
}

/**
 * Validates an Order and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Order.
 * @throws Error if validation fails.
 */
export function assertOrder(value: unknown): Order {
  return assertType(value, isOrder, 'Order');
}

/**
 * Validates a Trade and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Trade.
 * @throws Error if validation fails.
 */
export function assertTrade(value: unknown): Trade {
  return assertType(value, isTrade, 'Trade');
}

/**
 * Validates an OrderBook and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated OrderBook.
 * @throws Error if validation fails.
 */
export function assertOrderBook(value: unknown): OrderBook {
  return assertType(value, isOrderBook, 'OrderBook');
}

/**
 * Validates a Balance and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Balance.
 * @throws Error if validation fails.
 */
export function assertBalance(value: unknown): Balance {
  return assertType(value, isBalance, 'Balance');
}

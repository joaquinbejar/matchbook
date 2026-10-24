/**
 * Type guards and validation functions.
 *
 * @module guards/validators
 */
import type { Side, OrderType, TimeInForce, OrderStatus, SelfTradeBehavior, Market, Order, Trade, BookLevel, OrderBook, Balance } from '../types';
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
export declare function isSide(value: unknown): value is Side;
/**
 * Checks if a value is a valid OrderType.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderType.
 */
export declare function isOrderType(value: unknown): value is OrderType;
/**
 * Checks if a value is a valid TimeInForce.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid TimeInForce.
 */
export declare function isTimeInForce(value: unknown): value is TimeInForce;
/**
 * Checks if a value is a valid OrderStatus.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderStatus.
 */
export declare function isOrderStatus(value: unknown): value is OrderStatus;
/**
 * Checks if a value is a valid SelfTradeBehavior.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid SelfTradeBehavior.
 */
export declare function isSelfTradeBehavior(value: unknown): value is SelfTradeBehavior;
/**
 * Checks if a value is a non-empty string.
 *
 * @param value - The value to check.
 * @returns True if the value is a non-empty string.
 */
export declare function isNonEmptyString(value: unknown): value is string;
/**
 * Checks if a value is a valid numeric string.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid numeric string.
 */
export declare function isNumericString(value: unknown): boolean;
/**
 * Checks if a value is a valid positive numeric string.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid positive numeric string.
 */
export declare function isPositiveNumericString(value: unknown): boolean;
/**
 * Checks if a value is a valid non-negative numeric string.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid non-negative numeric string.
 */
export declare function isNonNegativeNumericString(value: unknown): boolean;
/**
 * Checks if a value is a valid BookLevel.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid BookLevel.
 */
export declare function isBookLevel(value: unknown): value is BookLevel;
/**
 * Checks if a value is a valid Market.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Market.
 */
export declare function isMarket(value: unknown): value is Market;
/**
 * Checks if a value is a valid Order.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Order.
 */
export declare function isOrder(value: unknown): value is Order;
/**
 * Checks if a value is a valid Trade.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Trade.
 */
export declare function isTrade(value: unknown): value is Trade;
/**
 * Checks if a value is a valid OrderBook.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderBook.
 */
export declare function isOrderBook(value: unknown): value is OrderBook;
/**
 * Checks if a value is a valid Balance.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Balance.
 */
export declare function isBalance(value: unknown): value is Balance;
/**
 * Validates a value and throws if invalid.
 *
 * @param value - The value to validate.
 * @param guard - The type guard function.
 * @param typeName - The name of the type for error messages.
 * @returns The validated value.
 * @throws Error if validation fails.
 */
export declare function assertType<T>(value: unknown, guard: (v: unknown) => v is T, typeName: string): T;
/**
 * Validates a Market and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Market.
 * @throws Error if validation fails.
 */
export declare function assertMarket(value: unknown): Market;
/**
 * Validates an Order and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Order.
 * @throws Error if validation fails.
 */
export declare function assertOrder(value: unknown): Order;
/**
 * Validates a Trade and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Trade.
 * @throws Error if validation fails.
 */
export declare function assertTrade(value: unknown): Trade;
/**
 * Validates an OrderBook and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated OrderBook.
 * @throws Error if validation fails.
 */
export declare function assertOrderBook(value: unknown): OrderBook;
/**
 * Validates a Balance and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Balance.
 * @throws Error if validation fails.
 */
export declare function assertBalance(value: unknown): Balance;
//# sourceMappingURL=validators.d.ts.map
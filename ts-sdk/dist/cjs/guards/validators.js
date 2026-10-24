"use strict";
/**
 * Type guards and validation functions.
 *
 * @module guards/validators
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.isSide = isSide;
exports.isOrderType = isOrderType;
exports.isTimeInForce = isTimeInForce;
exports.isOrderStatus = isOrderStatus;
exports.isSelfTradeBehavior = isSelfTradeBehavior;
exports.isNonEmptyString = isNonEmptyString;
exports.isNumericString = isNumericString;
exports.isPositiveNumericString = isPositiveNumericString;
exports.isNonNegativeNumericString = isNonNegativeNumericString;
exports.isBookLevel = isBookLevel;
exports.isMarket = isMarket;
exports.isOrder = isOrder;
exports.isTrade = isTrade;
exports.isOrderBook = isOrderBook;
exports.isBalance = isBalance;
exports.assertType = assertType;
exports.assertMarket = assertMarket;
exports.assertOrder = assertOrder;
exports.assertTrade = assertTrade;
exports.assertOrderBook = assertOrderBook;
exports.assertBalance = assertBalance;
const types_1 = require("../types");
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
function isSide(value) {
    return typeof value === 'string' && types_1.SIDES.includes(value);
}
/**
 * Checks if a value is a valid OrderType.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderType.
 */
function isOrderType(value) {
    return typeof value === 'string' && types_1.ORDER_TYPES.includes(value);
}
/**
 * Checks if a value is a valid TimeInForce.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid TimeInForce.
 */
function isTimeInForce(value) {
    return typeof value === 'string' && types_1.TIME_IN_FORCE_VALUES.includes(value);
}
/**
 * Checks if a value is a valid OrderStatus.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderStatus.
 */
function isOrderStatus(value) {
    return typeof value === 'string' && types_1.ORDER_STATUSES.includes(value);
}
/**
 * Checks if a value is a valid SelfTradeBehavior.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid SelfTradeBehavior.
 */
function isSelfTradeBehavior(value) {
    return typeof value === 'string' && types_1.SELF_TRADE_BEHAVIORS.includes(value);
}
/**
 * Checks if a value is a non-empty string.
 *
 * @param value - The value to check.
 * @returns True if the value is a non-empty string.
 */
function isNonEmptyString(value) {
    return typeof value === 'string' && value.length > 0;
}
/**
 * Checks if a value is a valid numeric string.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid numeric string.
 */
function isNumericString(value) {
    if (typeof value !== 'string' || value === '')
        return false;
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
function isPositiveNumericString(value) {
    if (!isNumericString(value))
        return false;
    return parseFloat(value) > 0;
}
/**
 * Checks if a value is a valid non-negative numeric string.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid non-negative numeric string.
 */
function isNonNegativeNumericString(value) {
    if (!isNumericString(value))
        return false;
    return parseFloat(value) >= 0;
}
/**
 * Checks if a value is a valid BookLevel.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid BookLevel.
 */
function isBookLevel(value) {
    if (typeof value !== 'object' || value === null)
        return false;
    const obj = value;
    return isNonEmptyString(obj.price) && isNonEmptyString(obj.quantity);
}
/**
 * Checks if a value is a valid Market.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Market.
 */
function isMarket(value) {
    if (typeof value !== 'object' || value === null)
        return false;
    const obj = value;
    return (isNonEmptyString(obj.address) &&
        isNonEmptyString(obj.baseMint) &&
        isNonEmptyString(obj.quoteMint) &&
        isNonEmptyString(obj.tickSize) &&
        isNonEmptyString(obj.lotSize) &&
        typeof obj.makerFee === 'number' &&
        typeof obj.takerFee === 'number');
}
/**
 * Checks if a value is a valid Order.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Order.
 */
function isOrder(value) {
    if (typeof value !== 'object' || value === null)
        return false;
    const obj = value;
    return (isNonEmptyString(obj.id) &&
        isNonEmptyString(obj.market) &&
        isNonEmptyString(obj.owner) &&
        isSide(obj.side) &&
        isNonEmptyString(obj.price) &&
        isNonEmptyString(obj.quantity) &&
        isNonEmptyString(obj.filledQuantity) &&
        isOrderType(obj.orderType) &&
        isOrderStatus(obj.status) &&
        isNonEmptyString(obj.createdAt));
}
/**
 * Checks if a value is a valid Trade.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Trade.
 */
function isTrade(value) {
    if (typeof value !== 'object' || value === null)
        return false;
    const obj = value;
    return (isNonEmptyString(obj.id) &&
        isNonEmptyString(obj.market) &&
        isNonEmptyString(obj.price) &&
        isNonEmptyString(obj.quantity) &&
        isSide(obj.side) &&
        isNonEmptyString(obj.timestamp));
}
/**
 * Checks if a value is a valid OrderBook.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid OrderBook.
 */
function isOrderBook(value) {
    if (typeof value !== 'object' || value === null)
        return false;
    const obj = value;
    if (!isNonEmptyString(obj.market))
        return false;
    if (!Array.isArray(obj.bids) || !Array.isArray(obj.asks))
        return false;
    return obj.bids.every(isBookLevel) && obj.asks.every(isBookLevel);
}
/**
 * Checks if a value is a valid Balance.
 *
 * @param value - The value to check.
 * @returns True if the value is a valid Balance.
 */
function isBalance(value) {
    if (typeof value !== 'object' || value === null)
        return false;
    const obj = value;
    return (isNonEmptyString(obj.market) &&
        isNonEmptyString(obj.owner) &&
        isNonEmptyString(obj.baseAvailable) &&
        isNonEmptyString(obj.baseLocked) &&
        isNonEmptyString(obj.quoteAvailable) &&
        isNonEmptyString(obj.quoteLocked));
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
function assertType(value, guard, typeName) {
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
function assertMarket(value) {
    return assertType(value, isMarket, 'Market');
}
/**
 * Validates an Order and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Order.
 * @throws Error if validation fails.
 */
function assertOrder(value) {
    return assertType(value, isOrder, 'Order');
}
/**
 * Validates a Trade and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Trade.
 * @throws Error if validation fails.
 */
function assertTrade(value) {
    return assertType(value, isTrade, 'Trade');
}
/**
 * Validates an OrderBook and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated OrderBook.
 * @throws Error if validation fails.
 */
function assertOrderBook(value) {
    return assertType(value, isOrderBook, 'OrderBook');
}
/**
 * Validates a Balance and throws if invalid.
 *
 * @param value - The value to validate.
 * @returns The validated Balance.
 * @throws Error if validation fails.
 */
function assertBalance(value) {
    return assertType(value, isBalance, 'Balance');
}
//# sourceMappingURL=validators.js.map
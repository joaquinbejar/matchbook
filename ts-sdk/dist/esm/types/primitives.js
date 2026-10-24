/**
 * Primitive types for Matchbook SDK.
 *
 * These types mirror the Rust SDK primitives and provide type safety
 * for JavaScript/TypeScript clients.
 *
 * @module types/primitives
 */
/**
 * All valid Side values.
 */
export const SIDES = ['bid', 'ask'];
/**
 * All valid OrderType values.
 */
export const ORDER_TYPES = ['limit', 'postOnly', 'ioc', 'fok'];
/**
 * All valid TimeInForce values.
 */
export const TIME_IN_FORCE_VALUES = ['gtc', 'ioc', 'fok', 'postOnly'];
/**
 * All valid OrderStatus values.
 */
export const ORDER_STATUSES = [
    'open',
    'partiallyFilled',
    'filled',
    'cancelled',
    'expired',
];
/**
 * All valid SelfTradeBehavior values.
 */
export const SELF_TRADE_BEHAVIORS = [
    'decrementTake',
    'cancelProvide',
    'abortTransaction',
];
//# sourceMappingURL=primitives.js.map
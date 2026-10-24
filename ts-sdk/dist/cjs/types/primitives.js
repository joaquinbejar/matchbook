"use strict";
/**
 * Primitive types for Matchbook SDK.
 *
 * These types mirror the Rust SDK primitives and provide type safety
 * for JavaScript/TypeScript clients.
 *
 * @module types/primitives
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SELF_TRADE_BEHAVIORS = exports.ORDER_STATUSES = exports.TIME_IN_FORCE_VALUES = exports.ORDER_TYPES = exports.SIDES = void 0;
/**
 * All valid Side values.
 */
exports.SIDES = ['bid', 'ask'];
/**
 * All valid OrderType values.
 */
exports.ORDER_TYPES = ['limit', 'postOnly', 'ioc', 'fok'];
/**
 * All valid TimeInForce values.
 */
exports.TIME_IN_FORCE_VALUES = ['gtc', 'ioc', 'fok', 'postOnly'];
/**
 * All valid OrderStatus values.
 */
exports.ORDER_STATUSES = [
    'open',
    'partiallyFilled',
    'filled',
    'cancelled',
    'expired',
];
/**
 * All valid SelfTradeBehavior values.
 */
exports.SELF_TRADE_BEHAVIORS = [
    'decrementTake',
    'cancelProvide',
    'abortTransaction',
];
//# sourceMappingURL=primitives.js.map
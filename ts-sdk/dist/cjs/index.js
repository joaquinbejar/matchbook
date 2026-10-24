"use strict";
/**
 * Matchbook SDK - TypeScript client library for the Matchbook CLOB.
 *
 * This package provides type definitions and utilities for interacting with
 * the Matchbook Central Limit Order Book on Solana.
 *
 * @packageDocumentation
 * @module @matchbook/sdk
 *
 * @example
 * ```typescript
 * import { Side, OrderType, Market, Order, isSide, isOrder } from '@matchbook/sdk';
 *
 * // Type-safe order creation
 * const side: Side = 'bid';
 * const orderType: OrderType = 'limit';
 *
 * // Runtime validation
 * if (isSide(unknownValue)) {
 *   console.log('Valid side:', unknownValue);
 * }
 * ```
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.MatchbookWsClient = exports.MatchbookClient = exports.isRateLimitError = exports.isApiError = exports.isHttpError = exports.isClientError = exports.UnauthorizedError = exports.NotFoundError = exports.WebSocketError = exports.RateLimitError = exports.TimeoutError = exports.ClientApiError = exports.HttpError = exports.ClientError = exports.validateConfig = exports.resolveConfig = exports.DEFAULT_TIMEOUT = exports.DEFAULT_WS_URL = exports.DEFAULT_BASE_URL = exports.assertBalance = exports.assertOrderBook = exports.assertTrade = exports.assertOrder = exports.assertMarket = exports.assertType = exports.isBalance = exports.isOrderBook = exports.isTrade = exports.isOrder = exports.isMarket = exports.isBookLevel = exports.isNonNegativeNumericString = exports.isPositiveNumericString = exports.isNumericString = exports.isNonEmptyString = exports.isSelfTradeBehavior = exports.isOrderStatus = exports.isTimeInForce = exports.isOrderType = exports.isSide = exports.WS_CHANNELS = exports.SELF_TRADE_BEHAVIORS = exports.ORDER_STATUSES = exports.TIME_IN_FORCE_VALUES = exports.ORDER_TYPES = exports.SIDES = void 0;
var types_1 = require("./types");
Object.defineProperty(exports, "SIDES", { enumerable: true, get: function () { return types_1.SIDES; } });
Object.defineProperty(exports, "ORDER_TYPES", { enumerable: true, get: function () { return types_1.ORDER_TYPES; } });
Object.defineProperty(exports, "TIME_IN_FORCE_VALUES", { enumerable: true, get: function () { return types_1.TIME_IN_FORCE_VALUES; } });
Object.defineProperty(exports, "ORDER_STATUSES", { enumerable: true, get: function () { return types_1.ORDER_STATUSES; } });
Object.defineProperty(exports, "SELF_TRADE_BEHAVIORS", { enumerable: true, get: function () { return types_1.SELF_TRADE_BEHAVIORS; } });
var api_1 = require("./api");
Object.defineProperty(exports, "WS_CHANNELS", { enumerable: true, get: function () { return api_1.WS_CHANNELS; } });
// Type guards and validators
var guards_1 = require("./guards");
Object.defineProperty(exports, "isSide", { enumerable: true, get: function () { return guards_1.isSide; } });
Object.defineProperty(exports, "isOrderType", { enumerable: true, get: function () { return guards_1.isOrderType; } });
Object.defineProperty(exports, "isTimeInForce", { enumerable: true, get: function () { return guards_1.isTimeInForce; } });
Object.defineProperty(exports, "isOrderStatus", { enumerable: true, get: function () { return guards_1.isOrderStatus; } });
Object.defineProperty(exports, "isSelfTradeBehavior", { enumerable: true, get: function () { return guards_1.isSelfTradeBehavior; } });
Object.defineProperty(exports, "isNonEmptyString", { enumerable: true, get: function () { return guards_1.isNonEmptyString; } });
Object.defineProperty(exports, "isNumericString", { enumerable: true, get: function () { return guards_1.isNumericString; } });
Object.defineProperty(exports, "isPositiveNumericString", { enumerable: true, get: function () { return guards_1.isPositiveNumericString; } });
Object.defineProperty(exports, "isNonNegativeNumericString", { enumerable: true, get: function () { return guards_1.isNonNegativeNumericString; } });
Object.defineProperty(exports, "isBookLevel", { enumerable: true, get: function () { return guards_1.isBookLevel; } });
Object.defineProperty(exports, "isMarket", { enumerable: true, get: function () { return guards_1.isMarket; } });
Object.defineProperty(exports, "isOrder", { enumerable: true, get: function () { return guards_1.isOrder; } });
Object.defineProperty(exports, "isTrade", { enumerable: true, get: function () { return guards_1.isTrade; } });
Object.defineProperty(exports, "isOrderBook", { enumerable: true, get: function () { return guards_1.isOrderBook; } });
Object.defineProperty(exports, "isBalance", { enumerable: true, get: function () { return guards_1.isBalance; } });
Object.defineProperty(exports, "assertType", { enumerable: true, get: function () { return guards_1.assertType; } });
Object.defineProperty(exports, "assertMarket", { enumerable: true, get: function () { return guards_1.assertMarket; } });
Object.defineProperty(exports, "assertOrder", { enumerable: true, get: function () { return guards_1.assertOrder; } });
Object.defineProperty(exports, "assertTrade", { enumerable: true, get: function () { return guards_1.assertTrade; } });
Object.defineProperty(exports, "assertOrderBook", { enumerable: true, get: function () { return guards_1.assertOrderBook; } });
Object.defineProperty(exports, "assertBalance", { enumerable: true, get: function () { return guards_1.assertBalance; } });
var client_1 = require("./client");
Object.defineProperty(exports, "DEFAULT_BASE_URL", { enumerable: true, get: function () { return client_1.DEFAULT_BASE_URL; } });
Object.defineProperty(exports, "DEFAULT_WS_URL", { enumerable: true, get: function () { return client_1.DEFAULT_WS_URL; } });
Object.defineProperty(exports, "DEFAULT_TIMEOUT", { enumerable: true, get: function () { return client_1.DEFAULT_TIMEOUT; } });
Object.defineProperty(exports, "resolveConfig", { enumerable: true, get: function () { return client_1.resolveConfig; } });
Object.defineProperty(exports, "validateConfig", { enumerable: true, get: function () { return client_1.validateConfig; } });
Object.defineProperty(exports, "ClientError", { enumerable: true, get: function () { return client_1.ClientError; } });
Object.defineProperty(exports, "HttpError", { enumerable: true, get: function () { return client_1.HttpError; } });
Object.defineProperty(exports, "ClientApiError", { enumerable: true, get: function () { return client_1.ApiError; } });
Object.defineProperty(exports, "TimeoutError", { enumerable: true, get: function () { return client_1.TimeoutError; } });
Object.defineProperty(exports, "RateLimitError", { enumerable: true, get: function () { return client_1.RateLimitError; } });
Object.defineProperty(exports, "WebSocketError", { enumerable: true, get: function () { return client_1.WebSocketError; } });
Object.defineProperty(exports, "NotFoundError", { enumerable: true, get: function () { return client_1.NotFoundError; } });
Object.defineProperty(exports, "UnauthorizedError", { enumerable: true, get: function () { return client_1.UnauthorizedError; } });
Object.defineProperty(exports, "isClientError", { enumerable: true, get: function () { return client_1.isClientError; } });
Object.defineProperty(exports, "isHttpError", { enumerable: true, get: function () { return client_1.isHttpError; } });
Object.defineProperty(exports, "isApiError", { enumerable: true, get: function () { return client_1.isApiError; } });
Object.defineProperty(exports, "isRateLimitError", { enumerable: true, get: function () { return client_1.isRateLimitError; } });
Object.defineProperty(exports, "MatchbookClient", { enumerable: true, get: function () { return client_1.MatchbookClient; } });
Object.defineProperty(exports, "MatchbookWsClient", { enumerable: true, get: function () { return client_1.MatchbookWsClient; } });
//# sourceMappingURL=index.js.map
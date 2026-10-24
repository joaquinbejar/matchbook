"use strict";
/**
 * Client module for Matchbook SDK.
 *
 * Provides HTTP and WebSocket clients for interacting with the Matchbook API.
 *
 * @module client
 *
 * @example
 * ```typescript
 * import { MatchbookClient, MatchbookWsClient } from '@matchbook/sdk';
 *
 * // HTTP client
 * const client = new MatchbookClient({
 *   baseUrl: 'https://api.matchbook.taunais.com/v1',
 *   apiKey: 'your-api-key',
 * });
 *
 * const markets = await client.getMarkets();
 *
 * // WebSocket client
 * const ws = new MatchbookWsClient({
 *   wsUrl: 'wss://ws.matchbook.taunais.com/v1/stream',
 * });
 *
 * await ws.connect();
 * ws.subscribeBook('ABC123...', (data) => console.log(data));
 * ```
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.MatchbookWsClient = exports.MatchbookClient = exports.isRateLimitError = exports.isApiError = exports.isHttpError = exports.isClientError = exports.UnauthorizedError = exports.NotFoundError = exports.WebSocketError = exports.RateLimitError = exports.TimeoutError = exports.ApiError = exports.HttpError = exports.ClientError = exports.validateConfig = exports.resolveConfig = exports.DEFAULT_TIMEOUT = exports.DEFAULT_WS_URL = exports.DEFAULT_BASE_URL = void 0;
var config_1 = require("./config");
Object.defineProperty(exports, "DEFAULT_BASE_URL", { enumerable: true, get: function () { return config_1.DEFAULT_BASE_URL; } });
Object.defineProperty(exports, "DEFAULT_WS_URL", { enumerable: true, get: function () { return config_1.DEFAULT_WS_URL; } });
Object.defineProperty(exports, "DEFAULT_TIMEOUT", { enumerable: true, get: function () { return config_1.DEFAULT_TIMEOUT; } });
Object.defineProperty(exports, "resolveConfig", { enumerable: true, get: function () { return config_1.resolveConfig; } });
Object.defineProperty(exports, "validateConfig", { enumerable: true, get: function () { return config_1.validateConfig; } });
var errors_1 = require("./errors");
Object.defineProperty(exports, "ClientError", { enumerable: true, get: function () { return errors_1.ClientError; } });
Object.defineProperty(exports, "HttpError", { enumerable: true, get: function () { return errors_1.HttpError; } });
Object.defineProperty(exports, "ApiError", { enumerable: true, get: function () { return errors_1.ApiError; } });
Object.defineProperty(exports, "TimeoutError", { enumerable: true, get: function () { return errors_1.TimeoutError; } });
Object.defineProperty(exports, "RateLimitError", { enumerable: true, get: function () { return errors_1.RateLimitError; } });
Object.defineProperty(exports, "WebSocketError", { enumerable: true, get: function () { return errors_1.WebSocketError; } });
Object.defineProperty(exports, "NotFoundError", { enumerable: true, get: function () { return errors_1.NotFoundError; } });
Object.defineProperty(exports, "UnauthorizedError", { enumerable: true, get: function () { return errors_1.UnauthorizedError; } });
Object.defineProperty(exports, "isClientError", { enumerable: true, get: function () { return errors_1.isClientError; } });
Object.defineProperty(exports, "isHttpError", { enumerable: true, get: function () { return errors_1.isHttpError; } });
Object.defineProperty(exports, "isApiError", { enumerable: true, get: function () { return errors_1.isApiError; } });
Object.defineProperty(exports, "isRateLimitError", { enumerable: true, get: function () { return errors_1.isRateLimitError; } });
var http_1 = require("./http");
Object.defineProperty(exports, "MatchbookClient", { enumerable: true, get: function () { return http_1.MatchbookClient; } });
var websocket_1 = require("./websocket");
Object.defineProperty(exports, "MatchbookWsClient", { enumerable: true, get: function () { return websocket_1.MatchbookWsClient; } });
//# sourceMappingURL=index.js.map
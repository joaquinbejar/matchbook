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

export type { ClientConfig, ResolvedConfig } from './config.js';
export {
  DEFAULT_BASE_URL,
  DEFAULT_WS_URL,
  DEFAULT_TIMEOUT,
  resolveConfig,
  validateConfig,
} from './config.js';

export {
  ClientError,
  HttpError,
  ApiError,
  TimeoutError,
  RateLimitError,
  WebSocketError,
  NotFoundError,
  UnauthorizedError,
  isClientError,
  isHttpError,
  isApiError,
  isRateLimitError,
} from './errors.js';

export { MatchbookClient } from './http.js';

export type { BookCallback, TradeCallback, OrderCallback, ErrorCallback } from './websocket.js';
export { MatchbookWsClient } from './websocket.js';

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
export type { Price, Quantity, Side, OrderType, TimeInForce, OrderStatus, SelfTradeBehavior, } from './types';
export { SIDES, ORDER_TYPES, TIME_IN_FORCE_VALUES, ORDER_STATUSES, SELF_TRADE_BEHAVIORS, } from './types';
export type { Market, MarketSummary, Order, PlaceOrderParams, CancelOrderParams, Trade, TradeFilter, BookLevel, OrderBook, BookChange, OrderBookUpdate, Balance, DepositParams, WithdrawParams, } from './types';
export type { ApiResponse, PaginatedResponse, ApiError, GetMarketsResponse, GetMarketResponse, GetMarketSummaryResponse, GetOrderBookResponse, GetTradesResponse, GetOrdersResponse, GetOrderResponse, GetBalancesResponse, GetBalanceResponse, PlaceOrderResponse, CancelOrderResponse, BuildTransactionResponse, } from './api';
export type { WsChannel, WsMessageBase, WsSubscribeMessage, WsUnsubscribeMessage, WsPingMessage, WsClientMessage, WsSubscribedMessage, WsUnsubscribedMessage, WsBookSnapshotMessage, WsBookUpdateMessage, WsTradeMessage, WsTickerMessage, WsOrderUpdateMessage, WsPongMessage, WsErrorMessage, WsServerMessage, WsMessage, } from './api';
export { WS_CHANNELS } from './api';
export { isSide, isOrderType, isTimeInForce, isOrderStatus, isSelfTradeBehavior, isNonEmptyString, isNumericString, isPositiveNumericString, isNonNegativeNumericString, isBookLevel, isMarket, isOrder, isTrade, isOrderBook, isBalance, assertType, assertMarket, assertOrder, assertTrade, assertOrderBook, assertBalance, } from './guards';
export type { ClientConfig, ResolvedConfig } from './client';
export { DEFAULT_BASE_URL, DEFAULT_WS_URL, DEFAULT_TIMEOUT, resolveConfig, validateConfig, ClientError, HttpError, ApiError as ClientApiError, TimeoutError, RateLimitError, WebSocketError, NotFoundError, UnauthorizedError, isClientError, isHttpError, isApiError, isRateLimitError, MatchbookClient, MatchbookWsClient, } from './client';
export type { BookCallback, TradeCallback, OrderCallback, ErrorCallback } from './client';
//# sourceMappingURL=index.d.ts.map
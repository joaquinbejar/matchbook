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

// Core types
export type {
  Price,
  Quantity,
  Side,
  OrderType,
  TimeInForce,
  OrderStatus,
  SelfTradeBehavior,
} from './types';

export {
  SIDES,
  ORDER_TYPES,
  TIME_IN_FORCE_VALUES,
  ORDER_STATUSES,
  SELF_TRADE_BEHAVIORS,
} from './types';

// Entity types
export type {
  Market,
  MarketSummary,
  Order,
  PlaceOrderParams,
  CancelOrderParams,
  Trade,
  TradeFilter,
  BookLevel,
  OrderBook,
  BookChange,
  OrderBookUpdate,
  Balance,
  DepositParams,
  WithdrawParams,
} from './types';

// API types
export type {
  ApiResponse,
  PaginatedResponse,
  ApiError,
  GetMarketsResponse,
  GetMarketResponse,
  GetMarketSummaryResponse,
  GetOrderBookResponse,
  GetTradesResponse,
  GetOrdersResponse,
  GetOrderResponse,
  GetBalancesResponse,
  GetBalanceResponse,
  PlaceOrderResponse,
  CancelOrderResponse,
  BuildTransactionResponse,
} from './api';

// WebSocket types
export type {
  WsChannel,
  WsMessageBase,
  WsSubscribeMessage,
  WsUnsubscribeMessage,
  WsPingMessage,
  WsClientMessage,
  WsSubscribedMessage,
  WsUnsubscribedMessage,
  WsBookSnapshotMessage,
  WsBookUpdateMessage,
  WsTradeMessage,
  WsTickerMessage,
  WsOrderUpdateMessage,
  WsPongMessage,
  WsErrorMessage,
  WsServerMessage,
  WsMessage,
} from './api';

export { WS_CHANNELS } from './api';

// Type guards and validators
export {
  isSide,
  isOrderType,
  isTimeInForce,
  isOrderStatus,
  isSelfTradeBehavior,
  isNonEmptyString,
  isNumericString,
  isPositiveNumericString,
  isNonNegativeNumericString,
  isBookLevel,
  isMarket,
  isOrder,
  isTrade,
  isOrderBook,
  isBalance,
  assertType,
  assertMarket,
  assertOrder,
  assertTrade,
  assertOrderBook,
  assertBalance,
} from './guards';

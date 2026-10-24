/**
 * API types for Matchbook SDK.
 *
 * @module api
 */

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
} from './rest.js';

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
} from './websocket.js';

export { WS_CHANNELS } from './websocket.js';

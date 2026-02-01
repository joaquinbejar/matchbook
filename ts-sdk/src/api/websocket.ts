/**
 * WebSocket API message types.
 *
 * @module api/websocket
 */

import type { Price, Quantity, Side, OrderStatus } from '../types';
import type { BookLevel, BookChange } from '../types';

/**
 * WebSocket channel types.
 */
export type WsChannel = 'book' | 'trades' | 'orders' | 'ticker';

/**
 * All valid WebSocket channels.
 */
export const WS_CHANNELS: readonly WsChannel[] = ['book', 'trades', 'orders', 'ticker'] as const;

/**
 * Base WebSocket message with type discriminator.
 */
export interface WsMessageBase {
  /** Message type. */
  type: string;
}

/**
 * Subscribe message (client to server).
 */
export interface WsSubscribeMessage extends WsMessageBase {
  type: 'subscribe';
  /** Channel to subscribe to. */
  channel: WsChannel;
  /** Market address (required for book, trades, ticker). */
  market?: string;
  /** Depth for book channel. */
  depth?: number;
}

/**
 * Unsubscribe message (client to server).
 */
export interface WsUnsubscribeMessage extends WsMessageBase {
  type: 'unsubscribe';
  /** Channel to unsubscribe from. */
  channel: WsChannel;
  /** Market address. */
  market?: string;
}

/**
 * Ping message (client to server).
 */
export interface WsPingMessage extends WsMessageBase {
  type: 'ping';
  /** Timestamp in milliseconds. */
  timestamp: number;
}

/**
 * Client-to-server message union type.
 */
export type WsClientMessage = WsSubscribeMessage | WsUnsubscribeMessage | WsPingMessage;

/**
 * Subscribed confirmation (server to client).
 */
export interface WsSubscribedMessage extends WsMessageBase {
  type: 'subscribed';
  /** Channel subscribed to. */
  channel: WsChannel;
  /** Market address. */
  market?: string;
}

/**
 * Unsubscribed confirmation (server to client).
 */
export interface WsUnsubscribedMessage extends WsMessageBase {
  type: 'unsubscribed';
  /** Channel unsubscribed from. */
  channel: WsChannel;
  /** Market address. */
  market?: string;
}

/**
 * Book snapshot message (server to client).
 */
export interface WsBookSnapshotMessage extends WsMessageBase {
  type: 'book_snapshot';
  /** Market address. */
  market: string;
  /** Slot number. */
  slot: number;
  /** Sequence number. */
  sequence: number;
  /** Bid levels. */
  bids: BookLevel[];
  /** Ask levels. */
  asks: BookLevel[];
}

/**
 * Book update message (server to client).
 */
export interface WsBookUpdateMessage extends WsMessageBase {
  type: 'book_update';
  /** Market address. */
  market: string;
  /** Slot number. */
  slot: number;
  /** Sequence number. */
  sequence: number;
  /** Bid changes. */
  bids: BookChange[];
  /** Ask changes. */
  asks: BookChange[];
}

/**
 * Trade message (server to client).
 */
export interface WsTradeMessage extends WsMessageBase {
  type: 'trade';
  /** Market address. */
  market: string;
  /** Trade ID. */
  id: string;
  /** Trade price. */
  price: Price;
  /** Trade quantity. */
  quantity: Quantity;
  /** Taker side. */
  side: Side;
  /** Timestamp (ISO 8601). */
  timestamp: string;
}

/**
 * Ticker message (server to client).
 */
export interface WsTickerMessage extends WsMessageBase {
  type: 'ticker';
  /** Market address. */
  market: string;
  /** Best bid price. */
  bestBid?: Price;
  /** Best ask price. */
  bestAsk?: Price;
  /** Last trade price. */
  lastPrice?: Price;
  /** 24h volume. */
  volume24h?: string;
  /** 24h price change percentage. */
  priceChange24h?: string;
  /** Timestamp (ISO 8601). */
  timestamp: string;
}

/**
 * Order update message (server to client, authenticated).
 */
export interface WsOrderUpdateMessage extends WsMessageBase {
  type: 'order_update';
  /** Market address. */
  market: string;
  /** Order ID. */
  orderId: string;
  /** Client order ID. */
  clientOrderId?: number;
  /** Order status. */
  status: OrderStatus;
  /** Filled quantity. */
  filledQuantity: Quantity;
  /** Remaining quantity. */
  remainingQuantity: Quantity;
  /** Average fill price. */
  averagePrice?: Price;
  /** Timestamp (ISO 8601). */
  timestamp: string;
}

/**
 * Pong message (server to client).
 */
export interface WsPongMessage extends WsMessageBase {
  type: 'pong';
  /** Timestamp in milliseconds. */
  timestamp: number;
}

/**
 * Error message (server to client).
 */
export interface WsErrorMessage extends WsMessageBase {
  type: 'error';
  /** Error code. */
  code: string;
  /** Error message. */
  message: string;
  /** Request ID if applicable. */
  requestId?: string;
}

/**
 * Server-to-client message union type.
 */
export type WsServerMessage =
  | WsSubscribedMessage
  | WsUnsubscribedMessage
  | WsBookSnapshotMessage
  | WsBookUpdateMessage
  | WsTradeMessage
  | WsTickerMessage
  | WsOrderUpdateMessage
  | WsPongMessage
  | WsErrorMessage;

/**
 * All WebSocket message types.
 */
export type WsMessage = WsClientMessage | WsServerMessage;

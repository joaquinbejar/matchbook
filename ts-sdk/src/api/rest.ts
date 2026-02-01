/**
 * REST API response types.
 *
 * @module api/rest
 */

import type { Market, Order, Trade, OrderBook, Balance, MarketSummary } from '../types';

/**
 * Generic API response wrapper.
 *
 * @template T - The data type contained in the response.
 */
export interface ApiResponse<T> {
  /** Response data. */
  data: T;

  /** Request timestamp (ISO 8601). */
  timestamp?: string;
}

/**
 * Paginated API response.
 *
 * @template T - The data type contained in the response.
 */
export interface PaginatedResponse<T> {
  /** Response data array. */
  data: T[];

  /** Pagination info. */
  pagination: {
    /** Total number of items. */
    total?: number;

    /** Number of items per page. */
    limit: number;

    /** Current offset. */
    offset: number;

    /** Whether there are more items. */
    hasMore: boolean;
  };

  /** Request timestamp (ISO 8601). */
  timestamp?: string;
}

/**
 * API error response.
 */
export interface ApiError {
  /** Error code. */
  code: string;

  /** Human-readable error message. */
  message: string;

  /** Additional error details. */
  details?: Record<string, unknown>;

  /** Request ID for debugging. */
  requestId?: string;
}

/**
 * GET /markets response.
 */
export type GetMarketsResponse = ApiResponse<Market[]>;

/**
 * GET /markets/:address response.
 */
export type GetMarketResponse = ApiResponse<Market>;

/**
 * GET /markets/:address/summary response.
 */
export type GetMarketSummaryResponse = ApiResponse<MarketSummary>;

/**
 * GET /markets/:address/orderbook response.
 */
export type GetOrderBookResponse = ApiResponse<OrderBook>;

/**
 * GET /markets/:address/trades response.
 */
export type GetTradesResponse = PaginatedResponse<Trade>;

/**
 * GET /orders response.
 */
export type GetOrdersResponse = PaginatedResponse<Order>;

/**
 * GET /orders/:id response.
 */
export type GetOrderResponse = ApiResponse<Order>;

/**
 * GET /balances response.
 */
export type GetBalancesResponse = ApiResponse<Balance[]>;

/**
 * GET /balances/:market response.
 */
export type GetBalanceResponse = ApiResponse<Balance>;

/**
 * POST /orders response (place order).
 */
export interface PlaceOrderResponse {
  /** Created order. */
  order: Order;

  /** Transaction signature. */
  signature?: string;
}

/**
 * DELETE /orders/:id response (cancel order).
 */
export interface CancelOrderResponse {
  /** Cancelled order ID. */
  orderId: string;

  /** Transaction signature. */
  signature?: string;

  /** Whether the cancellation was successful. */
  success: boolean;
}

/**
 * POST /tx/build response (build transaction).
 */
export interface BuildTransactionResponse {
  /** Serialized transaction (base64). */
  transaction: string;

  /** Recent blockhash used. */
  blockhash: string;

  /** Last valid block height. */
  lastValidBlockHeight: number;
}

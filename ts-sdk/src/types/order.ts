/**
 * Order type definitions.
 *
 * @module types/order
 */

import type { Price, Quantity, Side, OrderType, OrderStatus, TimeInForce, SelfTradeBehavior } from './primitives';

/**
 * Order details.
 *
 * Represents a single order with all its properties.
 *
 * @example
 * ```typescript
 * const order: Order = {
 *   id: "340282366920938463463374607431768211455",
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   owner: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
 *   side: "bid",
 *   price: "105.50",
 *   quantity: "100.0",
 *   filledQuantity: "25.0",
 *   orderType: "limit",
 *   status: "partiallyFilled",
 *   createdAt: "2026-01-30T12:00:00.000Z",
 * };
 * ```
 */
export interface Order {
  /** Unique order ID. */
  id: string;

  /** Market address. */
  market: string;

  /** Owner wallet address. */
  owner: string;

  /** Order side (bid/ask). */
  side: Side;

  /** Limit price. */
  price: Price;

  /** Original order quantity. */
  quantity: Quantity;

  /** Filled quantity. */
  filledQuantity: Quantity;

  /** Order type. */
  orderType: OrderType;

  /** Current order status. */
  status: OrderStatus;

  /** Client-provided order ID. */
  clientOrderId?: number;

  /** Time in force. */
  timeInForce?: TimeInForce;

  /** Self-trade behavior. */
  selfTradeBehavior?: SelfTradeBehavior;

  /** Average fill price. */
  averagePrice?: Price;

  /** Order creation timestamp (ISO 8601). */
  createdAt: string;

  /** Last update timestamp (ISO 8601). */
  updatedAt?: string;

  /** Expiry timestamp (ISO 8601). */
  expiresAt?: string;
}

/**
 * Parameters for placing a new order.
 *
 * @example
 * ```typescript
 * const params: PlaceOrderParams = {
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   side: "bid",
 *   price: "105.50",
 *   quantity: "100.0",
 *   orderType: "limit",
 * };
 * ```
 */
export interface PlaceOrderParams {
  /** Market address. */
  market: string;

  /** Order side. */
  side: Side;

  /** Limit price. */
  price: Price;

  /** Order quantity. */
  quantity: Quantity;

  /** Order type. */
  orderType: OrderType;

  /** Optional client order ID. */
  clientOrderId?: number;

  /** Time in force (defaults to GTC). */
  timeInForce?: TimeInForce;

  /** Self-trade behavior. */
  selfTradeBehavior?: SelfTradeBehavior;

  /** Expiry timestamp (ISO 8601). */
  expiresAt?: string;
}

/**
 * Parameters for cancelling an order.
 *
 * @example
 * ```typescript
 * const params: CancelOrderParams = {
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   orderId: "340282366920938463463374607431768211455",
 * };
 * ```
 */
export interface CancelOrderParams {
  /** Market address. */
  market: string;

  /** Order ID to cancel. */
  orderId: string;

  /** Alternative: client order ID. */
  clientOrderId?: number;
}

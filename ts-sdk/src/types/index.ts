/**
 * Type definitions for Matchbook SDK.
 *
 * @module types
 */

export type {
  Price,
  Quantity,
  Side,
  OrderType,
  TimeInForce,
  OrderStatus,
  SelfTradeBehavior,
} from './primitives.js';

export {
  SIDES,
  ORDER_TYPES,
  TIME_IN_FORCE_VALUES,
  ORDER_STATUSES,
  SELF_TRADE_BEHAVIORS,
} from './primitives.js';

export type { Market, MarketSummary } from './market.js';

export type { Order, PlaceOrderParams, CancelOrderParams } from './order.js';

export type { Trade, TradeFilter } from './trade.js';

export type { BookLevel, OrderBook, BookChange, OrderBookUpdate } from './book.js';

export type { Balance, DepositParams, WithdrawParams } from './balance.js';

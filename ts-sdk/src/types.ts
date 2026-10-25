/** Order side. */
export enum Side {
  /** Buy base with quote. */
  Bid = 0,
  /** Sell base for quote. */
  Ask = 1,
}

/** How an order interacts with the book. */
export enum OrderType {
  /** Match what crosses, rest the remainder. */
  Limit = 0,
  /** Rest only; rejected if it would cross. */
  PostOnly = 1,
  /** Rest only; if it would cross, reprice one tick behind the best opposite order. */
  PostOnlySlide = 2,
  /** Match what crosses, drop the remainder. */
  ImmediateOrCancel = 3,
  /** Fill completely or fail. */
  FillOrKill = 4,
}

/** What happens when an order would match an order of the same seat. */
export enum SelfTradeBehavior {
  /** Cancel the resting order and keep matching. */
  CancelProvide = 0,
  /** Reduce both orders by the overlap without trading. */
  DecrementTake = 1,
  /** Fail the whole instruction. */
  Abort = 2,
}

/** When a stop triggers. */
export enum TriggerDirection {
  /** When a trade happens at or above the trigger price. */
  Above = 0,
  /** When a trade happens at or below the trigger price. */
  Below = 1,
}

/** Market status. */
export enum MarketStatus {
  /** Everything allowed. */
  Active = 0,
  /** Only cancels, withdrawals and fee collection. */
  CancelOnly = 1,
  /** Only withdrawals. */
  Closed = 2,
}

/** Which orders `cancelAll` cancels. */
export enum CancelSide {
  Bids = 0,
  Asks = 1,
  Both = 2,
}

/** Why an order or stop left the book. */
export enum OutReason {
  Filled = 0,
  Cancelled = 1,
  Expired = 2,
  SelfTrade = 3,
  /** A stop whose trigger was met; its order follows. */
  Triggered = 4,
  /** A stop cancelled because its one-cancels-other partner triggered. */
  OcoCancelled = 5,
}

/** Permission bits a seat owner grants a delegate. */
export const Permission = {
  /** Place, reduce and cancel orders, and batch updates. */
  TRADE: 1n,
  /** Cancel and reduce orders only. */
  CANCEL: 2n,
  /** Every defined bit. */
  ALL: 3n,
} as const;

/** An order to place. Prices are in quote lots per base lot. */
export interface OrderArgs {
  side: Side;
  /** Limit price in quote lots per base lot. */
  price: bigint;
  /** Size in base lots. */
  baseLots: bigint;
  /** Default {@link OrderType.Limit}. */
  orderType?: OrderType;
  /** Default {@link SelfTradeBehavior.CancelProvide}. */
  selfTrade?: SelfTradeBehavior;
  /** Echoed in events and usable for cancels. Default 0. */
  clientOrderId?: bigint;
  /** Unix time at which a resting order dies. Default 0 (never). */
  expiresAt?: bigint;
  /** For immediate-or-cancel bids: maximum quote atoms to spend, fees included. */
  quoteBudget?: bigint;
  /** Maximum resting orders to touch. Default 64. */
  matchLimit?: number;
  /**
   * Makes the resting part a reverse order: each fill re-posts on the other
   * side this many parts per million away from the fill price.
   */
  reverseSpread?: number;
}

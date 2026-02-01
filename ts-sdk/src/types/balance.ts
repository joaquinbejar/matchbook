/**
 * Balance type definitions.
 *
 * @module types/balance
 */

import type { Quantity } from './primitives';

/**
 * User balance for a specific market.
 *
 * Represents the user's token balances including available,
 * locked (in orders), and unsettled amounts.
 *
 * @example
 * ```typescript
 * const balance: Balance = {
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   owner: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
 *   baseAvailable: "100.0",
 *   baseLocked: "25.0",
 *   quoteAvailable: "5000.0",
 *   quoteLocked: "1000.0",
 * };
 * ```
 */
export interface Balance {
  /** Market address. */
  market: string;

  /** Owner wallet address. */
  owner: string;

  /** Available base token balance. */
  baseAvailable: Quantity;

  /** Base tokens locked in open orders. */
  baseLocked: Quantity;

  /** Unsettled base tokens (from fills). */
  baseUnsettled?: Quantity;

  /** Available quote token balance. */
  quoteAvailable: Quantity;

  /** Quote tokens locked in open orders. */
  quoteLocked: Quantity;

  /** Unsettled quote tokens (from fills). */
  quoteUnsettled?: Quantity;

  /** Accumulated maker rebates in base. */
  baseRebates?: Quantity;

  /** Accumulated maker rebates in quote. */
  quoteRebates?: Quantity;

  /** Last update timestamp (ISO 8601). */
  updatedAt?: string;
}

/**
 * Deposit parameters.
 *
 * @example
 * ```typescript
 * const params: DepositParams = {
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   baseAmount: "100.0",
 *   quoteAmount: "5000.0",
 * };
 * ```
 */
export interface DepositParams {
  /** Market address. */
  market: string;

  /** Amount of base tokens to deposit. */
  baseAmount?: Quantity;

  /** Amount of quote tokens to deposit. */
  quoteAmount?: Quantity;
}

/**
 * Withdraw parameters.
 *
 * @example
 * ```typescript
 * const params: WithdrawParams = {
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   baseAmount: "50.0",
 * };
 * ```
 */
export interface WithdrawParams {
  /** Market address. */
  market: string;

  /** Amount of base tokens to withdraw. */
  baseAmount?: Quantity;

  /** Amount of quote tokens to withdraw. */
  quoteAmount?: Quantity;
}

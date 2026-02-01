/**
 * Market type definitions.
 *
 * @module types/market
 */

import type { Price, Quantity } from './primitives';

/**
 * Market configuration and state.
 *
 * Represents a trading market with its configuration parameters
 * and current state information.
 *
 * @example
 * ```typescript
 * const market: Market = {
 *   address: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   baseMint: "So11111111111111111111111111111111111111112",
 *   quoteMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
 *   baseSymbol: "SOL",
 *   quoteSymbol: "USDC",
 *   tickSize: "0.01",
 *   lotSize: "0.001",
 *   makerFee: -0.0002,
 *   takerFee: 0.0005,
 * };
 * ```
 */
export interface Market {
  /** Market account address (base58 encoded). */
  address: string;

  /** Base token mint address. */
  baseMint: string;

  /** Quote token mint address. */
  quoteMint: string;

  /** Base token symbol (e.g., "SOL"). */
  baseSymbol?: string;

  /** Quote token symbol (e.g., "USDC"). */
  quoteSymbol?: string;

  /** Minimum price increment. */
  tickSize: Price;

  /** Minimum quantity increment. */
  lotSize: Quantity;

  /** Maker fee rate (negative for rebate). */
  makerFee: number;

  /** Taker fee rate. */
  takerFee: number;

  /** Base token decimals. */
  baseDecimals?: number;

  /** Quote token decimals. */
  quoteDecimals?: number;

  /** Whether the market is active. */
  active?: boolean;

  /** Market creation timestamp (ISO 8601). */
  createdAt?: string;
}

/**
 * Market summary with 24h statistics.
 *
 * @example
 * ```typescript
 * const summary: MarketSummary = {
 *   market: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
 *   lastPrice: "105.50",
 *   volume24h: "1234567.89",
 *   priceChange24h: "2.34",
 *   high24h: "108.00",
 *   low24h: "102.00",
 * };
 * ```
 */
export interface MarketSummary {
  /** Market address. */
  market: string;

  /** Last trade price. */
  lastPrice?: Price;

  /** 24-hour trading volume in quote currency. */
  volume24h?: string;

  /** 24-hour price change percentage. */
  priceChange24h?: string;

  /** 24-hour high price. */
  high24h?: Price;

  /** 24-hour low price. */
  low24h?: Price;

  /** Best bid price. */
  bestBid?: Price;

  /** Best ask price. */
  bestAsk?: Price;

  /** Timestamp of the summary (ISO 8601). */
  timestamp?: string;
}

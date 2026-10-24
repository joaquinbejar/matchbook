/**
 * TypeScript SDK for MatchBook, a crankless central limit order book on
 * Solana. Built on `@solana/kit`.
 *
 * - {@link placeOrder}, {@link batchUpdate}, {@link swap} and the other
 *   builders return kit `Instruction`s.
 * - {@link decodeMarket} turns a market account into a {@link MarketView}:
 *   the book, seats, stops and an exact swap quote.
 * - {@link parseEventsFromLogs} decodes fills and other events.
 */
export * from './constants.js';
export * from './errors.js';
export * from './events.js';
export * from './instructions.js';
export * from './market.js';
export * from './pda.js';
export * from './types.js';

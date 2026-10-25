import { address } from '@solana/kit';

/** The MatchBook program. */
export const MATCHBOOK_PROGRAM_ADDRESS = address('MbKAoWL85AKTWWBvf5FbLJZFKQuBcH1DaWQdfnkJxTw');

/** SPL Token. */
export const TOKEN_PROGRAM_ADDRESS = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
/** Token-2022. */
export const TOKEN_2022_PROGRAM_ADDRESS = address('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb');
/** Associated Token Account program. */
export const ASSOCIATED_TOKEN_PROGRAM_ADDRESS = address(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL',
);
/** System program. */
export const SYSTEM_PROGRAM_ADDRESS = address('11111111111111111111111111111111');

/** Seed of the per-market PDA that owns both vaults. */
export const VAULT_AUTHORITY_SEED = 'vault_authority';

/** Size of the market header in bytes. */
export const HEADER_SIZE = 400;
/** Size of every node in the market account. */
export const NODE_SIZE = 88;
/** Null node index. */
export const NIL = 0xffffffff;
/** Low bits of an order id that hold the order's node index. */
export const ORDER_INDEX_BITS = 24n;
/** "MTCHBOOK", little-endian. */
export const MARKET_MAGIC = 0x4b4f4f424843544dn;

/** Largest number of cancels, and of orders, in one batch update. */
export const MAX_BATCH = 32;
/** Match limit used when an order or swap passes 0. */
export const DEFAULT_MATCH_LIMIT = 64;
/** Highest match limit of an order placed by a stop. */
export const MAX_STOP_MATCH_LIMIT = 16;
/** Stops placed automatically at the end of each trade, at most. */
export const AUTO_TRIGGER_LIMIT = 2;
/** Parts per million: the unit of reverse spreads. */
export const REVERSE_SPREAD_SCALE = 1_000_000;
/** Delegates a seat may have. */
export const MAX_DELEGATES = 8;
/** Market flag: approved seats only, no seatless swaps. */
export const MARKET_FLAG_PERMISSIONED = 1;

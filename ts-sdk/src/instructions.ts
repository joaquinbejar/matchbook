import {
  AccountRole,
  isTransactionSigner,
  type AccountMeta,
  type Address,
  type Instruction,
  type TransactionSigner,
} from '@solana/kit';

import { Writer } from './codec.js';
import {
  ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
  MATCHBOOK_PROGRAM_ADDRESS,
  MAX_BATCH,
  NIL,
  SYSTEM_PROGRAM_ADDRESS,
  TOKEN_PROGRAM_ADDRESS,
} from './constants.js';
import { findVault, findVaultAuthority } from './pda.js';
import {
  CancelSide,
  MarketStatus,
  OrderType,
  SelfTradeBehavior,
  Side,
  TriggerDirection,
  type OrderArgs,
} from './types.js';

/** Instruction discriminators. */
export enum Ix {
  CreateMarket = 0,
  ClaimSeat = 1,
  Deposit = 2,
  Withdraw = 3,
  PlaceOrder = 4,
  CancelOrder = 5,
  CancelAll = 6,
  CancelByClientId = 7,
  ReduceOrder = 8,
  BatchUpdate = 9,
  Swap = 10,
  CollectFees = 11,
  SetStatus = 12,
  PruneExpired = 13,
  SetDelegate = 14,
  RemoveDelegate = 15,
  PlaceStop = 16,
  TriggerStops = 17,
}

/** An account given either as an address or as a signer that will sign. */
export type AddressOrSigner = Address | TransactionSigner;

/** Size of an encoded order. */
export const ORDER_ARGS_LEN = 56;

const HEADER_LEN = 8;

function addressOf(a: AddressOrSigner): Address {
  return typeof a === 'string' ? a : a.address;
}

function signer(a: AddressOrSigner, writable: boolean): AccountMeta {
  const role = writable ? AccountRole.WRITABLE_SIGNER : AccountRole.READONLY_SIGNER;
  if (typeof a !== 'string' && isTransactionSigner(a)) {
    return { address: a.address, role, signer: a } as AccountMeta;
  }
  return { address: addressOf(a), role };
}

const writable = (a: Address): AccountMeta => ({ address: a, role: AccountRole.WRITABLE });
const readonly = (a: Address): AccountMeta => ({ address: a, role: AccountRole.READONLY });

function header(w: Writer, ix: Ix, a = 0, b = 0, c = 0, hint = NIL): Writer {
  return w.u8(ix).u8(a).u8(b).u8(c).u32(hint);
}

function build(
  accounts: AccountMeta[],
  data: Uint8Array,
  programAddress: Address = MATCHBOOK_PROGRAM_ADDRESS,
): Instruction {
  return { programAddress, accounts, data };
}

/** Writes an order in its 56-byte wire format. */
export function writeOrder(w: Writer, o: OrderArgs, referrerSeat = NIL): Writer {
  return w
    .u8(o.side)
    .u8(o.orderType ?? OrderType.Limit)
    .u8(o.selfTrade ?? SelfTradeBehavior.CancelProvide)
    .u8(0)
    .u32(o.matchLimit ?? 0)
    .u64(o.price)
    .u64(o.baseLots)
    .u64(o.clientOrderId ?? 0n)
    .u64(o.expiresAt ?? 0n)
    .u64(o.quoteBudget ?? 0n)
    .u32(o.reverseSpread ?? 0)
    .u32(referrerSeat);
}

/** Common inputs of instructions a seat (or its delegate) signs. */
export interface SeatInput {
  /** The seat's owner, or a delegate acting for it. */
  owner: AddressOrSigner;
  market: Address;
  /**
   * The seat's node index, from {@link MarketView.findSeat}. Any value works
   * (the program then searches), but the right one is cheapest.
   */
  seat?: number;
  programAddress?: Address;
}

/** A frontend or aggregator that earns a share of the taker fee. */
export interface Referrer {
  /** Owner of the referrer's seat on this market. */
  owner: Address;
  /** Its seat's node index (a hint). */
  seat?: number;
}

/** Creates a market. The market account must sign: it is created here. */
export async function createMarket(input: {
  payer: AddressOrSigner;
  market: AddressOrSigner;
  baseMint: Address;
  quoteMint: Address;
  baseTokenProgram?: Address;
  quoteTokenProgram?: Address;
  /** Collects fees and sets the status. */
  authority: Address;
  baseLotSize: bigint;
  quoteLotSize: bigint;
  minBaseLots?: bigint;
  takerFeeBps: number;
  makerRebateBps: number;
  /** Nodes to preallocate (each holds one order, seat, stop or delegate). */
  initialNodes?: number;
  /** Share of the taker fee paid to referrers, in basis points of the fee. */
  referralShareBps?: number;
  /** {@link MARKET_FLAG_PERMISSIONED} and future flags. */
  flags?: number;
  programAddress?: Address;
}): Promise<Instruction> {
  const programAddress = input.programAddress ?? MATCHBOOK_PROGRAM_ADDRESS;
  const market = addressOf(input.market);
  const baseTp = input.baseTokenProgram ?? TOKEN_PROGRAM_ADDRESS;
  const quoteTp = input.quoteTokenProgram ?? TOKEN_PROGRAM_ADDRESS;
  const w = header(new Writer(HEADER_LEN + 72), Ix.CreateMarket, 0, 0, 0, 0)
    .u64(input.baseLotSize)
    .u64(input.quoteLotSize)
    .u64(input.minBaseLots ?? 1n)
    .u32(input.takerFeeBps)
    .u32(input.makerRebateBps)
    .u32(input.initialNodes ?? 0)
    .u16(input.referralShareBps ?? 0)
    .u16(input.flags ?? 0)
    .address(input.authority);
  return build(
    [
      signer(input.payer, true),
      signer(input.market, true),
      readonly(input.baseMint),
      readonly(input.quoteMint),
      writable(await findVault(market, input.baseMint, baseTp, programAddress)),
      writable(await findVault(market, input.quoteMint, quoteTp, programAddress)),
      readonly(await findVaultAuthority(market, programAddress)),
      readonly(baseTp),
      readonly(quoteTp),
      readonly(ASSOCIATED_TOKEN_PROGRAM_ADDRESS),
      readonly(SYSTEM_PROGRAM_ADDRESS),
    ],
    w.bytes,
    programAddress,
  );
}

/**
 * Opens a seat (trading account) for `owner`. On a permissioned market the
 * market authority must sign too.
 */
export function claimSeat(input: {
  payer: AddressOrSigner;
  owner: AddressOrSigner;
  market: Address;
  authority?: AddressOrSigner;
  programAddress?: Address;
}): Instruction {
  const accounts = [
    signer(input.payer, true),
    signer(input.owner, false),
    writable(input.market),
    readonly(SYSTEM_PROGRAM_ADDRESS),
  ];
  if (input.authority) accounts.push(signer(input.authority, false));
  return build(accounts, header(new Writer(HEADER_LEN), Ix.ClaimSeat, 0, 0, 0, 0).bytes, input.programAddress);
}

/** Token accounts for one side of a deposit or withdrawal. */
export interface TokenSide {
  /** The user's token account. */
  userToken: Address;
  /** The market's vault for this mint. */
  vault: Address;
  mint: Address;
  tokenProgram?: Address;
}

/** Moves tokens from the owner's account into its seat. */
export function deposit(input: SeatInput & TokenSide & { amount: bigint }): Instruction {
  const w = header(new Writer(HEADER_LEN + 8), Ix.Deposit, 0, 0, 0, input.seat ?? NIL).u64(input.amount);
  return build(
    [
      signer(input.owner, false),
      writable(input.market),
      writable(input.userToken),
      writable(input.vault),
      readonly(input.mint),
      readonly(input.tokenProgram ?? TOKEN_PROGRAM_ADDRESS),
    ],
    w.bytes,
    input.programAddress,
  );
}

/** Moves free tokens from the seat to the owner's account. Owner only. */
export function withdraw(
  input: SeatInput & TokenSide & { vaultAuthority: Address; amount: bigint },
): Instruction {
  const w = header(new Writer(HEADER_LEN + 8), Ix.Withdraw, 0, 0, 0, input.seat ?? NIL).u64(input.amount);
  return build(
    [
      signer(input.owner, false),
      writable(input.market),
      writable(input.userToken),
      writable(input.vault),
      readonly(input.mint),
      readonly(input.vaultAuthority),
      readonly(input.tokenProgram ?? TOKEN_PROGRAM_ADDRESS),
    ],
    w.bytes,
    input.programAddress,
  );
}

/**
 * Places an order. The new order's id comes back as return data (0 if
 * nothing rested). The owner pays for account growth, so it must be
 * writable.
 */
export function placeOrder(input: SeatInput & { order: OrderArgs; referrer?: Referrer }): Instruction {
  const w = writeOrder(
    header(new Writer(HEADER_LEN + ORDER_ARGS_LEN), Ix.PlaceOrder, 0, 0, 0, input.seat ?? NIL),
    input.order,
    input.referrer?.seat ?? NIL,
  );
  const accounts = [signer(input.owner, true), writable(input.market), readonly(SYSTEM_PROGRAM_ADDRESS)];
  if (input.referrer) accounts.push(readonly(input.referrer.owner));
  return build(accounts, w.bytes, input.programAddress);
}

function cancelAccounts(input: SeatInput): AccountMeta[] {
  return [signer(input.owner, false), writable(input.market)];
}

/** Cancels an order or a stop by id. */
export function cancelOrder(input: SeatInput & { orderId: bigint }): Instruction {
  const w = header(new Writer(HEADER_LEN + 8), Ix.CancelOrder, 0, 0, 0, input.seat ?? NIL).u64(input.orderId);
  return build(cancelAccounts(input), w.bytes, input.programAddress);
}

/** Cancels every order and stop of the seat, optionally on one side. */
export function cancelAll(input: SeatInput & { side?: CancelSide }): Instruction {
  const w = header(new Writer(HEADER_LEN), Ix.CancelAll, input.side ?? CancelSide.Both, 0, 0, input.seat ?? NIL);
  return build(cancelAccounts(input), w.bytes, input.programAddress);
}

/** Cancels every order and stop of the seat with this client order id. */
export function cancelByClientId(input: SeatInput & { clientOrderId: bigint }): Instruction {
  const w = header(new Writer(HEADER_LEN + 8), Ix.CancelByClientId, 0, 0, 0, input.seat ?? NIL).u64(
    input.clientOrderId,
  );
  return build(cancelAccounts(input), w.bytes, input.programAddress);
}

/** Shrinks an order in place, keeping its place in the queue (0 cancels it). */
export function reduceOrder(input: SeatInput & { orderId: bigint; newBaseLots: bigint }): Instruction {
  const w = header(new Writer(HEADER_LEN + 16), Ix.ReduceOrder, 0, 0, 0, input.seat ?? NIL)
    .u64(input.orderId)
    .u64(input.newBaseLots);
  return build(cancelAccounts(input), w.bytes, input.programAddress);
}

/**
 * Cancels and places orders atomically, the market maker's requote. A
 * cancel of an order that is already gone is skipped, not an error. Return
 * data holds one u64 id per order.
 */
export function batchUpdate(
  input: SeatInput & {
    /** Cancel all bids, asks or both first. */
    cancelAll?: CancelSide;
    cancels?: bigint[];
    orders?: OrderArgs[];
  },
): Instruction {
  const cancels = input.cancels ?? [];
  const orders = input.orders ?? [];
  if (cancels.length > MAX_BATCH || orders.length > MAX_BATCH) {
    throw new Error(`at most ${MAX_BATCH} cancels and ${MAX_BATCH} orders per batch`);
  }
  // On the wire 0 means no cancel-all, then bids, asks, both.
  const all = input.cancelAll === undefined ? 0 : input.cancelAll + 1;
  const w = header(
    new Writer(HEADER_LEN + cancels.length * 8 + orders.length * ORDER_ARGS_LEN),
    Ix.BatchUpdate,
    all,
    cancels.length,
    orders.length,
    input.seat ?? NIL,
  );
  for (const c of cancels) w.u64(c);
  for (const o of orders) writeOrder(w, o);
  return build(
    [signer(input.owner, true), writable(input.market), readonly(SYSTEM_PROGRAM_ADDRESS)],
    w.bytes,
    input.programAddress,
  );
}

/** Every account of a market that token-moving instructions need. */
export interface MarketAccounts {
  market: Address;
  baseMint: Address;
  quoteMint: Address;
  baseVault: Address;
  quoteVault: Address;
  vaultAuthority: Address;
  baseTokenProgram: Address;
  quoteTokenProgram: Address;
}

/**
 * Trades from a wallet without a seat, as an aggregator would. The input
 * leaves the user's token account and the output arrives there in the same
 * instruction. Disabled on permissioned markets.
 */
export function swap(input: {
  user: AddressOrSigner;
  accounts: MarketAccounts;
  userBase: Address;
  userQuote: Address;
  /** Bid buys base, ask sells it. */
  side: Side;
  /** Ask: base lots to sell. Bid: maximum base lots to buy. */
  baseLots: bigint;
  /** Bid: maximum quote atoms to spend, fees included. */
  quoteBudget?: bigint;
  /** Worst acceptable price in quote lots per base lot. */
  limitPrice: bigint;
  /** Minimum atoms received (base for bids, quote for asks). */
  minOut?: bigint;
  matchLimit?: number;
  referrer?: Referrer;
  programAddress?: Address;
}): Instruction {
  const a = input.accounts;
  const w = header(new Writer(HEADER_LEN + 40), Ix.Swap, input.side, 0, 0, 0)
    .u64(input.baseLots)
    .u64(input.quoteBudget ?? 0n)
    .u64(input.limitPrice)
    .u64(input.minOut ?? 0n)
    .u32(input.matchLimit ?? 0)
    .u32(input.referrer?.seat ?? NIL);
  const accounts = [
    signer(input.user, false),
    writable(a.market),
    writable(input.userBase),
    writable(input.userQuote),
    writable(a.baseVault),
    writable(a.quoteVault),
    readonly(a.baseMint),
    readonly(a.quoteMint),
    readonly(a.vaultAuthority),
    readonly(a.baseTokenProgram),
    readonly(a.quoteTokenProgram),
  ];
  if (input.referrer) accounts.push(readonly(input.referrer.owner));
  return build(accounts, w.bytes, input.programAddress);
}

/**
 * Lets another key act for the seat: a session key for an app or a bot's
 * hot key. It trades or cancels as `permissions` allow, never withdraws.
 * Owner only. Granting an existing delegate again updates it.
 */
export function setDelegate(
  input: SeatInput & { delegate: Address; permissions: bigint; expiresAt?: bigint },
): Instruction {
  const w = header(new Writer(HEADER_LEN + 48), Ix.SetDelegate, 0, 0, 0, input.seat ?? NIL)
    .address(input.delegate)
    .u64(input.expiresAt ?? 0n)
    .u64(input.permissions);
  return build(
    [signer(input.owner, true), writable(input.market), readonly(SYSTEM_PROGRAM_ADDRESS)],
    w.bytes,
    input.programAddress,
  );
}

/** Revokes a delegate. Signed by the owner or by the delegate itself. */
export function removeDelegate(input: SeatInput & { delegate: Address }): Instruction {
  const w = header(new Writer(HEADER_LEN + 32), Ix.RemoveDelegate, 0, 0, 0, input.seat ?? NIL).address(
    input.delegate,
  );
  return build(cancelAccounts(input), w.bytes, input.programAddress);
}

/**
 * Places a stop: `order` is placed once a trade reaches `triggerPrice`. Its
 * funds are locked meanwhile. Trades set off stops themselves, so no keeper
 * is needed. Pass `ocoWith` (another stop's id) to make a bracket: when one
 * triggers, the other is cancelled. The stop id comes back as return data.
 */
export function placeStop(
  input: SeatInput & {
    triggerPrice: bigint;
    direction: TriggerDirection;
    order: OrderArgs;
    ocoWith?: bigint;
  },
): Instruction {
  const w = writeOrder(
    header(new Writer(HEADER_LEN + 72), Ix.PlaceStop, input.direction, 0, 0, input.seat ?? NIL),
    input.order,
  )
    .u64(input.triggerPrice)
    .u64(input.ocoWith ?? 0n);
  return build(
    [signer(input.owner, true), writable(input.market), readonly(SYSTEM_PROGRAM_ADDRESS)],
    w.bytes,
    input.programAddress,
  );
}

/** Places stops whose trigger has been reached. Anyone may send it. */
export function triggerStops(input: { market: Address; limit: number; programAddress?: Address }): Instruction {
  return build(
    [writable(input.market)],
    header(new Writer(HEADER_LEN), Ix.TriggerStops, 0, 0, 0, input.limit).bytes,
    input.programAddress,
  );
}

/** Removes up to `limit` expired orders from the top of the book. Anyone may send it. */
export function pruneExpired(input: { market: Address; limit: number; programAddress?: Address }): Instruction {
  return build(
    [writable(input.market)],
    header(new Writer(HEADER_LEN), Ix.PruneExpired, 0, 0, 0, input.limit).bytes,
    input.programAddress,
  );
}

/** Sends accrued fees to `destination`. Market authority only. */
export function collectFees(input: {
  authority: AddressOrSigner;
  accounts: MarketAccounts;
  destination: Address;
  programAddress?: Address;
}): Instruction {
  const a = input.accounts;
  return build(
    [
      signer(input.authority, false),
      writable(a.market),
      writable(input.destination),
      writable(a.quoteVault),
      readonly(a.quoteMint),
      readonly(a.vaultAuthority),
      readonly(a.quoteTokenProgram),
    ],
    header(new Writer(HEADER_LEN), Ix.CollectFees, 0, 0, 0, 0).bytes,
    input.programAddress,
  );
}

/** Changes the market status. Market authority only. */
export function setStatus(input: {
  authority: AddressOrSigner;
  market: Address;
  status: MarketStatus;
  programAddress?: Address;
}): Instruction {
  return build(
    [signer(input.authority, false), writable(input.market)],
    header(new Writer(HEADER_LEN), Ix.SetStatus, input.status, 0, 0, 0).bytes,
    input.programAddress,
  );
}

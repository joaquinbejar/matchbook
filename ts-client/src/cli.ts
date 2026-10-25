#!/usr/bin/env node
/**
 * `matchbook`: command-line client for the MatchBook order book.
 *
 * Prices are quote tokens per base token and sizes are base tokens; the
 * client converts them to the market's lots. Run `matchbook --help`.
 */
import { existsSync } from 'node:fs';

import { address, generateKeyPairSigner, type Address, type Instruction, type KeyPairSigner } from '@solana/kit';
import { getTransferSolInstruction } from '@solana-program/system';
import { Command, Option } from 'commander';
import {
  CancelSide,
  MARKET_FLAG_PERMISSIONED,
  MarketStatus,
  OrderType,
  Permission,
  REVERSE_SPREAD_SCALE,
  SelfTradeBehavior,
  Side,
  TOKEN_2022_PROGRAM_ADDRESS,
  TOKEN_PROGRAM_ADDRESS,
  TriggerDirection,
  cancelAll,
  cancelByClientId,
  cancelOrder,
  claimSeat,
  collectFees,
  createMarket,
  deposit,
  placeOrder,
  placeStop,
  pruneExpired,
  reduceOrder,
  removeDelegate,
  setDelegate,
  setStatus,
  swap,
  triggerStops,
  withdraw,
  type Level,
  type MatchbookEvent,
  type OrderArgs,
} from '@matchbook/sdk';

import {
  Chain,
  DEFAULT_KEYPAIR,
  bigintJson,
  clusterUrl,
  loadKeypair,
  newKeypairFile,
  now,
  requireSeat,
  returnedU64,
  type Executed,
  type MarketCtx,
} from './chain.js';
import { Units, amountToAtoms, formatAtoms, parseDuration } from './units.js';

interface Globals {
  url: string;
  keypair: string;
  cuLimit?: string;
  priorityFee?: string;
  json?: boolean;
}

const program = new Command('matchbook')
  .description('MatchBook order book client')
  .option('-u, --url <cluster>', 'localnet, devnet, mainnet, testnet or an RPC URL', process.env.MATCHBOOK_RPC ?? 'localnet')
  .option('-k, --keypair <path>', 'fee payer and default owner', process.env.MATCHBOOK_KEYPAIR ?? DEFAULT_KEYPAIR)
  .option('--cu-limit <units>', 'compute unit limit per transaction')
  .option('--priority-fee <microLamports>', 'priority fee per compute unit')
  .option('--json', 'machine-readable output where supported');

let chainPromise: Promise<Chain> | undefined;
function chain(): Promise<Chain> {
  chainPromise ??= (async () => {
    const g = program.opts<Globals>();
    return new Chain(clusterUrl(g.url), await loadKeypair(g.keypair), {
      cuLimit: g.cuLimit ? Number(g.cuLimit) : undefined,
      priorityMicroLamports: g.priorityFee ? BigInt(g.priorityFee) : undefined,
    });
  })();
  return chainPromise;
}
const json = () => program.opts<Globals>().json === true;
const out = (v: unknown) => console.log(JSON.stringify(v, bigintJson, 2));

const sideOf = (s: string): Side => {
  if (s === 'buy') return Side.Bid;
  if (s === 'sell') return Side.Ask;
  throw new Error(`side must be buy or sell, not ${s}`);
};
const sideName = (s: Side) => (s === Side.Bid ? 'buy' : 'sell');
const TYPES: Record<string, OrderType> = {
  limit: OrderType.Limit,
  'post-only': OrderType.PostOnly,
  'post-only-slide': OrderType.PostOnlySlide,
  ioc: OrderType.ImmediateOrCancel,
  fok: OrderType.FillOrKill,
};
const SELF_TRADE: Record<string, SelfTradeBehavior> = {
  'cancel-provide': SelfTradeBehavior.CancelProvide,
  'decrement-take': SelfTradeBehavior.DecrementTake,
  abort: SelfTradeBehavior.Abort,
};
const typeOption = () => new Option('--type <type>', 'order type').choices(Object.keys(TYPES));

function printExecuted(e: Executed, u: Units) {
  console.log(`signature ${e.signature}`);
  if (e.computeUnits !== undefined) console.log(`compute units ${e.computeUnits}`);
  for (const ev of e.events as MatchbookEvent[]) {
    if (ev.kind === 'fill') {
      console.log(`  fill ${u.lotsToString(ev.baseLots)} @ ${u.priceToString(ev.price)} against order ${ev.makerOrderId} of ${ev.maker}`);
    } else if (ev.kind === 'out') {
      console.log(`  out ${sideName(ev.side)} ${u.lotsToString(ev.baseLots)} @ ${u.priceToString(ev.price)} (order ${ev.orderId}, reason ${ev.reason})`);
    } else {
      console.log(`  reversed into ${sideName(ev.side)} ${u.lotsToString(ev.baseLots)} @ ${u.priceToString(ev.price)} (order ${ev.orderId})`);
    }
  }
}

/** Who signs a trading instruction: the wallet, or a session key for `--owner`. */
interface Acting {
  signer: KeyPairSigner;
  owner: Address;
}
async function acting(opts: { signer?: string; owner?: string }): Promise<Acting> {
  const c = await chain();
  if (!opts.signer) {
    if (opts.owner && opts.owner !== c.payer.address) throw new Error('--owner needs --signer (a session key)');
    return { signer: c.payer, owner: c.payer.address };
  }
  return { signer: await loadKeypair(opts.signer), owner: opts.owner ? address(opts.owner) : c.payer.address };
}
const traderOptions = (cmd: Command) =>
  cmd
    .option('--signer <path>', 'sign with this session key of the seat owner')
    .option('--owner <address>', 'seat owner when signing with a session key');

async function tokenAccounts(c: Chain, m: MarketCtx, owner: Address) {
  const a = m.accounts;
  return {
    base: await c.ata(owner, a.baseMint, a.baseTokenProgram),
    quote: await c.ata(owner, a.quoteMint, a.quoteTokenProgram),
  };
}

program
  .command('address')
  .description('show the wallet address and SOL balance')
  .action(async () => {
    const c = await chain();
    console.log(c.payer.address);
    console.log(`${formatAtoms(await c.solBalance(c.payer.address), 9)} SOL`);
  });

program
  .command('keygen <out>')
  .description('write a new keypair file (Solana CLI format)')
  .action(async (path: string) => {
    if (existsSync(path)) throw new Error(`${path} exists`);
    console.log((await newKeypairFile(path)).address);
  });

program
  .command('airdrop <sol>')
  .description('request SOL from the cluster faucet')
  .option('--to <address>', 'recipient')
  .action(async (sol: string, o: { to?: string }) => {
    const c = await chain();
    const to = o.to ? address(o.to) : c.payer.address;
    console.log(`airdropped ${sol} SOL to ${to}: ${await c.airdrop(to, amountToAtoms(sol, 9))}`);
  });

const token = program.command('token').description('test tokens: create mints, mint, read balances');
token
  .command('create-mint')
  .option('--decimals <n>', 'decimals', '6')
  .option('--token-2022', 'use Token-2022')
  .action(async (o: { decimals: string; token2022?: boolean }) => {
    const c = await chain();
    console.log(await c.createMint(Number(o.decimals), o.token2022 ? TOKEN_2022_PROGRAM_ADDRESS : TOKEN_PROGRAM_ADDRESS));
  });
token
  .command('mint <mint> <amount>')
  .option('--to <owner>', 'recipient')
  .action(async (mint: string, amount: string, o: { to?: string }) => {
    const c = await chain();
    const to = o.to ? address(o.to) : c.payer.address;
    const atoms = amountToAtoms(amount, await c.decimals(address(mint)));
    const ata = await c.mintTo(address(mint), to, atoms);
    console.log(`minted ${amount} to ${to} (${ata})`);
  });
token
  .command('balance <mint>')
  .option('--owner <owner>', 'owner')
  .action(async (mint: string, o: { owner?: string }) => {
    const c = await chain();
    const m = address(mint);
    const owner = o.owner ? address(o.owner) : c.payer.address;
    const ata = await c.ata(owner, m, await c.tokenProgramOf(m));
    console.log(formatAtoms(await c.tokenBalance(ata), await c.decimals(m)));
  });

const market = program.command('market').description('create, inspect and administer markets');
market
  .command('create')
  .requiredOption('--base-mint <mint>')
  .requiredOption('--quote-mint <mint>')
  .requiredOption('--base-lot-size <atoms>', 'base atoms per lot')
  .requiredOption('--quote-lot-size <atoms>', 'quote atoms per lot')
  .option('--min-base-lots <lots>', 'minimum order size in lots', '1')
  .option('--taker-fee-bps <bps>', 'taker fee', '10')
  .option('--maker-rebate-bps <bps>', 'maker rebate', '2')
  .option('--referral-share-bps <bps>', 'share of the taker fee for referrers', '0')
  .option('--initial-nodes <n>', 'nodes to preallocate', '64')
  .option('--permissioned', 'approved seats only, no seatless swaps')
  .option('--authority <address>', 'fee collector and status authority')
  .action(async (o) => {
    const c = await chain();
    const mk = await generateKeyPairSigner();
    const baseMint = address(o.baseMint);
    const quoteMint = address(o.quoteMint);
    const sig = await c.send([
      await createMarket({
        payer: c.payer,
        market: mk,
        baseMint,
        quoteMint,
        baseTokenProgram: await c.tokenProgramOf(baseMint),
        quoteTokenProgram: await c.tokenProgramOf(quoteMint),
        authority: o.authority ? address(o.authority) : c.payer.address,
        baseLotSize: BigInt(o.baseLotSize),
        quoteLotSize: BigInt(o.quoteLotSize),
        minBaseLots: BigInt(o.minBaseLots),
        takerFeeBps: Number(o.takerFeeBps),
        makerRebateBps: Number(o.makerRebateBps),
        referralShareBps: Number(o.referralShareBps),
        initialNodes: Number(o.initialNodes),
        flags: o.permissioned ? MARKET_FLAG_PERMISSIONED : 0,
      }),
    ]);
    if (json()) out({ market: mk.address, signature: sig });
    else console.log(`market ${mk.address}\nsignature ${sig}`);
  });
market
  .command('show <market>')
  .action(async (addr: string) => {
    const c = await chain();
    const m = await c.market(address(addr));
    const h = m.view.header;
    const u = m.units;
    const status = ['active', 'cancel-only', 'closed'][h.status] ?? 'closed';
    if (json()) {
      out({
        market: addr,
        baseMint: h.baseMint,
        quoteMint: h.quoteMint,
        authority: h.authority,
        status,
        permissioned: h.permissioned,
        baseLotSize: h.baseLotSize,
        quoteLotSize: h.quoteLotSize,
        tick: u.priceToString(1n),
        minSize: u.lotsToString(h.minBaseLots),
        takerFeeBps: h.takerFeeBps,
        makerRebateBps: h.makerRebateBps,
        orders: h.orderCount,
        seats: h.traderCount,
        stops: h.stopCount,
        lastPrice: u.priceToString(h.lastPrice),
        feesAccrued: u.quoteToString(h.feesAccrued),
      });
      return;
    }
    console.log(`market        ${addr}`);
    console.log(`base mint     ${h.baseMint} (${u.baseDecimals} decimals)`);
    console.log(`quote mint    ${h.quoteMint} (${u.quoteDecimals} decimals)`);
    console.log(`authority     ${h.authority}`);
    console.log(`status        ${status}${h.permissioned ? ', permissioned' : ''}`);
    console.log(`lot           ${u.lotsToString(1n)} base, tick ${u.priceToString(1n)} quote`);
    console.log(`min size      ${u.lotsToString(h.minBaseLots)}`);
    console.log(`fees          taker ${h.takerFeeBps} bps, maker rebate ${h.makerRebateBps} bps`);
    console.log(`orders        ${h.orderCount}  seats ${h.traderCount}  stops ${h.stopCount}  delegates ${h.delegateCount}`);
    console.log(`last price    ${h.lastPrice === 0n ? '-' : u.priceToString(h.lastPrice)}`);
    console.log(`volume        ${u.lotsToString(h.totalBaseLotsTraded)} base, ${u.quoteToString(h.totalQuoteAtomsTraded)} quote`);
    console.log(`fees accrued  ${u.quoteToString(h.feesAccrued)}`);
  });
market
  .command('set-status <market> <status>')
  .description('active, cancel-only or closed (authority only)')
  .action(async (addr: string, status: string) => {
    const c = await chain();
    const s = { active: MarketStatus.Active, 'cancel-only': MarketStatus.CancelOnly, closed: MarketStatus.Closed }[status];
    if (s === undefined) throw new Error('status must be active, cancel-only or closed');
    console.log(await c.send([setStatus({ authority: c.payer, market: address(addr), status: s })]));
  });
market
  .command('collect-fees <market>')
  .option('--to <account>', 'destination quote token account')
  .action(async (addr: string, o: { to?: string }) => {
    const c = await chain();
    const m = await c.market(address(addr));
    const a = m.accounts;
    const dest = o.to ? address(o.to) : await c.ata(c.payer.address, a.quoteMint, a.quoteTokenProgram);
    const fees = m.view.header.feesAccrued;
    const sig = await c.send([
      await c.ensureAtaIx(c.payer.address, a.quoteMint, a.quoteTokenProgram),
      collectFees({ authority: c.payer, accounts: a, destination: dest }),
    ]);
    console.log(`collected ${m.units.quoteToString(fees)} quote: ${sig}`);
  });
market
  .command('trigger-stops <market>')
  .option('--limit <n>', 'most stops to place', '8')
  .action(async (addr: string, o: { limit: string }) => {
    const c = await chain();
    const m = await c.market(address(addr));
    printExecuted(await c.execute([triggerStops({ market: address(addr), limit: Number(o.limit) })]), m.units);
  });
market
  .command('prune <market>')
  .option('--limit <n>', 'most orders to remove', '32')
  .action(async (addr: string, o: { limit: string }) => {
    const c = await chain();
    const m = await c.market(address(addr));
    printExecuted(await c.execute([pruneExpired({ market: address(addr), limit: Number(o.limit) })]), m.units);
  });

program
  .command('book <market>')
  .description('show the order book')
  .option('--depth <n>', 'levels per side', '10')
  .action(async (addr: string, o: { depth: string }) => {
    const c = await chain();
    const m = await c.market(address(addr));
    const u = m.units;
    const t = now();
    const depth = Number(o.depth);
    const asks = m.view.levels(Side.Ask, depth, t);
    const bids = m.view.levels(Side.Bid, depth, t);
    if (json()) {
      const lv = (l: Level[]) => l.map((x) => ({ price: u.priceToString(x.price), size: u.lotsToString(x.baseLots), orders: x.orders }));
      out({ bids: lv(bids), asks: lv(asks) });
      return;
    }
    const row = (l: Level, s: string) =>
      console.log(`${u.priceToString(l.price).padStart(16)} ${u.lotsToString(l.baseLots).padStart(16)} ${String(l.orders).padStart(7)}  ${s}`);
    console.log(`${'price'.padStart(16)} ${'size'.padStart(16)} ${'orders'.padStart(7)}`);
    for (const l of [...asks].reverse()) row(l, 'ask');
    if (bids[0] && asks[0]) {
      const a = u.priceToNumber(asks[0].price);
      const b = u.priceToNumber(bids[0].price);
      const mid = (a + b) / 2;
      console.log(` mid ${mid.toFixed(6)}  spread ${(((a - b) / mid) * 10_000).toFixed(1)} bps `.padStart(31, '-').padEnd(42, '-'));
    } else {
      console.log('-'.repeat(42));
    }
    for (const l of bids) row(l, 'bid');
  });

program
  .command('seat')
  .description('claim a seat (needed before deposits and resting orders)')
  .command('claim <market>')
  .option('--authority-keypair <path>', 'authority, for permissioned markets')
  .action(async (addr: string, o: { authorityKeypair?: string }) => {
    const c = await chain();
    const authority = o.authorityKeypair ? await loadKeypair(o.authorityKeypair) : undefined;
    const sig = await c.send([claimSeat({ payer: c.payer, owner: c.payer, market: address(addr), authority })]);
    const m = await c.market(address(addr));
    console.log(`seat ${requireSeat(m, c.payer.address)} claimed: ${sig}`);
  });

program
  .command('balance <market>')
  .description("show a seat's free and locked balances and the wallet's tokens")
  .option('--owner <address>', 'seat owner')
  .action(async (addr: string, o: { owner?: string }) => {
    const c = await chain();
    const owner = o.owner ? address(o.owner) : c.payer.address;
    const m = await c.market(address(addr));
    const u = m.units;
    const atas = await tokenAccounts(c, m, owner);
    const wallet = { base: await c.tokenBalance(atas.base), quote: await c.tokenBalance(atas.quote) };
    const seat = m.view.findSeat(owner);
    const free = { base: seat?.baseFree ?? 0n, quote: seat?.quoteFree ?? 0n };
    const locked = seat ? m.view.locked(owner) : { base: 0n, quote: 0n };
    if (json()) {
      out({
        owner,
        seat: seat?.index ?? null,
        base: { free: u.baseToString(free.base), locked: u.baseToString(locked.base), wallet: u.baseToString(wallet.base) },
        quote: { free: u.quoteToString(free.quote), locked: u.quoteToString(locked.quote), wallet: u.quoteToString(wallet.quote) },
      });
      return;
    }
    console.log(seat ? `seat ${seat.index} of ${owner}` : `${owner} has no seat on this market`);
    const r = (a: string, b: string, cc: string, d: string) => console.log(`${a.padStart(8)} ${b.padStart(18)} ${cc.padStart(18)} ${d.padStart(18)}`);
    r('', 'free', 'locked', 'wallet');
    r('base', u.baseToString(free.base), u.baseToString(locked.base), u.baseToString(wallet.base));
    r('quote', u.quoteToString(free.quote), u.quoteToString(locked.quote), u.quoteToString(wallet.quote));
  });

async function moveFunds(addr: string, o: { base?: string; quote?: string; all?: boolean }, isDeposit: boolean) {
  const c = await chain();
  const me = c.payer.address;
  const m = await c.market(address(addr));
  const a = m.accounts;
  const seat = requireSeat(m, me);
  const atas = await tokenAccounts(c, m, me);
  if (o.all) {
    const s = m.view.findSeat(me)!;
    o.base = m.units.baseToString(s.baseFree);
    o.quote = m.units.quoteToString(s.quoteFree);
  }
  const ixs: Instruction[] = [];
  const sides = [
    { amount: o.base, mint: a.baseMint, tp: a.baseTokenProgram, ata: atas.base, vault: a.baseVault, dec: m.units.baseDecimals },
    { amount: o.quote, mint: a.quoteMint, tp: a.quoteTokenProgram, ata: atas.quote, vault: a.quoteVault, dec: m.units.quoteDecimals },
  ];
  for (const s of sides) {
    if (s.amount === undefined) continue;
    const atoms = amountToAtoms(s.amount, s.dec);
    if (atoms === 0n) continue;
    const common = { owner: c.payer, market: a.market, seat, userToken: s.ata, vault: s.vault, mint: s.mint, tokenProgram: s.tp, amount: atoms };
    if (isDeposit) ixs.push(deposit(common));
    else ixs.push(await c.ensureAtaIx(me, s.mint, s.tp), withdraw({ ...common, vaultAuthority: a.vaultAuthority }));
  }
  if (!ixs.length) throw new Error('nothing to move: pass --base and/or --quote');
  console.log(`${isDeposit ? 'deposited' : 'withdrew'} ${await c.send(ixs)}`);
}

program
  .command('deposit <market>')
  .description('move tokens from the wallet into the seat')
  .option('--base <amount>', 'base tokens')
  .option('--quote <amount>', 'quote tokens')
  .action((addr: string, o) => moveFunds(addr, o, true));

program
  .command('withdraw <market>')
  .description('move tokens from the seat back to the wallet')
  .option('--base <amount>', 'base tokens')
  .option('--quote <amount>', 'quote tokens')
  .option('--all', 'every free token on both sides')
  .action((addr: string, o) => moveFunds(addr, o, false));

traderOptions(
  program
    .command('place <market> <side> <size> <price>')
    .description('place a limit, post-only, IOC, FOK or reverse order (side: buy or sell)')
    .addOption(typeOption().default('limit'))
    .option('--client-id <id>', 'client order id', '0')
    .option('--expires-in <duration>', 'expire a resting order after 90s, 30m, 12h, 7d')
    .option('--reverse-spread-bps <bps>', 're-post each fill on the other side this many bps away')
    .addOption(new Option('--self-trade <behavior>').choices(Object.keys(SELF_TRADE)).default('cancel-provide'))
    .option('--quote-budget <amount>', 'for IOC buys: most quote tokens to spend, fees included'),
).action(async (addr: string, side: string, size: string, price: string, o) => {
  const c = await chain();
  const a = await acting(o);
  const m = await c.market(address(addr));
  const u = m.units;
  const seat = requireSeat(m, a.owner);
  let reverseSpread = 0;
  if (o.reverseSpreadBps !== undefined) {
    const bps = Number(o.reverseSpreadBps);
    if (!(bps > 0 && bps < 10_000)) throw new Error('reverse spread must be in (0, 10000) bps');
    reverseSpread = Math.round((bps * REVERSE_SPREAD_SCALE) / 10_000);
  }
  const order: OrderArgs = {
    side: sideOf(side),
    price: u.priceToLots(price),
    baseLots: u.sizeToLots(size),
    orderType: TYPES[o.type],
    selfTrade: SELF_TRADE[o.selfTrade],
    clientOrderId: BigInt(o.clientId),
    expiresAt: o.expiresIn ? now() + parseDuration(o.expiresIn) : 0n,
    quoteBudget: o.quoteBudget ? amountToAtoms(o.quoteBudget, u.quoteDecimals) : undefined,
    reverseSpread,
  };
  const e = await c.execute([placeOrder({ owner: a.signer, market: address(addr), seat, order })]);
  const id = returnedU64(e);
  const desc = `${sideName(order.side)} ${u.lotsToString(order.baseLots)} @ ${u.priceToString(order.price)}`;
  console.log(id ? `order ${id}: ${desc}` : `${desc}: nothing rests`);
  printExecuted(e, u);
});

traderOptions(program.command('cancel <market> <orderId>').description('cancel one order or stop by id')).action(
  async (addr: string, id: string, o) => {
    const c = await chain();
    const a = await acting(o);
    const m = await c.market(address(addr));
    const e = await c.execute([cancelOrder({ owner: a.signer, market: address(addr), seat: requireSeat(m, a.owner), orderId: BigInt(id) })]);
    console.log(`cancelled ${id}`);
    printExecuted(e, m.units);
  },
);

traderOptions(
  program
    .command('cancel-all <market>')
    .description("cancel every order of the seat, on one side or both")
    .addOption(new Option('--side <side>').choices(['buy', 'sell'])),
).action(async (addr: string, o) => {
  const c = await chain();
  const a = await acting(o);
  const m = await c.market(address(addr));
  const before = m.view.ordersOf(a.owner).length;
  const side = o.side === undefined ? CancelSide.Both : o.side === 'buy' ? CancelSide.Bids : CancelSide.Asks;
  const e = await c.execute([cancelAll({ owner: a.signer, market: address(addr), seat: requireSeat(m, a.owner), side })]);
  await c.refresh(m);
  console.log(`cancelled ${before - m.view.ordersOf(a.owner).length} orders`);
  printExecuted(e, m.units);
});

traderOptions(program.command('cancel-client <market> <clientOrderId>').description('cancel every order and stop with a client order id')).action(
  async (addr: string, id: string, o) => {
    const c = await chain();
    const a = await acting(o);
    const m = await c.market(address(addr));
    const e = await c.execute([
      cancelByClientId({ owner: a.signer, market: address(addr), seat: requireSeat(m, a.owner), clientOrderId: BigInt(id) }),
    ]);
    console.log(`cancelled client id ${id}`);
    printExecuted(e, m.units);
  },
);

traderOptions(program.command('reduce <market> <orderId> <newSize>').description('shrink an order, keeping its time priority')).action(
  async (addr: string, id: string, size: string, o) => {
    const c = await chain();
    const a = await acting(o);
    const m = await c.market(address(addr));
    const lots = m.units.sizeToLots(size);
    const e = await c.execute([
      reduceOrder({ owner: a.signer, market: address(addr), seat: requireSeat(m, a.owner), orderId: BigInt(id), newBaseLots: lots }),
    ]);
    console.log(`reduced ${id} to ${m.units.lotsToString(lots)}`);
    printExecuted(e, m.units);
  },
);

program
  .command('orders <market>')
  .description("list a seat's resting orders and stops")
  .option('--owner <address>', 'seat owner')
  .action(async (addr: string, o: { owner?: string }) => {
    const c = await chain();
    const owner = o.owner ? address(o.owner) : c.payer.address;
    const m = await c.market(address(addr));
    const u = m.units;
    requireSeat(m, owner);
    const orders = m.view.ordersOf(owner);
    const stops = m.view.stops(owner);
    if (json()) {
      out({
        orders: orders.map((x) => ({ id: x.orderId, side: sideName(x.side), price: u.priceToString(x.price), size: u.lotsToString(x.baseLots), clientId: x.clientOrderId, expiresAt: x.expiresAt })),
        stops: stops.map((s) => ({
          id: s.orderId,
          side: sideName(s.side),
          trigger: u.priceToString(s.triggerPrice),
          when: s.direction === TriggerDirection.Above ? 'above' : 'below',
          price: u.priceToString(s.price),
          size: u.lotsToString(s.baseLots),
          ocoWith: s.ocoWith ?? null,
        })),
      });
      return;
    }
    console.log(`orders (${orders.length})`);
    for (const x of orders) {
      const exp = x.expiresAt === 0n ? '' : `  expires ${x.expiresAt}`;
      console.log(`  ${String(x.orderId).padStart(22)} ${sideName(x.side).padEnd(4)} ${u.lotsToString(x.baseLots).padStart(14)} @ ${u.priceToString(x.price).padEnd(14)} cid ${x.clientOrderId}${exp}`);
    }
    console.log(`stops (${stops.length})`);
    for (const s of stops) {
      const when = s.direction === TriggerDirection.Above ? '>=' : '<=';
      const oco = s.ocoWith === undefined ? '' : `  oco ${s.ocoWith}`;
      console.log(
        `  ${String(s.orderId).padStart(22)} when ${when} ${u.priceToString(s.triggerPrice).padEnd(12)} ${sideName(s.side).padEnd(4)} ${u.lotsToString(s.baseLots).padStart(14)} @ ${u.priceToString(s.price).padEnd(14)} ${OrderType[s.orderType]}${oco}`,
      );
    }
  });

/** The order a stop places: a limit, or an IOC up to `slippageBps` past the trigger. */
function stopOrder(side: Side, lots: bigint, trigger: bigint, limit: bigint | undefined, slippageBps: bigint, type: OrderType | undefined, clientOrderId: bigint): OrderArgs {
  const price =
    limit ?? (side === Side.Ask ? (trigger * (10_000n - (slippageBps > 9_999n ? 9_999n : slippageBps))) / 10_000n : (trigger * (10_000n + slippageBps)) / 10_000n);
  return {
    side,
    price: price < 1n ? 1n : price,
    baseLots: lots,
    orderType: type ?? (limit === undefined ? OrderType.ImmediateOrCancel : OrderType.Limit),
    clientOrderId,
  };
}

traderOptions(
  program
    .command('stop <market> <side> <size>')
    .description('place a stop order that waits for a trade at its trigger price')
    .requiredOption('--trigger <price>', 'trigger price')
    .addOption(new Option('--when <direction>', 'trigger at or above, or at or below').choices(['above', 'below']).makeOptionMandatory())
    .option('--limit <price>', 'limit price of the triggered order (default: trigger ± slippage, IOC)')
    .option('--slippage-bps <bps>', 'without --limit: how far past the trigger it may fill', '100')
    .addOption(typeOption())
    .option('--oco-with <stopId>', 'pair with this stop as one-cancels-other')
    .option('--client-id <id>', 'client order id', '0'),
).action(async (addr: string, side: string, size: string, o) => {
  const c = await chain();
  const a = await acting(o);
  const m = await c.market(address(addr));
  const u = m.units;
  const trigger = u.priceToLots(o.trigger);
  const order = stopOrder(sideOf(side), u.sizeToLots(size), trigger, o.limit ? u.priceToLots(o.limit) : undefined, BigInt(o.slippageBps), o.type ? TYPES[o.type] : undefined, BigInt(o.clientId));
  const direction = o.when === 'above' ? TriggerDirection.Above : TriggerDirection.Below;
  const e = await c.execute([
    placeStop({ owner: a.signer, market: address(addr), seat: requireSeat(m, a.owner), triggerPrice: trigger, direction, order, ocoWith: o.ocoWith ? BigInt(o.ocoWith) : undefined }),
  ]);
  const id = returnedU64(e);
  if (id === undefined) throw new Error('the program returned no stop id');
  console.log(`stop ${id}: ${sideName(order.side)} ${u.lotsToString(order.baseLots)} @ ${u.priceToString(order.price)} when a trade is ${o.when === 'above' ? 'at or above' : 'at or below'} ${u.priceToString(trigger)}`);
  printExecuted(e, u);
});

traderOptions(
  program
    .command('bracket <market> <side> <size>')
    .description('a stop-loss and a take-profit that cancel each other (sell closes a long, buy a short)')
    .requiredOption('--stop-loss <price>')
    .requiredOption('--take-profit <price>')
    .option('--slippage-bps <bps>', 'how far past the stop-loss its order may fill', '100')
    .option('--client-id <id>', 'client order id for both stops', '0'),
).action(async (addr: string, sideStr: string, size: string, o) => {
  const c = await chain();
  const a = await acting(o);
  const m = await c.market(address(addr));
  const u = m.units;
  const seat = requireSeat(m, a.owner);
  const side = sideOf(sideStr);
  const lots = u.sizeToLots(size);
  const sl = u.priceToLots(o.stopLoss);
  const tp = u.priceToLots(o.takeProfit);
  if (side === Side.Ask && sl >= tp) throw new Error('selling: the stop-loss must be below the take-profit');
  if (side === Side.Bid && sl <= tp) throw new Error('buying: the stop-loss must be above the take-profit');
  const [slDir, tpDir] = side === Side.Ask ? [TriggerDirection.Below, TriggerDirection.Above] : [TriggerDirection.Above, TriggerDirection.Below];
  const cid = BigInt(o.clientId);
  const e1 = await c.execute([
    placeStop({ owner: a.signer, market: address(addr), seat, triggerPrice: sl, direction: slDir, order: stopOrder(side, lots, sl, undefined, BigInt(o.slippageBps), undefined, cid) }),
  ]);
  const slId = returnedU64(e1);
  if (slId === undefined) throw new Error('no stop id');
  const e2 = await c.execute([
    placeStop({ owner: a.signer, market: address(addr), seat, triggerPrice: tp, direction: tpDir, order: stopOrder(side, lots, tp, tp, 0n, OrderType.Limit, cid), ocoWith: slId }),
  ]);
  console.log(`stop-loss   ${slId} at ${u.priceToString(sl)}`);
  console.log(`take-profit ${returnedU64(e2)} at ${u.priceToString(tp)} (one cancels the other)`);
});

program
  .command('swap <market> <side> <amount>')
  .description('trade from the wallet with no seat (buy: quote to spend; sell: base to sell)')
  .option('--slippage-bps <bps>', 'worst accepted shortfall from the quote', '50')
  .action(async (addr: string, sideStr: string, amount: string, o: { slippageBps: string }) => {
    const c = await chain();
    const me = c.payer.address;
    const m = await c.market(address(addr));
    const u = m.units;
    const a = m.accounts;
    const side = sideOf(sideStr);
    const max = (1n << 64n) - 1n;
    const atoms = amountToAtoms(amount, side === Side.Bid ? u.quoteDecimals : u.baseDecimals);
    const params =
      side === Side.Bid
        ? { side, baseLots: max / u.baseLotSize, quoteBudget: atoms, limitPrice: max }
        : { side, baseLots: atoms / u.baseLotSize, limitPrice: 1n };
    const q = m.view.quoteSwap({ ...params, now: now() });
    if (q.baseLots === 0n) throw new Error('nothing to trade against');
    const expected = side === Side.Bid ? q.baseAtoms : q.quoteNet;
    const minOut = (expected * (10_000n - BigInt(o.slippageBps))) / 10_000n;
    const atas = await tokenAccounts(c, m, me);
    const e = await c.execute([
      await c.ensureAtaIx(me, a.baseMint, a.baseTokenProgram),
      await c.ensureAtaIx(me, a.quoteMint, a.quoteTokenProgram),
      swap({ user: c.payer, accounts: a, userBase: atas.base, userQuote: atas.quote, ...params, minOut }),
    ]);
    console.log(
      `${side === Side.Bid ? 'bought' : 'sold'} ${u.baseToString(q.baseAtoms)} base for ${u.quoteToString(q.quoteNet)} quote (fee ${u.quoteToString(q.feeAtoms)}), average ${u.priceToNumber(BigInt(Math.round(q.averagePrice))).toFixed(6)}`,
    );
    printExecuted(e, u);
  });

const session = program.command('session').description('session keys: delegates that trade for a seat but cannot withdraw');
session
  .command('create <market>')
  .option('--out <path>', 'where to write the session keypair', 'session.json')
  .option('--expires-in <duration>', 'lifetime: 90s, 30m, 12h, 7d (0 = no expiry)', '1d')
  .option('--cancel-only', 'only cancel and reduce, never place')
  .option('--fund <sol>', 'SOL to send the session key for fees', '0.05')
  .action(async (addr: string, o: { out: string; expiresIn: string; cancelOnly?: boolean; fund: string }) => {
    if (existsSync(o.out)) throw new Error(`${o.out} exists; pick another --out`);
    const c = await chain();
    const m = await c.market(address(addr));
    const seat = requireSeat(m, c.payer.address);
    const key = await newKeypairFile(o.out);
    const secs = parseDuration(o.expiresIn);
    const ixs: Instruction[] = [
      setDelegate({
        owner: c.payer,
        market: address(addr),
        seat,
        delegate: key.address,
        permissions: o.cancelOnly ? Permission.CANCEL : Permission.TRADE,
        expiresAt: secs === 0n ? 0n : now() + secs,
      }),
    ];
    const fund = amountToAtoms(o.fund, 9);
    if (fund > 0n) ixs.push(getTransferSolInstruction({ source: c.payer, destination: key.address, amount: fund }));
    const sig = await c.send(ixs);
    console.log(`session key ${key.address} written to ${o.out}`);
    console.log(`${o.cancelOnly ? 'cancel-only' : 'trading'} for seat ${seat}, ${secs === 0n ? 'no expiry' : `expires in ${o.expiresIn}`}`);
    console.log(`signature ${sig}`);
  });
session
  .command('list <market>')
  .option('--owner <address>', 'seat owner')
  .action(async (addr: string, o: { owner?: string }) => {
    const c = await chain();
    const owner = o.owner ? address(o.owner) : c.payer.address;
    const m = await c.market(address(addr));
    const seat = requireSeat(m, owner);
    const t = now();
    for (const d of m.view.delegates().filter((x) => x.seat === seat)) {
      const perms = d.permissions & Permission.TRADE ? 'trade' : d.permissions & Permission.CANCEL ? 'cancel' : 'none';
      const exp = d.expiresAt === 0n ? 'never' : d.expiresAt <= t ? 'expired' : `in ${d.expiresAt - t}s`;
      console.log(`${d.delegate}  ${perms}  expires ${exp}`);
    }
  });
session
  .command('revoke <market> <delegate>')
  .action(async (addr: string, delegate: string) => {
    const c = await chain();
    const m = await c.market(address(addr));
    const sig = await c.send([removeDelegate({ owner: c.payer, market: address(addr), seat: requireSeat(m, c.payer.address), delegate: address(delegate) })]);
    console.log(`revoked ${delegate}: ${sig}`);
  });

program.parseAsync().catch((e: unknown) => {
  console.error(`error: ${(e as Error).message ?? e}`);
  process.exit(1);
});

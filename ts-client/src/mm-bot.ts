#!/usr/bin/env node
/**
 * `matchbook-mm`: a market-making bot for MatchBook.
 *
 * It quotes a ladder of post-only orders around a reference price and
 * replaces them in one `BatchUpdate` whenever the price moves or a quote
 * fills. It signs with a session key, which can trade the seat but never
 * withdraw from it. The quoting and risk rules are in `mm.ts`.
 */
import { address, type Address, type KeyPairSigner } from '@solana/kit';
import { Command } from 'commander';
import { CancelSide, Permission, batchUpdate, type Order } from '@matchbook/sdk';

import { Chain, DEFAULT_KEYPAIR, clusterUrl, loadKeypair, now, requireSeat, type MarketCtx } from './chain.js';
import {
  buildLadder,
  checkRisk,
  defaultQuoteParams,
  equity,
  ladderOrders,
  moved,
  type Inventory,
  type Ladder,
  type QuoteParams,
  type RiskLimits,
} from './mm.js';
import { parsePriceSource, type PriceSource } from './price.js';

/** Client order id of the bot's quotes ("MM"). */
const DEFAULT_CLIENT_ID = 0x4d4dn;
const MAX_LEVELS = 16;
const MAX_BATCH = 32;

const log = (msg: string) => console.error(`[${new Date().toISOString().slice(11, 19)}] ${msg}`);
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

class Bot {
  startEquity?: number;
  lastQuoted = 0;
  lastReference = 0;
  lastPriceAt = Date.now();
  lastLots = 0n;
  lastCount = 0;
  lastInventory?: Inventory;

  constructor(
    readonly chain: Chain,
    readonly signer: KeyPairSigner,
    readonly owner: Address,
    readonly market: MarketCtx,
    readonly seat: number,
    readonly params: QuoteParams,
    readonly limits: RiskLimits,
    readonly source: PriceSource,
    readonly clientId: bigint,
    readonly dryRun: boolean,
  ) {}

  myOrders(): Order[] {
    return this.market.view.ordersOf(this.owner).filter((o) => o.clientOrderId === this.clientId);
  }

  inventory(): Inventory {
    const s = this.market.view.findSeat(this.owner);
    if (!s) throw new Error('the seat is gone');
    const l = this.market.view.locked(this.owner);
    const u = this.market.units;
    return { base: u.baseToNumber(s.baseFree + l.base), quote: u.quoteToNumber(s.quoteFree + l.quote) };
  }

  async sendBatch(mine: Order[], ladder: Ladder) {
    const all = mine.length > MAX_BATCH;
    const orders = ladderOrders(ladder, this.clientId);
    const e = await this.chain.execute(
      [
        batchUpdate({
          owner: this.signer,
          market: this.market.view.address,
          seat: this.seat,
          cancelAll: all ? CancelSide.Both : undefined,
          cancels: all ? [] : mine.map((o) => o.orderId),
          orders,
        }),
      ],
      this.signer,
    );
    log(`requoted: ${all ? 'all' : mine.length} cancels, ${orders.length} orders, ${e.computeUnits ?? 0n} CU, ${e.signature}`);
  }

  async pull(why: string) {
    const mine = this.myOrders();
    if (!mine.length || this.dryRun) return;
    log(`pulling ${mine.length} quotes: ${why}`);
    await this.sendBatch(mine, { bids: [], asks: [], center: 0 });
    this.lastQuoted = 0;
  }

  /** One cycle; false to stop. */
  async cycle(): Promise<boolean> {
    await this.chain.refresh(this.market);
    let reference = this.lastReference;
    try {
      reference = await this.source(this.market);
      this.lastPriceAt = Date.now();
    } catch (e) {
      log(`price source failed: ${(e as Error).message}`);
    }
    const inv = this.inventory();
    if (this.lastInventory) {
      const db = inv.base - this.lastInventory.base;
      if (Math.abs(db) > 1e-12) {
        log(`filled: ${db > 0 ? 'bought' : 'sold'} ${Math.abs(db).toFixed(9)} base, inventory ${inv.base.toFixed(6)} base / ${inv.quote.toFixed(6)} quote`);
      }
    }
    this.lastInventory = inv;
    const eq = equity(inv, reference);
    this.startEquity ??= eq;
    const verdict = checkRisk(this.limits, this.startEquity, eq, Math.floor((Date.now() - this.lastPriceAt) / 1000), this.lastReference, reference);
    if (verdict.kind === 'halt') {
      await this.pull(verdict.why);
      log(`halting: ${verdict.why}`);
      return false;
    }
    if (verdict.kind === 'pause') {
      await this.pull(verdict.why);
      this.lastReference = reference;
      return true;
    }
    this.lastReference = reference;

    const ladder = buildLadder(reference, inv, this.params, this.market.units);
    const mine = this.myOrders();
    const lots = mine.reduce((s, o) => s + o.baseLots, 0n);
    const filled = mine.length !== this.lastCount || lots !== this.lastLots;
    if (!(moved(this.lastQuoted, reference, this.params.requoteBps) || filled)) return true;
    const u = this.market.units;
    const show = (qs: Ladder['bids']) => qs.map((q) => `${u.priceToString(q.price)}x${u.lotsToString(q.baseLots)}`).join(' ');
    log(
      `ref ${reference.toFixed(6)} center ${ladder.center.toFixed(6)} inv ${inv.base.toFixed(6)}/${inv.quote.toFixed(6)} pnl ${(eq - this.startEquity).toFixed(6)} | bids ${show(ladder.bids)} | asks ${show(ladder.asks)}`,
    );
    if (this.dryRun) {
      this.lastQuoted = reference;
      return true;
    }
    await this.sendBatch(mine, ladder);
    await this.chain.refresh(this.market);
    const after = this.myOrders();
    this.lastCount = after.length;
    this.lastLots = after.reduce((s, o) => s + o.baseLots, 0n);
    this.lastQuoted = reference;
    return true;
  }
}

const program = new Command('matchbook-mm')
  .description('MatchBook market-making bot')
  .option('-u, --url <cluster>', 'localnet, devnet, mainnet, testnet or an RPC URL', process.env.MATCHBOOK_RPC ?? 'localnet')
  .option('-k, --keypair <path>', 'wallet (seat owner, or the trader without a session key)', process.env.MATCHBOOK_KEYPAIR ?? DEFAULT_KEYPAIR)
  .requiredOption('--market <address>', 'market to quote', process.env.MATCHBOOK_MARKET)
  .option('--session-keypair <path>', 'session key to trade with (it also pays fees)', process.env.MATCHBOOK_SESSION)
  .option('--owner <address>', 'seat owner the session key trades for (default: the wallet)')
  .option('--price <source>', 'fixed:<p>, random:<start>[:<step bps>], book, or http:<url>#<json.path>', 'book')
  .option('--levels <n>', 'levels per side (at most 16)', '3')
  .option('--spread-bps <bps>', 'distance of the first level from the reference', '10')
  .option('--step-bps <bps>', 'distance between levels', '10')
  .requiredOption('--size <base>', 'size of the first level in base tokens')
  .option('--size-growth <x>', 'each level is this many times the previous one', '1')
  .option('--target-base <base>', 'base tokens to aim for (default: what the seat holds at start)')
  .option('--max-position <base>', 'largest distance from the target before one side goes quiet')
  .option('--skew-bps <bps>', 'ladder shift at a full max-position imbalance', '0')
  .option('--max-notional <quote>', 'most quote tokens per side (0 = all the seat holds)', '0')
  .option('--requote-bps <bps>', 'requote when the reference moves this far', '5')
  .option('--interval-ms <ms>', 'time between cycles', '2000')
  .option('--max-loss <quote>', 'cancel all and exit after losing this much equity (0 = off)', '0')
  .option('--max-price-age <secs>', 'pull quotes while the reference is older than this', '30')
  .option('--max-jump-bps <bps>', 'pull quotes for a cycle when the reference jumps this far (0 = off)', '0')
  .option('--client-id <id>', "client order id of the bot's quotes", DEFAULT_CLIENT_ID.toString())
  .option('--cycles <n>', 'stop after this many cycles (0 = until Ctrl-C)', '0')
  .option('--dry-run', 'print the quotes without sending anything')
  .option('--keep-quotes', 'leave the quotes on the book when exiting')
  .option('--cu-limit <units>', 'compute unit limit per transaction')
  .option('--priority-fee <microLamports>', 'priority fee per compute unit');

async function main() {
  program.parse();
  const o = program.opts();
  const levels = Number(o.levels);
  if (levels > MAX_LEVELS) throw new Error(`at most ${MAX_LEVELS} levels per side`);
  let signer: KeyPairSigner;
  let owner: Address;
  if (o.sessionKeypair) {
    signer = await loadKeypair(o.sessionKeypair);
    owner = o.owner ? address(o.owner) : (await loadKeypair(o.keypair)).address;
  } else {
    signer = await loadKeypair(o.keypair);
    owner = signer.address;
  }
  const chain = new Chain(clusterUrl(o.url), signer, {
    cuLimit: o.cuLimit ? Number(o.cuLimit) : undefined,
    priorityMicroLamports: o.priorityFee ? BigInt(o.priorityFee) : undefined,
  });
  const market = await chain.market(address(o.market));
  const seat = requireSeat(market, owner);
  if (signer.address !== owner) {
    const t = now();
    const ok = market.view
      .delegates()
      .some((d) => d.seat === seat && d.delegate === signer.address && (d.permissions & Permission.TRADE) !== 0n && (d.expiresAt === 0n || d.expiresAt > t));
    if (!ok) throw new Error(`${signer.address} is not a live trading session key of ${owner}; create one with \`matchbook session create\``);
  }
  const params: QuoteParams = {
    ...defaultQuoteParams,
    levels,
    spreadBps: Number(o.spreadBps),
    stepBps: Number(o.stepBps),
    size: Number(o.size),
    sizeGrowth: Number(o.sizeGrowth),
    maxPosition: o.maxPosition === undefined ? Infinity : Number(o.maxPosition),
    skewBps: Number(o.skewBps),
    maxNotional: Number(o.maxNotional),
    requoteBps: Number(o.requoteBps),
  };
  if (!(params.size > 0)) throw new Error('--size must be positive');
  const bot = new Bot(
    chain,
    signer,
    owner,
    market,
    seat,
    params,
    { maxLoss: Number(o.maxLoss), maxPriceAgeSecs: Number(o.maxPriceAge), maxJumpBps: Number(o.maxJumpBps) },
    parsePriceSource(o.price),
    BigInt(o.clientId),
    o.dryRun === true,
  );
  params.targetBase = o.targetBase === undefined ? bot.inventory().base : Number(o.targetBase);
  log(`quoting ${o.market} for seat ${seat} of ${owner} with ${signer.address}${o.dryRun ? ' (dry run)' : ''}`);
  log(`target ${params.targetBase} base, max position ${params.maxPosition}, ${levels} levels, spread ${params.spreadBps} bps, step ${params.stepBps} bps`);

  let stop = false;
  process.on('SIGINT', () => (stop = true));
  process.on('SIGTERM', () => (stop = true));
  const cycles = Number(o.cycles);
  for (let n = 1; !stop; n++) {
    try {
      if (!(await bot.cycle())) break;
    } catch (e) {
      log(`cycle failed: ${(e as Error).message}`);
    }
    if (cycles > 0 && n >= cycles) break;
    const until = Date.now() + Number(o.intervalMs);
    while (Date.now() < until && !stop) await sleep(50);
  }
  if (!o.keepQuotes) {
    await chain.refresh(market);
    await bot.pull('exiting');
  }
  log('stopped');
}

main().catch((e: unknown) => {
  console.error(`error: ${(e as Error).message ?? e}`);
  process.exit(1);
});

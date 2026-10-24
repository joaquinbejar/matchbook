import { generateKeyPairSigner, type Address, type KeyPairSigner } from '@solana/kit';
import { beforeAll, describe, expect, it } from 'vitest';

import {
  MatchbookErrorCode,
  OrderType,
  OutReason,
  Permission,
  Side,
  TriggerDirection,
  batchUpdate,
  cancelOrder,
  claimSeat,
  createMarket,
  deposit,
  parseEventsFromLogs,
  placeOrder,
  placeStop,
  setDelegate,
  swap,
  withdraw,
  type MarketAccounts,
} from '../src/index.js';
import { Harness, errorCode, haveProgram, returnedU64 } from './harness.js';

const LOT = 1_000_000n; // 0.001 of a 9-decimal token

describe.skipIf(!haveProgram)('MatchBook on a local validator', () => {
  let h: Harness;
  let accts: MarketAccounts;
  let market: Address;
  let mm: KeyPairSigner;
  let mmBase: Address;
  let mmQuote: Address;

  beforeAll(async () => {
    h = await Harness.create();
    const baseMint = await h.mint(9);
    const quoteMint = await h.mint(6);
    const marketKp = await generateKeyPairSigner();
    market = marketKp.address;
    await h.send([
      await createMarket({
        payer: h.payer,
        market: marketKp,
        baseMint,
        quoteMint,
        authority: h.payer.address,
        baseLotSize: LOT,
        quoteLotSize: 1n,
        takerFeeBps: 10,
        makerRebateBps: 2,
        referralShareBps: 2_000,
      }),
    ]);
    accts = await h.marketAccounts(market, baseMint, quoteMint);

    mm = await h.wallet();
    mmBase = await h.fund(mm.address, baseMint, 100n * LOT);
    mmQuote = await h.fund(mm.address, quoteMint, 100_000_000_000n);
    await h.send([claimSeat({ payer: mm, owner: mm, market })]);
    const seat = h.market(market).findSeat(mm.address)!.index;
    await h.send([
      deposit({ owner: mm, market, seat, userToken: mmBase, vault: accts.baseVault, mint: baseMint, amount: 100n * LOT }),
      deposit({
        owner: mm,
        market,
        seat,
        userToken: mmQuote,
        vault: accts.quoteVault,
        mint: quoteMint,
        amount: 100_000_000_000n,
      }),
    ]);
  });

  it('decodes the market and seats', () => {
    const m = h.market(market);
    expect(m.header.takerFeeBps).toBe(10);
    expect(m.header.referralShareBps).toBe(2_000);
    expect(m.header.permissioned).toBe(false);
    const s = m.findSeat(mm.address)!;
    expect(s.baseFree).toBe(100n * LOT);
    expect(s.quoteFree).toBe(100_000_000_000n);
  });

  it('requotes with a batch and shows the book', async () => {
    const seat = h.market(market).findSeat(mm.address)!.index;
    const r = await h.send([
      batchUpdate({
        owner: mm,
        market,
        seat,
        orders: [
          { side: Side.Ask, price: 100_100n, baseLots: 5n, orderType: OrderType.PostOnly },
          { side: Side.Ask, price: 100_200n, baseLots: 5n, orderType: OrderType.PostOnly },
          { side: Side.Ask, price: 100_200n, baseLots: 3n, orderType: OrderType.PostOnly },
          { side: Side.Bid, price: 99_900n, baseLots: 5n, orderType: OrderType.PostOnly },
          { side: Side.Bid, price: 99_800n, baseLots: 5n, orderType: OrderType.PostOnly },
        ],
      }),
    ]);
    expect(returnedU64(r, 0)).not.toBe(0n);
    const m = h.market(market);
    expect(m.levels(Side.Ask)).toEqual([
      { price: 100_100n, baseLots: 5n, orders: 1 },
      { price: 100_200n, baseLots: 8n, orders: 2 },
    ]);
    expect(m.levels(Side.Bid, 1)).toEqual([{ price: 99_900n, baseLots: 5n, orders: 1 }]);
    expect(m.header.bestAskPrice).toBe(100_100n);
    expect(m.header.bestBidPrice).toBe(99_900n);
    expect(m.ordersOf(mm.address)).toHaveLength(5);
  });

  it('quotes a swap exactly, then executes it with a referrer', async () => {
    const app = await h.wallet();
    await h.send([claimSeat({ payer: app, owner: app, market })]);
    const appSeat = h.market(market).findSeat(app.address)!.index;

    const user = await h.wallet();
    const userBase = await h.fund(user.address, accts.baseMint, 0n);
    const userQuote = await h.fund(user.address, accts.quoteMint, 1_000_000_000n);
    const budget = 700_000n;
    const q = h.market(market).quoteSwap({ side: Side.Bid, baseLots: 100n, quoteBudget: budget });
    expect(q.baseLots).toBe(6n); // 5 at 100,100 and 1 of the 8 at 100,200
    const r = await h.send([
      swap({
        user,
        accounts: accts,
        userBase,
        userQuote,
        side: Side.Bid,
        baseLots: 100n,
        quoteBudget: budget,
        limitPrice: 200_000n,
        minOut: q.baseAtoms,
        referrer: { owner: app.address, seat: appSeat },
      }),
    ]);
    expect(h.tokenBalance(userBase)).toBe(q.baseAtoms);
    expect(1_000_000_000n - h.tokenBalance(userQuote)).toBe(q.quoteNet);
    expect(h.market(market).findSeat(app.address)!.quoteFree).toBe((q.feeAtoms * 2_000n) / 10_000n);

    const fills = parseEventsFromLogs(r.logs()).filter((e) => e.kind === 'fill');
    expect(fills.map((f) => [f.price, f.baseLots])).toEqual([
      [100_100n, 5n],
      [100_200n, 1n],
    ]);
  });

  it('lets a session key trade but not withdraw', async () => {
    const session = await h.wallet();
    const seat = h.market(market).findSeat(mm.address)!.index;
    await h.send([setDelegate({ owner: mm, market, seat, delegate: session.address, permissions: Permission.TRADE })]);
    const r = await h.send([
      placeOrder({ owner: session, market, seat, order: { side: Side.Bid, price: 99_000n, baseLots: 1n } }),
    ]);
    const id = returnedU64(r);
    expect(h.market(market).ordersOf(mm.address).some((o) => o.orderId === id)).toBe(true);
    await h.send([cancelOrder({ owner: session, market, seat, orderId: id })]);
    const code = await errorCode(
      h.send([
        withdraw({
          owner: session,
          market,
          seat,
          userToken: mmBase,
          vault: accts.baseVault,
          mint: accts.baseMint,
          vaultAuthority: accts.vaultAuthority,
          amount: 1n,
        }),
      ]),
    );
    expect(code).toBe(MatchbookErrorCode.TraderNotFound);
  });

  it('fires a stop-loss bracket on a trade', async () => {
    const trader = await h.wallet();
    const tBase = await h.fund(trader.address, accts.baseMint, 10n * LOT);
    await h.send([claimSeat({ payer: trader, owner: trader, market })]);
    const seat = h.market(market).findSeat(trader.address)!.index;
    await h.send([
      deposit({ owner: trader, market, seat, userToken: tBase, vault: accts.baseVault, mint: accts.baseMint, amount: 10n * LOT }),
    ]);
    const sell = (price: bigint) => ({ side: Side.Ask, price, baseLots: 3n, orderType: OrderType.ImmediateOrCancel });
    const sl = returnedU64(
      await h.send([
        placeStop({ owner: trader, market, seat, direction: TriggerDirection.Below, triggerPrice: 99_900n, order: sell(99_000n) }),
      ]),
    );
    await h.send([
      placeStop({
        owner: trader,
        market,
        seat,
        direction: TriggerDirection.Above,
        triggerPrice: 101_000n,
        order: sell(100_500n),
        ocoWith: sl,
      }),
    ]);
    const stops = h.market(market).stops(trader.address);
    expect(stops).toHaveLength(2);
    expect(stops.find((s) => s.orderId === sl)!.ocoWith).toBeDefined();

    // An unrelated wallet sells into the 99,900 bid; that trade sets off the
    // stop-loss, and the take-profit is cancelled with it.
    const seller = await h.wallet();
    const sBase = await h.fund(seller.address, accts.baseMint, LOT);
    const sQuote = await h.fund(seller.address, accts.quoteMint, 0n);
    const r = await h.send([
      swap({
        user: seller,
        accounts: accts,
        userBase: sBase,
        userQuote: sQuote,
        side: Side.Ask,
        baseLots: 1n,
        limitPrice: 99_900n,
      }),
    ]);
    const outs = parseEventsFromLogs(r.logs()).filter((e) => e.kind === 'out');
    expect(outs.map((e) => e.reason)).toEqual([OutReason.Triggered, OutReason.OcoCancelled]);
    const t = h.market(market).findSeat(trader.address)!;
    expect(t.baseFree).toBe(7n * LOT);
    expect(h.market(market).stops(trader.address)).toHaveLength(0);
  });
});

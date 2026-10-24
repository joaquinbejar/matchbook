# @matchbook/sdk

TypeScript SDK for [MatchBook](../README.md), a crankless central limit order book on Solana.
Built on [`@solana/kit`](https://github.com/anza-xyz/kit).

- **Instructions** for every program instruction, returned as kit `Instruction`s.
- **Market decoding**: the book by order or by level, seats, stops and delegates from one
  account fetch.
- **Exact swap quotes**: `quoteSwap` replays the program's matching fill by fill, fees and
  rounding included, so what you quote is what executes.
- **Events**: fills, stops triggering and reverse orders flipping, decoded from transaction
  logs.

```bash
npm install @matchbook/sdk @solana/kit
```

## Read the book

```ts
import { address, createSolanaRpc } from '@solana/kit';
import { Side, decodeMarket } from '@matchbook/sdk';

const rpc = createSolanaRpc('https://api.devnet.solana.com');
const market = address('...');
const { value } = await rpc.getAccountInfo(market, { encoding: 'base64' }).send();
const view = decodeMarket(market, Buffer.from(value!.data[0], 'base64'));

view.levels(Side.Ask, 10); // [{ price, baseLots, orders }, ...] best first
view.header.lastPrice;
view.findSeat(myWallet);   // { index, baseFree, quoteFree }
```

Prices are integers in quote lots per base lot; `view.priceToAtoms(price)` converts one to
quote atoms per base atom.

## Trade

```ts
import { CancelSide, OrderType, Side, TriggerDirection, batchUpdate, placeOrder, placeStop } from '@matchbook/sdk';

const seat = view.findSeat(owner.address)!.index; // a hint that makes lookups cheapest

// A post-only bid.
placeOrder({ owner, market, seat, order: { side: Side.Bid, price: 99_900n, baseLots: 10n, orderType: OrderType.PostOnly } });

// A market maker's requote: cancel everything, place new quotes, atomically.
batchUpdate({ owner, market, seat, cancelAll: CancelSide.Both, orders: [/* ... */] });

// A reverse order: every fill re-posts on the other side 0.2% away.
placeOrder({ owner, market, seat, order: { side: Side.Ask, price: 100_000n, baseLots: 10n, reverseSpread: 2_000 } });

// A bracket: stop-loss and take-profit, one cancels the other.
placeStop({ owner, market, seat, direction: TriggerDirection.Below, triggerPrice: 95_000n,
  order: { side: Side.Ask, price: 94_000n, baseLots: 10n, orderType: OrderType.ImmediateOrCancel } });
```

`owner` can be the seat's owner or a delegate set with `setDelegate` (a session key that can
trade but never withdraw).

## Swap from a wallet

```ts
const q = view.quoteSwap({ side: Side.Bid, baseLots: 1_000n, quoteBudget: 5_000_000n });
swap({ user, accounts, userBase, userQuote, side: Side.Bid, baseLots: 1_000n,
  quoteBudget: 5_000_000n, limitPrice: 101_000n, minOut: q.baseAtoms,
  referrer: { owner: myAppWallet } }); // earns the market's referral share of the fee
```

## Tests

The tests run the compiled program in [LiteSVM](https://github.com/LiteSVM/litesvm):

```bash
cargo build-sbf --manifest-path ../program/Cargo.toml
npm test
```

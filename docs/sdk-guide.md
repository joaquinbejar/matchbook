# SDK guide

MatchBook has two clients. Both build every instruction, decode the market account and quote
swaps exactly.

| | TypeScript: `@matchbook/sdk` | Rust: `matchbook-sdk` |
| --- | --- | --- |
| Built on | `@solana/kit` | `solana-instruction`, `solana-pubkey` 3.x |
| Quotes | Replays the matching rules fill by fill | Runs the engine on a copy of the account |
| Extras | Event decoding from logs | Jupiter `Amm` adapter (`jupiter` feature) |

## Concepts

- **Seat.** A trader's balances and orders live in a seat inside the market account. Claim one
  with `claimSeat`, then `deposit`. Its node index is the `seat` hint trader instructions take.
- **Units.** Prices are quote lots per base lot and sizes are base lots. Balances and swap
  amounts are atoms (the mint's smallest unit).
- **Signers.** Trader instructions take the seat's owner or one of its delegates.
- **Order ids.** Returned in the transaction's return data (`PlaceOrder`, `PlaceStop`, one per
  order for `BatchUpdate`), in `Fill`/`Out` events and in the decoded book.

## TypeScript

```bash
npm install @matchbook/sdk @solana/kit
```

### Read a market

```ts
import { createSolanaRpc } from '@solana/kit';
import { Side, decodeMarket } from '@matchbook/sdk';

const rpc = createSolanaRpc(url);
const { value } = await rpc.getAccountInfo(market, { encoding: 'base64' }).send();
const view = decodeMarket(market, Buffer.from(value!.data[0], 'base64'));

view.levels(Side.Bid, 10);        // [{ price, baseLots, orders }] best first
view.orders(Side.Ask);            // every order with owner, id, expiry
view.findSeat(wallet);            // { index, baseFree, quoteFree, ... }
view.ordersOf(wallet);            // one trader's orders
view.stops(wallet);               // one trader's stops
view.header.lastPrice;
```

### Place, cancel, requote

```ts
import { CancelSide, OrderType, Side, batchUpdate, cancelOrder, placeOrder } from '@matchbook/sdk';

const seat = view.findSeat(owner.address)!.index;

placeOrder({
  owner, market, seat,
  order: { side: Side.Bid, price: 149_900n, baseLots: 100n, orderType: OrderType.PostOnly, clientOrderId: 7n },
});

cancelOrder({ owner, market, seat, orderId });

// Cancel everything and quote a new ladder in one instruction.
batchUpdate({
  owner, market, seat,
  cancelAll: CancelSide.Both,
  orders: [
    { side: Side.Bid, price: 149_900n, baseLots: 100n, orderType: OrderType.PostOnly },
    { side: Side.Ask, price: 150_100n, baseLots: 100n, orderType: OrderType.PostOnly },
  ],
});
```

Order fields: `orderType` (limit, post-only, post-only slide, IOC, fill or kill),
`selfTrade`, `matchLimit`, `clientOrderId`, `expiresAt` (unix seconds), `quoteBudget` (IOC
bids) and `reverseSpread` (parts per million).

### Reverse orders and stops

```ts
// Re-post each fill on the other side 0.2% away.
placeOrder({ owner, market, seat, order: { side: Side.Ask, price: 150_100n, baseLots: 100n, reverseSpread: 2_000 } });

// A bracket around a long position: the take-profit pairs with the stop-loss.
// Send the stop-loss first: its id is the u64 in that transaction's return data.
placeStop({
  owner, market, seat, direction: TriggerDirection.Below, triggerPrice: 145_000n,
  order: { side: Side.Ask, price: 144_000n, baseLots: 100n, orderType: OrderType.ImmediateOrCancel },
});
placeStop({
  owner, market, seat, direction: TriggerDirection.Above, triggerPrice: 160_000n, ocoWith: stopLossId,
  order: { side: Side.Ask, price: 159_500n, baseLots: 100n, orderType: OrderType.ImmediateOrCancel },
});
```

### Session keys

```ts
import { Permission, setDelegate } from '@matchbook/sdk';

setDelegate({ owner, market, seat, delegate: sessionKey.address, permissions: Permission.TRADE, expiresAt });
// sessionKey can now sign placeOrder, cancelOrder and batchUpdate for this seat, never withdraw.
```

### Swap from a wallet

```ts
const q = view.quoteSwap({ side: Side.Bid, baseLots: 1_000_000n, quoteBudget: 30_000_000n });
swap({
  user, accounts, userBase, userQuote,
  side: Side.Bid, baseLots: 1_000_000n, quoteBudget: 30_000_000n, limitPrice: 200_000n,
  minOut: q.baseAtoms,
  referrer: { owner: appWallet, seat: appSeat },   // optional: earns the referral share
});
```

`accounts` holds the market, mints, vaults, vault authority and token programs; `findVault`
and `findVaultAuthority` derive them.

### Events

```ts
import { parseEventsFromLogs } from '@matchbook/sdk';

for (const e of parseEventsFromLogs(tx.meta.logMessages)) {
  if (e.kind === 'fill') { /* e.maker, e.makerOrderId, e.price, e.baseLots, e.makerRemaining */ }
  if (e.kind === 'out') { /* e.reason: Expired, SelfTrade, Triggered, OcoCancelled */ }
  if (e.kind === 'reversed') { /* e.orderId, e.side, e.price, e.baseLots */ }
}
```

## Rust

```toml
[dependencies]
matchbook-sdk = "1.0.0"
```

### Instructions

```rust
use matchbook_sdk::ix::{self, CancelAll, MarketAccounts};
use matchbook_sdk::{engine::Side, OrderArgs, NO_HINT, TOKEN_PROGRAM_ID};

let accts = MarketAccounts::new(market, base_mint, quote_mint, TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID);
let seat = ix::claim_seat(&payer, &owner, &market, None);
let dep = ix::deposit(&owner, &accts, NO_HINT, &base_mint, &owner_base_ata, 1_000_000_000);
let quotes = ix::batch_update(&owner, &market, NO_HINT, CancelAll::Both, &[], &[
    OrderArgs::limit(Side::Bid as u8, 149_900, 100),
    OrderArgs::limit(Side::Ask as u8, 150_100, 100),
]);
```

### Snapshots and quotes

```rust
use matchbook_sdk::{engine::Side, MarketSnapshot};

let snap = MarketSnapshot::new(market, &account.data)?;
let accts = snap.accounts()?;                 // vaults and token programs from the header
let book = snap.levels(Side::Ask, now, 10)?;
let seat = snap.seat(&owner)?;                // Some((index, balances))
let out = snap.quote_exact_in(Side::Bid, 30_000_000, now)?;
println!("{} base atoms for {} quote atoms", out.base_atoms, out.quote_net(Side::Bid));
```

`now` is the cluster's unix time. It matters only when the book holds orders that expire,
which a quote must skip exactly as the program does.

### Jupiter

With the `jupiter` feature, `jupiter::MatchBookAmm` implements `jupiter_amm_interface::Amm`:

- `get_accounts_to_update` returns only the market account; mints, vaults and token programs
  are derived from its header.
- `quote` handles exact-in swaps in both directions, fees in the quote mint, using the clock
  from `AmmContext`.
- `get_swap_and_account_metas` returns the accounts of MatchBook's `Swap` instruction.
- `is_active` is false for permissioned or inactive markets.

Jupiter identifies each DEX by a variant of its `Swap` enum, which only Jupiter can add. Until
it adds one for MatchBook the adapter reports a placeholder (`jupiter::PLACEHOLDER_SWAP`);
`MatchBookAmm::swap_instruction` builds the full instruction for routers that compose
transactions themselves. `sdk/tests/jupiter.rs` checks quotes against executed swaps in
LiteSVM, including orders that expire between placement and quote.

# MatchBook

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

MatchBook is a central limit order book for Solana. It has no crank, costs less compute
than Manifest, Phoenix and OpenBook v2 on every benchmarked operation, and gives traders
order types the others do not have: reverse orders that always re-post, stop orders and
one-cancels-other brackets that fire without keepers, and scoped session keys.

**[Try it in the browser](playground/README.md):** the playground runs the real matching
engine, compiled to WebAssembly, on a SOL/USDC market.

## Why MatchBook

| Compute units per transaction | MatchBook | Manifest | Phoenix | OpenBook v2 |
| --- | ---: | ---: | ---: | ---: |
| Open a trading account | **504** | 710 | 16,241 | 16,914 |
| Limit order on a 2,000-order book | **1,285** | 1,881 | 18,332 | 17,256 |
| Cancel | **615** | 814 | 9,288 | 5,550 |
| IOC, 1 fill | **1,660** | 1,850 | 17,041 | 18,419 |
| Swap from a wallet, 1 fill | **4,487** | 9,031 | 22,181 | 30,610 |
| Requote: cancel 10, place 10 | **11,799** | 12,643 | 59,359 | 107,768 |

Every program measured in the same LiteSVM runtime, built from source; MatchBook's
numbers include its fees. Full table and method: [docs/benchmarks.md](docs/benchmarks.md).

| | MatchBook | Manifest | Phoenix | OpenBook v2 |
| --- | --- | --- | --- | --- |
| No crank | Yes | Yes | Yes | No: event heap |
| Anyone can open a seat | Yes | Yes | Authority approves | Yes |
| Rent paid when creating a market | Header only, grows with use | Header only | Whole book | Whole book |
| Reverse orders | Yes, flips guaranteed | Yes | No | No |
| Stop orders and OCO brackets on chain | Yes, no keeper | No | No | No |
| Session keys with scoped permissions | Yes, up to 8, expiring | No | No | One delegate |
| Referral fee sharing | Yes | No | No | Yes |
| Token-2022 mints | Yes | Yes | No | No |
| Swap from a wallet with no account | Yes | Yes | Yes | Yes |
| Oracle-pegged orders | No | No | No | Yes |

## Features

- **Crankless matching.** Makers are paid inside the taker's transaction. There is no event
  queue to drain and nothing for anyone to run.
- **One account per market that grows.** A market starts as a 400-byte header plus as many
  88-byte nodes as you preallocate, and grows by itself when it fills. Seats, orders, stops
  and delegates all live in it.
- **Order types.** Limit, post-only, post-only that slides behind the book, immediate or
  cancel, fill or kill, quote budgets for market buys, expiry times, client order ids and
  three self-trade behaviors.
- **Reverse orders.** A reverse order re-posts each fill on the other side of the book, a
  set distance from the fill price, which turns a ladder of orders into a market-making
  grid. MatchBook reserves the node for the flip when the order rests, so a flip never
  fails for lack of space.
- **Stops and brackets.** A stop locks its funds and waits for a trade at its trigger price.
  The trade that reaches the trigger places it, so no keeper is needed (a permissionless
  instruction places any backlog). Two stops can form a one-cancels-other bracket: a
  stop-loss and a take-profit around a position.
- **Session keys.** A seat owner can let up to 8 other keys trade or only cancel for it,
  each with an optional expiry. Delegates can never withdraw.
- **Swaps without an account.** Any wallet can trade against the book straight from its
  token accounts, with a minimum-out guard, which is what aggregators need.
- **Fees.** Taker fee, maker rebate and a referral share of the taker fee for the frontend
  that brought the trade.
- **Permissioned markets** when a market authority must approve each seat.
- **Batch requotes** of up to 32 cancels and 32 orders in one instruction.

## Repository

| Path | What it is |
| --- | --- |
| [`engine/`](engine) | The order book: a `no_std`, allocation-free Rust library over a byte buffer. The program, SDKs and playground all run this code. |
| [`program/`](program) | The Solana program, built with Pinocchio. |
| [`sdk/`](sdk) | Rust client: instructions, market snapshots, exact quotes and a Jupiter adapter. |
| [`ts-sdk/`](ts-sdk) | TypeScript client on `@solana/kit`. |
| [`playground/`](playground) | The engine in WebAssembly with a browser UI. |
| [`docs/`](docs) | Architecture, instruction reference, SDK guide and benchmarks. |

## Quick start

```bash
# Build the program and run every test
cargo build-sbf --manifest-path program/Cargo.toml
cargo test --workspace --all-features
(cd ts-sdk && npm install && npm test)

# Deploy to devnet and create a demo market
./scripts/deploy.sh devnet
cd ts-sdk && RPC_URL=https://api.devnet.solana.com npx tsx examples/demo-market.ts
```

Trading from TypeScript:

```ts
import { OrderType, Side, decodeMarket, placeOrder, swap } from '@matchbook/sdk';

const market = decodeMarket(address, accountData);
const seat = market.findSeat(owner.address)!.index;

// Rest a post-only bid that flips into an ask 0.2% higher whenever it fills.
const ix = placeOrder({
  owner, market: address, seat,
  order: { side: Side.Bid, price: 149_900n, baseLots: 100n, orderType: OrderType.PostOnly, reverseSpread: 2_000 },
});

// Quote a swap exactly, then send it with the quote as the minimum out.
const q = market.quoteSwap({ side: Side.Bid, baseLots: 1_000_000n, quoteBudget: 30_000_000n });
```

Quoting from Rust:

```rust
use matchbook_sdk::{engine::Side, MarketSnapshot};

let snap = MarketSnapshot::new(market, &account_data)?;
let out = snap.quote_exact_in(Side::Bid, 30_000_000, now)?;
```

More in [docs/getting-started.md](docs/getting-started.md) and [docs/sdk-guide.md](docs/sdk-guide.md).

## Documentation

- [Getting started](docs/getting-started.md): build, test, deploy and make a first market.
- [Architecture](docs/architecture.md): the account layout, matching, reverse orders, stops
  and how the compute budget is kept low.
- [Instruction reference](docs/instructions.md): every instruction, its accounts, data and errors.
- [SDK guide](docs/sdk-guide.md): TypeScript, Rust and Jupiter integration.
- [Benchmarks](docs/benchmarks.md): the full comparison and how to reproduce it.

## Status

MatchBook has not been audited. Use it on devnet, and on mainnet only with funds you can
afford to lose, until an audit is published.

## License

MIT. See [LICENSE](LICENSE).

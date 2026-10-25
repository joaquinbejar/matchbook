# MatchBook

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Solana](https://img.shields.io/badge/Solana-Pinocchio-9945FF.svg)](https://github.com/anza-xyz/pinocchio)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-%40solana%2Fkit-3178C6.svg)](https://github.com/anza-xyz/kit)

MatchBook is a central limit order book for Solana. It has no crank, costs less compute
than Manifest, Phoenix and OpenBook v2 on every benchmarked operation, and gives traders
order types the others do not have: reverse orders that always re-post, stop orders and
one-cancels-other brackets that fire without keepers, and scoped session keys.

**[Try it in the browser](playground/README.md):** the playground runs the real matching
engine, compiled to WebAssembly, on a SOL/USDC market.

## Table of contents

- [Why MatchBook](#why-matchbook)
- [Features](#features)
- [How it works](#how-it-works)
- [Repository](#repository)
- [Requirements](#requirements)
- [Build and test](#build-and-test)
- [Deploying to devnet](#deploying-to-devnet)
- [Using MatchBook](#using-matchbook)
- [Documentation](#documentation)
- [Status](#status)
- [Contribution and Contact](#contribution-and-contact)
- [License](#license)
- [Related projects](#related-projects)

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
- **Token-2022.** Mints with transfer fees are supported; `swap`'s minimum out is checked
  on what actually arrives in the wallet.

## How it works

A market is a single program-owned account: a 400-byte header followed by a pool of
88-byte nodes. Orders, seats, stops and delegates are all nodes, linked into red-black
trees (one per book side, one per stop direction) and a free list. Because the engine works
directly on that byte buffer with no allocation, the same Rust code runs on chain, in the
SDKs' exact quotes and in the browser playground.

When a taker crosses the book, every fill credits the maker's seat inside the same
transaction, so there is no event queue and no crank. Reverse orders reserve the node their
flip will need when they rest. Stops sit in their own trees; the trade that reaches a
trigger places up to two of them, and the permissionless `TriggerStops` instruction places
any backlog. Vaults are owned by a per-market PDA, and seats hold free and locked balances
so funds never leave the vaults between trades.

The full layout, matching loop and compute budget tricks are described in
[docs/architecture.md](docs/architecture.md).

## Repository

| Path | What it is |
| --- | --- |
| [`engine/`](engine) | The order book: a `no_std`, allocation-free Rust library over a byte buffer. The program, SDKs and playground all run this code. |
| [`program/`](program) | The Solana program, built with Pinocchio. |
| [`sdk/`](sdk) | Rust client: instructions, market snapshots, exact quotes and a Jupiter adapter. |
| [`ts-sdk/`](ts-sdk) | TypeScript client on `@solana/kit`. |
| [`client/`](client) | Rust CLI (`matchbook`) and market-making bot (`matchbook-mm`). |
| [`ts-client/`](ts-client) | The same CLI and bot in TypeScript. |
| [`playground/`](playground) | The engine in WebAssembly with a browser UI. |
| [`scripts/`](scripts) | Deployment, end-to-end client tests and competitor benchmark builds. |
| [`docs/`](docs) | Architecture, instruction reference, SDK guide and benchmarks. |

## Requirements

| Tool | Version | Used for |
| --- | --- | --- |
| Rust | stable (see `rust-toolchain.toml`) | Engine, program, Rust SDK and clients |
| Solana CLI (Agave) | 2.x or 3.x, with `cargo build-sbf` | Building and deploying the program |
| Node.js | 20 or later | TypeScript SDK and clients |
| `wasm32-unknown-unknown` target | optional | The playground |

Install the Solana CLI from Anza and check that the SBF toolchain is available:

```bash
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
solana --version
cargo build-sbf --version
```

## Build and test

```bash
make program        # cargo build-sbf: writes target/deploy/matchbook_program.so
make test           # Rust workspace, TypeScript SDK and TypeScript client tests
make lint           # clippy with -D warnings, eslint and tsc
make fmt-check      # rustfmt
make bench          # compute unit benchmarks
```

Or the same steps by hand:

```bash
cargo build-sbf --manifest-path program/Cargo.toml
cargo test --workspace --all-features
(cd ts-sdk && npm install && npm test)
(cd ts-client && npm install && npm test)
```

The program and SDK tests run the compiled program in [LiteSVM](https://github.com/LiteSVM/litesvm),
so build the `.so` first. The engine's model test compares the engine with a simple
reference implementation over random operation sequences; raise the number of cases with
`MODEL_CASES=20000 cargo test -p matchbook-engine --test model`.

## Deploying to devnet

This walks through a full devnet deployment: wallet, program, clients, a first market and a
market-making bot. The same steps work on a local validator (`localhost`) and, with real
care, on `mainnet-beta`.

### 1. Prepare a wallet

The Solana CLI wallet pays for the deployment and becomes the program's upgrade authority.

```bash
solana config set --url devnet
solana-keygen new -o ~/.config/solana/id.json      # skip if you already have one
solana config set --keypair ~/.config/solana/id.json
solana address
solana airdrop 2                                    # repeat if you need more
solana balance
```

The devnet faucet is rate limited. If `solana airdrop` fails, use
<https://faucet.solana.com> with the address printed by `solana address`.

Deploying needs enough SOL to pay rent for the program account, which is proportional to
the size of the `.so`, plus a temporary buffer during the upload (it is refunded at the end).
Check the cost before you start:

```bash
make program
solana rent $(wc -c < target/deploy/matchbook_program.so)
```

Keep a few SOL more than that figure to cover transaction fees and a retry.

### 2. Choose the program address

The address is the public key of `target/deploy/matchbook_program-keypair.json`.

- **First deployment.** You do not have the keypair behind the address declared in the
  repository, so `scripts/deploy.sh` generates a new one. It then writes the new address
  into `program/src/lib.rs` and `ts-sdk/src/constants.ts` before building, so the program
  and both SDKs agree (the Rust SDK reads the address from the program crate).
- **Redeploying or upgrading.** Keep `matchbook_program-keypair.json` and the script will
  deploy to the same address. `target/` is ignored by git, so back the file up somewhere
  safe: losing it means you can never upgrade that program again.
- **A vanity address.** Generate one with `solana-keygen grind --starts-with Mb:1` and
  copy it to `target/deploy/matchbook_program-keypair.json` before deploying.

### 3. Deploy the program

```bash
./scripts/deploy.sh devnet          # or: make deploy-devnet
```

The script checks that `solana`, `solana-keygen` and `cargo-build-sbf` are installed,
creates or reuses the program keypair, builds the program, prints the payer and its
balance, and runs `solana program deploy`. It also accepts `localhost`, `mainnet-beta` or
any RPC URL.

Check the result:

```bash
PROGRAM_ID=$(solana-keygen pubkey target/deploy/matchbook_program-keypair.json)
solana program show $PROGRAM_ID --url devnet
```

`Authority` must be your wallet and `Data Length` the size of the `.so`.

### 4. Build the clients for your address

The program address is compiled into the SDKs, so rebuild the clients after the script
has changed it:

```bash
# Rust CLI and bot
cargo install --path client --force

# TypeScript SDK, CLI and bot
(cd ts-sdk && npm install && npm run build)
(cd ts-client && npm install && npm run build && npm link)
```

### 5. Create a first market

The quickest path is the demo script. It creates two test mints and a market, gives the
payer a seat, quotes a grid of reverse orders and runs a swap from a fresh wallet at an
exact quote:

```bash
cd ts-sdk
RPC_URL=https://api.devnet.solana.com npx tsx examples/demo-market.ts
```

Or step by step with the CLI (Rust and TypeScript behave the same):

```bash
export MATCHBOOK_RPC=devnet

BASE=$(matchbook token create-mint --decimals 9)
QUOTE=$(matchbook token create-mint --decimals 6)
matchbook token mint $BASE 1000
matchbook token mint $QUOTE 200000

# 0.001 base per lot and 0.000001 quote per tick, 10 bps taker fee and 2 bps maker rebate
matchbook market create --base-mint $BASE --quote-mint $QUOTE \
  --base-lot-size 1000000 --quote-lot-size 1 --taker-fee-bps 10 --maker-rebate-bps 2
MARKET=...                          # printed by market create

matchbook market show $MARKET
matchbook seat claim $MARKET
matchbook deposit $MARKET --base 500 --quote 100000
matchbook place $MARKET buy 2 149 --type post-only
matchbook place $MARKET sell 2 151 --type post-only
matchbook book $MARKET
```

[Choosing lot sizes](docs/getting-started.md#choosing-lot-sizes) explains how the lot sizes
set the tick and the minimum order size.

### 6. Run the market maker

Give a session key a little SOL for fees and let the bot quote through it, so the bot never
holds the key that can withdraw:

```bash
matchbook session create $MARKET --out bot.json --expires-in 7d --fund 0.1
matchbook-mm --market $MARKET --session-keypair bot.json --owner $(matchbook address) \
  --price random:150:20 --levels 3 --spread-bps 8 --step-bps 6 --size 1 \
  --max-position 20 --max-notional 5000 --max-loss 200 --interval-ms 1500
```

Swap against it from another terminal with `matchbook swap $MARKET buy 100`. Ctrl-C makes
the bot cancel its quotes before it exits. Every option is described in
[docs/clients.md](docs/clients.md#market-maker).

### 7. Upgrades and operations

- **Upgrade.** Run `./scripts/deploy.sh devnet` again with the same program keypair and
  wallet. Existing markets keep working as long as the account layout does not change.
- **Freeze the program.** `solana program set-upgrade-authority $PROGRAM_ID --final`
  makes it immutable. This cannot be undone.
- **Market administration.** The market authority can run
  `matchbook market set-status $MARKET active|cancel-only|closed` and
  `matchbook market collect-fees $MARKET`. Closing requires an empty book with no stops.
- **Housekeeping.** `matchbook market prune $MARKET` removes expired orders and
  `matchbook market trigger-stops $MARKET` places any backlog of reached stops. Both are
  permissionless.

### Troubleshooting

| Problem | What to do |
| --- | --- |
| `cargo-build-sbf not found` | Install the Solana CLI from Anza and make sure its `bin` directory is on `PATH`. |
| `airdrop request failed` | The faucet is rate limited: wait, use <https://faucet.solana.com>, or transfer SOL from another wallet. |
| `insufficient funds` during deploy | Check `solana rent` for the `.so` size and top up the wallet. |
| A deploy stopped halfway | The upload is kept in a buffer. List it with `solana program show --buffers` and either resume with `solana program deploy --buffer <BUFFER> --program-id target/deploy/matchbook_program-keypair.json target/deploy/matchbook_program.so` or reclaim the SOL with `solana program close --buffers`. |
| Many failed transactions while uploading | The public RPC is congested. Deploy through a dedicated RPC URL (`./scripts/deploy.sh https://...`) or add `--with-compute-unit-price` to `solana program deploy`. |
| Clients send to the wrong program or find no market | The SDKs were built for another address. Rebuild them as in step 4. |

### Testing on a local validator

To try everything without devnet, start `solana-test-validator` in another terminal and
deploy to `localhost`, or run the end-to-end suite, which starts a validator with the
program preloaded and runs every CLI command and both bots against each other:

```bash
make program
./scripts/e2e-clients.sh
```

## Using MatchBook

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

From the command line:

```bash
matchbook place $MARKET buy 1 149.9 --type post-only --reverse-spread-bps 20
matchbook stop $MARKET sell 1 --trigger 140 --when below
matchbook bracket $MARKET sell 1 --stop-loss 139 --take-profit 170
matchbook swap $MARKET sell 0.5 --slippage-bps 30
matchbook withdraw $MARKET --all
```

More in [docs/getting-started.md](docs/getting-started.md), [docs/sdk-guide.md](docs/sdk-guide.md)
and [docs/clients.md](docs/clients.md).

## Documentation

- [Getting started](docs/getting-started.md): build, test, deploy and make a first market.
- [Architecture](docs/architecture.md): the account layout, matching, reverse orders, stops
  and how the compute budget is kept low.
- [Instruction reference](docs/instructions.md): every instruction, its accounts, data and errors.
- [SDK guide](docs/sdk-guide.md): TypeScript, Rust and Jupiter integration.
- [Clients and bots](docs/clients.md): the `matchbook` CLI and the `matchbook-mm` market maker.
- [Benchmarks](docs/benchmarks.md): the full comparison and how to reproduce it.
- [Playground](playground/README.md): the engine in the browser.

## Status

MatchBook has not been audited. Use it on devnet, and on mainnet only with funds you can
afford to lose, until an audit is published.

## Contribution and Contact

We welcome contributions to this project! If you would like to contribute, please follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bug fix.
3. Make your changes and ensure that the project still builds and all tests pass
   (`make all` runs the format check, the linters and every test).
4. Commit your changes and push your branch to your forked repository.
5. Submit a pull request to the main repository.

[CONTRIBUTING.md](CONTRIBUTING.md) has the full development workflow.

If you have any questions, issues, or would like to provide feedback, please feel free to contact the project maintainer:

### **Contact Information**

- **Author**: Joaquín Béjar García <https://joaquinbejar.taunais.com/>
- **Email**: jb@taunais.com
- **Telegram**: [@joaquin_bejar](https://t.me/joaquin_bejar)
- **Repository**: <https://github.com/joaquinbejar/matchbook>
- **Documentation**: [docs/](docs)

We appreciate your interest and look forward to your contributions!

## License

Licensed under **MIT** license. See [LICENSE](LICENSE).

<!-- related-projects:start -->
## Related projects

Repositories by the same author on order books, market making and Solana trading.

| Repository | Description |
|------------|-------------|
| [OrderBook-rs](https://github.com/joaquinbejar/OrderBook-rs) | High-performance, thread-safe limit order book in Rust with lock-free data structures. |
| [PriceLevel](https://github.com/joaquinbejar/PriceLevel) | Lock-free price level implementation, the building block of a limit order book. |
| [hft-clob-core](https://github.com/joaquinbejar/hft-clob-core) | Single-symbol CLOB matching engine with an integer-only hot path and deterministic replay. |
| [market-maker-rs](https://github.com/joaquinbejar/market-maker-rs) | Quantitative market making strategies, starting with the Avellaneda-Stoikov model. |
| [CLMM-Liquidity-Provider](https://github.com/joaquinbejar/CLMM-Liquidity-Provider) | Strategy optimization for liquidity providers on Solana CLMMs (Raydium, Orca, Meteora). |
| [DeFi-Arbitrage-Engine](https://github.com/joaquinbejar/DeFi-Arbitrage-Engine) | Real-time arbitrage engine for Solana DEXs with MEV protection. |
| [hydra-amm](https://github.com/joaquinbejar/hydra-amm) | Universal AMM engine: build, configure and operate any automated market maker. |

<!-- related-projects:end -->

# Getting started

## Requirements

- Rust (stable) and the Solana CLI with `cargo build-sbf` (Agave 2.x or 3.x)
- Node.js 20 or later for the TypeScript SDK
- Optional: the `wasm32-unknown-unknown` Rust target for the playground

## Build and test

```bash
cargo build-sbf --manifest-path program/Cargo.toml   # target/deploy/matchbook_program.so
cargo test --workspace --all-features                # engine, program, Rust SDK
(cd ts-sdk && npm install && npm test)               # TypeScript SDK against the built program
```

The program and SDK tests run the compiled program in [LiteSVM](https://github.com/LiteSVM/litesvm),
so build it first. The engine's model test compares the engine against a simple reference
implementation over random operation sequences; raise the number of cases with
`MODEL_CASES=20000 cargo test -p matchbook-engine --test model`.

## Deploy

```bash
solana config set --keypair ~/.config/solana/id.json   # the wallet that pays and owns the upgrade authority
./scripts/deploy.sh devnet                              # or localhost, mainnet-beta, or an RPC URL
```

The program address comes from `target/deploy/matchbook_program-keypair.json`. Keep that
file to redeploy to the same address. Without it the script generates a keypair and writes its
address into `program/src/lib.rs` and `ts-sdk/src/constants.ts` before building, so the program
and both SDKs agree (the Rust SDK takes the address from the program crate).

To try everything locally, start `solana-test-validator` in another terminal and deploy to
`localhost`.

## A first market

`ts-sdk/examples/demo-market.ts` creates two test mints and a market, gives the payer a seat,
quotes a grid of reverse orders and runs a swap from a fresh wallet at an exact quote:

```bash
cd ts-sdk
RPC_URL=https://api.devnet.solana.com npx tsx examples/demo-market.ts
```

```
market 8sUt6FSSJTs8gmkSyZ8WoBXRu9F5TAeER88VqAGKEVUH  base 211f…  quote 2uMm…
swap 4LQU…: quoted 199000000 base atoms for 29909680 quote atoms
  fill 100 lots at 150100
  reversed into a Bid of 100 lots at 149799
  fill 99 lots at 150200
  reversed into a Bid of 99 lots at 149899
asks 150200x1 150300x100 150400x100 150500x100
bids 149900x100 149899x99 149800x100 149799x100 149700x100
```

### Choosing lot sizes

Prices are integers in quote lots per base lot, so the lot sizes set the tick and the minimum
size. For SOL (9 decimals) against USDC (6 decimals):

| Choice | Value | Effect |
| --- | --- | --- |
| `base_lot_size` | 1,000,000 | Sizes in steps of 0.001 SOL |
| `quote_lot_size` | 1 | One price unit = 0.000001 USDC per 0.001 SOL = 0.001 USDC per SOL |
| Price 150,000 | | 150.000 USDC per SOL |

A smaller base lot gives finer sizes but coarser prices for the same quote lot, and the
reverse. Pick the tick you want to quote in first.

### Fees

`taker_fee_bps` is charged on the quote amount of every taker fill. `maker_rebate_bps` goes to
the maker from that fee, `referral_share_bps` of the fee goes to a referrer when one is
passed, and the rest accrues to the market for the authority to collect.

## Next

- [SDK guide](sdk-guide.md) for trading, quoting and decoding events from code.
- [Instruction reference](instructions.md) for the wire format.
- [Architecture](architecture.md) for how the book works.
- The [playground](../playground/README.md) to try order types without a validator.

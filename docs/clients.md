# Command-line clients and market-making bots

MatchBook ships two complete clients with the same commands, one in Rust
([`client/`](../client)) and one in TypeScript ([`ts-client/`](../ts-client)). Each has:

- `matchbook`, a CLI that creates markets, moves funds, places and cancels every kind of
  order (limit, post-only, post-only slide, IOC, fill or kill, reverse, stops and OCO
  brackets), swaps without a seat, reads the book and balances, and manages session keys.
- `matchbook-mm`, a market maker that quotes a ladder around a reference price, requotes
  atomically with one `BatchUpdate`, trades with a session key and enforces risk limits.

Both talk to any cluster through its RPC, and both read the Solana CLI keypair format.

## Install

Rust:

```bash
cargo install --path client      # installs matchbook and matchbook-mm
# or run from the workspace:
cargo run -p matchbook-client --bin matchbook -- --help
```

TypeScript (Node 20+):

```bash
(cd ts-sdk && npm install && npm run build)
cd ts-client && npm install && npm run build
node dist/cli.js --help           # or: npx tsx src/cli.ts --help
npm link                          # optional: puts matchbook and matchbook-mm on PATH
```

If both are installed, the one later on `PATH` wins; they behave the same.

## Connecting

| Option | Env var | Default |
| --- | --- | --- |
| `-u, --url` | `MATCHBOOK_RPC` | `localnet`. Also `devnet`, `mainnet`, `testnet` or any RPC URL. |
| `-k, --keypair` | `MATCHBOOK_KEYPAIR` | `~/.config/solana/id.json` |
| `--cu-limit` | | runtime default |
| `--priority-fee` | | 0 micro-lamports per compute unit |
| `--json` (CLI only) | | machine-readable output for `market show`, `book`, `balance`, `orders`, `market create` |

## Units

Commands take prices in **quote tokens per base token** and sizes in **base tokens**, and
convert them to the market's lots exactly. A price is rounded to the nearest tick, a size
down to a whole lot, and both commands refuse values below one tick or one lot. `market
show` prints the lot and tick.

## CLI walkthrough

```bash
export MATCHBOOK_RPC=devnet
matchbook address
matchbook airdrop 2

# Test tokens and a market: 0.001 base per lot, 0.000001 quote per tick.
BASE=$(matchbook token create-mint --decimals 9)
QUOTE=$(matchbook token create-mint --decimals 6)
matchbook token mint $BASE 1000
matchbook token mint $QUOTE 200000
matchbook market create --base-mint $BASE --quote-mint $QUOTE \
  --base-lot-size 1000000 --quote-lot-size 1 --taker-fee-bps 10 --maker-rebate-bps 2
MARKET=...                                   # printed by market create
matchbook market show $MARKET

# A seat holds your balances on the market.
matchbook seat claim $MARKET
matchbook deposit $MARKET --base 500 --quote 100000
matchbook balance $MARKET                    # free, locked in orders, and in the wallet

# Orders.
matchbook place $MARKET sell 2 151 --type post-only --client-id 11
matchbook place $MARKET buy 2 149
matchbook place $MARKET buy 1 148 --type post-only --reverse-spread-bps 50   # flips to an ask 0.5% up when filled
matchbook place $MARKET sell 1 160 --expires-in 1h
matchbook place $MARKET buy 1 150.5 --type ioc
matchbook place $MARKET buy 5 150 --type fok
matchbook book $MARKET
matchbook orders $MARKET
matchbook reduce $MARKET <ORDER_ID> 1.5      # keeps time priority
matchbook cancel $MARKET <ORDER_ID>
matchbook cancel-client $MARKET 11
matchbook cancel-all $MARKET --side sell

# Stops: placed by the trade that reaches the trigger, no keeper needed.
matchbook stop $MARKET sell 1 --trigger 140 --when below          # IOC up to 1% past the trigger
matchbook stop $MARKET buy 1 --trigger 160 --when above --limit 161
matchbook bracket $MARKET sell 1 --stop-loss 139 --take-profit 170  # OCO pair
matchbook market trigger-stops $MARKET                             # permissionless, if more are due

# Seatless swaps from the wallet, with an exact quote and a minimum out.
matchbook swap $MARKET buy 100          # spend 100 quote
matchbook swap $MARKET sell 0.5 --slippage-bps 30

# Out again.
matchbook withdraw $MARKET --base 1 --quote 10
matchbook withdraw $MARKET --all
```

Market authority commands: `market set-status <market> active|cancel-only|closed`,
`market collect-fees <market> [--to <quote token account>]`, and the permissionless
`market prune <market>` for expired orders. On a permissioned market the authority signs
each seat: `seat claim <market> --authority-keypair <path>`.

### Session keys

A session key is a delegate of your seat: it can place, reduce and cancel orders (or only
cancel, with `--cancel-only`) until it expires, and it can never withdraw or add delegates.

```bash
matchbook session create $MARKET --out bot.json --expires-in 7d --fund 0.1
matchbook session list $MARKET
matchbook place $MARKET buy 1 149 --signer bot.json --owner <YOUR_ADDRESS>
matchbook session revoke $MARKET <SESSION_ADDRESS>
```

`--fund` sends the key SOL for fees, so the bot never needs the main wallet.

## Market maker

```bash
matchbook session create $MARKET --out bot.json --expires-in 7d --fund 0.1
matchbook-mm --market $MARKET --session-keypair bot.json --owner <YOUR_ADDRESS> \
  --price http:https://api.coinbase.com/v2/prices/SOL-USD/spot#data.amount \
  --levels 3 --spread-bps 8 --step-bps 6 --size 1 --size-growth 1.5 \
  --max-position 20 --skew-bps 15 --max-notional 5000 --max-loss 200 \
  --requote-bps 3 --interval-ms 1500
```

Each cycle the bot reads the market and the reference price, and:

1. Builds a ladder: the first bid and ask `--spread-bps` from the reference, then a level
   every `--step-bps`, each `--size-growth` times the previous size.
2. Skews it for inventory: holding more base than `--target-base` (default: what the seat
   holds at start) moves the ladder down by up to `--skew-bps` at `--max-position`, so it
   sells more and buys less; holding less moves it up.
3. Caps it: past `--max-position` from the target it stops quoting the side that would
   grow the position, never quotes more than the seat holds, and commits at most
   `--max-notional` quote tokens per side.
4. Requotes only when the reference moved `--requote-bps` or one of its quotes filled. A
   requote is a `BatchUpdate` that cancels the bot's quotes and places the new ones, all
   post-only-slide, so a quote that would cross reprices behind the book instead of
   failing. A ladder too large for one transaction continues in more `BatchUpdate`s.
   Quotes use only free funds and what the bot's own cancels release, never funds locked
   in the seat's other orders or stops.

Risk limits:

| Option | Effect |
| --- | --- |
| `--max-loss` | Cancels everything and exits once equity (base marked at the reference, plus quote) falls this far below its start. |
| `--max-price-age` | Pulls quotes while the reference is older than this many seconds (a dead feed). |
| `--max-jump-bps` | Pulls quotes for a cycle when the reference jumps this far at once. |
| `--max-position`, `--max-notional` | Above. |

Price sources (`--price`):

| Spec | Reference |
| --- | --- |
| `fixed:150` | A constant. |
| `random:150:20` | A random walk from 150 in steps of up to 20 bps (demos and load tests). |
| `book` | The market's last trade, else its mid. |
| `http:<url>#<path>` | A number (or numeric string) at a dotted JSON path, e.g. `http:https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDC#price`. |

The bot tags its orders with `--client-id` (default `0x4d4d`) and only touches those, so
you can keep manual orders on the same seat. Ctrl-C cancels its quotes before exiting
(`--keep-quotes` leaves them). `--dry-run` prints the ladder without sending, and
`--cycles N` stops after N cycles. Without `--session-keypair` the bot trades as the
`--keypair` wallet itself.

The Rust and TypeScript bots run the same quoting and risk code paths and share their unit
tests (`client/src/mm.rs`, `ts-client/tests/mm.test.ts`), so they quote identically.

## Testing on a local validator

`scripts/e2e-clients.sh` starts `solana-test-validator` with the program preloaded and runs
every command of each CLI on its own market. Each bot then quotes through a session key
while the other language's CLI swaps against it, checking that fills are seen, quotes are
replaced and pulled on exit, and that cancel-only markets reject new orders.

```bash
cargo build-sbf --manifest-path program/Cargo.toml
./scripts/e2e-clients.sh
```

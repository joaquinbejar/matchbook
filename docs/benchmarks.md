# Benchmarks

Compute units per transaction for the same scenarios on MatchBook and on the three
established Solana order books, each built from source and run in the same
[LiteSVM](https://github.com/LiteSVM/litesvm) runtime. Lower is better.

| Operation | MatchBook | Manifest | Phoenix v1 | OpenBook v2 |
| --- | ---: | ---: | ---: | ---: |
| Open a trading account (seat) | **507** | 710 | 16,241 ¹ | 16,914 |
| Deposit | **1,860** | 3,231 | 19,237 | 14,440 |
| Limit order, rests on an empty book | **953** | 1,231 | 16,632 | 15,436 |
| Limit order, rests on a 2,000-order book | **1,288** | 1,881 | 18,332 | 17,256 ² |
| Post-only order, 2,000-order book | **1,258** | 1,774 | 16,674 | 17,752 ² |
| Cancel the newest order | **614** | 814 ³ | 9,288 | 5,550 |
| IOC, 1 fill | **1,660** | 1,850 | 17,041 | 18,419 |
| IOC, sweeps 5 orders | **7,035** | 7,168 | 24,423 | 35,566 |
| IOC, sweeps 20 orders | **20,541** | 21,281 | 46,651 | 86,744 |
| Limit order, 2 fills then rests | **3,472** | 3,483 | 19,824 | 21,723 |
| Swap from a wallet, no account, 1 fill | **4,487** | 9,031 | 22,181 | 30,610 |
| Swap from a wallet, no account, 5 fills | **8,487** | 12,011 | 28,535 | 46,336 |
| Requote: cancel 10 + place 10 | **11,802** | 12,643 | 59,359 ⁴ | 107,768 ⁵ |
| Withdraw | **1,738** | 3,455 | 14,770 | 19,118 |
| Settle makers after fills (crank) | **0** | 0 | 0 | 28,552 per 8 fills ⁶ |

MatchBook charges a 10 bps taker fee and pays a 2 bps maker rebate in these runs; Manifest
charges no fees, and Phoenix and OpenBook charge 5 bps. Fee accounting is included in every
MatchBook number above.

1. Phoenix seats are permissioned: this is the trader's request, and the market authority must
   then approve it in a second transaction.
2. An OpenBook v2 book side holds about 500 orders and an open-orders account holds 24, so its
   deep book has 961 orders spread over 40 accounts.
3. With the order's index as a hint. Without it Manifest scans the book: 92,809.
4. Phoenix has no cancel-and-replace instruction; the cancel and the placement are two
   instructions in one transaction.
5. `cancel_all_and_place_orders`.
6. OpenBook v2 pays makers through an event heap that a crank must drain, at most 8 events per
   call. Every fill above eventually costs this too. MatchBook, Manifest and Phoenix settle
   makers inside the taker's transaction.

## Method

- Every program runs the same sequence: create a market, open accounts for a market maker and
  a taker, deposit, build a book of 1,000 bids and 1,000 asks in batches of 8, then the
  measured operations in table order. The book is the same shape for every program (prices in
  ticks of one quote lot per base lot, 1 base lot = 0.001 SOL, 1 quote lot = 0.000001 USDC).
- MatchBook's market has 4,000 nodes preallocated, Manifest's 4,000 blocks, so neither pays for
  account growth in a measured operation. Phoenix and OpenBook allocate their books up front.
- Numbers are whole-transaction compute units as reported by the runtime, including the
  program's own event logging.

## Reproducing

```bash
# MatchBook
cargo build-sbf --manifest-path program/Cargo.toml
cargo test -p matchbook-program --test bench -- --nocapture

# Competitors, built from source at pinned commits
./scripts/build-competitors.sh
cargo test -p matchbook-program \
  --test compare_manifest --test compare_phoenix --test compare_openbook -- --nocapture
```

`scripts/build-competitors.sh` pins Manifest at `51f8083`, Phoenix v1 at `6ae54af` and
OpenBook v2 at `f3e1742`, and builds each with the platform tools of its own era: OpenBook v2
overflows its stack frames under newer compilers, and Phoenix links solana-program 1.14,
whose cross-program-invocation ABI predates today's runtime. Phoenix gets a small patch
(`scripts/competitors/phoenix-stable-cpi.patch`) that routes its CPIs through the stable
layout; it changes no logic and adds a few hundred compute units per CPI at most.

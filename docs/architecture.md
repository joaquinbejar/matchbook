# Architecture

MatchBook is three layers over one piece of code:

- **`matchbook-engine`** is the order book itself: a `no_std` Rust library with no
  allocator that operates on a byte buffer. It knows nothing about Solana.
- **`matchbook-program`** is a thin Pinocchio program that maps instructions onto engine
  calls, moves tokens and logs events.
- **Clients** (the Rust SDK, the TypeScript SDK, the Jupiter adapter and the WebAssembly
  playground) decode the same buffer. The Rust clients run the engine itself, so a quote
  is computed by the code that will execute the trade.

## The market account

A market is one account owned by the program:

```
┌──────────────── 400-byte header ────────────────┬──── node pool: N × 88 bytes ────┐
│ mints, vaults, authority, lot sizes, fees,      │ order │ seat │ free │ stop │ ... │
│ tree roots, best/worst order cache, free list,  │       │      │      │      │     │
│ counters, last trade price, stop trigger cache  │       │      │      │      │     │
└─────────────────────────────────────────────────┴─────────────────────────────────┘
```

Every node is 88 bytes and starts with the same tree prefix (tag, flags, left, right,
parent). The tag says what the node holds:

| Tag | Node | Lives in |
| --- | --- | --- |
| Order | A resting order: price, size, owner seat, client id, expiry, sequence number, reverse spread | The bids or asks tree, and its seat's order list |
| Trader | A seat: owner key and free base and quote balances | The traders tree |
| Delegate | A key that may act for a seat, with permissions and an expiry | The traders tree |
| Stop | An order waiting for a trigger price | The stops-above or stops-below tree, and its seat's order list |
| Free | A recycled node | The free list |

All links are `u32` node indices, so the account can be moved or grown without fixing up
pointers. Creating a market pays rent for the header and the nodes you preallocate.
When an instruction needs more nodes than are free, the program reallocates the account
and charges the extra rent to the signer, so a market costs what it holds.

### Order ids

An order id is `sequence << 24 | node index`. The sequence number comes from a market
counter, so ids never repeat, and the index makes cancel by id a direct lookup: the program
reads the node, checks that its sequence matches and that it belongs to the signer's seat,
and unlinks it. No search.

### Trees

Bids, asks, seats and both stop books are red-black trees keyed by `u128`:

| Tree | Key | Best entry |
| --- | --- | --- |
| Asks | `price << 64 \| seq` | Minimum: lowest price, then oldest |
| Bids | `price << 64 \| !seq` | Maximum: highest price, then oldest |
| Traders | First 16 bytes of the owner key | Lookup by owner |
| Stops above | `trigger << 64 \| seq` | Minimum trigger |
| Stops below | `trigger << 64 \| !seq` | Maximum trigger |

The header caches the best and worst order of each side and their prices. Matching starts
at the cached best order and walks the tree in order. A new order that lands at either end
of its side, which is where most orders land, is attached to the cached edge instead of
being inserted from the root.

## Matching without a crank

A taker order walks the opposite side from the best price while it crosses. Each fill
updates both seats in the same instruction: the maker's locked funds become the taker's,
the maker gets the rebate and the market keeps the rest of the taker fee. Nothing is
deferred, so there is no event queue and no crank. OpenBook v2 instead writes each fill to
an event heap that a separate transaction must process before the maker is paid.

Funds live in seats. A resting order locks base (asks) or quote (bids) from its seat's free
balance, and a cancel unlocks it. Withdrawals move free balances back to the owner's token
accounts. A wallet without a seat can still trade through `Swap`, which matches like an
immediate-or-cancel order and settles straight to its token accounts.

Each operation validates everything before it changes anything, so an error leaves the
market exactly as it was. Expired orders met while matching are removed and skipped.

## Reverse orders

A reverse order carries a spread in parts per million. When it fills, the filled size is
re-posted on the other side: a filled bid at price P becomes an ask at
`ceil(P × (1 + spread))`, and a filled ask becomes a bid at `floor(P × (1 − spread))`.
Rounding is always away from the fill, so the maker never gets less than the spread.

Two details make the flip reliable:

- **Space is reserved up front.** A reverse order needs two nodes to rest, its own and one
  held for its partner. `header.reverse_reserve` counts the held nodes and other operations
  cannot use them, so a flip never fails because the market is full.
- **Partners merge.** An order and the order it flipped into are linked. Later fills of the
  original add size to the existing partner at the partner's price instead of creating a
  new order each time, so a grid keeps one order per rung.

## Stops

A stop stores an order and a trigger: "above" fires when a trade happens at or above the
trigger, "below" when one happens at or below it. Its funds are locked when it is placed:
base for a sell, and for a buy the notional, the taker fee and, on a market with a fee, one
atom a fill for its rounding. So a triggered order never fails for lack of funds.

The header caches the lowest above-trigger and the highest below-trigger, so checking
whether a trade set anything off is two comparisons. When it did, the instruction that
traded places up to two triggered stops itself before returning. Any backlog is placed by
`TriggerStops`, which anyone may call. No keeper network is involved.

A trigger stays reached once a trade reaches it. The header keeps the highest and lowest
trade prices since the stops last caught up, so a stop still fires when a triggered order,
or a later trade while a backlog waits, moves the price back across its trigger.

Two stops of one seat can be paired one-cancels-other. When either fires, the other is
removed with reason `OcoCancelled`. A stop-loss and a take-profit around one position is
the usual pair.

## Events

Each instruction logs at most one `sol_log_data` entry with its events packed back to back:

| Tag | Event | Bytes | Fields |
| --- | --- | --- | --- |
| 1 | Fill | 73 | maker, maker order id, maker client id, price, base lots, maker remaining |
| 2 | Out | 67 | owner, order id, client id, side, price, base lots, reason |
| 3 | Reversed | 74 | owner, new or merged order id, client id, side, price, base lots, expiry |

Out reasons are expired, self-trade, triggered and OCO-cancelled; fills and the owner's own
cancels are implied by the instruction. The taker's identity, the quote amounts and fees
follow from the instruction and the market's lot sizes and fee rates, so they are not
repeated per fill. Both SDKs decode these logs.

## Vaults and tokens

Each market's vaults are the associated token accounts of one PDA,
`["vault_authority", market]`. Transfers use `transfer_checked`, so both SPL Token and
Token-2022 mints work; the program checks that the vault received what it expected, which
rejects mints with transfer fees.

## Keeping compute low

The benchmarks in [benchmarks.md](benchmarks.md) come from choices like these:

- One account and index links, so every instruction touches the fewest accounts and never
  deserializes the book.
- Pinocchio instead of Anchor: no account deserialization framework, no allocator.
- A fixed 8-byte instruction header with the seat index as a hint, so a trader's seat is
  found without a tree search when the hint is right.
- Edge attachment for new orders and cached best and worst orders.
- The clock sysvar is read only when some order can expire.
- Rare features (reverse orders, stops, referrals, delegates) sit on cold paths and are
  checked through header counters, so an ordinary order pays almost nothing for them.
- Events are built only for fields a sink uses, buffered, and logged once.

## Engine invariants

`Market::check_invariants` walks the whole buffer and verifies the trees (ordering, colors,
parent links), the free list, the seat order lists, the caches, the reverse reserve, stop
pairing and the balance identity: free plus locked balances plus fees equal the tokens the
vaults must hold. The engine's model test runs thousands of random operation sequences
against a simple reference model and checks these invariants after each step.

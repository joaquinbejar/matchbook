# matchbook-sdk

Rust client for MatchBook, a crankless order book on Solana.

- `ix`: every instruction with its complete account list (`MarketAccounts::new` derives the vaults).
- `MarketSnapshot`: decode a market account, read levels, orders and seats, and quote swaps.
  Quotes run the program's own matching engine on a copy of the account, so a quote is
  exactly what the swap executes against the same state and clock.
- `jupiter` feature: `MatchBookAmm` implements `jupiter_amm_interface::Amm`. It needs only
  the market account (mints, vaults and token programs come from its header).

```rust
use matchbook_sdk::{engine::Side, MarketSnapshot};

let snap = MarketSnapshot::new(market, &account_data)?;
let out = snap.quote_exact_in(Side::Bid, 1_000_000, now)?; // spend up to 1 USDC
println!("buys {} base atoms for {} quote atoms", out.base_atoms, out.quote_net(Side::Bid));
```

Jupiter routes by a `Swap` enum variant that only Jupiter can add, so until MatchBook has
one the adapter reports a placeholder (`jupiter::PLACEHOLDER_SWAP`) and the account metas of
MatchBook's own `Swap` instruction. `MatchBookAmm::swap_instruction` builds that instruction
for callers that compose transactions themselves.

`tests/jupiter.rs` runs the compiled program in LiteSVM and checks that adapter quotes match
executed swaps, including orders that expire between placement and the quote.

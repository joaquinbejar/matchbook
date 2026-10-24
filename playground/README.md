# MatchBook playground

The MatchBook engine compiled to WebAssembly and driven from one HTML page: a SOL/USDC
market where you can place limit, post-only, IOC and fill-or-kill orders, reverse orders
and stops, and watch fills, flips and triggers on an event tape. Every action runs the same
Rust code as the on-chain program, and the page checks the market's invariants after each one.

```bash
rustup target add wasm32-unknown-unknown
./playground/build.sh          # writes playground/dist/index.html
```

The page is self-contained (the module is inlined), so it can be opened from disk or served
from any static host such as GitHub Pages.

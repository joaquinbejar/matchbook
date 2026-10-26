# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.1] - 2026-10-26

### Changed
- Dependencies: reqwest 0.13, base64 0.23, bytemuck 1.25, rust_decimal 1.43 and a refreshed
  lockfile (drops the yanked keccak 0.1.5); ts-sdk and ts-client on
  @solana-program/system 0.16, @solana-program/token 0.18 and TypeScript 6.0.3.

## [1.0.0] - 2026-10-26

First stable release of the redesign. Published to crates.io: `matchbook-engine`,
`matchbook-program`, `matchbook-sdk` and `matchbook-client`.

### Added
- `matchbook` CLI in Rust (`client/`) and TypeScript (`ts-client/`): markets, deposits and
  withdrawals, every order type including reverse orders, stops and OCO brackets, seatless
  swaps with exact quotes, book and balances, and session keys.
- `matchbook-mm` market-making bot in both languages: inventory-skewed ladders requoted in
  one batch, session-key signing, position, notional, loss, stale-price and jump limits.
- Rust SDK: event decoding, a seat's orders, stops and locked funds, delegates, error names.
- TypeScript SDK: `MarketView.locked`.
- `scripts/e2e-clients.sh`: both clients against a local validator.

## [1.0.0-beta.1] - 2026-10-24

A redesign: a crankless order book in one growable account, with a new program, SDKs and
playground. Not compatible with 0.x markets.

### Added
- `matchbook-engine`: a `no_std` order book over a byte buffer, with red-black trees in a
  node pool, cached book edges and a model test against a reference implementation.
- Crankless matching: makers are settled inside the taker's instruction.
- Order types: limit, post-only, post-only slide, IOC, fill or kill, quote budgets, expiry,
  client order ids, self-trade behaviors, batch updates of up to 32 cancels and 32 orders.
- Reverse orders with a reserved node, so flips never fail for lack of space.
- Stop orders and one-cancels-other brackets placed by the trades that reach them, plus a
  permissionless `TriggerStops`.
- Session keys: up to 8 delegates per seat with trade or cancel permissions and expiry.
- Seatless swaps with a minimum-out guard, referral fee sharing and permissioned markets.
- Token-2022 support through `transfer_checked`.
- Rust SDK with exact quotes and a Jupiter `Amm` adapter; TypeScript SDK on `@solana/kit`.
- WebAssembly playground, deploy script and a demo-market example.
- Benchmarks against Manifest, Phoenix v1 and OpenBook v2.

### Removed
- The Anchor program, crank, indexer, REST and WebSocket services and their deployment files.

## [0.2.0] - 2026-09-16

### Added
- `place_order` instruction wired into the program module (#131)
- Comprehensive developer documentation
- OpenAPI 3.0 specification for REST API
- Architecture documentation with diagrams
- Getting started guide for integrators
- API and WebSocket reference documentation
- SDK usage guide for Rust and TypeScript
- FAQ documentation
- Runbooks for operational troubleshooting
- Incident response and maintenance procedures
- Alerting rules with Prometheus Alertmanager
- Monitoring setup with Prometheus and Grafana
- CI/CD pipelines for automated testing and deployment
- Kubernetes manifests for production deployment
- Docker configuration for all services

### Changed
- Updated README with project overview and quick start guide
- TypeScript SDK migrated off `node10` module resolution for TypeScript 7 readiness (#138)
- CI uses the stable Rust toolchain; Docker images build on Rust 1.88 (#116)
- Dependency updates across the Rust workspace and the TypeScript SDK (Dependabot)

### Fixed
- Clippy lints for Solana macros (#116)
- `security-events: write` permission for SARIF upload in the security workflow

## [0.1.0] - 2026-01-30

### Added
- Initial release of Matchbook
- On-chain Solana program with order book implementation
- Core instructions: CreateMarket, CreateOpenOrders, Deposit, PlaceOrder, CancelOrder, MatchOrders, ConsumeEvents, Withdraw
- Red-black tree order book data structure
- Event queue for matching events
- Indexer service with Geyser integration
- REST API server with market data endpoints
- WebSocket API for real-time streaming
- Crank service for automated order matching
- Rust SDK for client integration
- TypeScript SDK for web applications
- Docker Compose for local development

### Security
- Non-custodial design with user-controlled funds
- On-chain validation for all operations
- Signed request authentication for trading endpoints

[Unreleased]: https://github.com/joaquinbejar/matchbook/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/joaquinbejar/matchbook/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/joaquinbejar/matchbook/compare/v0.2.1...v1.0.0
[0.1.0]: https://github.com/joaquinbejar/matchbook/releases/tag/v0.1.0

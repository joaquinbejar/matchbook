# Matchbook GitHub Issue Plan

> **WARNING**: This file must NEVER be committed to the repository.

## Labels

| Label | Color | Description |
|-------|-------|-------------|
| on-chain | 7B68EE | Solana program |
| off-chain | 4682B4 | Backend services |
| sdk | 20B2AA | Client SDKs |
| infra | 708090 | Deployment and ops |
| docs | 0075ca | Documentation |
| testing | d4c5f9 | Tests |

## Milestones

| Milestone | Description |
|-----------|-------------|
| M1: On-Chain Core | Core program with all instructions |
| M2: Off-Chain Services | Indexer, API, Crank |
| M3: Client SDKs | Rust and TypeScript SDKs |
| M4: Production Ready | Deployment, monitoring, docs |

## Issue Breakdown

### Milestone 1: On-Chain Core (16 issues)

| # | Title | Dependencies | Labels |
|---|-------|--------------|--------|
| 1 | Project scaffolding | - | on-chain, infra |
| 2 | State: Market account | #1 | on-chain |
| 3 | State: OrderBook accounts (B+ tree) | #1 | on-chain |
| 4 | State: EventQueue account | #1 | on-chain |
| 5 | State: OpenOrders account | #1 | on-chain |
| 6 | Error types | #1 | on-chain |
| 7 | Instruction: CreateMarket | #2, #3, #4, #6 | on-chain |
| 8 | Instruction: CreateOpenOrders | #5, #6 | on-chain |
| 9 | Instruction: Deposit | #5, #7 | on-chain |
| 10 | Instruction: Withdraw | #5, #7 | on-chain |
| 11 | Instruction: PlaceOrder | #3, #4, #5, #7 | on-chain |
| 12 | Instruction: CancelOrder | #3, #4, #5, #11 | on-chain |
| 13 | Instruction: CancelAllOrders | #12 | on-chain |
| 14 | Instruction: MatchOrders (Crank) | #3, #4, #11 | on-chain |
| 15 | Instruction: ConsumeEvents | #4, #5, #14 | on-chain |
| 16 | Integration tests + Devnet deployment | #7-#15 | on-chain, testing |

### Milestone 2: Off-Chain Services (9 issues)

| # | Title | Dependencies | Labels |
|---|-------|--------------|--------|
| 17 | Database schema (PostgreSQL + TimescaleDB) | M1 complete | off-chain, infra |
| 18 | Indexer: Geyser listener | M1 complete | off-chain |
| 19 | Indexer: Account parser | #18 | off-chain |
| 20 | Indexer: Book builder | #19 | off-chain |
| 21 | Indexer: Event processor | #19 | off-chain |
| 22 | API: REST endpoints | #17, #20, #21 | off-chain |
| 23 | API: WebSocket server | #20, #21 | off-chain |
| 24 | Crank service | M1 complete | off-chain |
| 25 | Redis integration | #22, #23 | off-chain |

### Milestone 3: Client SDKs (7 issues)

| # | Title | Dependencies | Labels |
|---|-------|--------------|--------|
| 26 | Rust SDK: Core types | M1 complete | sdk |
| 27 | Rust SDK: Instruction builders | #26 | sdk |
| 28 | Rust SDK: HTTP client | #22, #26 | sdk |
| 29 | Rust SDK: WebSocket client | #23, #26 | sdk |
| 30 | TypeScript SDK: Types | M1 complete | sdk |
| 31 | TypeScript SDK: Client | #22, #23, #30 | sdk |
| 32 | SDK examples | #27, #28, #29, #31 | sdk, docs |

### Milestone 4: Production Ready (7 issues)

| # | Title | Dependencies | Labels |
|---|-------|--------------|--------|
| 33 | Docker configuration | M2 complete | infra |
| 34 | Kubernetes manifests | #33 | infra |
| 35 | CI/CD pipeline | #33 | infra |
| 36 | Monitoring setup (Prometheus + Grafana) | #34 | infra |
| 37 | Alerting rules | #36 | infra |
| 38 | Operational runbooks | #36, #37 | infra, docs |
| 39 | README and public documentation | M1, M2, M3 complete | docs |

## Issue Tracking

| Issue # | GitHub # | Created | Title |
|---------|----------|---------|-------|
| 1 | [#1](https://github.com/joaquinbejar/matchbook/issues/1) | ✅ | Project scaffolding |
| 2 | [#2](https://github.com/joaquinbejar/matchbook/issues/2) | ✅ | State: Market account |
| 3 | [#3](https://github.com/joaquinbejar/matchbook/issues/3) | ✅ | State: OrderBook accounts |
| 4 | [#4](https://github.com/joaquinbejar/matchbook/issues/4) | ✅ | State: EventQueue account |
| 5 | [#5](https://github.com/joaquinbejar/matchbook/issues/5) | ✅ | State: OpenOrders account |
| 6 | [#6](https://github.com/joaquinbejar/matchbook/issues/6) | ✅ | Error types |
| 7 | [#7](https://github.com/joaquinbejar/matchbook/issues/7) | ✅ | Instruction: CreateMarket |
| 8 | [#8](https://github.com/joaquinbejar/matchbook/issues/8) | ✅ | Instruction: CreateOpenOrders |
| 9 | [#9](https://github.com/joaquinbejar/matchbook/issues/9) | ✅ | Instruction: Deposit |
| 10 | [#10](https://github.com/joaquinbejar/matchbook/issues/10) | ✅ | Instruction: Withdraw |
| 11 | [#11](https://github.com/joaquinbejar/matchbook/issues/11) | ✅ | Instruction: PlaceOrder |
| 12 | [#12](https://github.com/joaquinbejar/matchbook/issues/12) | ✅ | Instruction: CancelOrder |
| 13 | [#13](https://github.com/joaquinbejar/matchbook/issues/13) | ✅ | Instruction: CancelAllOrders |
| 14 | [#14](https://github.com/joaquinbejar/matchbook/issues/14) | ✅ | Instruction: MatchOrders |
| 15 | [#15](https://github.com/joaquinbejar/matchbook/issues/15) | ✅ | Instruction: ConsumeEvents |
| 16 | [#16](https://github.com/joaquinbejar/matchbook/issues/16) | ✅ | Integration tests + Devnet deployment |
| 17 | [#17](https://github.com/joaquinbejar/matchbook/issues/17) | ✅ | Database schema |
| 18 | [#18](https://github.com/joaquinbejar/matchbook/issues/18) | ✅ | Indexer: Geyser listener |
| 19 | [#19](https://github.com/joaquinbejar/matchbook/issues/19) | ✅ | Indexer: Account parser |
| 20 | [#20](https://github.com/joaquinbejar/matchbook/issues/20) | ✅ | Indexer: Book builder |
| 21 | [#21](https://github.com/joaquinbejar/matchbook/issues/21) | ✅ | Indexer: Event processor |
| 22 | [#22](https://github.com/joaquinbejar/matchbook/issues/22) | ✅ | API: REST endpoints |
| 23 | [#23](https://github.com/joaquinbejar/matchbook/issues/23) | ✅ | API: WebSocket server |
| 24 | [#24](https://github.com/joaquinbejar/matchbook/issues/24) | ✅ | Crank service |
| 25 | [#25](https://github.com/joaquinbejar/matchbook/issues/25) | ✅ | Redis integration |
| 26 | [#26](https://github.com/joaquinbejar/matchbook/issues/26) | ✅ | Rust SDK: Core types |
| 27 | [#27](https://github.com/joaquinbejar/matchbook/issues/27) | ✅ | Rust SDK: Instruction builders |
| 28 | [#28](https://github.com/joaquinbejar/matchbook/issues/28) | ✅ | Rust SDK: HTTP client |
| 29 | [#29](https://github.com/joaquinbejar/matchbook/issues/29) | ✅ | Rust SDK: WebSocket client |
| 30 | [#30](https://github.com/joaquinbejar/matchbook/issues/30) | ✅ | TypeScript SDK: Types |
| 31 | [#31](https://github.com/joaquinbejar/matchbook/issues/31) | ✅ | TypeScript SDK: Client |
| 32 | [#32](https://github.com/joaquinbejar/matchbook/issues/32) | ✅ | SDK examples |
| 33 | [#33](https://github.com/joaquinbejar/matchbook/issues/33) | ✅ | Docker configuration |
| 34 | [#34](https://github.com/joaquinbejar/matchbook/issues/34) | ✅ | Kubernetes manifests |
| 35 | [#35](https://github.com/joaquinbejar/matchbook/issues/35) | ✅ | CI/CD pipeline |
| 36 | [#36](https://github.com/joaquinbejar/matchbook/issues/36) | ✅ | Monitoring setup |
| 37 | [#37](https://github.com/joaquinbejar/matchbook/issues/37) | ✅ | Alerting rules |
| 38 | [#38](https://github.com/joaquinbejar/matchbook/issues/38) | ✅ | Operational runbooks |
| 39 | [#39](https://github.com/joaquinbejar/matchbook/issues/39) | ✅ | README and public documentation |

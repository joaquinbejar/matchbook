# Matchbook

[![CI](https://github.com/joaquinbejar/matchbook/actions/workflows/ci.yml/badge.svg)](https://github.com/joaquinbejar/matchbook/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance, non-custodial Central Limit Order Book (CLOB) on Solana.

## Overview

Matchbook is a decentralized exchange infrastructure that provides:

- **On-chain order book**: Fully transparent order matching on Solana
- **Non-custodial**: Users maintain control of their funds at all times
- **High performance**: Optimized for Solana's parallel transaction processing
- **Real-time data**: WebSocket streaming for live market updates
- **Developer-friendly**: REST API, WebSocket API, and SDKs for Rust and TypeScript

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Clients                                     │
│                    (Web Apps, Trading Bots, SDKs)                        │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    ▼               ▼               ▼
            ┌───────────┐   ┌───────────┐   ┌───────────┐
            │ REST API  │   │ WebSocket │   │  Direct   │
            │  :8080    │   │   :8081   │   │  On-chain │
            └─────┬─────┘   └─────┬─────┘   └─────┬─────┘
                  │               │               │
                  └───────────────┼───────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │                           │
                    ▼                           ▼
            ┌───────────────┐         ┌─────────────────┐
            │   Indexer     │         │  Solana Program │
            │   (Geyser)    │◄────────│   (On-chain)    │
            └───────┬───────┘         └─────────────────┘
                    │
                    ▼
            ┌───────────────┐
            │   Database    │
            │  (TimescaleDB)│
            └───────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+
- Solana CLI 1.18+
- Node.js 18+ (for TypeScript SDK)
- Docker (for local development)

### Local Development

```bash
# Clone the repository
git clone https://github.com/joaquinbejar/matchbook.git
cd matchbook

# Start local infrastructure
docker-compose up -d

# Build the on-chain program
cargo build-sbf

# Run tests
cargo test --all-features

# Deploy to localnet
solana-test-validator &
solana program deploy target/deploy/matchbook_program.so
```

### Using the SDK

#### Rust

```rust
use matchbook_sdk::{Client, PlaceOrderParams, Side, OrderType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.matchbook.io")?;
    
    // Get markets
    let markets = client.get_markets().await?;
    
    // Place an order
    let tx = client.place_order(PlaceOrderParams {
        market: markets[0].address,
        side: Side::Bid,
        price: 100_000_000, // $100.00 in base units
        quantity: 1_000_000_000, // 1 SOL in lamports
        order_type: OrderType::Limit,
        ..Default::default()
    }).await?;
    
    println!("Order placed: {}", tx.signature);
    Ok(())
}
```

#### TypeScript

```typescript
import { MatchbookClient, Side, OrderType } from '@matchbook/sdk';

const client = new MatchbookClient('https://api.matchbook.io');

// Get markets
const markets = await client.getMarkets();

// Place an order
const tx = await client.placeOrder({
  market: markets[0].address,
  side: Side.Bid,
  price: '100.00',
  quantity: '1.0',
  orderType: OrderType.Limit,
});

console.log('Order placed:', tx.signature);
```

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | System architecture and design |
| [Getting Started](docs/getting-started.md) | Step-by-step integration guide |
| [API Reference](docs/api-reference.md) | REST API documentation |
| [WebSocket Reference](docs/websocket-reference.md) | WebSocket API documentation |
| [SDK Guide](docs/sdk-guide.md) | SDK usage for Rust and TypeScript |
| [Deployment](docs/docker.md) | Docker and Kubernetes deployment |
| [Monitoring](docs/monitoring.md) | Prometheus and Grafana setup |
| [FAQ](docs/faq.md) | Frequently asked questions |

## Project Structure

```
matchbook/
├── program/           # Solana on-chain program
├── sdk/               # Rust client SDK
├── ts-sdk/            # TypeScript client SDK
├── indexer/           # Geyser-based indexer service
├── api/               # REST and WebSocket API server
├── crank/             # Order matching crank service
├── k8s/               # Kubernetes manifests
├── monitoring/        # Prometheus and Grafana configs
└── docs/              # Documentation
```

## Crates

| Crate | Description |
|-------|-------------|
| `matchbook_program` | On-chain Solana program |
| `matchbook_sdk` | Rust client SDK |
| `matchbook_types` | Shared types and utilities |

## API Endpoints

### REST API

| Endpoint | Description |
|----------|-------------|
| `GET /v1/markets` | List all markets |
| `GET /v1/markets/{address}/orderbook` | Get order book snapshot |
| `GET /v1/markets/{address}/trades` | Get recent trades |
| `POST /v1/tx/place-order` | Build place order transaction |
| `POST /v1/tx/cancel-order` | Build cancel order transaction |

### WebSocket Channels

| Channel | Description |
|---------|-------------|
| `book` | Order book updates |
| `trades` | Trade stream |
| `ticker` | Price ticker |
| `orders` | User order updates (authenticated) |

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests: `cargo test --all-features`
5. Run lints: `cargo clippy --all-targets --all-features -- -D warnings`
6. Submit a pull request

## Security

For security concerns, please see [SECURITY.md](SECURITY.md) or email security@matchbook.io.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Links

- [Documentation](https://docs.matchbook.io)
- [API Reference](https://api.matchbook.io/docs)
- [Discord](https://discord.gg/matchbook)
- [Twitter](https://twitter.com/matchbook_io)

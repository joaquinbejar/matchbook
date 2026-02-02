# Matchbook Rust SDK Examples

This directory contains example applications demonstrating how to use the Matchbook Rust SDK.

## Prerequisites

- Rust 1.70 or later
- A Matchbook API endpoint (devnet or local)
- Environment variables configured (see below)

## Environment Variables

All examples use environment variables for configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `MATCHBOOK_API_URL` | REST API base URL | `https://api.matchbook.example/v1` |
| `MATCHBOOK_MARKET` | Market address to use | Required for market_data |
| `MATCHBOOK_OWNER` | Wallet address | Required for user_data |

## Examples

### 1. Market Data (`market_data.rs`)

Fetches and displays market information, order book, and recent trades.

```bash
MATCHBOOK_MARKET=ABC123... cargo run --example market_data
```

**What it demonstrates:**
- Fetching all markets
- Getting specific market details
- Fetching order book with depth
- Fetching recent trades
- Calculating spread

### 2. User Data (`user_data.rs`)

Fetches user-specific data including orders, trades, and balances.

```bash
MATCHBOOK_OWNER=9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM cargo run --example user_data
```

**What it demonstrates:**
- Fetching open orders
- Fetching trade history
- Fetching balances
- Filtering by market

## Running on Devnet

1. Configure environment variables to point to devnet endpoints
2. Use a test market address

```bash
export MATCHBOOK_API_URL=https://devnet-api.matchbook.example/v1
export MATCHBOOK_MARKET=<devnet-market-address>
export MATCHBOOK_OWNER=<your-wallet-address>
```

## Error Handling

All examples include basic error handling. In production code, you should:

1. Use typed errors for different failure modes
2. Implement retry logic for transient failures
3. Handle rate limiting gracefully
4. Log errors for debugging

## Rate Limiting

Be aware of API rate limits:

- REST API: Typically 100 requests/second
- Don't spam requests

## Further Reading

- [SDK Documentation](../README.md)
- [API Protocol Specification](../../.internalDoc/05-protocols.md)
- [Domain Model](../../.internalDoc/02-domain-model.md)

# Getting Started

This guide walks you through integrating with Matchbook, from setting up your environment to placing your first order.

## Prerequisites

- **Solana Wallet**: A Solana wallet with SOL for transaction fees
- **Tokens**: Base and quote tokens for the market you want to trade
- **Development Environment**: Node.js 18+ or Rust 1.75+

## Installation

### TypeScript SDK

```bash
npm install @matchbook/sdk
# or
yarn add @matchbook/sdk
# or
pnpm add @matchbook/sdk
```

### Rust SDK

Add to your `Cargo.toml`:

```toml
[dependencies]
matchbook-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

### 1. Initialize the Client

#### TypeScript

```typescript
import { MatchbookClient } from '@matchbook/sdk';

// For mainnet
const client = new MatchbookClient('https://api.matchbook.taunais.com');

// For devnet
const client = new MatchbookClient('https://api.devnet.matchbook.taunais.com');
```

#### Rust

```rust
use matchbook_sdk::Client;

// For mainnet
let client = Client::new("https://api.matchbook.taunais.com")?;

// For devnet
let client = Client::new("https://api.devnet.matchbook.taunais.com")?;
```

### 2. Get Available Markets

#### TypeScript

```typescript
const markets = await client.getMarkets();

for (const market of markets) {
  console.log(`${market.baseSymbol}/${market.quoteSymbol}: ${market.address}`);
}
```

#### Rust

```rust
let markets = client.get_markets().await?;

for market in &markets {
    println!("{}/{}: {}", market.base_symbol, market.quote_symbol, market.address);
}
```

### 3. Get Order Book

#### TypeScript

```typescript
const orderbook = await client.getOrderbook(marketAddress, { depth: 20 });

console.log('Best bid:', orderbook.bids[0]);
console.log('Best ask:', orderbook.asks[0]);
```

#### Rust

```rust
let orderbook = client.get_orderbook(&market_address, Some(20)).await?;

println!("Best bid: {:?}", orderbook.bids.first());
println!("Best ask: {:?}", orderbook.asks.first());
```

### 4. Create Open Orders Account

Before trading, you need an Open Orders account for each market:

#### TypeScript

```typescript
import { Keypair } from '@solana/web3.js';

const wallet = Keypair.fromSecretKey(/* your secret key */);

// Check if account exists
const openOrders = await client.getOpenOrders(wallet.publicKey, marketAddress);

if (!openOrders) {
  // Create the account
  const tx = await client.createOpenOrders({
    market: marketAddress,
    owner: wallet.publicKey,
  });
  
  // Sign and send
  tx.sign([wallet]);
  const signature = await client.sendTransaction(tx);
  console.log('Open Orders created:', signature);
}
```

#### Rust

```rust
use solana_sdk::signer::Signer;

let wallet = /* your keypair */;

// Check if account exists
let open_orders = client.get_open_orders(&wallet.pubkey(), &market_address).await?;

if open_orders.is_none() {
    // Create the account
    let tx = client.create_open_orders(CreateOpenOrdersParams {
        market: market_address,
        owner: wallet.pubkey(),
    }).await?;
    
    // Sign and send
    let signature = client.sign_and_send(&tx, &[&wallet]).await?;
    println!("Open Orders created: {}", signature);
}
```

### 5. Deposit Funds

Deposit tokens to your Open Orders account:

#### TypeScript

```typescript
// Deposit 100 USDC (6 decimals)
const tx = await client.deposit({
  market: marketAddress,
  owner: wallet.publicKey,
  amount: 100_000_000n, // 100 USDC
  side: 'quote', // 'base' for base token, 'quote' for quote token
});

tx.sign([wallet]);
const signature = await client.sendTransaction(tx);
console.log('Deposited:', signature);
```

#### Rust

```rust
// Deposit 100 USDC (6 decimals)
let tx = client.deposit(DepositParams {
    market: market_address,
    owner: wallet.pubkey(),
    amount: 100_000_000, // 100 USDC
    side: Side::Quote,
}).await?;

let signature = client.sign_and_send(&tx, &[&wallet]).await?;
println!("Deposited: {}", signature);
```

### 6. Place an Order

#### TypeScript

```typescript
import { Side, OrderType, TimeInForce } from '@matchbook/sdk';

const tx = await client.placeOrder({
  market: marketAddress,
  owner: wallet.publicKey,
  side: Side.Bid,
  price: '100.50', // Price in quote currency
  quantity: '1.0', // Quantity in base currency
  orderType: OrderType.Limit,
  timeInForce: TimeInForce.GTC, // Good-til-cancelled
  clientOrderId: 12345, // Optional: your order ID
});

tx.sign([wallet]);
const signature = await client.sendTransaction(tx);
console.log('Order placed:', signature);
```

#### Rust

```rust
use matchbook_sdk::{Side, OrderType, TimeInForce};

let tx = client.place_order(PlaceOrderParams {
    market: market_address,
    owner: wallet.pubkey(),
    side: Side::Bid,
    price: 100_500_000, // $100.50 in base units (6 decimals)
    quantity: 1_000_000_000, // 1.0 in base units (9 decimals)
    order_type: OrderType::Limit,
    time_in_force: TimeInForce::GTC,
    client_order_id: Some(12345),
    ..Default::default()
}).await?;

let signature = client.sign_and_send(&tx, &[&wallet]).await?;
println!("Order placed: {}", signature);
```

### 7. Cancel an Order

#### TypeScript

```typescript
const tx = await client.cancelOrder({
  market: marketAddress,
  owner: wallet.publicKey,
  orderId: '340282366920938463463374607431768211455',
  side: Side.Bid,
});

tx.sign([wallet]);
const signature = await client.sendTransaction(tx);
console.log('Order cancelled:', signature);
```

#### Rust

```rust
let tx = client.cancel_order(CancelOrderParams {
    market: market_address,
    owner: wallet.pubkey(),
    order_id: order_id,
    side: Side::Bid,
}).await?;

let signature = client.sign_and_send(&tx, &[&wallet]).await?;
println!("Order cancelled: {}", signature);
```

### 8. Withdraw Funds

#### TypeScript

```typescript
const tx = await client.withdraw({
  market: marketAddress,
  owner: wallet.publicKey,
});

tx.sign([wallet]);
const signature = await client.sendTransaction(tx);
console.log('Withdrawn:', signature);
```

## WebSocket Streaming

### Subscribe to Order Book Updates

#### TypeScript

```typescript
import { MatchbookWebSocket } from '@matchbook/sdk';

const ws = new MatchbookWebSocket('wss://ws.matchbook.taunais.com');

ws.on('open', () => {
  // Subscribe to order book
  ws.subscribe('book', marketAddress, { depth: 20 });
  
  // Subscribe to trades
  ws.subscribe('trades', marketAddress);
});

ws.on('book_snapshot', (data) => {
  console.log('Order book snapshot:', data);
});

ws.on('book_update', (data) => {
  console.log('Order book update:', data);
});

ws.on('trade', (data) => {
  console.log('New trade:', data);
});

ws.connect();
```

#### Rust

```rust
use matchbook_sdk::WebSocketClient;

let mut ws = WebSocketClient::connect("wss://ws.matchbook.taunais.com").await?;

// Subscribe to order book
ws.subscribe_book(&market_address, Some(20)).await?;

// Subscribe to trades
ws.subscribe_trades(&market_address).await?;

// Handle messages
while let Some(msg) = ws.next().await {
    match msg? {
        Message::BookSnapshot(snapshot) => {
            println!("Snapshot: {:?}", snapshot);
        }
        Message::BookUpdate(update) => {
            println!("Update: {:?}", update);
        }
        Message::Trade(trade) => {
            println!("Trade: {:?}", trade);
        }
        _ => {}
    }
}
```

## Error Handling

### TypeScript

```typescript
import { MatchbookError, ErrorCode } from '@matchbook/sdk';

try {
  const tx = await client.placeOrder(params);
} catch (error) {
  if (error instanceof MatchbookError) {
    switch (error.code) {
      case ErrorCode.InsufficientFunds:
        console.error('Not enough funds to place order');
        break;
      case ErrorCode.InvalidPrice:
        console.error('Price is invalid for this market');
        break;
      case ErrorCode.RateLimitExceeded:
        console.error('Too many requests, please slow down');
        break;
      default:
        console.error('Error:', error.message);
    }
  }
}
```

### Rust

```rust
use matchbook_sdk::{Error, ErrorCode};

match client.place_order(params).await {
    Ok(tx) => { /* success */ }
    Err(Error::Api { code, message }) => {
        match code {
            ErrorCode::InsufficientFunds => {
                eprintln!("Not enough funds to place order");
            }
            ErrorCode::InvalidPrice => {
                eprintln!("Price is invalid for this market");
            }
            _ => {
                eprintln!("Error: {}", message);
            }
        }
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

## Best Practices

### 1. Use Client Order IDs

Always set `clientOrderId` to track your orders:

```typescript
const tx = await client.placeOrder({
  // ...
  clientOrderId: Date.now(), // Or your own ID scheme
});
```

### 2. Handle Reconnections

WebSocket connections may drop. Implement reconnection logic:

```typescript
ws.on('close', () => {
  setTimeout(() => {
    ws.connect();
    // Resubscribe to channels
  }, 1000);
});
```

### 3. Validate Prices and Quantities

Ensure values conform to market tick and lot sizes:

```typescript
const market = await client.getMarket(marketAddress);

// Round price to tick size
const price = Math.round(desiredPrice / market.tickSize) * market.tickSize;

// Round quantity to lot size
const quantity = Math.round(desiredQty / market.lotSize) * market.lotSize;
```

### 4. Monitor Your Orders

Subscribe to the `orders` channel for real-time updates:

```typescript
ws.subscribe('orders', wallet.publicKey.toString());

ws.on('order_update', (update) => {
  console.log(`Order ${update.orderId}: ${update.status}`);
});
```

## Next Steps

- [API Reference](./api-reference.md) - Complete REST API documentation
- [WebSocket Reference](./websocket-reference.md) - WebSocket API documentation
- [SDK Guide](./sdk-guide.md) - Advanced SDK usage
- [FAQ](./faq.md) - Frequently asked questions

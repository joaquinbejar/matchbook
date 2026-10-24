# SDK Guide

This guide covers advanced usage of the Matchbook SDKs for Rust and TypeScript.

## Installation

### Rust SDK

```toml
[dependencies]
matchbook-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
solana-sdk = "1.18"
```

### TypeScript SDK

```bash
npm install @matchbook/sdk @solana/web3.js
```

## Client Configuration

### Rust

```rust
use matchbook_sdk::{Client, ClientConfig};

// Basic configuration
let client = Client::new("https://api.matchbook.taunais.com")?;

// Advanced configuration
let client = Client::with_config(ClientConfig {
    api_url: "https://api.matchbook.taunais.com".to_string(),
    ws_url: Some("wss://ws.matchbook.taunais.com".to_string()),
    api_key: Some("your-api-key".to_string()),
    timeout: Duration::from_secs(30),
    max_retries: 3,
})?;
```

### TypeScript

```typescript
import { MatchbookClient, ClientConfig } from '@matchbook/sdk';

// Basic configuration
const client = new MatchbookClient('https://api.matchbook.taunais.com');

// Advanced configuration
const client = new MatchbookClient({
  apiUrl: 'https://api.matchbook.taunais.com',
  wsUrl: 'wss://ws.matchbook.taunais.com',
  apiKey: 'your-api-key',
  timeout: 30000,
  maxRetries: 3,
});
```

## Working with Markets

### Get Market Details

#### Rust

```rust
use matchbook_sdk::Pubkey;

let market_address = Pubkey::from_str("7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF")?;
let market = client.get_market(&market_address).await?;

println!("Market: {}/{}", market.base_symbol, market.quote_symbol);
println!("Tick size: {}", market.tick_size);
println!("Lot size: {}", market.lot_size);
println!("Best bid: {}", market.stats.best_bid);
println!("Best ask: {}", market.stats.best_ask);
```

#### TypeScript

```typescript
import { PublicKey } from '@solana/web3.js';

const marketAddress = new PublicKey('7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF');
const market = await client.getMarket(marketAddress);

console.log(`Market: ${market.baseSymbol}/${market.quoteSymbol}`);
console.log(`Tick size: ${market.tickSize}`);
console.log(`Lot size: ${market.lotSize}`);
console.log(`Best bid: ${market.stats.bestBid}`);
console.log(`Best ask: ${market.stats.bestAsk}`);
```

### Price and Quantity Conversion

Markets have specific tick sizes and lot sizes. Use helpers to convert:

#### Rust

```rust
use matchbook_sdk::{Market, PriceConverter};

let market = client.get_market(&market_address).await?;

// Convert human-readable price to native units
let price_native = market.price_to_native(105.50)?;

// Convert human-readable quantity to native units
let qty_native = market.quantity_to_native(1.5)?;

// Convert back to human-readable
let price_human = market.price_to_human(price_native);
let qty_human = market.quantity_to_human(qty_native);
```

#### TypeScript

```typescript
const market = await client.getMarket(marketAddress);

// Convert human-readable price to native units
const priceNative = market.priceToNative('105.50');

// Convert human-readable quantity to native units
const qtyNative = market.quantityToNative('1.5');

// Convert back to human-readable
const priceHuman = market.priceToHuman(priceNative);
const qtyHuman = market.quantityToHuman(qtyNative);
```

## Order Management

### Order Types

| Type | Description |
|------|-------------|
| `Limit` | Execute at specified price or better |
| `Market` | Execute immediately at best available price |
| `PostOnly` | Only add liquidity, cancel if would take |

### Time in Force

| TIF | Description |
|-----|-------------|
| `GTC` | Good-til-cancelled |
| `IOC` | Immediate-or-cancel |
| `FOK` | Fill-or-kill |

### Place Order with All Options

#### Rust

```rust
use matchbook_sdk::{PlaceOrderParams, Side, OrderType, TimeInForce, SelfTradeBehavior};

let params = PlaceOrderParams {
    market: market_address,
    owner: wallet.pubkey(),
    side: Side::Bid,
    price: market.price_to_native(105.00)?,
    quantity: market.quantity_to_native(10.0)?,
    order_type: OrderType::Limit,
    time_in_force: TimeInForce::GTC,
    client_order_id: Some(12345),
    self_trade_behavior: SelfTradeBehavior::CancelTaker,
    limit_price: None, // For market orders with price protection
};

let tx = client.place_order(params).await?;
let signature = client.sign_and_send(&tx, &[&wallet]).await?;
```

#### TypeScript

```typescript
import { Side, OrderType, TimeInForce, SelfTradeBehavior } from '@matchbook/sdk';

const tx = await client.placeOrder({
  market: marketAddress,
  owner: wallet.publicKey,
  side: Side.Bid,
  price: market.priceToNative('105.00'),
  quantity: market.quantityToNative('10.0'),
  orderType: OrderType.Limit,
  timeInForce: TimeInForce.GTC,
  clientOrderId: 12345,
  selfTradeBehavior: SelfTradeBehavior.CancelTaker,
});

tx.sign([wallet]);
const signature = await client.sendTransaction(tx);
```

### Batch Operations

#### Cancel All Orders

```rust
// Cancel all orders on one side
let tx = client.cancel_all(CancelAllParams {
    market: market_address,
    owner: wallet.pubkey(),
    side: Some(Side::Bid), // None for both sides
}).await?;
```

```typescript
const tx = await client.cancelAll({
  market: marketAddress,
  owner: wallet.publicKey,
  side: Side.Bid, // undefined for both sides
});
```

### Query Orders

#### Rust

```rust
// Get all open orders
let orders = client.get_orders(&wallet.pubkey(), None).await?;

// Filter by market
let orders = client.get_orders(&wallet.pubkey(), Some(&market_address)).await?;

for order in &orders {
    println!(
        "Order {}: {} {} @ {} (filled: {}/{})",
        order.order_id,
        order.side,
        order.remaining_quantity,
        order.price,
        order.filled_quantity,
        order.original_quantity
    );
}
```

#### TypeScript

```typescript
// Get all open orders
const orders = await client.getOrders(wallet.publicKey);

// Filter by market
const orders = await client.getOrders(wallet.publicKey, { market: marketAddress });

for (const order of orders) {
  console.log(
    `Order ${order.orderId}: ${order.side} ${order.remainingQuantity} @ ${order.price}`
  );
}
```

## WebSocket Streaming

### Connection Management

#### Rust

```rust
use matchbook_sdk::{WebSocketClient, Message};

let mut ws = WebSocketClient::connect("wss://ws.matchbook.taunais.com").await?;

// Subscribe to channels
ws.subscribe_book(&market_address, Some(20)).await?;
ws.subscribe_trades(&market_address).await?;
ws.subscribe_ticker(&market_address).await?;

// Handle messages
loop {
    tokio::select! {
        Some(msg) = ws.next() => {
            match msg? {
                Message::BookSnapshot(snapshot) => {
                    println!("Snapshot: {} bids, {} asks", 
                        snapshot.bids.len(), 
                        snapshot.asks.len()
                    );
                }
                Message::BookUpdate(update) => {
                    // Apply incremental update
                }
                Message::Trade(trade) => {
                    println!("Trade: {} @ {}", trade.quantity, trade.price);
                }
                Message::Ticker(ticker) => {
                    println!("Ticker: bid={} ask={}", ticker.best_bid, ticker.best_ask);
                }
                _ => {}
            }
        }
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
            ws.ping().await?;
        }
    }
}
```

#### TypeScript

```typescript
import { MatchbookWebSocket } from '@matchbook/sdk';

const ws = new MatchbookWebSocket('wss://ws.matchbook.taunais.com');

ws.on('open', () => {
  ws.subscribeBook(marketAddress, { depth: 20 });
  ws.subscribeTrades(marketAddress);
  ws.subscribeTicker(marketAddress);
});

ws.on('book_snapshot', (snapshot) => {
  console.log(`Snapshot: ${snapshot.bids.length} bids, ${snapshot.asks.length} asks`);
});

ws.on('book_update', (update) => {
  // Apply incremental update
});

ws.on('trade', (trade) => {
  console.log(`Trade: ${trade.quantity} @ ${trade.price}`);
});

ws.on('ticker', (ticker) => {
  console.log(`Ticker: bid=${ticker.bestBid} ask=${ticker.bestAsk}`);
});

ws.on('close', () => {
  // Implement reconnection logic
  setTimeout(() => ws.connect(), 1000);
});

ws.connect();
```

### Order Book Maintenance

#### Rust

```rust
use std::collections::BTreeMap;

struct LocalOrderBook {
    bids: BTreeMap<u64, u64>, // price -> quantity
    asks: BTreeMap<u64, u64>,
    sequence: u64,
}

impl LocalOrderBook {
    fn apply_snapshot(&mut self, snapshot: &BookSnapshot) {
        self.bids.clear();
        self.asks.clear();
        
        for (price, qty) in &snapshot.bids {
            self.bids.insert(*price, *qty);
        }
        for (price, qty) in &snapshot.asks {
            self.asks.insert(*price, *qty);
        }
        self.sequence = snapshot.sequence;
    }
    
    fn apply_update(&mut self, update: &BookUpdate) -> Result<(), Error> {
        if update.sequence != self.sequence + 1 {
            return Err(Error::SequenceGap);
        }
        
        for change in &update.changes {
            let book = match change.side {
                Side::Bid => &mut self.bids,
                Side::Ask => &mut self.asks,
            };
            
            if change.quantity == 0 {
                book.remove(&change.price);
            } else {
                book.insert(change.price, change.quantity);
            }
        }
        
        self.sequence = update.sequence;
        Ok(())
    }
}
```

#### TypeScript

```typescript
class LocalOrderBook {
  bids = new Map<string, string>();
  asks = new Map<string, string>();
  sequence = 0;

  applySnapshot(snapshot: BookSnapshot) {
    this.bids.clear();
    this.asks.clear();
    
    for (const [price, qty] of snapshot.bids) {
      this.bids.set(price, qty);
    }
    for (const [price, qty] of snapshot.asks) {
      this.asks.set(price, qty);
    }
    this.sequence = snapshot.sequence;
  }

  applyUpdate(update: BookUpdate): boolean {
    if (update.sequence !== this.sequence + 1) {
      return false; // Sequence gap
    }

    for (const [price, qty] of update.bids || []) {
      if (qty === '0') {
        this.bids.delete(price);
      } else {
        this.bids.set(price, qty);
      }
    }

    for (const [price, qty] of update.asks || []) {
      if (qty === '0') {
        this.asks.delete(price);
      } else {
        this.asks.set(price, qty);
      }
    }

    this.sequence = update.sequence;
    return true;
  }
}
```

## Error Handling

### Rust

```rust
use matchbook_sdk::{Error, ApiError};

match client.place_order(params).await {
    Ok(tx) => {
        // Success
    }
    Err(Error::Api(ApiError { code, message, .. })) => {
        match code.as_str() {
            "INSUFFICIENT_FUNDS" => {
                eprintln!("Not enough funds");
            }
            "INVALID_PRICE" => {
                eprintln!("Price doesn't conform to tick size");
            }
            "RATE_LIMIT_EXCEEDED" => {
                eprintln!("Rate limited, retry after delay");
            }
            _ => {
                eprintln!("API error: {}", message);
            }
        }
    }
    Err(Error::Network(e)) => {
        eprintln!("Network error: {}", e);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

### TypeScript

```typescript
import { MatchbookError, ErrorCode } from '@matchbook/sdk';

try {
  const tx = await client.placeOrder(params);
} catch (error) {
  if (error instanceof MatchbookError) {
    switch (error.code) {
      case ErrorCode.InsufficientFunds:
        console.error('Not enough funds');
        break;
      case ErrorCode.InvalidPrice:
        console.error("Price doesn't conform to tick size");
        break;
      case ErrorCode.RateLimitExceeded:
        console.error('Rate limited, retry after delay');
        await sleep(error.retryAfter * 1000);
        break;
      default:
        console.error('API error:', error.message);
    }
  } else {
    console.error('Unexpected error:', error);
  }
}
```

## Advanced Patterns

### Market Making

```typescript
class SimpleMarketMaker {
  private client: MatchbookClient;
  private market: Market;
  private spread: number;
  private size: string;
  private orders: Map<string, Order> = new Map();

  async run() {
    // Cancel existing orders
    await this.cancelAllOrders();

    // Get current mid price
    const ticker = await this.client.getTicker(this.market.address);
    const mid = (parseFloat(ticker.bestBid) + parseFloat(ticker.bestAsk)) / 2;

    // Place bid and ask
    const bidPrice = (mid * (1 - this.spread / 2)).toFixed(2);
    const askPrice = (mid * (1 + this.spread / 2)).toFixed(2);

    await Promise.all([
      this.placeOrder(Side.Bid, bidPrice, this.size),
      this.placeOrder(Side.Ask, askPrice, this.size),
    ]);
  }

  private async placeOrder(side: Side, price: string, quantity: string) {
    const tx = await this.client.placeOrder({
      market: this.market.address,
      owner: this.wallet.publicKey,
      side,
      price,
      quantity,
      orderType: OrderType.PostOnly,
      clientOrderId: Date.now(),
    });

    tx.sign([this.wallet]);
    await this.client.sendTransaction(tx);
  }
}
```

### Order Tracking

```typescript
class OrderTracker {
  private orders = new Map<number, OrderState>();
  private ws: MatchbookWebSocket;

  constructor(ws: MatchbookWebSocket, owner: PublicKey) {
    ws.subscribeOrders(owner);
    
    ws.on('order_update', (update) => {
      this.handleOrderUpdate(update);
    });
    
    ws.on('fill', (fill) => {
      this.handleFill(fill);
    });
  }

  private handleOrderUpdate(update: OrderUpdate) {
    const order = this.orders.get(update.clientOrderId);
    if (order) {
      order.status = update.status;
      order.filledQuantity = update.filledQuantity;
      order.remainingQuantity = update.remainingQuantity;
      
      if (update.status === 'filled' || update.status === 'cancelled') {
        this.emit('orderComplete', order);
      }
    }
  }

  private handleFill(fill: Fill) {
    const order = this.orders.get(fill.clientOrderId);
    if (order) {
      order.fills.push(fill);
      this.emit('fill', { order, fill });
    }
  }
}
```

## Testing

### Mock Client

```typescript
import { MockMatchbookClient } from '@matchbook/sdk/testing';

const mockClient = new MockMatchbookClient();

// Set up mock responses
mockClient.setMarkets([
  {
    address: 'mock-market',
    baseSymbol: 'SOL',
    quoteSymbol: 'USDC',
    // ...
  },
]);

mockClient.setOrderbook('mock-market', {
  bids: [['100.00', '10.0']],
  asks: [['101.00', '10.0']],
});

// Use in tests
const markets = await mockClient.getMarkets();
expect(markets).toHaveLength(1);
```

## Related Documentation

- [API Reference](./api-reference.md) - REST API documentation
- [WebSocket Reference](./websocket-reference.md) - WebSocket API documentation
- [Getting Started](./getting-started.md) - Integration guide

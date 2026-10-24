# Frequently Asked Questions

## General

### What is Matchbook?

Matchbook is a high-performance, non-custodial Central Limit Order Book (CLOB) built on Solana. It enables decentralized trading with on-chain order matching and settlement.

### Is Matchbook custodial?

No. Matchbook is fully non-custodial. Your funds are held in on-chain vaults controlled by the program, and only you can withdraw them using your wallet signature.

### Which networks are supported?

- **Mainnet**: Production environment on Solana mainnet-beta
- **Devnet**: Testing environment on Solana devnet

### What tokens can I trade?

Matchbook supports any SPL token pair. Available markets are listed at `GET /v1/markets`.

---

## Trading

### How do I place an order?

1. Connect your Solana wallet
2. Create an Open Orders account for the market (one-time)
3. Deposit tokens to your Open Orders account
4. Place orders using the API or SDK

See [Getting Started](./getting-started.md) for detailed instructions.

### What order types are supported?

| Type | Description |
|------|-------------|
| **Limit** | Execute at specified price or better |
| **Market** | Execute immediately at best available price |
| **Post-Only** | Only add liquidity; cancel if would take |

### What is Time in Force?

| TIF | Description |
|-----|-------------|
| **GTC** | Good-til-cancelled - remains until filled or cancelled |
| **IOC** | Immediate-or-cancel - fill what's possible, cancel rest |
| **FOK** | Fill-or-kill - fill entirely or cancel entirely |

### What are maker and taker fees?

- **Taker fee**: Paid when you remove liquidity (your order matches immediately)
- **Maker fee**: Paid (or rebate received) when you add liquidity (your order rests on the book)

Fee rates vary by market. Check `taker_fee_bps` and `maker_fee_bps` in market details.

### Why was my order cancelled?

Orders may be cancelled for several reasons:

- **Self-trade prevention**: Your order would trade against yourself
- **Post-only rejection**: A post-only order would have taken liquidity
- **IOC/FOK expiry**: Order couldn't be filled as specified
- **Insufficient funds**: Not enough balance to place the order

### How do I check my order status?

```typescript
// Via REST API
const orders = await client.getOrders(walletAddress);

// Via WebSocket
ws.subscribeOrders(walletAddress);
ws.on('order_update', (update) => {
  console.log(`Order ${update.orderId}: ${update.status}`);
});
```

---

## Technical

### What is an Open Orders account?

An Open Orders account is a Solana account that tracks your orders and balances for a specific market. You need one per market you trade.

### Why do I need to deposit before trading?

Deposits move tokens from your wallet to the market's vaults. This enables atomic order matching without requiring token transfers during each trade.

### How do I withdraw my funds?

Call the settle/withdraw endpoint to move available funds back to your wallet:

```typescript
const tx = await client.withdraw({
  market: marketAddress,
  owner: wallet.publicKey,
});
```

### What is the crank?

The crank is an off-chain service that:
1. Monitors markets for matchable orders
2. Submits `MatchOrders` transactions to execute trades
3. Processes the event queue

Anyone can run a crank and earn rewards for successful matches.

### Why isn't my order matching?

Check these common issues:

1. **Price**: Your bid may be below the best ask (or ask above best bid)
2. **Crank**: The crank may be temporarily unavailable
3. **Market state**: The market may be paused or in cancel-only mode

### How do I handle WebSocket disconnections?

Implement reconnection with exponential backoff:

```typescript
ws.on('close', () => {
  setTimeout(() => {
    ws.connect();
    // Resubscribe to channels
  }, Math.min(1000 * Math.pow(2, retryCount), 30000));
});
```

### What are sequence numbers?

Sequence numbers ensure you process order book updates in order. If you receive an update with an unexpected sequence, resubscribe to get a fresh snapshot.

---

## Integration

### How do I get an API key?

API keys are optional but provide higher rate limits. Contact us at api@matchbook.taunais.com for access.

### What are the rate limits?

| Tier | Requests/min | WebSocket Connections |
|------|--------------|----------------------|
| Free | 60 | 2 |
| Basic | 600 | 10 |
| Pro | 6000 | 50 |

### Can I run my own infrastructure?

Yes! Matchbook is open source. See [Deployment](./docker.md) for self-hosting instructions.

### How do I report issues?

- **Bugs**: Open an issue on [GitHub](https://github.com/joaquinbejar/matchbook/issues)
- **Security**: See [SECURITY.md](../SECURITY.md)
- **Support**: Join our [Discord](https://discord.gg/matchbook)

---

## Troubleshooting

### "Insufficient funds" error

Ensure you have:
1. Deposited enough tokens to your Open Orders account
2. Account for fees in your order size
3. Not locked all funds in existing orders

### "Invalid price" error

Prices must conform to the market's tick size. Round your price:

```typescript
const tickSize = parseFloat(market.tickSize);
const price = Math.round(desiredPrice / tickSize) * tickSize;
```

### "Invalid quantity" error

Quantities must conform to the market's lot size:

```typescript
const lotSize = parseFloat(market.lotSize);
const quantity = Math.round(desiredQty / lotSize) * lotSize;
```

### WebSocket not receiving updates

1. Check your subscription was confirmed (`subscribed` message)
2. Ensure you're sending heartbeats every 30 seconds
3. Verify the market address is correct

### Transaction failed on-chain

Check the transaction logs for the specific error:

```bash
solana confirm -v <SIGNATURE>
```

Common issues:
- **Blockhash expired**: Transaction took too long to confirm
- **Insufficient SOL**: Need SOL for transaction fees
- **Account not found**: Open Orders account doesn't exist

---

## Related Documentation

- [Getting Started](./getting-started.md) - Integration guide
- [API Reference](./api-reference.md) - REST API documentation
- [WebSocket Reference](./websocket-reference.md) - WebSocket API documentation
- [SDK Guide](./sdk-guide.md) - SDK usage examples

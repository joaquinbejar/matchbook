/**
 * Simple Market Maker Example
 *
 * This example demonstrates a basic market making strategy using the Matchbook TypeScript SDK.
 * It quotes both sides of the order book with a configurable spread.
 *
 * WARNING:
 * This is an **educational example only** and is NOT suitable for production use.
 * A real market maker would need:
 * - Sophisticated pricing models
 * - Risk management
 * - Inventory management
 * - Latency optimization
 * - Proper error handling and recovery
 *
 * Environment Variables:
 * - MATCHBOOK_API_URL - REST API base URL
 * - MATCHBOOK_MARKET - Market address
 * - MATCHBOOK_API_KEY - API key for authentication
 * - MM_SPREAD_BPS - Spread in basis points (default: 10 = 0.1%)
 * - MM_ORDER_SIZE - Order size (default: 1.0)
 *
 * Usage:
 * ```bash
 * MATCHBOOK_MARKET=ABC123... MATCHBOOK_API_KEY=key npx ts-node examples/simple-mm.ts
 * ```
 */

import { MatchbookClient, type PlaceOrderParams } from '../src';

interface MmConfig {
  market: string;
  spreadBps: number;
  orderSize: string;
  updateIntervalMs: number;
}

interface MarketMakerState {
  bidOrderId: string | null;
  askOrderId: string | null;
}

/**
 * Calculate bid and ask prices based on mid price and spread.
 */
function calculateQuotes(midPrice: number, spreadBps: number): { bid: string; ask: string } {
  // spreadBps = 10 means 0.1% total spread, so 0.05% on each side
  const halfSpreadFactor = spreadBps / 20000; // bps to decimal, then half

  const bidPrice = midPrice * (1 - halfSpreadFactor);
  const askPrice = midPrice * (1 + halfSpreadFactor);

  return {
    bid: bidPrice.toFixed(2),
    ask: askPrice.toFixed(2),
  };
}

/**
 * Update quotes based on current market state.
 */
async function updateQuotes(
  client: MatchbookClient,
  config: MmConfig,
  state: MarketMakerState
): Promise<void> {
  // Step 1: Fetch current order book
  const book = await client.getOrderbook(config.market, 5);

  // Step 2: Calculate mid price
  let midPrice: number;
  if (book.bids.length > 0 && book.asks.length > 0) {
    const bestBid = parseFloat(book.bids[0].price);
    const bestAsk = parseFloat(book.asks[0].price);
    midPrice = (bestBid + bestAsk) / 2;
  } else if (book.bids.length > 0) {
    midPrice = parseFloat(book.bids[0].price);
  } else if (book.asks.length > 0) {
    midPrice = parseFloat(book.asks[0].price);
  } else {
    console.log('  No orders in book, skipping update');
    return;
  }

  console.log(`  Mid price: ${midPrice.toFixed(2)}`);

  // Step 3: Calculate our quotes
  const { bid, ask } = calculateQuotes(midPrice, config.spreadBps);
  console.log(`  Our bid: ${bid}, Our ask: ${ask}`);

  // Step 4: Cancel existing orders (if any)
  // In a real MM, you'd be smarter about when to cancel vs amend
  if (state.bidOrderId || state.askOrderId) {
    console.log('  Cancelling existing orders...');
    // In production, you'd actually cancel here
    state.bidOrderId = null;
    state.askOrderId = null;
  }

  // Step 5: Place new orders
  console.log('  Placing new orders...');

  // Place bid
  const bidParams: PlaceOrderParams = {
    market: config.market,
    side: 'bid',
    price: bid,
    quantity: config.orderSize,
    orderType: 'postOnly', // Use postOnly to ensure we're maker
  };

  try {
    await client.buildPlaceOrderTx(bidParams);
    console.log(`    Bid order built: ${config.orderSize} @ ${bid}`);
    // In production, you'd sign and send the transaction
    // state.bidOrderId = orderId;
  } catch (error) {
    console.log(`    Failed to build bid order: ${error}`);
  }

  // Place ask
  const askParams: PlaceOrderParams = {
    market: config.market,
    side: 'ask',
    price: ask,
    quantity: config.orderSize,
    orderType: 'postOnly',
  };

  try {
    await client.buildPlaceOrderTx(askParams);
    console.log(`    Ask order built: ${config.orderSize} @ ${ask}`);
    // In production, you'd sign and send the transaction
    // state.askOrderId = orderId;
  } catch (error) {
    console.log(`    Failed to build ask order: ${error}`);
  }
}

/**
 * Run the market maker loop.
 */
async function runMarketMaker(
  client: MatchbookClient,
  config: MmConfig,
  iterations: number
): Promise<void> {
  console.log('\n--- Starting Market Maker ---');
  console.log(`  Market: ${config.market}`);
  console.log(`  Spread: ${config.spreadBps} bps`);
  console.log(`  Order size: ${config.orderSize}`);
  console.log(`  Update interval: ${config.updateIntervalMs}ms`);
  console.log();

  const state: MarketMakerState = {
    bidOrderId: null,
    askOrderId: null,
  };

  for (let i = 1; i <= iterations; i++) {
    console.log(`Iteration ${i}/${iterations}:`);

    try {
      await updateQuotes(client, config, state);
    } catch (error) {
      console.log(`  Error updating quotes: ${error}`);
    }

    if (i < iterations) {
      console.log(`  Sleeping for ${config.updateIntervalMs}ms...\n`);
      await new Promise((resolve) => setTimeout(resolve, config.updateIntervalMs));
    }
  }

  // Cleanup: Cancel all orders
  console.log('\n--- Shutting Down ---');
  console.log('  Cancelling all orders...');
  // In production, you'd cancel all open orders here
}

async function main(): Promise<void> {
  // Load configuration from environment variables
  const apiUrl = process.env.MATCHBOOK_API_URL ?? 'https://api.matchbook.example/v1';
  const marketAddress = process.env.MATCHBOOK_MARKET;
  const apiKey = process.env.MATCHBOOK_API_KEY;

  if (!marketAddress) {
    console.error('Error: MATCHBOOK_MARKET environment variable is required');
    process.exit(1);
  }

  if (!apiKey) {
    console.error('Error: MATCHBOOK_API_KEY environment variable is required');
    process.exit(1);
  }

  const spreadBps = parseInt(process.env.MM_SPREAD_BPS ?? '10', 10);
  const orderSize = process.env.MM_ORDER_SIZE ?? '1.0';

  console.log('=== Matchbook Simple Market Maker Example ===\n');
  console.log('⚠️  WARNING: This is an educational example only!');
  console.log('⚠️  Do NOT use this for real trading without significant modifications.\n');

  // Create HTTP client
  const client = new MatchbookClient({
    baseUrl: apiUrl,
    apiKey,
  });

  // Create market maker configuration
  const config: MmConfig = {
    market: marketAddress,
    spreadBps,
    orderSize,
    updateIntervalMs: 5000,
  };

  // Run for 3 iterations (for demonstration)
  await runMarketMaker(client, config, 3);

  console.log('\n=== Example Complete ===');
  console.log('\nKey takeaways for a real market maker:');
  console.log('  1. Use WebSocket for real-time book updates');
  console.log('  2. Implement proper inventory management');
  console.log('  3. Add risk limits and circuit breakers');
  console.log('  4. Handle partial fills and order amendments');
  console.log('  5. Monitor latency and optimize for speed');
  console.log('  6. Implement proper logging and monitoring');
}

main().catch(console.error);

/**
 * Market Data Example
 *
 * This example demonstrates how to fetch and display market information,
 * order book data, and recent trades using the Matchbook TypeScript SDK.
 *
 * Environment Variables:
 * - MATCHBOOK_API_URL - REST API base URL (default: https://api.matchbook.example/v1)
 * - MATCHBOOK_MARKET - Market address to query
 *
 * Usage:
 * ```bash
 * MATCHBOOK_MARKET=ABC123... npx ts-node examples/market-data.ts
 * ```
 */

import { MatchbookClient, type Market, type OrderBook, type Trade } from '../src';

async function main(): Promise<void> {
  // Load configuration from environment variables
  const apiUrl = process.env.MATCHBOOK_API_URL ?? 'https://api.matchbook.example/v1';
  const marketAddress = process.env.MATCHBOOK_MARKET;

  if (!marketAddress) {
    console.error('Error: MATCHBOOK_MARKET environment variable is required');
    process.exit(1);
  }

  console.log('=== Matchbook Market Data Example ===\n');
  console.log(`API URL: ${apiUrl}`);
  console.log(`Market: ${marketAddress}\n`);

  // Create the HTTP client
  const client = new MatchbookClient({ baseUrl: apiUrl });

  // Step 1: Fetch all available markets
  console.log('--- Fetching All Markets ---');
  try {
    const markets = await client.getMarkets();
    console.log(`Found ${markets.length} markets:`);
    for (const market of markets.slice(0, 5)) {
      console.log(`  - ${market.address} (${market.baseSymbol ?? '???'}/${market.quoteSymbol ?? '???'})`);
    }
    if (markets.length > 5) {
      console.log(`  ... and ${markets.length - 5} more`);
    }
  } catch (error) {
    console.log(`Error fetching markets: ${error}`);
  }
  console.log();

  // Step 2: Fetch specific market details
  console.log('--- Fetching Market Details ---');
  try {
    const market = await client.getMarket(marketAddress);
    console.log(`Market: ${market.address}`);
    console.log(`  Pair: ${market.baseSymbol ?? '???'}/${market.quoteSymbol ?? '???'}`);
    console.log(`  Base Mint: ${market.baseMint}`);
    console.log(`  Quote Mint: ${market.quoteMint}`);
    console.log(`  Tick Size: ${market.tickSize}`);
    console.log(`  Lot Size: ${market.lotSize}`);
    console.log(`  Maker Fee: ${(market.makerFee * 100).toFixed(4)}%`);
    console.log(`  Taker Fee: ${(market.takerFee * 100).toFixed(4)}%`);
  } catch (error) {
    console.log(`Error fetching market: ${error}`);
  }
  console.log();

  // Step 3: Fetch order book
  console.log('--- Fetching Order Book (depth=10) ---');
  try {
    const book = await client.getOrderbook(marketAddress, 10);
    console.log(`Order Book for ${book.market}`);

    console.log('\n  ASKS (selling):');
    for (let i = 0; i < Math.min(5, book.asks.length); i++) {
      const level = book.asks[i];
      console.log(`    ${i + 1}: ${level.quantity} @ ${level.price} (${level.orders ?? 0} orders)`);
    }

    console.log('\n  BIDS (buying):');
    for (let i = 0; i < Math.min(5, book.bids.length); i++) {
      const level = book.bids[i];
      console.log(`    ${i + 1}: ${level.quantity} @ ${level.price} (${level.orders ?? 0} orders)`);
    }

    // Calculate spread if we have both sides
    if (book.bids.length > 0 && book.asks.length > 0) {
      const bestBid = parseFloat(book.bids[0].price);
      const bestAsk = parseFloat(book.asks[0].price);
      const spread = bestAsk - bestBid;
      const spreadPct = (spread / bestBid) * 100;
      console.log(`\n  Spread: ${spread.toFixed(4)} (${spreadPct.toFixed(4)}%)`);
    }
  } catch (error) {
    console.log(`Error fetching order book: ${error}`);
  }
  console.log();

  // Step 4: Fetch recent trades
  console.log('--- Fetching Recent Trades (limit=10) ---');
  try {
    const trades = await client.getTrades(marketAddress, { limit: 10 });
    console.log('Recent trades:');
    for (const trade of trades.slice(0, 10)) {
      console.log(`  ${trade.side} ${trade.quantity} @ ${trade.price} (${trade.timestamp})`);
    }
  } catch (error) {
    console.log(`Error fetching trades: ${error}`);
  }

  console.log('\n=== Example Complete ===');
}

main().catch(console.error);

/**
 * Place Order Example
 *
 * This example demonstrates how to place a limit order using the Matchbook TypeScript SDK.
 * It shows how to build a transaction, which can then be signed and sent.
 *
 * Environment Variables:
 * - MATCHBOOK_API_URL - REST API base URL
 * - MATCHBOOK_MARKET - Market address
 * - MATCHBOOK_API_KEY - API key for authentication
 *
 * Usage:
 * ```bash
 * MATCHBOOK_MARKET=ABC123... MATCHBOOK_API_KEY=your-key npx ts-node examples/place-order.ts
 * ```
 *
 * Warning:
 * This example places a real order on devnet. Make sure you understand the
 * implications before running it.
 */

import { MatchbookClient, type PlaceOrderParams } from '../src';

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

  console.log('=== Matchbook Place Order Example ===\n');
  console.log(`API URL: ${apiUrl}`);
  console.log(`Market: ${marketAddress}\n`);

  // Create the HTTP client with authentication
  const client = new MatchbookClient({
    baseUrl: apiUrl,
    apiKey,
  });

  // Step 1: Fetch current order book to determine a reasonable price
  console.log('--- Fetching Current Order Book ---');
  try {
    const book = await client.getOrderbook(marketAddress, 5);

    let referencePrice = '100.0';
    if (book.bids.length > 0) {
      console.log(`Best bid: ${book.bids[0].price}`);
      referencePrice = book.bids[0].price;
    } else {
      console.log('No bids in order book, using default price');
    }

    // Place a bid slightly below the best bid (to avoid immediate fill)
    // This is for demonstration - in production, you'd have your own pricing logic
    const orderPrice = (parseFloat(referencePrice) - 1.0).toFixed(2);
    console.log(`Order price: ${orderPrice}`);
    console.log();

    // Step 2: Build the place order parameters
    console.log('--- Building Order ---');
    const orderParams: PlaceOrderParams = {
      market: marketAddress,
      side: 'bid',
      price: orderPrice,
      quantity: '1.0',
      orderType: 'limit',
    };

    console.log('Order parameters:');
    console.log(`  Market: ${orderParams.market}`);
    console.log(`  Side: ${orderParams.side}`);
    console.log(`  Price: ${orderParams.price}`);
    console.log(`  Quantity: ${orderParams.quantity}`);
    console.log(`  Type: ${orderParams.orderType}`);
    console.log();

    // Step 3: Build the transaction
    console.log('--- Building Transaction ---');
    const txResponse = await client.buildPlaceOrderTx(orderParams);

    console.log('Transaction built successfully!');
    console.log(`  Blockhash: ${txResponse.blockhash}`);
    console.log(`  Last valid block height: ${txResponse.lastValidBlockHeight}`);
    console.log(`  Transaction (base64): ${txResponse.transaction.slice(0, 50)}...`);

    // In a real application, you would:
    // 1. Deserialize the transaction
    // 2. Sign it with your wallet
    // 3. Send it to the network
    // 4. Wait for confirmation

    console.log('\nNote: To complete the order, you would need to:');
    console.log('  1. Deserialize the transaction from base64');
    console.log('  2. Sign it with your wallet keypair');
    console.log('  3. Send it to the Solana network');
    console.log('  4. Wait for confirmation');
  } catch (error) {
    console.log(`Error: ${error}`);
    console.log('\nThis is expected if the API is not running or the market doesn\'t exist.');
  }

  // Step 4: Monitor order status (if we had placed an order)
  console.log('\n--- Monitoring Orders ---');
  console.log('To monitor your orders, you would call:');
  console.log('  const orders = await client.getOrders(ownerAddress, marketAddress);');

  console.log('\n=== Example Complete ===');
}

main().catch(console.error);

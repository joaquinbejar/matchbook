/**
 * Stream Order Book Example
 *
 * This example demonstrates how to subscribe to real-time order book updates
 * using the Matchbook WebSocket client.
 *
 * Environment Variables:
 * - MATCHBOOK_WS_URL - WebSocket URL (default: wss://ws.matchbook.example/v1/stream)
 * - MATCHBOOK_MARKET - Market address to subscribe to
 * - MATCHBOOK_API_KEY - Optional API key for authentication
 *
 * Usage:
 * ```bash
 * MATCHBOOK_MARKET=ABC123... npx ts-node examples/stream-book.ts
 * ```
 */

import {
  MatchbookWsClient,
  type WsBookSnapshotMessage,
  type WsBookUpdateMessage,
} from '../src';

async function main(): Promise<void> {
  // Load configuration from environment variables
  const wsUrl = process.env.MATCHBOOK_WS_URL ?? 'wss://ws.matchbook.example/v1/stream';
  const marketAddress = process.env.MATCHBOOK_MARKET;
  const apiKey = process.env.MATCHBOOK_API_KEY;

  if (!marketAddress) {
    console.error('Error: MATCHBOOK_MARKET environment variable is required');
    process.exit(1);
  }

  console.log('=== Matchbook Stream Order Book Example ===\n');
  console.log(`WebSocket URL: ${wsUrl}`);
  console.log(`Market: ${marketAddress}`);
  console.log(`API Key: ${apiKey ? 'set' : 'not set'}`);
  console.log();

  // Create WebSocket client
  const ws = new MatchbookWsClient({
    wsUrl,
    apiKey,
  });

  // Set up error handler
  ws.onError((error) => {
    console.error('WebSocket error:', error.message);
  });

  // Set up disconnect handler
  ws.onDisconnect(() => {
    console.log('WebSocket disconnected');
  });

  console.log('--- Connecting to WebSocket ---');
  console.log('Note: This example shows the structure of WebSocket streaming.');
  console.log('In a real scenario, you would connect and receive updates.\n');

  // In a real application, you would connect and subscribe:
  //
  // try {
  //   await ws.connect();
  //   console.log('Connected!');
  //
  //   // Subscribe to order book updates
  //   const subscriptionId = ws.subscribeBook(marketAddress, (data) => {
  //     if (data.type === 'book_snapshot') {
  //       handleSnapshot(data);
  //     } else {
  //       handleUpdate(data);
  //     }
  //   }, 20);
  //
  //   console.log(`Subscribed with ID: ${subscriptionId}`);
  //
  //   // Run for some time...
  //   await new Promise(resolve => setTimeout(resolve, 60000));
  //
  //   // Cleanup
  //   ws.unsubscribe(subscriptionId);
  //   ws.disconnect();
  // } catch (error) {
  //   console.error('Connection failed:', error);
  // }

  // Demonstrate message handling
  console.log('--- Message Handling Examples ---\n');

  console.log('When you receive a book_snapshot message:');
  console.log('  - Replace your local order book with the snapshot');
  console.log('  - Store the sequence number for ordering');
  console.log();

  console.log('Example snapshot handler:');
  console.log(`
  function handleSnapshot(data: WsBookSnapshotMessage) {
    localBook = {
      market: data.market,
      bids: data.bids,
      asks: data.asks,
      sequence: data.sequence,
    };
    console.log(\`Received snapshot with \${data.bids.length} bids and \${data.asks.length} asks\`);
  }
  `);

  console.log('When you receive a book_update message:');
  console.log('  - Check sequence number is consecutive');
  console.log('  - Apply delta updates to your local book');
  console.log('  - Quantity of "0" means remove the level');
  console.log();

  console.log('Example update handler:');
  console.log(`
  function handleUpdate(data: WsBookUpdateMessage) {
    // Check sequence
    if (data.sequence !== localBook.sequence + 1) {
      console.warn('Sequence gap detected, requesting new snapshot');
      return;
    }

    // Apply bid updates
    for (const change of data.bids) {
      if (change.quantity === '0') {
        // Remove level
        localBook.bids = localBook.bids.filter(l => l.price !== change.price);
      } else {
        // Update or insert level
        const idx = localBook.bids.findIndex(l => l.price === change.price);
        if (idx >= 0) {
          localBook.bids[idx].quantity = change.quantity;
        } else {
          localBook.bids.push({ price: change.price, quantity: change.quantity });
          localBook.bids.sort((a, b) => parseFloat(b.price) - parseFloat(a.price));
        }
      }
    }

    // Apply ask updates (similar logic)
    // ...

    localBook.sequence = data.sequence;
  }
  `);

  // Demonstrate proper cleanup
  console.log('--- Proper Cleanup ---\n');
  console.log('Always unsubscribe and close connections properly:');
  console.log(`
  // Unsubscribe from channel
  ws.unsubscribe(subscriptionId);

  // Disconnect
  ws.disconnect();
  `);

  // Demonstrate reconnection handling
  console.log('--- Reconnection Handling ---\n');
  console.log('The WebSocket client handles reconnection automatically:');
  console.log('  - Exponential backoff (1s, 2s, 4s, ... up to 30s)');
  console.log('  - Automatic resubscription after reconnect');
  console.log('  - Request new snapshot after reconnect to ensure consistency');

  // Demonstrate rate limiting awareness
  console.log('\n--- Rate Limiting ---\n');
  console.log('Be aware of rate limits:');
  console.log("  - Don't subscribe/unsubscribe too frequently");
  console.log('  - Handle rate limit errors gracefully');
  console.log('  - Use heartbeat pings to keep connection alive');

  console.log('\n=== Example Complete ===');
  console.log('\nTo run a real WebSocket connection, you would need:');
  console.log('  1. A running Matchbook API server');
  console.log('  2. A valid market address');
  console.log('  3. Network connectivity to the WebSocket endpoint');
}

main().catch(console.error);

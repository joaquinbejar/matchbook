/**
 * WebSocket client implementation.
 *
 * @module client/websocket
 */

import type {
  WsChannel,
  WsServerMessage,
  WsBookSnapshotMessage,
  WsBookUpdateMessage,
  WsTradeMessage,
  WsOrderUpdateMessage,
} from '../api/index.js';

import type { ResolvedConfig } from './config.js';
import { resolveConfig, validateConfig, type ClientConfig } from './config.js';
import { WebSocketError } from './errors.js';

/**
 * Subscription callback types.
 */
export type BookCallback = (data: WsBookSnapshotMessage | WsBookUpdateMessage) => void;
export type TradeCallback = (data: WsTradeMessage) => void;
export type OrderCallback = (data: WsOrderUpdateMessage) => void;
export type ErrorCallback = (error: Error) => void;

/**
 * Subscription info.
 */
interface Subscription {
  id: string;
  channel: WsChannel;
  market?: string;
  callback: BookCallback | TradeCallback | OrderCallback;
}

/**
 * WebSocket client for real-time streaming.
 *
 * Provides methods for subscribing to order book updates, trades, and user orders.
 *
 * @example
 * ```typescript
 * const ws = new MatchbookWsClient({
 *   wsUrl: 'wss://ws.matchbook.taunais.com/v1/stream',
 *   apiKey: 'your-api-key',
 * });
 *
 * await ws.connect();
 *
 * // Subscribe to order book updates
 * const subId = ws.subscribeBook('ABC123...', (data) => {
 *   console.log('Book update:', data);
 * });
 *
 * // Later: unsubscribe
 * ws.unsubscribe(subId);
 *
 * // Disconnect
 * ws.disconnect();
 * ```
 */
export class MatchbookWsClient {
  private readonly config: ResolvedConfig;
  private ws: WebSocket | null = null;
  private subscriptions: Map<string, Subscription> = new Map();
  private nextSubId = 1;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private reconnectDelay = 1000;
  private maxReconnectDelay = 30000;
  private pingInterval: ReturnType<typeof setInterval> | null = null;
  private onErrorCallback: ErrorCallback | null = null;
  private onDisconnectCallback: (() => void) | null = null;

  /**
   * Creates a new WebSocket client.
   *
   * @param config - Client configuration.
   * @throws Error if configuration is invalid.
   */
  constructor(config: ClientConfig = {}) {
    this.config = resolveConfig(config);
    validateConfig(this.config);
  }

  /**
   * Returns the client configuration.
   */
  get configuration(): ResolvedConfig {
    return this.config;
  }

  /**
   * Returns true if connected.
   */
  get isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }

  /**
   * Connects to the WebSocket server.
   *
   * @returns Promise that resolves when connected.
   * @throws WebSocketError if connection fails.
   */
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.isConnected) {
        resolve();
        return;
      }

      const url = this.config.apiKey
        ? `${this.config.wsUrl}?api_key=${this.config.apiKey}`
        : this.config.wsUrl;

      try {
        this.ws = new WebSocket(url);
      } catch (error) {
        reject(new WebSocketError(`Failed to create WebSocket: ${error}`));
        return;
      }

      this.ws.onopen = () => {
        this.reconnectAttempts = 0;
        this.startPingInterval();
        resolve();
      };

      this.ws.onerror = () => {
        const error = new WebSocketError('WebSocket error');
        if (this.onErrorCallback) {
          this.onErrorCallback(error);
        }
        reject(error);
      };

      this.ws.onclose = () => {
        this.stopPingInterval();
        if (this.onDisconnectCallback) {
          this.onDisconnectCallback();
        }
        this.handleReconnect();
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };
    });
  }

  /**
   * Disconnects from the WebSocket server.
   */
  disconnect(): void {
    this.stopPingInterval();
    this.maxReconnectAttempts = 0; // Prevent reconnection

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.subscriptions.clear();
  }

  /**
   * Sets the error callback.
   *
   * @param callback - Callback for errors.
   */
  onError(callback: ErrorCallback): void {
    this.onErrorCallback = callback;
  }

  /**
   * Sets the disconnect callback.
   *
   * @param callback - Callback for disconnection.
   */
  onDisconnect(callback: () => void): void {
    this.onDisconnectCallback = callback;
  }

  /**
   * Subscribes to order book updates.
   *
   * @param market - Market address.
   * @param callback - Callback for book updates.
   * @param depth - Optional depth (number of levels).
   * @returns Subscription ID.
   */
  subscribeBook(market: string, callback: BookCallback, depth?: number): string {
    const id = this.generateSubId();
    const subscription: Subscription = {
      id,
      channel: 'book',
      market,
      callback,
    };

    this.subscriptions.set(id, subscription);
    this.sendSubscribe('book', market, depth);

    return id;
  }

  /**
   * Subscribes to trade stream.
   *
   * @param market - Market address.
   * @param callback - Callback for trades.
   * @returns Subscription ID.
   */
  subscribeTrades(market: string, callback: TradeCallback): string {
    const id = this.generateSubId();
    const subscription: Subscription = {
      id,
      channel: 'trades',
      market,
      callback,
    };

    this.subscriptions.set(id, subscription);
    this.sendSubscribe('trades', market);

    return id;
  }

  /**
   * Subscribes to user order updates.
   *
   * @param callback - Callback for order updates.
   * @returns Subscription ID.
   */
  subscribeOrders(callback: OrderCallback): string {
    const id = this.generateSubId();
    const subscription: Subscription = {
      id,
      channel: 'orders',
      callback,
    };

    this.subscriptions.set(id, subscription);
    this.sendSubscribe('orders');

    return id;
  }

  /**
   * Unsubscribes from a channel.
   *
   * @param subscriptionId - Subscription ID to unsubscribe.
   */
  unsubscribe(subscriptionId: string): void {
    const subscription = this.subscriptions.get(subscriptionId);
    if (!subscription) {
      return;
    }

    this.subscriptions.delete(subscriptionId);
    this.sendUnsubscribe(subscription.channel, subscription.market);
  }

  /**
   * Generates a unique subscription ID.
   */
  private generateSubId(): string {
    return `sub_${this.nextSubId++}`;
  }

  /**
   * Sends a subscribe message.
   */
  private sendSubscribe(channel: WsChannel, market?: string, depth?: number): void {
    if (!this.isConnected) {
      return;
    }

    const message: Record<string, unknown> = {
      type: 'subscribe',
      channel,
    };

    if (market) {
      message.market = market;
    }

    if (depth !== undefined) {
      message.depth = depth;
    }

    this.ws!.send(JSON.stringify(message));
  }

  /**
   * Sends an unsubscribe message.
   */
  private sendUnsubscribe(channel: WsChannel, market?: string): void {
    if (!this.isConnected) {
      return;
    }

    const message: Record<string, unknown> = {
      type: 'unsubscribe',
      channel,
    };

    if (market) {
      message.market = market;
    }

    this.ws!.send(JSON.stringify(message));
  }

  /**
   * Handles incoming messages.
   */
  private handleMessage(data: string): void {
    let message: WsServerMessage;
    try {
      message = JSON.parse(data);
    } catch {
      return;
    }

    switch (message.type) {
      case 'book_snapshot':
      case 'book_update':
        this.dispatchBookMessage(message as WsBookSnapshotMessage | WsBookUpdateMessage);
        break;

      case 'trade':
        this.dispatchTradeMessage(message as WsTradeMessage);
        break;

      case 'order_update':
        this.dispatchOrderMessage(message as WsOrderUpdateMessage);
        break;

      case 'pong':
        // Heartbeat response, no action needed
        break;

      case 'error':
        if (this.onErrorCallback) {
          const errorMsg = message as { code: string; message: string };
          this.onErrorCallback(new WebSocketError(`${errorMsg.code}: ${errorMsg.message}`));
        }
        break;
    }
  }

  /**
   * Dispatches book messages to subscribers.
   */
  private dispatchBookMessage(message: WsBookSnapshotMessage | WsBookUpdateMessage): void {
    for (const subscription of this.subscriptions.values()) {
      if (
        subscription.channel === 'book' &&
        subscription.market === message.market
      ) {
        (subscription.callback as BookCallback)(message);
      }
    }
  }

  /**
   * Dispatches trade messages to subscribers.
   */
  private dispatchTradeMessage(message: WsTradeMessage): void {
    for (const subscription of this.subscriptions.values()) {
      if (
        subscription.channel === 'trades' &&
        subscription.market === message.market
      ) {
        (subscription.callback as TradeCallback)(message);
      }
    }
  }

  /**
   * Dispatches order messages to subscribers.
   */
  private dispatchOrderMessage(message: WsOrderUpdateMessage): void {
    for (const subscription of this.subscriptions.values()) {
      if (subscription.channel === 'orders') {
        (subscription.callback as OrderCallback)(message);
      }
    }
  }

  /**
   * Starts the ping interval.
   */
  private startPingInterval(): void {
    this.stopPingInterval();
    this.pingInterval = setInterval(() => {
      if (this.isConnected) {
        this.ws!.send(JSON.stringify({
          type: 'ping',
          timestamp: Date.now(),
        }));
      }
    }, 30000);
  }

  /**
   * Stops the ping interval.
   */
  private stopPingInterval(): void {
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }

  /**
   * Handles reconnection logic.
   */
  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(
      this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1),
      this.maxReconnectDelay
    );

    setTimeout(async () => {
      try {
        await this.connect();
        // Resubscribe to all channels
        this.resubscribeAll();
      } catch {
        // Will retry via onclose handler
      }
    }, delay);
  }

  /**
   * Resubscribes to all active subscriptions.
   */
  private resubscribeAll(): void {
    for (const subscription of this.subscriptions.values()) {
      this.sendSubscribe(subscription.channel, subscription.market);
    }
  }
}

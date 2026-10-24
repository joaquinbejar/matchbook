/**
 * WebSocket client implementation.
 *
 * @module client/websocket
 */
import type { WsBookSnapshotMessage, WsBookUpdateMessage, WsTradeMessage, WsOrderUpdateMessage } from '../api';
import type { ResolvedConfig } from './config';
import { type ClientConfig } from './config';
/**
 * Subscription callback types.
 */
export type BookCallback = (data: WsBookSnapshotMessage | WsBookUpdateMessage) => void;
export type TradeCallback = (data: WsTradeMessage) => void;
export type OrderCallback = (data: WsOrderUpdateMessage) => void;
export type ErrorCallback = (error: Error) => void;
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
export declare class MatchbookWsClient {
    private readonly config;
    private ws;
    private subscriptions;
    private nextSubId;
    private reconnectAttempts;
    private maxReconnectAttempts;
    private reconnectDelay;
    private maxReconnectDelay;
    private pingInterval;
    private onErrorCallback;
    private onDisconnectCallback;
    /**
     * Creates a new WebSocket client.
     *
     * @param config - Client configuration.
     * @throws Error if configuration is invalid.
     */
    constructor(config?: ClientConfig);
    /**
     * Returns the client configuration.
     */
    get configuration(): ResolvedConfig;
    /**
     * Returns true if connected.
     */
    get isConnected(): boolean;
    /**
     * Connects to the WebSocket server.
     *
     * @returns Promise that resolves when connected.
     * @throws WebSocketError if connection fails.
     */
    connect(): Promise<void>;
    /**
     * Disconnects from the WebSocket server.
     */
    disconnect(): void;
    /**
     * Sets the error callback.
     *
     * @param callback - Callback for errors.
     */
    onError(callback: ErrorCallback): void;
    /**
     * Sets the disconnect callback.
     *
     * @param callback - Callback for disconnection.
     */
    onDisconnect(callback: () => void): void;
    /**
     * Subscribes to order book updates.
     *
     * @param market - Market address.
     * @param callback - Callback for book updates.
     * @param depth - Optional depth (number of levels).
     * @returns Subscription ID.
     */
    subscribeBook(market: string, callback: BookCallback, depth?: number): string;
    /**
     * Subscribes to trade stream.
     *
     * @param market - Market address.
     * @param callback - Callback for trades.
     * @returns Subscription ID.
     */
    subscribeTrades(market: string, callback: TradeCallback): string;
    /**
     * Subscribes to user order updates.
     *
     * @param callback - Callback for order updates.
     * @returns Subscription ID.
     */
    subscribeOrders(callback: OrderCallback): string;
    /**
     * Unsubscribes from a channel.
     *
     * @param subscriptionId - Subscription ID to unsubscribe.
     */
    unsubscribe(subscriptionId: string): void;
    /**
     * Generates a unique subscription ID.
     */
    private generateSubId;
    /**
     * Sends a subscribe message.
     */
    private sendSubscribe;
    /**
     * Sends an unsubscribe message.
     */
    private sendUnsubscribe;
    /**
     * Handles incoming messages.
     */
    private handleMessage;
    /**
     * Dispatches book messages to subscribers.
     */
    private dispatchBookMessage;
    /**
     * Dispatches trade messages to subscribers.
     */
    private dispatchTradeMessage;
    /**
     * Dispatches order messages to subscribers.
     */
    private dispatchOrderMessage;
    /**
     * Starts the ping interval.
     */
    private startPingInterval;
    /**
     * Stops the ping interval.
     */
    private stopPingInterval;
    /**
     * Handles reconnection logic.
     */
    private handleReconnect;
    /**
     * Resubscribes to all active subscriptions.
     */
    private resubscribeAll;
}
//# sourceMappingURL=websocket.d.ts.map
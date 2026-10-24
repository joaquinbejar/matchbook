/**
 * WebSocket client implementation.
 *
 * @module client/websocket
 */
import { resolveConfig, validateConfig } from './config';
import { WebSocketError } from './errors';
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
    /**
     * Creates a new WebSocket client.
     *
     * @param config - Client configuration.
     * @throws Error if configuration is invalid.
     */
    constructor(config = {}) {
        this.ws = null;
        this.subscriptions = new Map();
        this.nextSubId = 1;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 10;
        this.reconnectDelay = 1000;
        this.maxReconnectDelay = 30000;
        this.pingInterval = null;
        this.onErrorCallback = null;
        this.onDisconnectCallback = null;
        this.config = resolveConfig(config);
        validateConfig(this.config);
    }
    /**
     * Returns the client configuration.
     */
    get configuration() {
        return this.config;
    }
    /**
     * Returns true if connected.
     */
    get isConnected() {
        return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
    }
    /**
     * Connects to the WebSocket server.
     *
     * @returns Promise that resolves when connected.
     * @throws WebSocketError if connection fails.
     */
    connect() {
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
            }
            catch (error) {
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
    disconnect() {
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
    onError(callback) {
        this.onErrorCallback = callback;
    }
    /**
     * Sets the disconnect callback.
     *
     * @param callback - Callback for disconnection.
     */
    onDisconnect(callback) {
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
    subscribeBook(market, callback, depth) {
        const id = this.generateSubId();
        const subscription = {
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
    subscribeTrades(market, callback) {
        const id = this.generateSubId();
        const subscription = {
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
    subscribeOrders(callback) {
        const id = this.generateSubId();
        const subscription = {
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
    unsubscribe(subscriptionId) {
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
    generateSubId() {
        return `sub_${this.nextSubId++}`;
    }
    /**
     * Sends a subscribe message.
     */
    sendSubscribe(channel, market, depth) {
        if (!this.isConnected) {
            return;
        }
        const message = {
            type: 'subscribe',
            channel,
        };
        if (market) {
            message.market = market;
        }
        if (depth !== undefined) {
            message.depth = depth;
        }
        this.ws.send(JSON.stringify(message));
    }
    /**
     * Sends an unsubscribe message.
     */
    sendUnsubscribe(channel, market) {
        if (!this.isConnected) {
            return;
        }
        const message = {
            type: 'unsubscribe',
            channel,
        };
        if (market) {
            message.market = market;
        }
        this.ws.send(JSON.stringify(message));
    }
    /**
     * Handles incoming messages.
     */
    handleMessage(data) {
        let message;
        try {
            message = JSON.parse(data);
        }
        catch {
            return;
        }
        switch (message.type) {
            case 'book_snapshot':
            case 'book_update':
                this.dispatchBookMessage(message);
                break;
            case 'trade':
                this.dispatchTradeMessage(message);
                break;
            case 'order_update':
                this.dispatchOrderMessage(message);
                break;
            case 'pong':
                // Heartbeat response, no action needed
                break;
            case 'error':
                if (this.onErrorCallback) {
                    const errorMsg = message;
                    this.onErrorCallback(new WebSocketError(`${errorMsg.code}: ${errorMsg.message}`));
                }
                break;
        }
    }
    /**
     * Dispatches book messages to subscribers.
     */
    dispatchBookMessage(message) {
        for (const subscription of this.subscriptions.values()) {
            if (subscription.channel === 'book' &&
                subscription.market === message.market) {
                subscription.callback(message);
            }
        }
    }
    /**
     * Dispatches trade messages to subscribers.
     */
    dispatchTradeMessage(message) {
        for (const subscription of this.subscriptions.values()) {
            if (subscription.channel === 'trades' &&
                subscription.market === message.market) {
                subscription.callback(message);
            }
        }
    }
    /**
     * Dispatches order messages to subscribers.
     */
    dispatchOrderMessage(message) {
        for (const subscription of this.subscriptions.values()) {
            if (subscription.channel === 'orders') {
                subscription.callback(message);
            }
        }
    }
    /**
     * Starts the ping interval.
     */
    startPingInterval() {
        this.stopPingInterval();
        this.pingInterval = setInterval(() => {
            if (this.isConnected) {
                this.ws.send(JSON.stringify({
                    type: 'ping',
                    timestamp: Date.now(),
                }));
            }
        }, 30000);
    }
    /**
     * Stops the ping interval.
     */
    stopPingInterval() {
        if (this.pingInterval) {
            clearInterval(this.pingInterval);
            this.pingInterval = null;
        }
    }
    /**
     * Handles reconnection logic.
     */
    handleReconnect() {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            return;
        }
        this.reconnectAttempts++;
        const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), this.maxReconnectDelay);
        setTimeout(async () => {
            try {
                await this.connect();
                // Resubscribe to all channels
                this.resubscribeAll();
            }
            catch {
                // Will retry via onclose handler
            }
        }, delay);
    }
    /**
     * Resubscribes to all active subscriptions.
     */
    resubscribeAll() {
        for (const subscription of this.subscriptions.values()) {
            this.sendSubscribe(subscription.channel, subscription.market);
        }
    }
}
//# sourceMappingURL=websocket.js.map
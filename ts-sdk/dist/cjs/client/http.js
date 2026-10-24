"use strict";
/**
 * HTTP client implementation.
 *
 * @module client/http
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.MatchbookClient = void 0;
const config_1 = require("./config");
const errors_1 = require("./errors");
/**
 * HTTP client for the Matchbook REST API.
 *
 * Provides methods for fetching market data, user data, and building transactions.
 *
 * @example
 * ```typescript
 * const client = new MatchbookClient({
 *   baseUrl: 'https://api.matchbook.taunais.com/v1',
 *   apiKey: 'your-api-key',
 * });
 *
 * // Get all markets
 * const markets = await client.getMarkets();
 *
 * // Get order book
 * const orderbook = await client.getOrderbook('ABC123...', 20);
 * ```
 */
class MatchbookClient {
    /**
     * Creates a new HTTP client.
     *
     * @param config - Client configuration.
     * @throws Error if configuration is invalid.
     */
    constructor(config = {}) {
        this.config = (0, config_1.resolveConfig)(config);
        (0, config_1.validateConfig)(this.config);
    }
    /**
     * Returns the client configuration.
     */
    get configuration() {
        return this.config;
    }
    /**
     * Makes an HTTP request.
     *
     * @param method - HTTP method.
     * @param path - Request path.
     * @param options - Request options.
     * @returns Response data.
     */
    async request(method, path, options = {}) {
        const url = new URL(path, this.config.baseUrl);
        // Add query parameters
        if (options.params) {
            for (const [key, value] of Object.entries(options.params)) {
                if (value !== undefined) {
                    url.searchParams.set(key, String(value));
                }
            }
        }
        // Build headers
        const headers = {
            'Content-Type': 'application/json',
            ...this.config.headers,
        };
        if (this.config.apiKey) {
            headers['Authorization'] = `Bearer ${this.config.apiKey}`;
        }
        // Create abort controller for timeout
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);
        try {
            const response = await fetch(url.toString(), {
                method,
                headers,
                body: options.body ? JSON.stringify(options.body) : undefined,
                signal: controller.signal,
            });
            clearTimeout(timeoutId);
            // Handle error responses
            if (!response.ok) {
                await this.handleErrorResponse(response);
            }
            // Parse response
            const data = await response.json();
            return data.data ?? data;
        }
        catch (error) {
            clearTimeout(timeoutId);
            if (error instanceof Error && error.name === 'AbortError') {
                throw new errors_1.TimeoutError(this.config.timeout);
            }
            throw error;
        }
    }
    /**
     * Handles error responses.
     *
     * @param response - Fetch response.
     * @throws Appropriate error based on status code.
     */
    async handleErrorResponse(response) {
        let body;
        try {
            body = await response.json();
        }
        catch {
            body = await response.text();
        }
        const status = response.status;
        switch (status) {
            case 401:
                throw new errors_1.UnauthorizedError();
            case 404:
                throw new errors_1.NotFoundError(body?.message ?? 'Resource not found');
            case 429: {
                const retryAfter = response.headers.get('Retry-After');
                throw new errors_1.RateLimitError(retryAfter ? parseInt(retryAfter, 10) * 1000 : undefined);
            }
            default:
                if (body && typeof body === 'object' && 'error' in body) {
                    const errorBody = body;
                    throw new errors_1.ApiError(errorBody.error.message, errorBody.error.code, errorBody.error.request_id);
                }
                throw new errors_1.HttpError(`HTTP ${status}`, status, body);
        }
    }
    // ============================================================
    // Market Data Methods
    // ============================================================
    /**
     * Gets all available markets.
     *
     * @returns List of markets.
     *
     * @example
     * ```typescript
     * const markets = await client.getMarkets();
     * console.log(`Found ${markets.length} markets`);
     * ```
     */
    async getMarkets() {
        return this.request('GET', '/markets');
    }
    /**
     * Gets a specific market by address.
     *
     * @param address - Market address.
     * @returns Market details.
     *
     * @example
     * ```typescript
     * const market = await client.getMarket('ABC123...');
     * console.log(`Market: ${market.baseSymbol}/${market.quoteSymbol}`);
     * ```
     */
    async getMarket(address) {
        return this.request('GET', `/markets/${address}`);
    }
    /**
     * Gets the order book for a market.
     *
     * @param market - Market address.
     * @param depth - Number of levels to return (default: 20).
     * @returns Order book with bids and asks.
     *
     * @example
     * ```typescript
     * const orderbook = await client.getOrderbook('ABC123...', 10);
     * console.log(`Best bid: ${orderbook.bids[0]?.price}`);
     * ```
     */
    async getOrderbook(market, depth) {
        return this.request('GET', `/markets/${market}/orderbook`, {
            params: { depth },
        });
    }
    /**
     * Gets recent trades for a market.
     *
     * @param market - Market address.
     * @param options - Filter options.
     * @returns List of trades.
     *
     * @example
     * ```typescript
     * const trades = await client.getTrades('ABC123...', { limit: 50 });
     * console.log(`Last trade: ${trades[0]?.price}`);
     * ```
     */
    async getTrades(market, options) {
        return this.request('GET', `/markets/${market}/trades`, {
            params: {
                limit: options?.limit,
                before: options?.before,
                after: options?.after,
            },
        });
    }
    // ============================================================
    // User Data Methods
    // ============================================================
    /**
     * Gets orders for a user.
     *
     * @param owner - Owner wallet address.
     * @param market - Optional market filter.
     * @returns List of orders.
     *
     * @example
     * ```typescript
     * const orders = await client.getOrders('9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM');
     * console.log(`Found ${orders.length} orders`);
     * ```
     */
    async getOrders(owner, market) {
        return this.request('GET', '/orders', {
            params: { owner, market },
        });
    }
    /**
     * Gets trades for a user.
     *
     * @param owner - Owner wallet address.
     * @param market - Optional market filter.
     * @returns List of trades.
     *
     * @example
     * ```typescript
     * const trades = await client.getUserTrades('9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM');
     * console.log(`Found ${trades.length} trades`);
     * ```
     */
    async getUserTrades(owner, market) {
        return this.request('GET', '/trades', {
            params: { owner, market },
        });
    }
    /**
     * Gets balances for a user.
     *
     * @param owner - Owner wallet address.
     * @returns List of balances.
     *
     * @example
     * ```typescript
     * const balances = await client.getBalances('9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM');
     * for (const balance of balances) {
     *   console.log(`Market ${balance.market}: ${balance.baseAvailable} base`);
     * }
     * ```
     */
    async getBalances(owner) {
        return this.request('GET', '/balances', {
            params: { owner },
        });
    }
    // ============================================================
    // Transaction Building Methods
    // ============================================================
    /**
     * Builds a place order transaction.
     *
     * @param params - Order parameters.
     * @returns Transaction to sign and send.
     *
     * @example
     * ```typescript
     * const tx = await client.buildPlaceOrderTx({
     *   market: 'ABC123...',
     *   side: 'bid',
     *   price: '105.50',
     *   quantity: '100',
     *   orderType: 'limit',
     * });
     * // Sign and send tx.transaction
     * ```
     */
    async buildPlaceOrderTx(params) {
        return this.request('POST', '/tx/place-order', {
            body: params,
        });
    }
    /**
     * Builds a cancel order transaction.
     *
     * @param params - Cancel parameters.
     * @returns Transaction to sign and send.
     *
     * @example
     * ```typescript
     * const tx = await client.buildCancelOrderTx({
     *   market: 'ABC123...',
     *   orderId: '123456789',
     * });
     * // Sign and send tx.transaction
     * ```
     */
    async buildCancelOrderTx(params) {
        return this.request('POST', '/tx/cancel-order', {
            body: params,
        });
    }
    /**
     * Builds a deposit transaction.
     *
     * @param params - Deposit parameters.
     * @returns Transaction to sign and send.
     *
     * @example
     * ```typescript
     * const tx = await client.buildDepositTx({
     *   market: 'ABC123...',
     *   baseAmount: '100',
     *   quoteAmount: '5000',
     * });
     * // Sign and send tx.transaction
     * ```
     */
    async buildDepositTx(params) {
        return this.request('POST', '/tx/deposit', {
            body: params,
        });
    }
    /**
     * Builds a withdraw transaction.
     *
     * @param params - Withdraw parameters.
     * @returns Transaction to sign and send.
     *
     * @example
     * ```typescript
     * const tx = await client.buildWithdrawTx({
     *   market: 'ABC123...',
     *   baseAmount: '50',
     * });
     * // Sign and send tx.transaction
     * ```
     */
    async buildWithdrawTx(params) {
        return this.request('POST', '/tx/withdraw', {
            body: params,
        });
    }
}
exports.MatchbookClient = MatchbookClient;
//# sourceMappingURL=http.js.map
/**
 * HTTP client implementation.
 *
 * @module client/http
 */
import type { Market, Order, Trade, OrderBook, Balance, TradeFilter, PlaceOrderParams, CancelOrderParams, DepositParams, WithdrawParams } from '../types';
import type { BuildTransactionResponse } from '../api';
import type { ResolvedConfig } from './config';
import { type ClientConfig } from './config';
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
export declare class MatchbookClient {
    private readonly config;
    /**
     * Creates a new HTTP client.
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
     * Makes an HTTP request.
     *
     * @param method - HTTP method.
     * @param path - Request path.
     * @param options - Request options.
     * @returns Response data.
     */
    private request;
    /**
     * Handles error responses.
     *
     * @param response - Fetch response.
     * @throws Appropriate error based on status code.
     */
    private handleErrorResponse;
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
    getMarkets(): Promise<Market[]>;
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
    getMarket(address: string): Promise<Market>;
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
    getOrderbook(market: string, depth?: number): Promise<OrderBook>;
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
    getTrades(market: string, options?: TradeFilter): Promise<Trade[]>;
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
    getOrders(owner: string, market?: string): Promise<Order[]>;
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
    getUserTrades(owner: string, market?: string): Promise<Trade[]>;
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
    getBalances(owner: string): Promise<Balance[]>;
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
    buildPlaceOrderTx(params: PlaceOrderParams): Promise<BuildTransactionResponse>;
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
    buildCancelOrderTx(params: CancelOrderParams): Promise<BuildTransactionResponse>;
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
    buildDepositTx(params: DepositParams): Promise<BuildTransactionResponse>;
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
    buildWithdrawTx(params: WithdrawParams): Promise<BuildTransactionResponse>;
}
//# sourceMappingURL=http.d.ts.map
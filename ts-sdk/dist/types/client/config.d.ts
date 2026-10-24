/**
 * Client configuration.
 *
 * @module client/config
 */
/**
 * Default REST API base URL.
 */
export declare const DEFAULT_BASE_URL = "https://api.matchbook.example/v1";
/**
 * Default WebSocket URL.
 */
export declare const DEFAULT_WS_URL = "wss://ws.matchbook.example/v1/stream";
/**
 * Default request timeout in milliseconds.
 */
export declare const DEFAULT_TIMEOUT = 30000;
/**
 * Client configuration options.
 *
 * @example
 * ```typescript
 * const config: ClientConfig = {
 *   baseUrl: 'https://api.matchbook.taunais.com/v1',
 *   wsUrl: 'wss://ws.matchbook.taunais.com/v1/stream',
 *   timeout: 30000,
 *   apiKey: 'your-api-key',
 * };
 * ```
 */
export interface ClientConfig {
    /** REST API base URL. */
    baseUrl?: string;
    /** WebSocket URL. */
    wsUrl?: string;
    /** Request timeout in milliseconds. */
    timeout?: number;
    /** Optional API key for authentication. */
    apiKey?: string;
    /** Custom headers to include in requests. */
    headers?: Record<string, string>;
}
/**
 * Resolved client configuration with all defaults applied.
 */
export interface ResolvedConfig {
    /** REST API base URL. */
    baseUrl: string;
    /** WebSocket URL. */
    wsUrl: string;
    /** Request timeout in milliseconds. */
    timeout: number;
    /** Optional API key for authentication. */
    apiKey?: string;
    /** Custom headers to include in requests. */
    headers: Record<string, string>;
}
/**
 * Resolves client configuration with defaults.
 *
 * @param config - Partial configuration.
 * @returns Resolved configuration with all defaults applied.
 */
export declare function resolveConfig(config?: ClientConfig): ResolvedConfig;
/**
 * Validates client configuration.
 *
 * @param config - Configuration to validate.
 * @throws Error if configuration is invalid.
 */
export declare function validateConfig(config: ResolvedConfig): void;
//# sourceMappingURL=config.d.ts.map
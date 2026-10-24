"use strict";
/**
 * Client configuration.
 *
 * @module client/config
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.DEFAULT_TIMEOUT = exports.DEFAULT_WS_URL = exports.DEFAULT_BASE_URL = void 0;
exports.resolveConfig = resolveConfig;
exports.validateConfig = validateConfig;
/**
 * Default REST API base URL.
 */
exports.DEFAULT_BASE_URL = 'https://api.matchbook.example/v1';
/**
 * Default WebSocket URL.
 */
exports.DEFAULT_WS_URL = 'wss://ws.matchbook.example/v1/stream';
/**
 * Default request timeout in milliseconds.
 */
exports.DEFAULT_TIMEOUT = 30000;
/**
 * Resolves client configuration with defaults.
 *
 * @param config - Partial configuration.
 * @returns Resolved configuration with all defaults applied.
 */
function resolveConfig(config = {}) {
    return {
        baseUrl: config.baseUrl ?? exports.DEFAULT_BASE_URL,
        wsUrl: config.wsUrl ?? exports.DEFAULT_WS_URL,
        timeout: config.timeout ?? exports.DEFAULT_TIMEOUT,
        apiKey: config.apiKey,
        headers: config.headers ?? {},
    };
}
/**
 * Validates client configuration.
 *
 * @param config - Configuration to validate.
 * @throws Error if configuration is invalid.
 */
function validateConfig(config) {
    if (!config.baseUrl) {
        throw new Error('baseUrl is required');
    }
    if (!config.baseUrl.startsWith('http://') && !config.baseUrl.startsWith('https://')) {
        throw new Error('baseUrl must start with http:// or https://');
    }
    if (!config.wsUrl) {
        throw new Error('wsUrl is required');
    }
    if (!config.wsUrl.startsWith('ws://') && !config.wsUrl.startsWith('wss://')) {
        throw new Error('wsUrl must start with ws:// or wss://');
    }
    if (config.timeout <= 0) {
        throw new Error('timeout must be positive');
    }
}
//# sourceMappingURL=config.js.map
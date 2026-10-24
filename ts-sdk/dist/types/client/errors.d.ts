/**
 * Client error types.
 *
 * @module client/errors
 */
/**
 * Base class for client errors.
 */
export declare class ClientError extends Error {
    /** Error name. */
    name: string;
    constructor(message: string);
}
/**
 * HTTP request error.
 */
export declare class HttpError extends ClientError {
    name: string;
    /** HTTP status code. */
    readonly status: number;
    /** Response body if available. */
    readonly body?: unknown;
    constructor(message: string, status: number, body?: unknown);
}
/**
 * API error returned by the server.
 */
export declare class ApiError extends ClientError {
    name: string;
    /** Error code from the API. */
    readonly code: string;
    /** Request ID for debugging. */
    readonly requestId?: string;
    constructor(message: string, code: string, requestId?: string);
}
/**
 * Request timeout error.
 */
export declare class TimeoutError extends ClientError {
    name: string;
    /** Timeout duration in milliseconds. */
    readonly timeout: number;
    constructor(timeout: number);
}
/**
 * Rate limit error (HTTP 429).
 */
export declare class RateLimitError extends ClientError {
    name: string;
    /** Retry after duration in milliseconds. */
    readonly retryAfter?: number;
    constructor(retryAfter?: number);
}
/**
 * WebSocket connection error.
 */
export declare class WebSocketError extends ClientError {
    name: string;
    /** WebSocket close code if available. */
    readonly closeCode?: number;
    constructor(message: string, closeCode?: number);
}
/**
 * Not found error (HTTP 404).
 */
export declare class NotFoundError extends ClientError {
    name: string;
    /** Resource type that was not found. */
    readonly resource?: string;
    constructor(message: string, resource?: string);
}
/**
 * Unauthorized error (HTTP 401).
 */
export declare class UnauthorizedError extends ClientError {
    name: string;
    constructor(message?: string);
}
/**
 * Checks if an error is a ClientError.
 *
 * @param error - The error to check.
 * @returns True if the error is a ClientError.
 */
export declare function isClientError(error: unknown): error is ClientError;
/**
 * Checks if an error is an HttpError.
 *
 * @param error - The error to check.
 * @returns True if the error is an HttpError.
 */
export declare function isHttpError(error: unknown): error is HttpError;
/**
 * Checks if an error is an ApiError.
 *
 * @param error - The error to check.
 * @returns True if the error is an ApiError.
 */
export declare function isApiError(error: unknown): error is ApiError;
/**
 * Checks if an error is a RateLimitError.
 *
 * @param error - The error to check.
 * @returns True if the error is a RateLimitError.
 */
export declare function isRateLimitError(error: unknown): error is RateLimitError;
//# sourceMappingURL=errors.d.ts.map
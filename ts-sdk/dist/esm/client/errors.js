/**
 * Client error types.
 *
 * @module client/errors
 */
/**
 * Base class for client errors.
 */
export class ClientError extends Error {
    constructor(message) {
        super(message);
        /** Error name. */
        this.name = 'ClientError';
        Object.setPrototypeOf(this, ClientError.prototype);
    }
}
/**
 * HTTP request error.
 */
export class HttpError extends ClientError {
    constructor(message, status, body) {
        super(message);
        this.name = 'HttpError';
        this.status = status;
        this.body = body;
        Object.setPrototypeOf(this, HttpError.prototype);
    }
}
/**
 * API error returned by the server.
 */
export class ApiError extends ClientError {
    constructor(message, code, requestId) {
        super(message);
        this.name = 'ApiError';
        this.code = code;
        this.requestId = requestId;
        Object.setPrototypeOf(this, ApiError.prototype);
    }
}
/**
 * Request timeout error.
 */
export class TimeoutError extends ClientError {
    constructor(timeout) {
        super(`Request timed out after ${timeout}ms`);
        this.name = 'TimeoutError';
        this.timeout = timeout;
        Object.setPrototypeOf(this, TimeoutError.prototype);
    }
}
/**
 * Rate limit error (HTTP 429).
 */
export class RateLimitError extends ClientError {
    constructor(retryAfter) {
        const message = retryAfter
            ? `Rate limited. Retry after ${retryAfter}ms`
            : 'Rate limited';
        super(message);
        this.name = 'RateLimitError';
        this.retryAfter = retryAfter;
        Object.setPrototypeOf(this, RateLimitError.prototype);
    }
}
/**
 * WebSocket connection error.
 */
export class WebSocketError extends ClientError {
    constructor(message, closeCode) {
        super(message);
        this.name = 'WebSocketError';
        this.closeCode = closeCode;
        Object.setPrototypeOf(this, WebSocketError.prototype);
    }
}
/**
 * Not found error (HTTP 404).
 */
export class NotFoundError extends ClientError {
    constructor(message, resource) {
        super(message);
        this.name = 'NotFoundError';
        this.resource = resource;
        Object.setPrototypeOf(this, NotFoundError.prototype);
    }
}
/**
 * Unauthorized error (HTTP 401).
 */
export class UnauthorizedError extends ClientError {
    constructor(message = 'Unauthorized') {
        super(message);
        this.name = 'UnauthorizedError';
        Object.setPrototypeOf(this, UnauthorizedError.prototype);
    }
}
/**
 * Checks if an error is a ClientError.
 *
 * @param error - The error to check.
 * @returns True if the error is a ClientError.
 */
export function isClientError(error) {
    return error instanceof ClientError;
}
/**
 * Checks if an error is an HttpError.
 *
 * @param error - The error to check.
 * @returns True if the error is an HttpError.
 */
export function isHttpError(error) {
    return error instanceof HttpError;
}
/**
 * Checks if an error is an ApiError.
 *
 * @param error - The error to check.
 * @returns True if the error is an ApiError.
 */
export function isApiError(error) {
    return error instanceof ApiError;
}
/**
 * Checks if an error is a RateLimitError.
 *
 * @param error - The error to check.
 * @returns True if the error is a RateLimitError.
 */
export function isRateLimitError(error) {
    return error instanceof RateLimitError;
}
//# sourceMappingURL=errors.js.map
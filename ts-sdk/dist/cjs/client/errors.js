"use strict";
/**
 * Client error types.
 *
 * @module client/errors
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.UnauthorizedError = exports.NotFoundError = exports.WebSocketError = exports.RateLimitError = exports.TimeoutError = exports.ApiError = exports.HttpError = exports.ClientError = void 0;
exports.isClientError = isClientError;
exports.isHttpError = isHttpError;
exports.isApiError = isApiError;
exports.isRateLimitError = isRateLimitError;
/**
 * Base class for client errors.
 */
class ClientError extends Error {
    constructor(message) {
        super(message);
        /** Error name. */
        this.name = 'ClientError';
        Object.setPrototypeOf(this, ClientError.prototype);
    }
}
exports.ClientError = ClientError;
/**
 * HTTP request error.
 */
class HttpError extends ClientError {
    constructor(message, status, body) {
        super(message);
        this.name = 'HttpError';
        this.status = status;
        this.body = body;
        Object.setPrototypeOf(this, HttpError.prototype);
    }
}
exports.HttpError = HttpError;
/**
 * API error returned by the server.
 */
class ApiError extends ClientError {
    constructor(message, code, requestId) {
        super(message);
        this.name = 'ApiError';
        this.code = code;
        this.requestId = requestId;
        Object.setPrototypeOf(this, ApiError.prototype);
    }
}
exports.ApiError = ApiError;
/**
 * Request timeout error.
 */
class TimeoutError extends ClientError {
    constructor(timeout) {
        super(`Request timed out after ${timeout}ms`);
        this.name = 'TimeoutError';
        this.timeout = timeout;
        Object.setPrototypeOf(this, TimeoutError.prototype);
    }
}
exports.TimeoutError = TimeoutError;
/**
 * Rate limit error (HTTP 429).
 */
class RateLimitError extends ClientError {
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
exports.RateLimitError = RateLimitError;
/**
 * WebSocket connection error.
 */
class WebSocketError extends ClientError {
    constructor(message, closeCode) {
        super(message);
        this.name = 'WebSocketError';
        this.closeCode = closeCode;
        Object.setPrototypeOf(this, WebSocketError.prototype);
    }
}
exports.WebSocketError = WebSocketError;
/**
 * Not found error (HTTP 404).
 */
class NotFoundError extends ClientError {
    constructor(message, resource) {
        super(message);
        this.name = 'NotFoundError';
        this.resource = resource;
        Object.setPrototypeOf(this, NotFoundError.prototype);
    }
}
exports.NotFoundError = NotFoundError;
/**
 * Unauthorized error (HTTP 401).
 */
class UnauthorizedError extends ClientError {
    constructor(message = 'Unauthorized') {
        super(message);
        this.name = 'UnauthorizedError';
        Object.setPrototypeOf(this, UnauthorizedError.prototype);
    }
}
exports.UnauthorizedError = UnauthorizedError;
/**
 * Checks if an error is a ClientError.
 *
 * @param error - The error to check.
 * @returns True if the error is a ClientError.
 */
function isClientError(error) {
    return error instanceof ClientError;
}
/**
 * Checks if an error is an HttpError.
 *
 * @param error - The error to check.
 * @returns True if the error is an HttpError.
 */
function isHttpError(error) {
    return error instanceof HttpError;
}
/**
 * Checks if an error is an ApiError.
 *
 * @param error - The error to check.
 * @returns True if the error is an ApiError.
 */
function isApiError(error) {
    return error instanceof ApiError;
}
/**
 * Checks if an error is a RateLimitError.
 *
 * @param error - The error to check.
 * @returns True if the error is a RateLimitError.
 */
function isRateLimitError(error) {
    return error instanceof RateLimitError;
}
//# sourceMappingURL=errors.js.map
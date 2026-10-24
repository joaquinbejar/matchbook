/**
 * Client error types.
 *
 * @module client/errors
 */

/**
 * Base class for client errors.
 */
export class ClientError extends Error {
  /** Error name. */
  override name = 'ClientError';

  constructor(message: string) {
    super(message);
    Object.setPrototypeOf(this, ClientError.prototype);
  }
}

/**
 * HTTP request error.
 */
export class HttpError extends ClientError {
  override name = 'HttpError';

  /** HTTP status code. */
  readonly status: number;

  /** Response body if available. */
  readonly body?: unknown;

  constructor(message: string, status: number, body?: unknown) {
    super(message);
    this.status = status;
    this.body = body;
    Object.setPrototypeOf(this, HttpError.prototype);
  }
}

/**
 * API error returned by the server.
 */
export class ApiError extends ClientError {
  override name = 'ApiError';

  /** Error code from the API. */
  readonly code: string;

  /** Request ID for debugging. */
  readonly requestId?: string;

  constructor(message: string, code: string, requestId?: string) {
    super(message);
    this.code = code;
    this.requestId = requestId;
    Object.setPrototypeOf(this, ApiError.prototype);
  }
}

/**
 * Request timeout error.
 */
export class TimeoutError extends ClientError {
  override name = 'TimeoutError';

  /** Timeout duration in milliseconds. */
  readonly timeout: number;

  constructor(timeout: number) {
    super(`Request timed out after ${timeout}ms`);
    this.timeout = timeout;
    Object.setPrototypeOf(this, TimeoutError.prototype);
  }
}

/**
 * Rate limit error (HTTP 429).
 */
export class RateLimitError extends ClientError {
  override name = 'RateLimitError';

  /** Retry after duration in milliseconds. */
  readonly retryAfter?: number;

  constructor(retryAfter?: number) {
    const message = retryAfter
      ? `Rate limited. Retry after ${retryAfter}ms`
      : 'Rate limited';
    super(message);
    this.retryAfter = retryAfter;
    Object.setPrototypeOf(this, RateLimitError.prototype);
  }
}

/**
 * WebSocket connection error.
 */
export class WebSocketError extends ClientError {
  override name = 'WebSocketError';

  /** WebSocket close code if available. */
  readonly closeCode?: number;

  constructor(message: string, closeCode?: number) {
    super(message);
    this.closeCode = closeCode;
    Object.setPrototypeOf(this, WebSocketError.prototype);
  }
}

/**
 * Not found error (HTTP 404).
 */
export class NotFoundError extends ClientError {
  override name = 'NotFoundError';

  /** Resource type that was not found. */
  readonly resource?: string;

  constructor(message: string, resource?: string) {
    super(message);
    this.resource = resource;
    Object.setPrototypeOf(this, NotFoundError.prototype);
  }
}

/**
 * Unauthorized error (HTTP 401).
 */
export class UnauthorizedError extends ClientError {
  override name = 'UnauthorizedError';

  constructor(message = 'Unauthorized') {
    super(message);
    Object.setPrototypeOf(this, UnauthorizedError.prototype);
  }
}

/**
 * Checks if an error is a ClientError.
 *
 * @param error - The error to check.
 * @returns True if the error is a ClientError.
 */
export function isClientError(error: unknown): error is ClientError {
  return error instanceof ClientError;
}

/**
 * Checks if an error is an HttpError.
 *
 * @param error - The error to check.
 * @returns True if the error is an HttpError.
 */
export function isHttpError(error: unknown): error is HttpError {
  return error instanceof HttpError;
}

/**
 * Checks if an error is an ApiError.
 *
 * @param error - The error to check.
 * @returns True if the error is an ApiError.
 */
export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError;
}

/**
 * Checks if an error is a RateLimitError.
 *
 * @param error - The error to check.
 * @returns True if the error is a RateLimitError.
 */
export function isRateLimitError(error: unknown): error is RateLimitError {
  return error instanceof RateLimitError;
}

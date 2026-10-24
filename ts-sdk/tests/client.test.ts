/**
 * Client tests.
 */

import { describe, it, expect } from 'vitest';
import {
  MatchbookClient,
  MatchbookWsClient,
  resolveConfig,
  validateConfig,
  ClientError,
  HttpError,
  TimeoutError,
  RateLimitError,
  NotFoundError,
  UnauthorizedError,
  isClientError,
  isHttpError,
  DEFAULT_BASE_URL,
  DEFAULT_WS_URL,
  DEFAULT_TIMEOUT,
} from '../src/client';

describe('Client Configuration', () => {
  describe('resolveConfig', () => {
    it('uses defaults when no config provided', () => {
      const config = resolveConfig();
      expect(config.baseUrl).toBe(DEFAULT_BASE_URL);
      expect(config.wsUrl).toBe(DEFAULT_WS_URL);
      expect(config.timeout).toBe(DEFAULT_TIMEOUT);
      expect(config.apiKey).toBeUndefined();
    });

    it('uses provided values', () => {
      const config = resolveConfig({
        baseUrl: 'https://custom.api.com',
        wsUrl: 'wss://custom.ws.com',
        timeout: 5000,
        apiKey: 'test-key',
      });
      expect(config.baseUrl).toBe('https://custom.api.com');
      expect(config.wsUrl).toBe('wss://custom.ws.com');
      expect(config.timeout).toBe(5000);
      expect(config.apiKey).toBe('test-key');
    });

    it('merges partial config with defaults', () => {
      const config = resolveConfig({
        apiKey: 'test-key',
      });
      expect(config.baseUrl).toBe(DEFAULT_BASE_URL);
      expect(config.apiKey).toBe('test-key');
    });
  });

  describe('validateConfig', () => {
    it('accepts valid config', () => {
      const config = resolveConfig();
      expect(() => validateConfig(config)).not.toThrow();
    });

    it('rejects empty baseUrl', () => {
      const config = resolveConfig({ baseUrl: '' });
      expect(() => validateConfig(config)).toThrow('baseUrl is required');
    });

    it('rejects invalid baseUrl scheme', () => {
      const config = resolveConfig({ baseUrl: 'ftp://example.com' });
      expect(() => validateConfig(config)).toThrow('baseUrl must start with http://');
    });

    it('rejects empty wsUrl', () => {
      const config = resolveConfig({ wsUrl: '' });
      expect(() => validateConfig(config)).toThrow('wsUrl is required');
    });

    it('rejects invalid wsUrl scheme', () => {
      const config = resolveConfig({ wsUrl: 'http://example.com' });
      expect(() => validateConfig(config)).toThrow('wsUrl must start with ws://');
    });

    it('rejects non-positive timeout', () => {
      const config = resolveConfig({ timeout: 0 });
      expect(() => validateConfig(config)).toThrow('timeout must be positive');
    });
  });
});

describe('Client Errors', () => {
  describe('ClientError', () => {
    it('creates error with message', () => {
      const error = new ClientError('test error');
      expect(error.message).toBe('test error');
      expect(error.name).toBe('ClientError');
    });
  });

  describe('HttpError', () => {
    it('creates error with status and body', () => {
      const error = new HttpError('Not Found', 404, { detail: 'resource not found' });
      expect(error.message).toBe('Not Found');
      expect(error.status).toBe(404);
      expect(error.body).toEqual({ detail: 'resource not found' });
    });
  });

  describe('TimeoutError', () => {
    it('creates error with timeout', () => {
      const error = new TimeoutError(5000);
      expect(error.message).toBe('Request timed out after 5000ms');
      expect(error.timeout).toBe(5000);
    });
  });

  describe('RateLimitError', () => {
    it('creates error with retry after', () => {
      const error = new RateLimitError(60000);
      expect(error.message).toBe('Rate limited. Retry after 60000ms');
      expect(error.retryAfter).toBe(60000);
    });

    it('creates error without retry after', () => {
      const error = new RateLimitError();
      expect(error.message).toBe('Rate limited');
      expect(error.retryAfter).toBeUndefined();
    });
  });

  describe('NotFoundError', () => {
    it('creates error with resource', () => {
      const error = new NotFoundError('Market not found', 'market');
      expect(error.message).toBe('Market not found');
      expect(error.resource).toBe('market');
    });
  });

  describe('UnauthorizedError', () => {
    it('creates error with default message', () => {
      const error = new UnauthorizedError();
      expect(error.message).toBe('Unauthorized');
    });

    it('creates error with custom message', () => {
      const error = new UnauthorizedError('Invalid API key');
      expect(error.message).toBe('Invalid API key');
    });
  });

  describe('Error type guards', () => {
    it('isClientError returns true for ClientError', () => {
      expect(isClientError(new ClientError('test'))).toBe(true);
      expect(isClientError(new HttpError('test', 500))).toBe(true);
      expect(isClientError(new Error('test'))).toBe(false);
    });

    it('isHttpError returns true for HttpError', () => {
      expect(isHttpError(new HttpError('test', 500))).toBe(true);
      expect(isHttpError(new ClientError('test'))).toBe(false);
    });
  });
});

describe('MatchbookClient', () => {
  describe('constructor', () => {
    it('creates client with default config', () => {
      const client = new MatchbookClient();
      expect(client.configuration.baseUrl).toBe(DEFAULT_BASE_URL);
    });

    it('creates client with custom config', () => {
      const client = new MatchbookClient({
        baseUrl: 'https://custom.api.com',
        apiKey: 'test-key',
      });
      expect(client.configuration.baseUrl).toBe('https://custom.api.com');
      expect(client.configuration.apiKey).toBe('test-key');
    });

    it('throws on invalid config', () => {
      expect(() => new MatchbookClient({ baseUrl: '' })).toThrow();
    });
  });
});

describe('MatchbookWsClient', () => {
  describe('constructor', () => {
    it('creates client with default config', () => {
      const client = new MatchbookWsClient();
      expect(client.configuration.wsUrl).toBe(DEFAULT_WS_URL);
    });

    it('creates client with custom config', () => {
      const client = new MatchbookWsClient({
        wsUrl: 'wss://custom.ws.com',
        apiKey: 'test-key',
      });
      expect(client.configuration.wsUrl).toBe('wss://custom.ws.com');
      expect(client.configuration.apiKey).toBe('test-key');
    });

    it('throws on invalid config', () => {
      expect(() => new MatchbookWsClient({ wsUrl: '' })).toThrow();
    });
  });

  describe('isConnected', () => {
    it('returns false when not connected', () => {
      const client = new MatchbookWsClient();
      expect(client.isConnected).toBe(false);
    });
  });
});

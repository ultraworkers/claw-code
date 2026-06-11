/**
 * Retry Logic Tests
 *
 * Full test coverage for exponential backoff retry utilities.
 * Tests various scenarios including success, failure, and edge cases.
 */

import {
  sendWithRetry,
  withRetry,
  sleep,
  calculateDelay,
  isRetryableError,
  createSmartRetryOptions,
  RetryOptions,
} from '../src/retry';
import { StubLoggingHook } from '../src/StubLoggingHook';
import { LogEntry, LoggingHook, SendResult } from '../src/LoggingHook';

describe('retry utilities', () => {
  // Helper to create valid log entries
  const createValidEntry = (overrides: Partial<LogEntry> = {}): LogEntry => ({
    timestamp: new Date().toISOString(),
    level: 'INFO',
    category: 'test',
    action: 'test_action',
    target: '/test/path',
    result: 'success',
    ...overrides,
  });

  describe('sleep', () => {
    it('should delay execution for specified milliseconds', async () => {
      const start = Date.now();
      await sleep(50);
      const elapsed = Date.now() - start;

      // Allow some tolerance for timing
      expect(elapsed).toBeGreaterThanOrEqual(40);
      expect(elapsed).toBeLessThan(150);
    });

    it('should handle zero delay', async () => {
      const start = Date.now();
      await sleep(0);
      const elapsed = Date.now() - start;

      expect(elapsed).toBeLessThan(50);
    });
  });

  describe('calculateDelay', () => {
    const defaultOptions = {
      baseDelayMs: 1000,
      maxDelayMs: 30000,
      backoffMultiplier: 2,
      jitter: false,
    };

    it('should calculate exponential backoff correctly', () => {
      expect(calculateDelay(0, defaultOptions)).toBe(1000); // 1000 * 2^0
      expect(calculateDelay(1, defaultOptions)).toBe(2000); // 1000 * 2^1
      expect(calculateDelay(2, defaultOptions)).toBe(4000); // 1000 * 2^2
      expect(calculateDelay(3, defaultOptions)).toBe(8000); // 1000 * 2^3
    });

    it('should cap at max delay', () => {
      const options = { ...defaultOptions, maxDelayMs: 5000 };

      expect(calculateDelay(0, options)).toBe(1000);
      expect(calculateDelay(1, options)).toBe(2000);
      expect(calculateDelay(2, options)).toBe(4000);
      expect(calculateDelay(3, options)).toBe(5000); // Capped
      expect(calculateDelay(10, options)).toBe(5000); // Still capped
    });

    it('should add jitter when enabled', () => {
      const options = { ...defaultOptions, jitter: true };

      // Run multiple times to verify jitter adds randomness
      const delays = new Set<number>();
      for (let i = 0; i < 10; i++) {
        delays.add(calculateDelay(1, options));
      }

      // With jitter, we should get some variation (not guaranteed but highly likely)
      // Base delay at attempt 1 is 2000, jitter adds 0-25% (0-500)
      const delaysArray = Array.from(delays);
      delaysArray.forEach((delay) => {
        expect(delay).toBeGreaterThanOrEqual(2000);
        expect(delay).toBeLessThanOrEqual(2500);
      });
    });

    it('should handle different multipliers', () => {
      const options = { ...defaultOptions, backoffMultiplier: 3 };

      expect(calculateDelay(0, options)).toBe(1000); // 1000 * 3^0
      expect(calculateDelay(1, options)).toBe(3000); // 1000 * 3^1
      expect(calculateDelay(2, options)).toBe(9000); // 1000 * 3^2
    });
  });

  describe('withRetry', () => {
    it('should succeed on first attempt', async () => {
      const fn = jest.fn().mockResolvedValue('success');

      const result = await withRetry(fn, { maxRetries: 3 });

      expect(result.success).toBe(true);
      expect(result.result).toBe('success');
      expect(result.attempts).toBe(1);
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it('should retry on failure and eventually succeed', async () => {
      const fn = jest
        .fn()
        .mockRejectedValueOnce(new Error('Fail 1'))
        .mockRejectedValueOnce(new Error('Fail 2'))
        .mockResolvedValue('success');

      const result = await withRetry(fn, {
        maxRetries: 3,
        baseDelayMs: 10, // Short delay for tests
      });

      expect(result.success).toBe(true);
      expect(result.result).toBe('success');
      expect(result.attempts).toBe(3);
      expect(fn).toHaveBeenCalledTimes(3);
    });

    it('should fail after max retries', async () => {
      const fn = jest.fn().mockRejectedValue(new Error('Always fails'));

      const result = await withRetry(fn, {
        maxRetries: 2,
        baseDelayMs: 10,
      });

      expect(result.success).toBe(false);
      expect(result.attempts).toBe(3); // Initial + 2 retries
      expect(result.lastError?.message).toBe('Always fails');
      expect(fn).toHaveBeenCalledTimes(3);
    });

    it('should call onRetry callback on each retry', async () => {
      const fn = jest
        .fn()
        .mockRejectedValueOnce(new Error('Fail 1'))
        .mockRejectedValueOnce(new Error('Fail 2'))
        .mockResolvedValue('success');

      const onRetry = jest.fn();

      await withRetry(fn, {
        maxRetries: 3,
        baseDelayMs: 10,
        onRetry,
      });

      expect(onRetry).toHaveBeenCalledTimes(2);
      expect(onRetry).toHaveBeenNthCalledWith(1, 1, expect.any(Error), expect.any(Number));
      expect(onRetry).toHaveBeenNthCalledWith(2, 2, expect.any(Error), expect.any(Number));
    });

    it('should use default options when not specified', async () => {
      const fn = jest.fn().mockResolvedValue('success');

      const result = await withRetry(fn);

      expect(result.success).toBe(true);
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it('should convert non-Error exceptions to Error', async () => {
      const fn = jest.fn().mockRejectedValue('string error');

      const result = await withRetry(fn, { maxRetries: 0 });

      expect(result.success).toBe(false);
      expect(result.lastError).toBeInstanceOf(Error);
      expect(result.lastError?.message).toBe('string error');
    });
  });

  describe('sendWithRetry', () => {
    let hook: StubLoggingHook;

    beforeEach(async () => {
      hook = new StubLoggingHook();
      await hook.initialize({});
    });

    afterEach(async () => {
      if (hook.isInitialized()) {
        await hook.shutdown();
      }
    });

    it('should send successfully on first attempt', async () => {
      const entries = [createValidEntry()];

      const result = await sendWithRetry(hook, entries);

      expect(result.success).toBe(true);
      expect(result.entries_sent).toBe(1);
      expect(hook.getBufferSize()).toBe(1);
    });

    it('should retry on hook failure', async () => {
      // Create a mock hook that fails twice then succeeds
      let attempts = 0;
      const mockHook: LoggingHook = {
        initialize: async () => {},
        send: async (entries: LogEntry[]): Promise<SendResult> => {
          attempts++;
          if (attempts < 3) {
            throw new Error('Connection refused');
          }
          return { success: true, entries_sent: entries.length };
        },
        flush: async () => {},
        shutdown: async () => {},
      };

      const entries = [createValidEntry()];
      const result = await sendWithRetry(mockHook, entries, {
        maxRetries: 3,
        baseDelayMs: 10,
      });

      expect(result.success).toBe(true);
      expect(result.entries_sent).toBe(1);
      expect(attempts).toBe(3);
    });

    it('should return failure after all retries exhausted', async () => {
      const mockHook: LoggingHook = {
        initialize: async () => {},
        send: async (): Promise<SendResult> => {
          throw new Error('Always fails');
        },
        flush: async () => {},
        shutdown: async () => {},
      };

      const entries = [createValidEntry()];
      const result = await sendWithRetry(mockHook, entries, {
        maxRetries: 2,
        baseDelayMs: 10,
      });

      expect(result.success).toBe(false);
      expect(result.entries_sent).toBe(0);
      expect(result.errors).toBeDefined();
      expect(result.errors!.some((e) => e.includes('Failed after'))).toBe(true);
    });

    it('should retry on unsuccessful send result', async () => {
      let attempts = 0;
      const mockHook: LoggingHook = {
        initialize: async () => {},
        send: async (entries: LogEntry[]): Promise<SendResult> => {
          attempts++;
          if (attempts < 2) {
            return { success: false, entries_sent: 0, errors: ['Rate limited'] };
          }
          return { success: true, entries_sent: entries.length };
        },
        flush: async () => {},
        shutdown: async () => {},
      };

      const entries = [createValidEntry()];
      const result = await sendWithRetry(mockHook, entries, {
        maxRetries: 3,
        baseDelayMs: 10,
      });

      expect(result.success).toBe(true);
      expect(attempts).toBe(2);
    });
  });

  describe('isRetryableError', () => {
    describe('retryable errors', () => {
      it.each([
        ['Connection timeout', true],
        ['Network error', true],
        ['ECONNREFUSED', true],
        ['ECONNRESET', true],
        ['ETIMEDOUT', true],
        ['Socket error', true],
        ['HTTP 500 Internal Server Error', true],
        ['HTTP 502 Bad Gateway', true],
        ['HTTP 503 Service Unavailable', true],
        ['HTTP 504 Gateway Timeout', true],
      ])('should classify "%s" as retryable: %s', (message, expected) => {
        expect(isRetryableError(new Error(message))).toBe(expected);
      });
    });

    describe('non-retryable errors', () => {
      it.each([
        ['Authentication failed', false],
        ['Unauthorized access', false],
        ['403 Forbidden', false],
        ['Invalid request body', false],
        ['Validation error', false],
        ['404 Not Found', false],
        ['401 Unauthorized', false],
      ])('should classify "%s" as non-retryable: %s', (message, expected) => {
        expect(isRetryableError(new Error(message))).toBe(expected);
      });
    });

    it('should default to retryable for unknown errors', () => {
      expect(isRetryableError(new Error('Something weird happened'))).toBe(true);
    });
  });

  describe('createSmartRetryOptions', () => {
    it('should create options with selective retry', async () => {
      const options = createSmartRetryOptions({
        maxRetries: 3,
        baseDelayMs: 10,
      });

      // Test that non-retryable error is re-thrown immediately
      const fn = jest.fn().mockRejectedValue(new Error('401 Unauthorized'));

      await expect(withRetry(fn, options)).rejects.toThrow('401 Unauthorized');
      expect(fn).toHaveBeenCalledTimes(1); // No retries
    });

    it('should retry retryable errors', async () => {
      const options = createSmartRetryOptions({
        maxRetries: 2,
        baseDelayMs: 10,
      });

      const fn = jest.fn().mockRejectedValue(new Error('Connection timeout'));

      const result = await withRetry(fn, options);

      expect(result.success).toBe(false);
      expect(fn).toHaveBeenCalledTimes(3); // Initial + 2 retries
    });

    it('should call original onRetry callback', async () => {
      const originalOnRetry = jest.fn();
      const options = createSmartRetryOptions({
        maxRetries: 2,
        baseDelayMs: 10,
        onRetry: originalOnRetry,
      });

      const fn = jest.fn().mockRejectedValue(new Error('Network error'));

      await withRetry(fn, options);

      expect(originalOnRetry).toHaveBeenCalled();
    });

    it('should preserve base options', () => {
      const baseOptions: RetryOptions = {
        maxRetries: 5,
        baseDelayMs: 500,
        maxDelayMs: 10000,
        backoffMultiplier: 3,
        jitter: false,
      };

      const smartOptions = createSmartRetryOptions(baseOptions);

      expect(smartOptions.maxRetries).toBe(5);
      expect(smartOptions.baseDelayMs).toBe(500);
      expect(smartOptions.maxDelayMs).toBe(10000);
      expect(smartOptions.backoffMultiplier).toBe(3);
      expect(smartOptions.jitter).toBe(false);
    });
  });

  describe('integration scenarios', () => {
    it('should handle realistic retry scenario with logging', async () => {
      const retryLog: string[] = [];
      let attempts = 0;

      const mockHook: LoggingHook = {
        initialize: async () => {},
        send: async (entries: LogEntry[]): Promise<SendResult> => {
          attempts++;
          if (attempts < 3) {
            throw new Error('Service temporarily unavailable');
          }
          return { success: true, entries_sent: entries.length };
        },
        flush: async () => {},
        shutdown: async () => {},
      };

      const result = await sendWithRetry(
        mockHook,
        [createValidEntry()],
        {
          maxRetries: 5,
          baseDelayMs: 10,
          onRetry: (attempt, error, delay) => {
            retryLog.push(`Attempt ${attempt} failed: ${error.message}, retrying in ${delay}ms`);
          },
        }
      );

      expect(result.success).toBe(true);
      expect(retryLog.length).toBe(2);
      expect(retryLog[0]).toContain('Attempt 1 failed');
      expect(retryLog[1]).toContain('Attempt 2 failed');
    });

    it('should respect timeout-like behavior with quick retries', async () => {
      const startTime = Date.now();
      let attempts = 0;

      const fn = jest.fn(async () => {
        attempts++;
        if (attempts < 4) {
          throw new Error('Still failing');
        }
        return 'success';
      });

      const result = await withRetry(fn, {
        maxRetries: 5,
        baseDelayMs: 20,
        maxDelayMs: 100,
        jitter: false,
      });

      const elapsed = Date.now() - startTime;

      expect(result.success).toBe(true);
      expect(attempts).toBe(4);
      // Should have delays of ~20ms, ~40ms, ~80ms = ~140ms total minimum
      expect(elapsed).toBeGreaterThanOrEqual(100);
    });
  });
});

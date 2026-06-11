/**
 * Retry Utilities
 *
 * Implements exponential backoff retry logic as per LOGGING_INTEGRATION.md.
 * Used for reliable delivery of logs to external systems.
 */

import { LoggingHook, LogEntry, SendResult } from './LoggingHook';

/**
 * Options for retry behavior
 */
export interface RetryOptions {
  /** Maximum number of retry attempts (default: 3) */
  maxRetries?: number;
  /** Base delay between retries in milliseconds (default: 1000) */
  baseDelayMs?: number;
  /** Maximum delay between retries in milliseconds (default: 30000) */
  maxDelayMs?: number;
  /** Multiplier for exponential backoff (default: 2) */
  backoffMultiplier?: number;
  /** Whether to add jitter to delays (default: true) */
  jitter?: boolean;
  /** Callback for retry events */
  onRetry?: (attempt: number, error: Error, nextDelayMs: number) => void;
}

/**
 * Result of a retry operation
 */
export interface RetryResult<T> {
  /** The result if successful */
  result?: T;
  /** Whether the operation succeeded */
  success: boolean;
  /** Number of attempts made */
  attempts: number;
  /** Last error if failed */
  lastError?: Error;
}

/**
 * Sleep for a given number of milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Calculate delay for a retry attempt with exponential backoff
 */
export function calculateDelay(
  attempt: number,
  options: Required<Pick<RetryOptions, 'baseDelayMs' | 'maxDelayMs' | 'backoffMultiplier' | 'jitter'>>
): number {
  // Exponential backoff: baseDelay * multiplier^attempt
  let delay = options.baseDelayMs * Math.pow(options.backoffMultiplier, attempt);

  // Cap at max delay
  delay = Math.min(delay, options.maxDelayMs);

  // Add jitter (0-25% of delay)
  if (options.jitter) {
    const jitterAmount = delay * 0.25 * Math.random();
    delay += jitterAmount;
  }

  return Math.floor(delay);
}

/**
 * Default retry options
 */
const DEFAULT_RETRY_OPTIONS: Required<Omit<RetryOptions, 'onRetry'>> = {
  maxRetries: 3,
  baseDelayMs: 1000,
  maxDelayMs: 30000,
  backoffMultiplier: 2,
  jitter: true,
};

/**
 * Execute an async function with retry logic
 *
 * @param fn The async function to execute
 * @param options Retry options
 * @returns Result with success status and attempts made
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  options: RetryOptions = {}
): Promise<RetryResult<T>> {
  const opts = {
    ...DEFAULT_RETRY_OPTIONS,
    ...options,
  };

  let lastError: Error | undefined;

  for (let attempt = 0; attempt <= opts.maxRetries; attempt++) {
    try {
      const result = await fn();
      return {
        result,
        success: true,
        attempts: attempt + 1,
      };
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      // Don't delay after the last attempt
      if (attempt < opts.maxRetries) {
        const delay = calculateDelay(attempt, opts);

        // Call retry callback if provided
        if (options.onRetry) {
          options.onRetry(attempt + 1, lastError, delay);
        }

        await sleep(delay);
      }
    }
  }

  return {
    success: false,
    attempts: opts.maxRetries + 1,
    lastError,
  };
}

/**
 * Send log entries with retry logic
 *
 * As per LOGGING_INTEGRATION.md error handling section:
 * - Retry with exponential backoff
 * - Attempt 1: immediate
 * - Attempt 2: wait 1s
 * - Attempt 3: wait 2s
 * - Attempt 4: wait 4s
 *
 * @param hook The logging hook to use
 * @param entries Log entries to send
 * @param options Retry options
 * @returns Send result
 */
export async function sendWithRetry(
  hook: LoggingHook,
  entries: LogEntry[],
  options: RetryOptions = {}
): Promise<SendResult> {
  const retryResult = await withRetry(
    async () => {
      const result = await hook.send(entries);
      // Treat unsuccessful sends as errors to trigger retry
      if (!result.success) {
        throw new Error(result.errors?.join(', ') || 'Send failed');
      }
      return result;
    },
    options
  );

  if (retryResult.success && retryResult.result) {
    return retryResult.result;
  }

  // All retries failed
  return {
    success: false,
    entries_sent: 0,
    errors: [
      `Failed after ${retryResult.attempts} attempts`,
      ...(retryResult.lastError ? [retryResult.lastError.message] : []),
    ],
  };
}

/**
 * Check if an error is retryable
 * Network errors and timeouts are typically retryable.
 * Authentication errors and validation errors are not.
 */
export function isRetryableError(error: Error): boolean {
  const message = error.message.toLowerCase();

  // Non-retryable errors
  const nonRetryable = [
    'authentication',
    'unauthorized',
    'forbidden',
    'invalid',
    'validation',
    'not found',
    '401',
    '403',
    '404',
  ];

  for (const term of nonRetryable) {
    if (message.includes(term)) {
      return false;
    }
  }

  // Retryable errors
  const retryable = [
    'timeout',
    'network',
    'connection',
    'econnrefused',
    'econnreset',
    'etimedout',
    'socket',
    '500',
    '502',
    '503',
    '504',
  ];

  for (const term of retryable) {
    if (message.includes(term)) {
      return true;
    }
  }

  // Default: assume retryable
  return true;
}

/**
 * Create retry options with selective retry based on error type
 */
export function createSmartRetryOptions(
  baseOptions: RetryOptions = {}
): RetryOptions {
  return {
    ...baseOptions,
    onRetry: (attempt, error, nextDelayMs) => {
      // Skip remaining retries for non-retryable errors
      if (!isRetryableError(error)) {
        throw error;
      }
      // Call original onRetry if provided
      if (baseOptions.onRetry) {
        baseOptions.onRetry(attempt, error, nextDelayMs);
      }
    },
  };
}

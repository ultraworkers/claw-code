/**
 * Guardrails Logging Example - Main Export
 *
 * This module demonstrates guardrails-compliant testing patterns
 * based on the logging hooks from LOGGING_INTEGRATION.md.
 */

// Core interfaces and types
export {
  LoggingHook,
  LogEntry,
  LogLevel,
  HookConfig,
  AuthConfig,
  SendResult,
  createLogEntry,
} from './LoggingHook';

// Stub implementation for testing
export { StubLoggingHook } from './StubLoggingHook';

// Console implementation for development
export { ConsoleLoggingHook, ConsoleHookOptions } from './ConsoleLoggingHook';

// Retry utilities
export {
  sendWithRetry,
  withRetry,
  sleep,
  calculateDelay,
  isRetryableError,
  createSmartRetryOptions,
  RetryOptions,
  RetryResult,
} from './retry';

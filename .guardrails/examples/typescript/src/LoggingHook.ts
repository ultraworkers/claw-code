/**
 * LoggingHook Interface
 *
 * Defines the standard interface for logging hooks as per LOGGING_INTEGRATION.md.
 * All logging hook implementations must implement this interface.
 */

/**
 * Log levels matching the LOGGING_PATTERNS.md specification
 */
export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

/**
 * Standard log entry structure from LOGGING_PATTERNS.md
 */
export interface LogEntry {
  /** ISO8601 timestamp when the action occurred */
  timestamp: string;
  /** Log level */
  level: LogLevel;
  /** Category of operation (file_operation, git_operation, validation, decision) */
  category: string;
  /** What action was performed */
  action: string;
  /** What was acted upon */
  target: string;
  /** Result of the operation */
  result: 'success' | 'failure' | 'skipped';
  /** How long the operation took in milliseconds */
  duration_ms?: number;
  /** Additional context */
  metadata?: Record<string, unknown>;
  /** Error message if failed */
  error?: string;
  /** Stack trace if error */
  stack_trace?: string;
  /** Which agent logged this */
  agent_id?: string;
}

/**
 * Authentication configuration for external systems
 */
export interface AuthConfig {
  type: 'bearer' | 'basic' | 'api_key';
  token_env?: string;
  username?: string;
  password_env?: string;
}

/**
 * Hook configuration options
 */
export interface HookConfig {
  /** URL or path for the logging endpoint */
  endpoint?: string;
  /** Authentication configuration */
  auth?: AuthConfig;
  /** Number of entries to batch before sending */
  batch_size?: number;
  /** Timeout for send operations in milliseconds */
  timeout_ms?: number;
  /** Number of retry attempts */
  retry_count?: number;
  /** Delay between retries in milliseconds */
  retry_delay_ms?: number;
}

/**
 * Result of a send operation
 */
export interface SendResult {
  /** Whether the send was successful */
  success: boolean;
  /** Number of entries that were sent */
  entries_sent: number;
  /** Any errors that occurred */
  errors?: string[];
}

/**
 * Standard logging hook interface as defined in LOGGING_INTEGRATION.md
 *
 * All logging hook implementations must implement these methods:
 * - initialize: Setup the hook with configuration
 * - send: Send log entries to external system
 * - flush: Flush any buffered entries
 * - shutdown: Clean shutdown of the hook
 */
export interface LoggingHook {
  /**
   * Initialize the hook with configuration
   * @param config Hook configuration options
   */
  initialize(config: HookConfig): Promise<void>;

  /**
   * Send log entries to external system
   * @param entries Array of log entries to send
   * @returns Result of the send operation
   */
  send(entries: LogEntry[]): Promise<SendResult>;

  /**
   * Flush any buffered entries
   */
  flush(): Promise<void>;

  /**
   * Clean shutdown of the hook
   */
  shutdown(): Promise<void>;
}

/**
 * Helper function to create a log entry with current timestamp
 */
export function createLogEntry(
  level: LogLevel,
  category: string,
  action: string,
  target: string,
  result: 'success' | 'failure' | 'skipped',
  metadata?: Record<string, unknown>
): LogEntry {
  return {
    timestamp: new Date().toISOString(),
    level,
    category,
    action,
    target,
    result,
    ...(metadata && { metadata }),
  };
}

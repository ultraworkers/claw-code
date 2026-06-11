/**
 * ConsoleLoggingHook
 *
 * Console output implementation for development.
 * Formats log entries in human-readable format as per LOGGING_PATTERNS.md.
 */

import {
  LoggingHook,
  LogEntry,
  HookConfig,
  SendResult,
} from './LoggingHook';

/**
 * Options for console output formatting
 */
export interface ConsoleHookOptions {
  /** Include timestamp in output (default: true) */
  showTimestamp?: boolean;
  /** Include duration in output (default: true) */
  showDuration?: boolean;
  /** Include metadata in output (default: false) */
  showMetadata?: boolean;
  /** Use colors in output (default: true) */
  useColors?: boolean;
}

/**
 * Console logging hook that outputs formatted log entries to console.
 * Implements the human-readable format from LOGGING_PATTERNS.md.
 */
export class ConsoleLoggingHook implements LoggingHook {
  private config: HookConfig = {};
  private options: ConsoleHookOptions;
  private initialized: boolean = false;
  private shutdownCalled: boolean = false;
  private totalEntriesSent: number = 0;

  // Console function references (allows mocking in tests)
  private consoleFn: {
    log: (...args: unknown[]) => void;
    warn: (...args: unknown[]) => void;
    error: (...args: unknown[]) => void;
  };

  constructor(options: ConsoleHookOptions = {}, consoleFn?: typeof console) {
    this.options = {
      showTimestamp: true,
      showDuration: true,
      showMetadata: false,
      useColors: true,
      ...options,
    };
    this.consoleFn = consoleFn || console;
  }

  /**
   * Initialize the console hook
   */
  async initialize(config: HookConfig): Promise<void> {
    if (this.shutdownCalled) {
      throw new Error('Cannot initialize after shutdown');
    }
    this.config = config;
    this.initialized = true;
  }

  /**
   * Format and output log entries to console
   */
  async send(entries: LogEntry[]): Promise<SendResult> {
    if (!this.initialized) {
      throw new Error('Hook not initialized. Call initialize() first.');
    }
    if (this.shutdownCalled) {
      throw new Error('Hook has been shut down');
    }
    if (!entries || entries.length === 0) {
      return { success: true, entries_sent: 0 };
    }

    const errors: string[] = [];
    let entriesSent = 0;

    for (const entry of entries) {
      try {
        const formatted = this.formatEntry(entry);
        this.outputEntry(entry.level, formatted);
        entriesSent++;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        errors.push(`Failed to log entry: ${errorMessage}`);
      }
    }

    this.totalEntriesSent += entriesSent;

    return {
      success: errors.length === 0,
      entries_sent: entriesSent,
      ...(errors.length > 0 && { errors }),
    };
  }

  /**
   * Flush - no-op for console hook
   */
  async flush(): Promise<void> {
    // Console output is immediate, nothing to flush
  }

  /**
   * Shutdown the hook
   */
  async shutdown(): Promise<void> {
    this.shutdownCalled = true;
    this.initialized = false;
  }

  /**
   * Format a log entry to human-readable string
   * Format: [timestamp] LEVEL [category] action target -> result (duration)
   */
  formatEntry(entry: LogEntry): string {
    const parts: string[] = [];

    // Timestamp
    if (this.options.showTimestamp && entry.timestamp) {
      const date = new Date(entry.timestamp);
      const formatted = date.toISOString().replace('T', ' ').slice(0, 19);
      parts.push(`[${formatted}]`);
    }

    // Level
    parts.push(entry.level.padEnd(5));

    // Category
    parts.push(`[${entry.category}]`);

    // Action and target
    parts.push(entry.action);
    parts.push(entry.target);

    // Arrow and result
    parts.push('->');
    parts.push(entry.result);

    // Duration
    if (this.options.showDuration && entry.duration_ms !== undefined) {
      parts.push(`(${entry.duration_ms}ms)`);
    }

    // Error message
    if (entry.error) {
      parts.push(`| Error: ${entry.error}`);
    }

    // Metadata
    if (this.options.showMetadata && entry.metadata) {
      parts.push(`| ${JSON.stringify(entry.metadata)}`);
    }

    return parts.join(' ');
  }

  /**
   * Output to appropriate console method based on level
   */
  private outputEntry(level: string, message: string): void {
    switch (level) {
      case 'ERROR':
        this.consoleFn.error(message);
        break;
      case 'WARN':
        this.consoleFn.warn(message);
        break;
      default:
        this.consoleFn.log(message);
    }
  }

  // --- Utility methods ---

  /**
   * Check if hook is initialized
   */
  isInitialized(): boolean {
    return this.initialized;
  }

  /**
   * Check if hook has been shut down
   */
  isShutdown(): boolean {
    return this.shutdownCalled;
  }

  /**
   * Get total entries sent since initialization
   */
  getTotalEntriesSent(): number {
    return this.totalEntriesSent;
  }

  /**
   * Get current options
   */
  getOptions(): ConsoleHookOptions {
    return { ...this.options };
  }
}

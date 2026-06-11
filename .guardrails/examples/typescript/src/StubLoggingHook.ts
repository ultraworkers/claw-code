/**
 * StubLoggingHook
 *
 * Buffer-based stub implementation for development and testing.
 * As per LOGGING_INTEGRATION.md, this implementation stores logs in memory
 * for inspection during tests.
 */

import {
  LoggingHook,
  LogEntry,
  HookConfig,
  SendResult,
} from './LoggingHook';

/**
 * Stub logging hook that buffers entries in memory.
 * Useful for development and testing without external dependencies.
 */
export class StubLoggingHook implements LoggingHook {
  private buffer: LogEntry[] = [];
  private config: HookConfig = {};
  private initialized: boolean = false;
  private shutdownCalled: boolean = false;

  /**
   * Initialize the stub hook with configuration
   */
  async initialize(config: HookConfig): Promise<void> {
    if (this.shutdownCalled) {
      throw new Error('Cannot initialize after shutdown');
    }
    this.config = config;
    this.initialized = true;
  }

  /**
   * Send log entries to the internal buffer
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

    // Validate entries have required fields
    const errors: string[] = [];
    const validEntries: LogEntry[] = [];

    for (const entry of entries) {
      if (!entry.timestamp || !entry.level || !entry.category || !entry.action || !entry.target || !entry.result) {
        errors.push(`Invalid entry: missing required fields`);
      } else {
        validEntries.push(entry);
      }
    }

    this.buffer.push(...validEntries);

    return {
      success: errors.length === 0,
      entries_sent: validEntries.length,
      ...(errors.length > 0 && { errors }),
    };
  }

  /**
   * Flush the buffer (clears all entries)
   */
  async flush(): Promise<void> {
    if (this.shutdownCalled) {
      throw new Error('Hook has been shut down');
    }
    this.buffer = [];
  }

  /**
   * Shutdown the hook
   */
  async shutdown(): Promise<void> {
    await this.flush();
    this.shutdownCalled = true;
    this.initialized = false;
  }

  // --- Test utility methods ---

  /**
   * Get a copy of all buffered entries
   */
  getBuffer(): LogEntry[] {
    return [...this.buffer];
  }

  /**
   * Get the count of buffered entries
   */
  getBufferSize(): number {
    return this.buffer.length;
  }

  /**
   * Get entries filtered by level
   */
  getEntriesByLevel(level: string): LogEntry[] {
    return this.buffer.filter((entry) => entry.level === level);
  }

  /**
   * Get entries filtered by category
   */
  getEntriesByCategory(category: string): LogEntry[] {
    return this.buffer.filter((entry) => entry.category === category);
  }

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
   * Get the current configuration
   */
  getConfig(): HookConfig {
    return { ...this.config };
  }

  /**
   * Clear buffer without going through flush (for testing)
   */
  clearBuffer(): void {
    this.buffer = [];
  }
}

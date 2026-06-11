/**
 * ConsoleLoggingHook Tests
 *
 * Full test coverage for the console output implementation.
 * Uses mock console to capture and verify output.
 */

import { ConsoleLoggingHook, ConsoleHookOptions } from '../src/ConsoleLoggingHook';
import { LogEntry, HookConfig } from '../src/LoggingHook';

describe('ConsoleLoggingHook', () => {
  let hook: ConsoleLoggingHook;
  let mockConsole: {
    log: jest.Mock;
    warn: jest.Mock;
    error: jest.Mock;
  };

  // Helper to create valid log entries
  const createValidEntry = (overrides: Partial<LogEntry> = {}): LogEntry => ({
    timestamp: '2026-01-15T10:30:00.000Z',
    level: 'INFO',
    category: 'file_operation',
    action: 'read',
    target: '/path/to/file.ts',
    result: 'success',
    ...overrides,
  });

  beforeEach(() => {
    mockConsole = {
      log: jest.fn(),
      warn: jest.fn(),
      error: jest.fn(),
    };
    hook = new ConsoleLoggingHook({}, mockConsole as unknown as typeof console);
  });

  afterEach(async () => {
    if (hook.isInitialized() && !hook.isShutdown()) {
      await hook.shutdown();
    }
  });

  describe('initialization', () => {
    it('should initialize successfully', async () => {
      await hook.initialize({});
      expect(hook.isInitialized()).toBe(true);
    });

    it('should not allow initialization after shutdown', async () => {
      await hook.initialize({});
      await hook.shutdown();

      await expect(hook.initialize({})).rejects.toThrow('Cannot initialize after shutdown');
    });

    it('should accept custom options', async () => {
      const options: ConsoleHookOptions = {
        showTimestamp: false,
        showDuration: false,
        showMetadata: true,
        useColors: false,
      };

      hook = new ConsoleLoggingHook(options, mockConsole as unknown as typeof console);
      await hook.initialize({});

      expect(hook.getOptions()).toEqual(options);
    });

    it('should use default options when not specified', async () => {
      hook = new ConsoleLoggingHook({}, mockConsole as unknown as typeof console);
      await hook.initialize({});

      const options = hook.getOptions();
      expect(options.showTimestamp).toBe(true);
      expect(options.showDuration).toBe(true);
      expect(options.showMetadata).toBe(false);
      expect(options.useColors).toBe(true);
    });
  });

  describe('send', () => {
    beforeEach(async () => {
      await hook.initialize({});
    });

    it('should output INFO entries to console.log', async () => {
      const entry = createValidEntry({ level: 'INFO' });
      await hook.send([entry]);

      expect(mockConsole.log).toHaveBeenCalledTimes(1);
      expect(mockConsole.warn).not.toHaveBeenCalled();
      expect(mockConsole.error).not.toHaveBeenCalled();
    });

    it('should output DEBUG entries to console.log', async () => {
      const entry = createValidEntry({ level: 'DEBUG' });
      await hook.send([entry]);

      expect(mockConsole.log).toHaveBeenCalledTimes(1);
    });

    it('should output WARN entries to console.warn', async () => {
      const entry = createValidEntry({ level: 'WARN' });
      await hook.send([entry]);

      expect(mockConsole.warn).toHaveBeenCalledTimes(1);
      expect(mockConsole.log).not.toHaveBeenCalled();
      expect(mockConsole.error).not.toHaveBeenCalled();
    });

    it('should output ERROR entries to console.error', async () => {
      const entry = createValidEntry({ level: 'ERROR' });
      await hook.send([entry]);

      expect(mockConsole.error).toHaveBeenCalledTimes(1);
      expect(mockConsole.log).not.toHaveBeenCalled();
      expect(mockConsole.warn).not.toHaveBeenCalled();
    });

    it('should handle multiple entries', async () => {
      const entries = [
        createValidEntry({ level: 'INFO' }),
        createValidEntry({ level: 'WARN' }),
        createValidEntry({ level: 'ERROR' }),
      ];

      const result = await hook.send(entries);

      expect(result.success).toBe(true);
      expect(result.entries_sent).toBe(3);
      expect(mockConsole.log).toHaveBeenCalledTimes(1);
      expect(mockConsole.warn).toHaveBeenCalledTimes(1);
      expect(mockConsole.error).toHaveBeenCalledTimes(1);
    });

    it('should handle empty entries array', async () => {
      const result = await hook.send([]);

      expect(result.success).toBe(true);
      expect(result.entries_sent).toBe(0);
      expect(mockConsole.log).not.toHaveBeenCalled();
    });

    it('should throw if not initialized', async () => {
      const uninitializedHook = new ConsoleLoggingHook(
        {},
        mockConsole as unknown as typeof console
      );

      await expect(uninitializedHook.send([createValidEntry()])).rejects.toThrow(
        'Hook not initialized'
      );
    });

    it('should throw if shut down', async () => {
      await hook.shutdown();

      await expect(hook.send([createValidEntry()])).rejects.toThrow('shut down');
    });

    it('should track total entries sent', async () => {
      await hook.send([createValidEntry()]);
      await hook.send([createValidEntry(), createValidEntry()]);

      expect(hook.getTotalEntriesSent()).toBe(3);
    });
  });

  describe('formatEntry', () => {
    beforeEach(async () => {
      await hook.initialize({});
    });

    it('should format entry with all components', async () => {
      const entry = createValidEntry({
        timestamp: '2026-01-15T10:30:00.000Z',
        level: 'INFO',
        category: 'file_operation',
        action: 'read',
        target: '/path/to/file.ts',
        result: 'success',
        duration_ms: 45,
      });

      const formatted = hook.formatEntry(entry);

      expect(formatted).toContain('[2026-01-15 10:30:00]');
      expect(formatted).toContain('INFO');
      expect(formatted).toContain('[file_operation]');
      expect(formatted).toContain('read');
      expect(formatted).toContain('/path/to/file.ts');
      expect(formatted).toContain('->');
      expect(formatted).toContain('success');
      expect(formatted).toContain('(45ms)');
    });

    it('should include error message when present', async () => {
      const entry = createValidEntry({
        result: 'failure',
        error: 'File not found',
      });

      const formatted = hook.formatEntry(entry);

      expect(formatted).toContain('Error: File not found');
    });

    it('should hide timestamp when option is false', async () => {
      hook = new ConsoleLoggingHook(
        { showTimestamp: false },
        mockConsole as unknown as typeof console
      );
      await hook.initialize({});

      const entry = createValidEntry();
      const formatted = hook.formatEntry(entry);

      expect(formatted).not.toContain('[2026-01-15');
    });

    it('should hide duration when option is false', async () => {
      hook = new ConsoleLoggingHook(
        { showDuration: false },
        mockConsole as unknown as typeof console
      );
      await hook.initialize({});

      const entry = createValidEntry({ duration_ms: 100 });
      const formatted = hook.formatEntry(entry);

      expect(formatted).not.toContain('(100ms)');
    });

    it('should show metadata when option is true', async () => {
      hook = new ConsoleLoggingHook(
        { showMetadata: true },
        mockConsole as unknown as typeof console
      );
      await hook.initialize({});

      const entry = createValidEntry({
        metadata: { lines_read: 100 },
      });

      const formatted = hook.formatEntry(entry);

      expect(formatted).toContain('{"lines_read":100}');
    });

    it('should not show metadata when option is false', async () => {
      hook = new ConsoleLoggingHook(
        { showMetadata: false },
        mockConsole as unknown as typeof console
      );
      await hook.initialize({});

      const entry = createValidEntry({
        metadata: { lines_read: 100 },
      });

      const formatted = hook.formatEntry(entry);

      expect(formatted).not.toContain('lines_read');
    });
  });

  describe('flush', () => {
    it('should be a no-op (console output is immediate)', async () => {
      await hook.initialize({});
      await expect(hook.flush()).resolves.not.toThrow();
    });
  });

  describe('shutdown', () => {
    it('should mark hook as shut down', async () => {
      await hook.initialize({});
      await hook.shutdown();

      expect(hook.isShutdown()).toBe(true);
      expect(hook.isInitialized()).toBe(false);
    });

    it('should handle multiple shutdown calls', async () => {
      await hook.initialize({});
      await hook.shutdown();

      await expect(hook.shutdown()).resolves.not.toThrow();
    });
  });

  describe('edge cases', () => {
    beforeEach(async () => {
      await hook.initialize({});
    });

    it('should handle entries without optional fields', async () => {
      const entry: LogEntry = {
        timestamp: '2026-01-15T10:30:00.000Z',
        level: 'INFO',
        category: 'test',
        action: 'action',
        target: 'target',
        result: 'success',
      };

      const result = await hook.send([entry]);

      expect(result.success).toBe(true);
      expect(mockConsole.log).toHaveBeenCalled();
    });

    it('should handle all log levels correctly', async () => {
      const levels: Array<LogEntry['level']> = ['DEBUG', 'INFO', 'WARN', 'ERROR'];

      for (const level of levels) {
        await hook.send([createValidEntry({ level })]);
      }

      expect(mockConsole.log).toHaveBeenCalledTimes(2); // DEBUG and INFO
      expect(mockConsole.warn).toHaveBeenCalledTimes(1);
      expect(mockConsole.error).toHaveBeenCalledTimes(1);
    });

    it('should handle entries with special characters in target', async () => {
      const entry = createValidEntry({
        target: '/path/with spaces/and "quotes"/file.ts',
      });

      const result = await hook.send([entry]);

      expect(result.success).toBe(true);
      const logged = mockConsole.log.mock.calls[0][0];
      expect(logged).toContain('/path/with spaces/and "quotes"/file.ts');
    });

    it('should handle very long target paths', async () => {
      const longPath = '/very/long/' + 'path/'.repeat(50) + 'file.ts';
      const entry = createValidEntry({ target: longPath });

      const result = await hook.send([entry]);

      expect(result.success).toBe(true);
    });
  });
});

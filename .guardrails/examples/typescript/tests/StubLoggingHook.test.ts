/**
 * StubLoggingHook Tests
 *
 * Full test coverage for the buffer-based stub implementation.
 * Follows guardrails-compliant testing patterns.
 */

import { StubLoggingHook } from '../src/StubLoggingHook';
import { LogEntry, HookConfig, createLogEntry } from '../src/LoggingHook';

describe('StubLoggingHook', () => {
  let hook: StubLoggingHook;

  // Helper to create valid log entries
  const createValidEntry = (overrides: Partial<LogEntry> = {}): LogEntry => ({
    timestamp: new Date().toISOString(),
    level: 'INFO',
    category: 'file_operation',
    action: 'read',
    target: '/path/to/file.ts',
    result: 'success',
    ...overrides,
  });

  beforeEach(() => {
    hook = new StubLoggingHook();
  });

  afterEach(async () => {
    if (hook.isInitialized() && !hook.isShutdown()) {
      await hook.shutdown();
    }
  });

  describe('initialization', () => {
    it('should initialize successfully with empty config', async () => {
      await hook.initialize({});
      expect(hook.isInitialized()).toBe(true);
    });

    it('should initialize with full config', async () => {
      const config: HookConfig = {
        endpoint: 'https://example.com/logs',
        batch_size: 100,
        timeout_ms: 5000,
        retry_count: 3,
      };

      await hook.initialize(config);
      expect(hook.isInitialized()).toBe(true);
      expect(hook.getConfig()).toEqual(config);
    });

    it('should not allow re-initialization after shutdown', async () => {
      await hook.initialize({});
      await hook.shutdown();

      await expect(hook.initialize({})).rejects.toThrow('Cannot initialize after shutdown');
    });
  });

  describe('send', () => {
    beforeEach(async () => {
      await hook.initialize({});
    });

    it('should buffer valid entries', async () => {
      const entry = createValidEntry();
      const result = await hook.send([entry]);

      expect(result.success).toBe(true);
      expect(result.entries_sent).toBe(1);
      expect(hook.getBufferSize()).toBe(1);
      expect(hook.getBuffer()[0]).toEqual(entry);
    });

    it('should handle multiple entries', async () => {
      const entries = [
        createValidEntry({ action: 'read' }),
        createValidEntry({ action: 'write' }),
        createValidEntry({ action: 'delete' }),
      ];

      const result = await hook.send(entries);

      expect(result.success).toBe(true);
      expect(result.entries_sent).toBe(3);
      expect(hook.getBufferSize()).toBe(3);
    });

    it('should handle empty entries array', async () => {
      const result = await hook.send([]);

      expect(result.success).toBe(true);
      expect(result.entries_sent).toBe(0);
      expect(hook.getBufferSize()).toBe(0);
    });

    it('should accumulate entries across multiple sends', async () => {
      await hook.send([createValidEntry({ action: 'first' })]);
      await hook.send([createValidEntry({ action: 'second' })]);
      await hook.send([createValidEntry({ action: 'third' })]);

      expect(hook.getBufferSize()).toBe(3);
    });

    it('should reject entries with missing required fields', async () => {
      const invalidEntry = {
        timestamp: new Date().toISOString(),
        level: 'INFO',
        // missing category, action, target, result
      } as LogEntry;

      const result = await hook.send([invalidEntry]);

      expect(result.success).toBe(false);
      expect(result.entries_sent).toBe(0);
      expect(result.errors).toBeDefined();
      expect(result.errors!.length).toBeGreaterThan(0);
    });

    it('should filter out invalid entries while keeping valid ones', async () => {
      const validEntry = createValidEntry();
      const invalidEntry = { timestamp: 'invalid' } as LogEntry;

      const result = await hook.send([validEntry, invalidEntry]);

      expect(result.success).toBe(false); // Has errors
      expect(result.entries_sent).toBe(1); // Only valid entry sent
      expect(hook.getBufferSize()).toBe(1);
    });

    it('should throw if not initialized', async () => {
      const uninitializedHook = new StubLoggingHook();

      await expect(uninitializedHook.send([createValidEntry()])).rejects.toThrow(
        'Hook not initialized'
      );
    });

    it('should throw if shut down', async () => {
      await hook.shutdown();

      await expect(hook.send([createValidEntry()])).rejects.toThrow('shut down');
    });

    it('should preserve entry metadata', async () => {
      const entry = createValidEntry({
        duration_ms: 150,
        metadata: { lines_read: 100, file_size: 2048 },
        agent_id: 'test-agent',
      });

      await hook.send([entry]);

      const buffered = hook.getBuffer()[0];
      expect(buffered.duration_ms).toBe(150);
      expect(buffered.metadata).toEqual({ lines_read: 100, file_size: 2048 });
      expect(buffered.agent_id).toBe('test-agent');
    });
  });

  describe('flush', () => {
    beforeEach(async () => {
      await hook.initialize({});
    });

    it('should clear the buffer', async () => {
      await hook.send([createValidEntry(), createValidEntry()]);
      expect(hook.getBufferSize()).toBe(2);

      await hook.flush();
      expect(hook.getBufferSize()).toBe(0);
    });

    it('should handle flush on empty buffer', async () => {
      await expect(hook.flush()).resolves.not.toThrow();
      expect(hook.getBufferSize()).toBe(0);
    });

    it('should throw if shut down', async () => {
      await hook.shutdown();
      await expect(hook.flush()).rejects.toThrow('shut down');
    });
  });

  describe('shutdown', () => {
    it('should flush buffer on shutdown', async () => {
      await hook.initialize({});
      await hook.send([createValidEntry()]);

      await hook.shutdown();

      expect(hook.getBufferSize()).toBe(0);
      expect(hook.isShutdown()).toBe(true);
      expect(hook.isInitialized()).toBe(false);
    });

    it('should handle multiple shutdown calls gracefully', async () => {
      await hook.initialize({});
      await hook.shutdown();

      // Second shutdown should not throw
      await expect(hook.shutdown()).resolves.not.toThrow();
    });
  });

  describe('filtering methods', () => {
    beforeEach(async () => {
      await hook.initialize({});
      await hook.send([
        createValidEntry({ level: 'DEBUG', category: 'file_operation' }),
        createValidEntry({ level: 'INFO', category: 'git_operation' }),
        createValidEntry({ level: 'WARN', category: 'file_operation' }),
        createValidEntry({ level: 'ERROR', category: 'validation' }),
        createValidEntry({ level: 'INFO', category: 'file_operation' }),
      ]);
    });

    it('should filter entries by level', () => {
      const infoEntries = hook.getEntriesByLevel('INFO');
      expect(infoEntries.length).toBe(2);
      infoEntries.forEach((entry) => expect(entry.level).toBe('INFO'));
    });

    it('should filter entries by category', () => {
      const fileOps = hook.getEntriesByCategory('file_operation');
      expect(fileOps.length).toBe(3);
      fileOps.forEach((entry) => expect(entry.category).toBe('file_operation'));
    });

    it('should return empty array for non-existent level', () => {
      const entries = hook.getEntriesByLevel('CRITICAL');
      expect(entries).toEqual([]);
    });

    it('should return empty array for non-existent category', () => {
      const entries = hook.getEntriesByCategory('non_existent');
      expect(entries).toEqual([]);
    });
  });

  describe('utility methods', () => {
    it('should return a copy of buffer, not the original', async () => {
      await hook.initialize({});
      await hook.send([createValidEntry()]);

      const buffer1 = hook.getBuffer();
      const buffer2 = hook.getBuffer();

      expect(buffer1).toEqual(buffer2);
      expect(buffer1).not.toBe(buffer2);
    });

    it('should clear buffer without flush using clearBuffer', async () => {
      await hook.initialize({});
      await hook.send([createValidEntry()]);

      hook.clearBuffer();

      expect(hook.getBufferSize()).toBe(0);
    });
  });

  describe('createLogEntry helper', () => {
    it('should create a valid log entry', () => {
      const entry = createLogEntry(
        'INFO',
        'file_operation',
        'read',
        '/path/to/file',
        'success'
      );

      expect(entry.level).toBe('INFO');
      expect(entry.category).toBe('file_operation');
      expect(entry.action).toBe('read');
      expect(entry.target).toBe('/path/to/file');
      expect(entry.result).toBe('success');
      expect(entry.timestamp).toBeDefined();
    });

    it('should include optional metadata', () => {
      const entry = createLogEntry(
        'DEBUG',
        'validation',
        'lint',
        '/path/to/file',
        'success',
        { errors: [], warnings: ['minor issue'] }
      );

      expect(entry.metadata).toEqual({ errors: [], warnings: ['minor issue'] });
    });

    it('should create entries with valid ISO8601 timestamps', () => {
      const entry = createLogEntry('INFO', 'test', 'action', 'target', 'success');
      const timestamp = new Date(entry.timestamp);

      expect(timestamp.toISOString()).toBe(entry.timestamp);
    });
  });
});

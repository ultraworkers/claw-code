-- Drop file_reads table

DROP INDEX IF EXISTS idx_file_reads_session_path;
DROP INDEX IF EXISTS idx_file_reads_read_at;
DROP INDEX IF EXISTS idx_file_reads_path;
DROP INDEX IF EXISTS idx_file_reads_session;
DROP TABLE IF EXISTS file_reads;

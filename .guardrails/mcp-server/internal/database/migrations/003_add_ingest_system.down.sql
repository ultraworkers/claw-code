-- Migration: Revert ingest system tables and columns
-- Created: 2026-02-08

-- Drop indexes
DROP INDEX IF EXISTS idx_documents_source;
DROP INDEX IF EXISTS idx_documents_orphaned;
DROP INDEX IF EXISTS idx_ingest_jobs_status;
DROP INDEX IF EXISTS idx_ingest_jobs_started_at;
DROP INDEX IF EXISTS idx_update_checks_checked_at;

-- Drop tables
DROP TABLE IF EXISTS update_checks;
DROP TABLE IF EXISTS ingest_jobs;

-- Drop columns from documents
ALTER TABLE documents DROP COLUMN IF EXISTS source;
ALTER TABLE documents DROP COLUMN IF EXISTS content_hash;
ALTER TABLE documents DROP COLUMN IF EXISTS file_path;
ALTER TABLE documents DROP COLUMN IF EXISTS orphaned;

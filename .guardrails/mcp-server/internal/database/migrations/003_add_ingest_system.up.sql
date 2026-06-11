-- Migration: Add ingest system tables and columns
-- Created: 2026-02-08

-- Add source column to documents table
ALTER TABLE documents ADD COLUMN IF NOT EXISTS source VARCHAR(20) DEFAULT 'system';
ALTER TABLE documents ADD COLUMN IF NOT EXISTS content_hash VARCHAR(64);
ALTER TABLE documents ADD COLUMN IF NOT EXISTS file_path VARCHAR(500);
ALTER TABLE documents ADD COLUMN IF NOT EXISTS orphaned BOOLEAN DEFAULT FALSE;

-- Create index on source for filtering
CREATE INDEX IF NOT EXISTS idx_documents_source ON documents(source);
CREATE INDEX IF NOT EXISTS idx_documents_orphaned ON documents(orphaned) WHERE orphaned = TRUE;

-- Create ingest_jobs table
CREATE TABLE IF NOT EXISTS ingest_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source VARCHAR(20) NOT NULL, -- 'repo', 'upload', 'folder'
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- 'pending', 'running', 'completed', 'failed'
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    files_processed INTEGER DEFAULT 0,
    files_added INTEGER DEFAULT 0,
    files_updated INTEGER DEFAULT 0,
    files_orphaned INTEGER DEFAULT 0,
    errors JSONB DEFAULT '[]',
    metadata JSONB DEFAULT '{}',
    created_by VARCHAR(100) -- user or system
);

-- Create index on status for querying active jobs
CREATE INDEX IF NOT EXISTS idx_ingest_jobs_status ON ingest_jobs(status);
CREATE INDEX IF NOT EXISTS idx_ingest_jobs_started_at ON ingest_jobs(started_at DESC);

-- Create update_checks table
CREATE TABLE IF NOT EXISTS update_checks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    checked_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    docker_current_version VARCHAR(50),
    docker_latest_version VARCHAR(50),
    docker_release_notes TEXT,
    docker_update_available BOOLEAN DEFAULT FALSE,
    guardrail_current_commit VARCHAR(64),
    guardrail_latest_commit VARCHAR(64),
    guardrail_new_files INTEGER DEFAULT 0,
    guardrail_modified_files INTEGER DEFAULT 0,
    guardrail_deleted_files INTEGER DEFAULT 0,
    guardrail_update_available BOOLEAN DEFAULT FALSE,
    metadata JSONB DEFAULT '{}'
);

-- Create index on check time
CREATE INDEX IF NOT EXISTS idx_update_checks_checked_at ON update_checks(checked_at DESC);

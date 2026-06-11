-- Add file_reads table for tracking file reads

CREATE TABLE IF NOT EXISTS file_reads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    read_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    content_hash VARCHAR(64),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_file_reads_session ON file_reads(session_id);
CREATE INDEX IF NOT EXISTS idx_file_reads_path ON file_reads(file_path);
CREATE INDEX IF NOT EXISTS idx_file_reads_read_at ON file_reads(read_at DESC);

-- Unique constraint to prevent duplicate entries for same session+file
CREATE UNIQUE INDEX IF NOT EXISTS idx_file_reads_session_path
    ON file_reads(session_id, file_path);

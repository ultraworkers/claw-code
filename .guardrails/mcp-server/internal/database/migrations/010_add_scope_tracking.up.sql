-- Migration to add scope boundary tracking tables
-- These tables track what files are in scope and monitor file changes

-- Create enum type for file change types
CREATE TYPE change_type_enum AS ENUM ('addition', 'modification', 'deletion');

-- Table 1: scope_definitions
-- Tracks what files are in scope for a session/task
CREATE TABLE IF NOT EXISTS scope_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    task_id VARCHAR(100),
    scope_description TEXT,
    scope_boundaries TEXT,
    affected_files TEXT[],
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    -- Indexes created separately below
);

-- Table 2: file_changes
-- Tracks actual file modifications made during sessions
CREATE TABLE IF NOT EXISTS file_changes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    change_type change_type_enum NOT NULL,
    lines_added INT DEFAULT 0,
    lines_removed INT DEFAULT 0,
    diff_summary TEXT,
    is_within_scope BOOLEAN NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient queries on scope_definitions
CREATE INDEX IF NOT EXISTS idx_scope_definitions_session_id ON scope_definitions(session_id);
CREATE INDEX IF NOT EXISTS idx_scope_definitions_task_id ON scope_definitions(task_id);
CREATE INDEX IF NOT EXISTS idx_scope_definitions_created_at ON scope_definitions(created_at);

-- Indexes for efficient queries on file_changes
CREATE INDEX IF NOT EXISTS idx_file_changes_session_id ON file_changes(session_id);
CREATE INDEX IF NOT EXISTS idx_file_changes_file_path ON file_changes(file_path);
CREATE INDEX IF NOT EXISTS idx_file_changes_change_type ON file_changes(change_type);
CREATE INDEX IF NOT EXISTS idx_file_changes_is_within_scope ON file_changes(is_within_scope);
CREATE INDEX IF NOT EXISTS idx_file_changes_created_at ON file_changes(created_at);

-- Partial index for out-of-scope changes (most critical to track)
CREATE INDEX IF NOT EXISTS idx_file_changes_out_of_scope ON file_changes(session_id, file_path)
    WHERE is_within_scope = false;

-- Comments for documentation
COMMENT ON TABLE scope_definitions IS 'Tracks scope boundaries defined for sessions/tasks, including what IS and IS NOT in scope';
COMMENT ON TABLE file_changes IS 'Tracks actual file modifications to validate against defined scope boundaries';
COMMENT ON COLUMN scope_definitions.scope_description IS 'Description of what IS in scope for the session/task';
COMMENT ON COLUMN scope_definitions.scope_boundaries IS 'Description of what is NOT in scope (boundaries)';
COMMENT ON COLUMN scope_definitions.affected_files IS 'List of file paths that can be modified within scope';
COMMENT ON COLUMN file_changes.lines_added IS 'Number of lines added in this change';
COMMENT ON COLUMN file_changes.lines_removed IS 'Number of lines removed in this change';
COMMENT ON COLUMN file_changes.diff_summary IS 'Brief description of what changed';
COMMENT ON COLUMN file_changes.is_within_scope IS 'Whether this change was within the defined scope boundaries';

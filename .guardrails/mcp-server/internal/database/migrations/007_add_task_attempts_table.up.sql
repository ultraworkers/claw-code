-- Add task_attempts table for three strikes tracking

CREATE TABLE IF NOT EXISTS task_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    task_id VARCHAR(100),
    attempt_number INTEGER NOT NULL,
    attempted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    error_message TEXT,
    error_category VARCHAR(50),
    resolution VARCHAR(50) NOT NULL DEFAULT 'pending',
    resolved_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_error_category CHECK (
        error_category IS NULL OR
        error_category IN ('syntax', 'runtime', 'logic', 'timeout', 'other')
    ),
    CONSTRAINT valid_resolution CHECK (
        resolution IN ('pending', 'resolved', 'escalated', 'abandoned')
    )
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_task_attempts_session ON task_attempts(session_id);
CREATE INDEX IF NOT EXISTS idx_task_attempts_task ON task_attempts(task_id);
CREATE INDEX IF NOT EXISTS idx_task_attempts_session_task ON task_attempts(session_id, task_id);
CREATE INDEX IF NOT EXISTS idx_task_attempts_attempted_at ON task_attempts(attempted_at DESC);

-- Partial index for pending attempts (most queried)
CREATE INDEX IF NOT EXISTS idx_task_attempts_pending ON task_attempts(session_id, task_id, resolution)
    WHERE resolution = 'pending';

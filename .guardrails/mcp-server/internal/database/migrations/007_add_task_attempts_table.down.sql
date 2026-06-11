-- Drop task_attempts table

DROP INDEX IF EXISTS idx_task_attempts_pending;
DROP INDEX IF EXISTS idx_task_attempts_attempted_at;
DROP INDEX IF EXISTS idx_task_attempts_session_task;
DROP INDEX IF EXISTS idx_task_attempts_task;
DROP INDEX IF EXISTS idx_task_attempts_session;

DROP TABLE IF EXISTS task_attempts;

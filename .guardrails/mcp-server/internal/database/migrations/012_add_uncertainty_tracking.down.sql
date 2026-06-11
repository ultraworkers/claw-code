-- Revert uncertainty tracking table
DROP INDEX IF EXISTS idx_uncertainty_created_at;
DROP INDEX IF EXISTS idx_uncertainty_escalation;
DROP INDEX IF EXISTS idx_uncertainty_level;
DROP INDEX IF EXISTS idx_uncertainty_task_id;
DROP INDEX IF EXISTS idx_uncertainty_session_id;

DROP TABLE IF EXISTS uncertainty_tracking;

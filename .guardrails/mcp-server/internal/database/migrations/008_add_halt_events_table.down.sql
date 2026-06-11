-- Drop halt_events table

DROP INDEX IF EXISTS idx_halt_events_critical_unresolved;
DROP INDEX IF EXISTS idx_halt_events_unacknowledged;
DROP INDEX IF EXISTS idx_halt_events_acknowledged;
DROP INDEX IF EXISTS idx_halt_events_triggered_at;
DROP INDEX IF EXISTS idx_halt_events_severity;
DROP INDEX IF EXISTS idx_halt_events_halt_type;
DROP INDEX IF EXISTS idx_halt_events_session_id;

DROP TABLE IF EXISTS halt_events;

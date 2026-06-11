-- Down migration for uncertainty_tracking fix
-- This restores the original table structure with FK constraints

DROP TABLE IF EXISTS uncertainty_tracking;
DROP TYPE IF EXISTS uncertainty_level_enum;

-- Note: The original table had FK constraints to session_metadata and tasks tables
-- which don't exist. This down migration just removes the table.
-- To fully restore, those referenced tables would need to exist first.

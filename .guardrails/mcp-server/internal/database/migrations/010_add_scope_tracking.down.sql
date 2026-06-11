-- Drop scope tracking tables and enum type

-- Drop indexes on file_changes first
DROP INDEX IF EXISTS idx_file_changes_out_of_scope;
DROP INDEX IF EXISTS idx_file_changes_created_at;
DROP INDEX IF EXISTS idx_file_changes_is_within_scope;
DROP INDEX IF EXISTS idx_file_changes_change_type;
DROP INDEX IF EXISTS idx_file_changes_file_path;
DROP INDEX IF EXISTS idx_file_changes_session_id;

-- Drop indexes on scope_definitions
DROP INDEX IF EXISTS idx_scope_definitions_created_at;
DROP INDEX IF EXISTS idx_scope_definitions_task_id;
DROP INDEX IF EXISTS idx_scope_definitions_session_id;

-- Drop tables
DROP TABLE IF EXISTS file_changes;
DROP TABLE IF EXISTS scope_definitions;

-- Drop enum type
DROP TYPE IF EXISTS change_type_enum;

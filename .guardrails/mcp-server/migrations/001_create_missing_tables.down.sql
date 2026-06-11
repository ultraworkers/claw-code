-- Migration: Rollback for missing tables
-- Description: Drops all 6 tables created by the up migration

-- Drop partition management function
DROP FUNCTION IF EXISTS create_failure_registry_partition();

-- Drop tables in reverse order (respecting dependencies)
DROP TABLE IF EXISTS production_code_tracking CASCADE;
DROP TYPE IF EXISTS code_type_enum;

DROP TABLE IF EXISTS uncertainty_tracking CASCADE;
DROP TABLE IF EXISTS task_attempts CASCADE;
DROP TABLE IF EXISTS file_reads CASCADE;

-- Drop partitions first
DROP TABLE IF EXISTS failure_registry_y2026m02 CASCADE;
DROP TABLE IF EXISTS failure_registry CASCADE;

DROP TABLE IF EXISTS prevention_rules CASCADE;

-- Note: The pgcrypto extension is not dropped as it may be used by other tables

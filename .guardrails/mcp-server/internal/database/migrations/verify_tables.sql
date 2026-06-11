-- Verify all required tables exist and have correct structure
-- Run this after migrations to verify database state

-- Check all required tables exist
SELECT 'prevention_rules' as table_name, EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'prevention_rules'
) as exists;

SELECT 'failure_registry' as table_name, EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'failure_registry'
) as exists;

SELECT 'file_reads' as table_name, EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'file_reads'
) as exists;

SELECT 'task_attempts' as table_name, EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'task_attempts'
) as exists;

SELECT 'uncertainty_tracking' as table_name, EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'uncertainty_tracking'
) as exists;

SELECT 'production_code_tracking' as table_name, EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'production_code_tracking'
) as exists;

-- Verify table structures
\echo '=== prevention_rules columns ==='
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_name = 'prevention_rules'
ORDER BY ordinal_position;

\echo '=== failure_registry columns ==='
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_name = 'failure_registry'
ORDER BY ordinal_position;

\echo '=== file_reads columns ==='
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_name = 'file_reads'
ORDER BY ordinal_position;

\echo '=== task_attempts columns ==='
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_name = 'task_attempts'
ORDER BY ordinal_position;

\echo '=== uncertainty_tracking columns ==='
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_name = 'uncertainty_tracking'
ORDER BY ordinal_position;

\echo '=== production_code_tracking columns ==='
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_name = 'production_code_tracking'
ORDER BY ordinal_position;

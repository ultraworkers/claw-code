-- Fix redundant and missing indexes

-- Drop redundant index (slug is already UNIQUE)
DROP INDEX IF EXISTS idx_documents_slug;

-- Add missing index on rule_id (frequently queried, though UNIQUE already creates one)
-- This ensures explicit visibility for query planning
CREATE INDEX IF NOT EXISTS idx_rules_rule_id ON prevention_rules(rule_id);

-- Add composite index for common active rules queries
CREATE INDEX IF NOT EXISTS idx_rules_enabled_severity ON prevention_rules(enabled, severity)
    WHERE enabled = true;

-- Add index for time-based document queries
CREATE INDEX IF NOT EXISTS idx_documents_created ON documents(created_at DESC);

-- Add composite index for failure dashboard queries
CREATE INDEX IF NOT EXISTS idx_failures_status_severity ON failure_registry(status, severity)
    WHERE status = 'active';

-- Add index for pattern lookups (used in rule matching)
CREATE INDEX IF NOT EXISTS idx_rules_pattern_hash ON prevention_rules(pattern_hash)
    WHERE pattern_hash IS NOT NULL;

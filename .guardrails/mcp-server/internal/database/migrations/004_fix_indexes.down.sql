-- Revert index changes
CREATE INDEX IF NOT EXISTS idx_documents_slug ON documents(slug);

DROP INDEX IF EXISTS idx_rules_rule_id;
DROP INDEX IF EXISTS idx_rules_enabled_severity;
DROP INDEX IF EXISTS idx_documents_created;
DROP INDEX IF EXISTS idx_failures_status_severity;
DROP INDEX IF EXISTS idx_rules_pattern_hash;

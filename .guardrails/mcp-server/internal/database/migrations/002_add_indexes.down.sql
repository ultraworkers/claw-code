-- Drop all indexes
DROP INDEX IF EXISTS idx_documents_category;
DROP INDEX IF EXISTS idx_documents_slug;
DROP INDEX IF EXISTS idx_documents_updated;
DROP INDEX IF EXISTS idx_documents_search;
DROP INDEX IF EXISTS idx_documents_metadata;

DROP INDEX IF EXISTS idx_rules_document;
DROP INDEX IF EXISTS idx_rules_enabled;
DROP INDEX IF EXISTS idx_rules_severity;
DROP INDEX IF EXISTS idx_rules_category;
DROP INDEX IF EXISTS idx_rules_covering;

DROP INDEX IF EXISTS idx_failures_status;
DROP INDEX IF EXISTS idx_failures_category;
DROP INDEX IF EXISTS idx_failures_created;
DROP INDEX IF EXISTS idx_failures_files;
DROP INDEX IF EXISTS idx_failures_project;
DROP INDEX IF EXISTS idx_failures_covering;

DROP INDEX IF EXISTS idx_projects_slug;
DROP INDEX IF EXISTS idx_projects_active_rules;
DROP INDEX IF EXISTS idx_projects_metadata;

DROP INDEX IF EXISTS idx_audit_time;
DROP INDEX IF EXISTS idx_audit_actor;
DROP INDEX IF EXISTS idx_audit_type;
DROP INDEX IF EXISTS idx_audit_resource;

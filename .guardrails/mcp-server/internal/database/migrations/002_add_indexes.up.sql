-- Add indexes for performance

-- Document indexes
CREATE INDEX IF NOT EXISTS idx_documents_category ON documents(category);
CREATE INDEX IF NOT EXISTS idx_documents_slug ON documents(slug);
CREATE INDEX IF NOT EXISTS idx_documents_updated ON documents(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_documents_search ON documents USING GIN(search_vector);
CREATE INDEX IF NOT EXISTS idx_documents_metadata ON documents USING GIN(metadata);

-- Prevention rules indexes
CREATE INDEX IF NOT EXISTS idx_rules_document ON prevention_rules(document_id);
CREATE INDEX IF NOT EXISTS idx_rules_enabled ON prevention_rules(enabled) WHERE enabled = true;
CREATE INDEX IF NOT EXISTS idx_rules_severity ON prevention_rules(severity);
CREATE INDEX IF NOT EXISTS idx_rules_category ON prevention_rules(category);
CREATE INDEX IF NOT EXISTS idx_rules_covering ON prevention_rules(document_id, rule_id, name, severity, enabled)
    INCLUDE (pattern, message);

-- Failure registry indexes
CREATE INDEX IF NOT EXISTS idx_failures_status ON failure_registry(status);
CREATE INDEX IF NOT EXISTS idx_failures_category ON failure_registry(category);
CREATE INDEX IF NOT EXISTS idx_failures_created ON failure_registry(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_failures_files ON failure_registry USING GIN(affected_files);
CREATE INDEX IF NOT EXISTS idx_failures_project ON failure_registry(project_slug);
CREATE INDEX IF NOT EXISTS idx_failures_covering ON failure_registry(status, created_at DESC, severity)
    INCLUDE (failure_id, category, error_message);

-- Projects indexes
CREATE INDEX IF NOT EXISTS idx_projects_slug ON projects(slug);
CREATE INDEX IF NOT EXISTS idx_projects_active_rules ON projects USING GIN(active_rules);
CREATE INDEX IF NOT EXISTS idx_projects_metadata ON projects USING GIN(metadata);

-- Audit log indexes
CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_log(actor);
CREATE INDEX IF NOT EXISTS idx_audit_type ON audit_log(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource);

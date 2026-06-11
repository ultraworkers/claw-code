-- Drop triggers
DROP TRIGGER IF EXISTS trigger_documents_updated_at ON documents;
DROP TRIGGER IF EXISTS trigger_documents_search_vector ON documents;
DROP TRIGGER IF EXISTS trigger_rules_updated_at ON prevention_rules;
DROP TRIGGER IF EXISTS trigger_failures_updated_at ON failure_registry;
DROP TRIGGER IF EXISTS trigger_projects_updated_at ON projects;

-- Drop functions
DROP FUNCTION IF EXISTS update_updated_at_column();
DROP FUNCTION IF EXISTS update_document_search_vector();

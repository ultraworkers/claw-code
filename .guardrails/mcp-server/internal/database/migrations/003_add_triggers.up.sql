-- Add triggers for automatic timestamp and search vector updates

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Function to update search vector on document changes
CREATE OR REPLACE FUNCTION update_document_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.content, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.category, '')), 'C');
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger for documents updated_at
DROP TRIGGER IF EXISTS trigger_documents_updated_at ON documents;
CREATE TRIGGER trigger_documents_updated_at
    BEFORE UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger for documents search vector on insert/update
DROP TRIGGER IF EXISTS trigger_documents_search_vector ON documents;
CREATE TRIGGER trigger_documents_search_vector
    BEFORE INSERT OR UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_document_search_vector();

-- Trigger for prevention_rules updated_at
DROP TRIGGER IF EXISTS trigger_rules_updated_at ON prevention_rules;
CREATE TRIGGER trigger_rules_updated_at
    BEFORE UPDATE ON prevention_rules
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger for failure_registry updated_at
DROP TRIGGER IF EXISTS trigger_failures_updated_at ON failure_registry;
CREATE TRIGGER trigger_failures_updated_at
    BEFORE UPDATE ON failure_registry
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger for projects updated_at
DROP TRIGGER IF EXISTS trigger_projects_updated_at ON projects;
CREATE TRIGGER trigger_projects_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Update existing documents to populate search_vector
UPDATE documents SET updated_at = updated_at WHERE search_vector IS NULL;

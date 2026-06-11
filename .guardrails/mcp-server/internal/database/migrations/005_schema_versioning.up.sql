-- Add schema migrations tracking table
CREATE TABLE IF NOT EXISTS schema_migrations (
    version BIGINT PRIMARY KEY,
    applied_at TIMESTAMP NOT NULL DEFAULT NOW(),
    description TEXT,
    checksum VARCHAR(64) NOT NULL
);

-- Add comment for documentation
COMMENT ON TABLE schema_migrations IS 'Tracks applied database migrations';
COMMENT ON COLUMN schema_migrations.version IS 'Migration version number (Unix timestamp or sequential)';
COMMENT ON COLUMN schema_migrations.checksum IS 'SHA-256 hash of migration file for integrity verification';

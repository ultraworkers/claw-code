-- Create enum types for fix_type and verification_status
CREATE TYPE fix_type AS ENUM ('regex', 'code_change', 'config');
CREATE TYPE verification_status AS ENUM ('confirmed', 'modified', 'removed');

-- Create fix_verification_tracking table
CREATE TABLE fix_verification_tracking (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(255) NOT NULL,
    failure_id VARCHAR(255) REFERENCES failure_registry(id),
    fix_hash VARCHAR(64) NOT NULL, -- SHA256 hash
    file_path TEXT NOT NULL, -- where the fix lives
    fix_content TEXT, -- what the fix looks like
    fix_type fix_type NOT NULL,
    verified_at TIMESTAMP,
    verification_status verification_status,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes
CREATE INDEX idx_fix_verification_session_file ON fix_verification_tracking(session_id, file_path);
CREATE UNIQUE INDEX idx_fix_verification_session_failure ON fix_verification_tracking(session_id, failure_id);
CREATE INDEX idx_fix_verification_status ON fix_verification_tracking(verification_status);
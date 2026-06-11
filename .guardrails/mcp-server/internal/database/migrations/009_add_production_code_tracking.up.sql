-- Migration to create production code tracking table
-- This table tracks code existence for guardrail validation

CREATE TYPE code_type_enum AS ENUM ('production', 'test', 'infrastructure');

CREATE TABLE production_code_tracking (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(255) NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    code_type code_type_enum NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    verified_at TIMESTAMP WITH TIME ZONE NULL,

    -- Constraints
    CONSTRAINT unique_session_file UNIQUE (session_id, file_path)
);

-- Indexes
CREATE INDEX idx_production_code_tracking_session_id ON production_code_tracking(session_id);
CREATE INDEX idx_production_code_tracking_session_id_type ON production_code_tracking(session_id, code_type);
CREATE INDEX idx_production_code_tracking_created_at ON production_code_tracking(created_at);
COMMENT ON TABLE production_code_tracking IS 'Tracks production code existence for guardrail validation';

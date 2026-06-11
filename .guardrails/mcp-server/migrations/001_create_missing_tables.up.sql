-- Migration: Create missing tables for guardrail MCP server
-- Description: Creates all 6 tables identified in the test report
-- Tables: prevention_rules, failure_registry, file_reads, task_attempts, uncertainty_tracking, production_code_tracking

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================================
-- Table 1: prevention_rules
-- Purpose: Store guardrail prevention rules for pattern matching
-- ============================================================================
CREATE TABLE IF NOT EXISTS prevention_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id VARCHAR(50) UNIQUE NOT NULL CHECK (LENGTH(TRIM(rule_id)) > 0),
    name VARCHAR(255) NOT NULL,
    pattern TEXT NOT NULL,
    pattern_hash VARCHAR(64),
    message TEXT NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('critical', 'error', 'warning', 'info')),
    enabled BOOLEAN NOT NULL DEFAULT true,
    document_id UUID,
    category VARCHAR(50),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for prevention_rules
CREATE INDEX IF NOT EXISTS idx_prevention_rules_enabled ON prevention_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_prevention_rules_severity ON prevention_rules(severity);
CREATE INDEX IF NOT EXISTS idx_prevention_rules_category ON prevention_rules(category);
CREATE INDEX IF NOT EXISTS idx_prevention_rules_updated_at ON prevention_rules(updated_at DESC);

-- Comments
COMMENT ON TABLE prevention_rules IS 'Stores guardrail prevention rules for pattern matching and validation';
COMMENT ON COLUMN prevention_rules.rule_id IS 'Unique identifier for the rule (e.g., RULE-001)';
COMMENT ON COLUMN prevention_rules.pattern IS 'Regex pattern or text to match against';
COMMENT ON COLUMN prevention_rules.severity IS 'Critical=halt, Error=block, Warning=confirm, Info=log';

-- ============================================================================
-- Table 2: failure_registry
-- Purpose: Track failures and regressions for continuous learning
-- ============================================================================
CREATE TABLE IF NOT EXISTS failure_registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    failure_id VARCHAR(50) UNIQUE NOT NULL,
    category VARCHAR(50) NOT NULL,
    severity VARCHAR(10) NOT NULL CHECK (severity IN ('critical', 'high', 'medium', 'low')),
    error_message TEXT NOT NULL,
    root_cause TEXT,
    affected_files TEXT[],
    regression_pattern VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'resolved', 'deprecated')),
    project_slug VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- Create initial partition for current month
CREATE TABLE IF NOT EXISTS failure_registry_y2026m02 PARTITION OF failure_registry
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

-- Indexes for failure_registry
CREATE INDEX IF NOT EXISTS idx_failure_registry_status ON failure_registry(status);
CREATE INDEX IF NOT EXISTS idx_failure_registry_category ON failure_registry(category);
CREATE INDEX IF NOT EXISTS idx_failure_registry_severity ON failure_registry(severity);
CREATE INDEX IF NOT EXISTS idx_failure_registry_project_slug ON failure_registry(project_slug);
CREATE INDEX IF NOT EXISTS idx_failure_registry_created_at ON failure_registry(created_at DESC);

-- Partial index for active failures
CREATE INDEX IF NOT EXISTS idx_failure_registry_active ON failure_registry(category, severity)
    WHERE status = 'active';

-- Comments
COMMENT ON TABLE failure_registry IS 'Tracks failures and regressions for continuous learning and prevention';
COMMENT ON COLUMN failure_registry.failure_id IS 'Unique identifier for the failure (e.g., FAIL-001)';
COMMENT ON COLUMN failure_registry.regression_pattern IS 'Pattern to detect if this failure reoccurs';

-- ============================================================================
-- Table 3: file_reads
-- Purpose: Track file reads within sessions for audit and validation
-- ============================================================================
CREATE TABLE IF NOT EXISTS file_reads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    read_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    content_hash VARCHAR(64),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for file_reads
CREATE INDEX IF NOT EXISTS idx_file_reads_session ON file_reads(session_id);
CREATE INDEX IF NOT EXISTS idx_file_reads_path ON file_reads(file_path);
CREATE INDEX IF NOT EXISTS idx_file_reads_read_at ON file_reads(read_at DESC);

-- Unique constraint to prevent duplicate entries for same session+file
CREATE UNIQUE INDEX IF NOT EXISTS idx_file_reads_session_path
    ON file_reads(session_id, file_path);

-- Comments
COMMENT ON TABLE file_reads IS 'Tracks file reads within sessions for audit and validation purposes';
COMMENT ON COLUMN file_reads.content_hash IS 'SHA256 hash of file content at time of read';

-- ============================================================================
-- Table 4: task_attempts
-- Purpose: Track task attempts for three-strikes pattern detection
-- ============================================================================
CREATE TABLE IF NOT EXISTS task_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    task_id VARCHAR(100),
    attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
    attempted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    error_message TEXT,
    error_category VARCHAR(50),
    resolution VARCHAR(50) NOT NULL DEFAULT 'pending',
    resolved_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_error_category CHECK (
        error_category IS NULL OR
        error_category IN ('syntax', 'runtime', 'logic', 'timeout', 'other')
    ),
    CONSTRAINT valid_resolution CHECK (
        resolution IN ('pending', 'resolved', 'escalated', 'abandoned')
    )
);

-- Indexes for task_attempts
CREATE INDEX IF NOT EXISTS idx_task_attempts_session ON task_attempts(session_id);
CREATE INDEX IF NOT EXISTS idx_task_attempts_task ON task_attempts(task_id);
CREATE INDEX IF NOT EXISTS idx_task_attempts_session_task ON task_attempts(session_id, task_id);
CREATE INDEX IF NOT EXISTS idx_task_attempts_attempted_at ON task_attempts(attempted_at DESC);

-- Partial index for pending attempts (most queried)
CREATE INDEX IF NOT EXISTS idx_task_attempts_pending ON task_attempts(session_id, task_id, resolution)
    WHERE resolution = 'pending';

-- Comments
COMMENT ON TABLE task_attempts IS 'Tracks task attempts for three-strikes pattern detection';
COMMENT ON COLUMN task_attempts.attempt_number IS 'Sequential attempt number for this task';
COMMENT ON COLUMN task_attempts.resolution IS 'pending=active, resolved=fixed, escalated=human, abandoned=given up';

-- ============================================================================
-- Table 5: uncertainty_tracking
-- Purpose: Track uncertainty levels and decision-making context
-- ============================================================================
CREATE TABLE IF NOT EXISTS uncertainty_tracking (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    task_id VARCHAR(100),
    uncertainty_level VARCHAR(50) NOT NULL,
    decision_made TEXT,
    context_data JSONB,
    escalation_required BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_uncertainty_level CHECK (uncertainty_level IN (
        'critical',
        'blocked',
        'high',
        'medium',
        'investigating',
        'low',
        'resolved'
    ))
);

-- Indexes for uncertainty_tracking
CREATE INDEX IF NOT EXISTS idx_uncertainty_session_id ON uncertainty_tracking(session_id);
CREATE INDEX IF NOT EXISTS idx_uncertainty_task_id ON uncertainty_tracking(task_id);
CREATE INDEX IF NOT EXISTS idx_uncertainty_level ON uncertainty_tracking(uncertainty_level);
CREATE INDEX IF NOT EXISTS idx_uncertainty_escalation ON uncertainty_tracking(escalation_required) WHERE escalation_required = true;
CREATE INDEX IF NOT EXISTS idx_uncertainty_created_at ON uncertainty_tracking(created_at DESC);

-- Comments
COMMENT ON TABLE uncertainty_tracking IS 'Tracks uncertainty levels during MCP operations and decision-making context';
COMMENT ON COLUMN uncertainty_tracking.uncertainty_level IS 'Critical=system blocked; Blocked=unresolvable; High=major questions; Medium=some questions; Investigating=actively researching; Low=minor doubts; Resolved=clarity achieved';
COMMENT ON COLUMN uncertainty_tracking.escalation_required IS 'Whether human intervention is needed';

-- ============================================================================
-- Table 6: production_code_tracking
-- Purpose: Track production code existence for guardrail validation
-- ============================================================================
CREATE TYPE code_type_enum AS ENUM ('production', 'test', 'infrastructure');

CREATE TABLE IF NOT EXISTS production_code_tracking (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(255) NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    code_type code_type_enum NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    verified_at TIMESTAMP WITH TIME ZONE,

    -- Constraints
    CONSTRAINT unique_session_file UNIQUE (session_id, file_path)
);

-- Indexes for production_code_tracking
CREATE INDEX IF NOT EXISTS idx_production_code_session_id ON production_code_tracking(session_id);
CREATE INDEX IF NOT EXISTS idx_production_code_session_id_type ON production_code_tracking(session_id, code_type);
CREATE INDEX IF NOT EXISTS idx_production_code_created_at ON production_code_tracking(created_at);

-- Comments
COMMENT ON TABLE production_code_tracking IS 'Tracks production code existence for guardrail validation';
COMMENT ON COLUMN production_code_tracking.code_type IS 'production=main code, test=test files, infrastructure=config/build';
COMMENT ON COLUMN production_code_tracking.verified_at IS 'When the code was verified as still existing';

-- ============================================================================
-- Partition management function
-- ============================================================================
CREATE OR REPLACE FUNCTION create_failure_registry_partition()
RETURNS void AS $$
DECLARE
    partition_date DATE;
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    partition_date := DATE_TRUNC('month', NOW());
    partition_name := 'failure_registry_y' || TO_CHAR(partition_date, 'YYYY') || 'm' || TO_CHAR(partition_date, 'MM');
    start_date := partition_date;
    end_date := partition_date + INTERVAL '1 month';

    EXECUTE format('CREATE TABLE IF NOT EXISTS %I PARTITION OF failure_registry FOR VALUES FROM (%L) TO (%L)',
                   partition_name, start_date, end_date);
END;
$$ LANGUAGE plpgsql;

-- Create next month's partition proactively
SELECT create_failure_registry_partition();

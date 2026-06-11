-- Migration: Add uncertainty tracking table
-- Description: Tracks uncertainty levels and decision-making context

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS uncertainty_tracking (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES session_metadata(session_id) ON DELETE CASCADE,
    task_id UUID REFERENCES tasks(task_id) ON DELETE SET NULL,
    uncertainty_level VARCHAR(50) NOT NULL,
    decision_made TEXT,
    context_data JSONB,
    escalation_required BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Indexes for performance
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

-- Indexes for query performance
CREATE INDEX idx_uncertainty_session_id ON uncertainty_tracking(session_id);
CREATE INDEX idx_uncertainty_task_id ON uncertainty_tracking(task_id);
CREATE INDEX idx_uncertainty_level ON uncertainty_tracking(uncertainty_level);
CREATE INDEX idx_uncertainty_escalation ON uncertainty_tracking(escalation_required) WHERE escalation_required = true;
CREATE INDEX idx_uncertainty_created_at ON uncertainty_tracking(created_at DESC);

-- Partitioning setup (for future scalability with large data volumes)
-- ALTER TABLE uncertainty_tracking SET (timescaledb.hypertable, partition_column = 'created_at');

-- Add comment documentation
COMMENT ON TABLE uncertainty_tracking IS 'Tracks uncertainty levels during MCP operations and decision-making context';
COMMENT ON COLUMN uncertainty_tracking.uncertainty_level IS 'Critical=system blocked; Blocked=unresolvable; High=major questions; Medium=some questions; Investigating=actively researching; Low=minor doubts; Resolved=clarity achieved';
COMMENT ON COLUMN uncertainty_tracking.escalation_required IS 'Whether human intervention is needed';

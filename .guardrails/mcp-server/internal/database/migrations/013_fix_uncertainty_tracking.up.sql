-- Fix uncertainty_tracking table - remove FK constraints to non-existent tables
-- The original migration referenced session_metadata(session_id) and tasks(task_id)
-- which don't exist in the schema.

-- Drop the table if it exists (to remove FK constraints)
DROP TABLE IF EXISTS uncertainty_tracking;

-- Drop the enum type if it exists
DROP TYPE IF EXISTS uncertainty_level_enum;

-- Create enum type for uncertainty levels
CREATE TYPE uncertainty_level_enum AS ENUM (
    'critical',
    'blocked',
    'high',
    'medium',
    'investigating',
    'low',
    'resolved'
);

-- Create uncertainty_tracking table WITHOUT FK constraints
CREATE TABLE uncertainty_tracking (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    task_id VARCHAR(100),
    uncertainty_level uncertainty_level_enum NOT NULL DEFAULT 'medium',
    decision_made TEXT,
    context_data JSONB DEFAULT '{}',
    escalation_required BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_uncertainty_session_id ON uncertainty_tracking(session_id);
CREATE INDEX idx_uncertainty_task_id ON uncertainty_tracking(task_id);
CREATE INDEX idx_uncertainty_level ON uncertainty_tracking(uncertainty_level);
CREATE INDEX idx_uncertainty_escalation ON uncertainty_tracking(escalation_required) WHERE escalation_required = true;
CREATE INDEX idx_uncertainty_created_at ON uncertainty_tracking(created_at DESC);

-- Add comments for documentation
COMMENT ON TABLE uncertainty_tracking IS 'Tracks uncertainty levels during MCP operations and decision-making context';
COMMENT ON COLUMN uncertainty_tracking.uncertainty_level IS 'Critical=system blocked; Blocked=unresolvable; High=major questions; Medium=some questions; Investigating=actively researching; Low=minor doubts; Resolved=clarity achieved';
COMMENT ON COLUMN uncertainty_tracking.escalation_required IS 'Whether human intervention is needed';

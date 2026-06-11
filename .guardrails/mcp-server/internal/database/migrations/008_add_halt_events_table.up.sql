-- Add halt_events table for halt conditions enforcement

CREATE TABLE IF NOT EXISTS halt_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(100) NOT NULL,
    halt_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    description TEXT,
    triggered_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    acknowledged BOOLEAN NOT NULL DEFAULT false,
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    resolution VARCHAR(50) DEFAULT 'pending',
    context_data JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_halt_type CHECK (
        halt_type IN ('code_safety', 'scope', 'environment', 'execution', 'security', 'uncertainty')
    ),
    CONSTRAINT valid_severity CHECK (
        severity IN ('low', 'medium', 'high', 'critical')
    ),
    CONSTRAINT valid_resolution CHECK (
        resolution IN ('pending', 'resolved', 'escalated', 'dismissed')
    )
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_halt_events_session_id ON halt_events(session_id);
CREATE INDEX IF NOT EXISTS idx_halt_events_halt_type ON halt_events(halt_type);
CREATE INDEX IF NOT EXISTS idx_halt_events_severity ON halt_events(severity);
CREATE INDEX IF NOT EXISTS idx_halt_events_triggered_at ON halt_events(triggered_at DESC);
CREATE INDEX IF NOT EXISTS idx_halt_events_acknowledged ON halt_events(acknowledged);

-- Partial index for unacknowledged events (most queried)
CREATE INDEX IF NOT EXISTS idx_halt_events_unacknowledged ON halt_events(session_id, severity)
    WHERE acknowledged = false;

-- Partial index for critical unresolved events
CREATE INDEX IF NOT EXISTS idx_halt_events_critical_unresolved ON halt_events(session_id, triggered_at)
    WHERE severity = 'critical' AND resolution = 'pending';

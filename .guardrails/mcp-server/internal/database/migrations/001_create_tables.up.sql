-- Initial schema creation

-- Documents table
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(255) UNIQUE NOT NULL,
    title VARCHAR(500) NOT NULL,
    content TEXT NOT NULL,
    search_vector tsvector,
    category VARCHAR(50) NOT NULL CHECK (category IN ('workflow', 'standard', 'guide', 'reference')),
    path VARCHAR(500) NOT NULL,
    version INTEGER DEFAULT 1,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Prevention rules table
CREATE TABLE IF NOT EXISTS prevention_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id VARCHAR(50) UNIQUE NOT NULL CHECK (LENGTH(TRIM(rule_id)) > 0),
    name VARCHAR(255) NOT NULL,
    pattern TEXT NOT NULL,
    pattern_hash VARCHAR(64),
    message TEXT NOT NULL,
    severity VARCHAR(10) NOT NULL CHECK (severity IN ('error', 'warning', 'info')),
    enabled BOOLEAN NOT NULL DEFAULT true,
    document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    category VARCHAR(50),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Failure registry table (partitioned by time)
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
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- Projects table
CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL CHECK (LENGTH(TRIM(slug)) > 0),
    guardrail_context TEXT,
    active_rules VARCHAR(50)[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Audit log table (partitioned by time)
CREATE TABLE IF NOT EXISTS audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id VARCHAR(50) NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    event_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    actor VARCHAR(64) NOT NULL,
    action VARCHAR(50) NOT NULL,
    resource VARCHAR(255),
    status VARCHAR(20) NOT NULL,
    details JSONB DEFAULT '{}',
    client_ip INET,
    request_id VARCHAR(50),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (timestamp);

-- Create initial partition for current month
CREATE TABLE IF NOT EXISTS failure_registry_y2026m02 PARTITION OF failure_registry
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

CREATE TABLE IF NOT EXISTS audit_log_y2026m02 PARTITION OF audit_log
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

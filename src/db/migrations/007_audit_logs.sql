-- Audit logs table for comprehensive event tracking

CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    actor_identity_id UUID REFERENCES identities(id) ON DELETE SET NULL,
    delegation_chain JSONB,

    -- Event details
    event_type VARCHAR(100) NOT NULL,
    action VARCHAR(255) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id VARCHAR(255),

    -- Decision (for authorization events)
    decision VARCHAR(50),
    decision_reason TEXT,

    -- Context
    request_id UUID,
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Tamper-proofing
    signature VARCHAR(255),
    previous_event_hash VARCHAR(64),

    CONSTRAINT valid_decision CHECK (decision IS NULL OR decision IN ('allow', 'deny'))
);

CREATE INDEX idx_audit_tenant_time ON audit_logs(tenant_id, timestamp DESC);
CREATE INDEX idx_audit_actor ON audit_logs(actor_identity_id) WHERE actor_identity_id IS NOT NULL;
CREATE INDEX idx_audit_event_type ON audit_logs(event_type);
CREATE INDEX idx_audit_request ON audit_logs(request_id) WHERE request_id IS NOT NULL;
CREATE INDEX idx_audit_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_decision ON audit_logs(decision) WHERE decision IS NOT NULL;
CREATE INDEX idx_audit_timestamp ON audit_logs(timestamp DESC);

-- Partitioning by month for better performance (optional)
-- This can be set up later for production deployments
-- CREATE TABLE audit_logs_2026_02 PARTITION OF audit_logs
--     FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

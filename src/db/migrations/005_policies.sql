-- Cedar policies table

CREATE TABLE policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    policy_cedar TEXT NOT NULL,
    resource_type VARCHAR(100),
    priority INTEGER NOT NULL DEFAULT 0,
    effect VARCHAR(10) NOT NULL DEFAULT 'allow',
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_effect CHECK (effect IN ('allow', 'deny')),
    CONSTRAINT valid_policy_status CHECK (status IN ('active', 'inactive', 'deleted'))
);

CREATE INDEX idx_policies_tenant ON policies(tenant_id);
CREATE INDEX idx_policies_status ON policies(status);
CREATE INDEX idx_policies_resource_type ON policies(resource_type) WHERE resource_type IS NOT NULL;
CREATE INDEX idx_policies_tenant_status ON policies(tenant_id, status);

CREATE TRIGGER update_policies_updated_at BEFORE UPDATE ON policies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

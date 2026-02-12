-- Identities table (Users, Services, Agents)

CREATE TABLE identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    identity_type VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'active',

    -- For agents: delegation tracking
    parent_identity_id UUID REFERENCES identities(id) ON DELETE CASCADE,
    task_id VARCHAR(255),
    task_scope JSONB,
    expires_at TIMESTAMPTZ,

    -- Authentication credentials
    password_hash VARCHAR(255),
    api_key_hash VARCHAR(255),

    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,

    CONSTRAINT valid_identity_type CHECK (identity_type IN ('user', 'service', 'agent')),
    CONSTRAINT valid_identity_status CHECK (status IN ('active', 'suspended', 'deleted')),
    CONSTRAINT user_must_have_email CHECK (
        identity_type != 'user' OR email IS NOT NULL
    ),
    CONSTRAINT agent_must_have_parent CHECK (
        identity_type != 'agent' OR parent_identity_id IS NOT NULL
    )
);

CREATE INDEX idx_identities_tenant ON identities(tenant_id);
CREATE INDEX idx_identities_type ON identities(identity_type);
CREATE INDEX idx_identities_email ON identities(email) WHERE email IS NOT NULL;
CREATE INDEX idx_identities_parent ON identities(parent_identity_id) WHERE parent_identity_id IS NOT NULL;
CREATE INDEX idx_identities_expires ON identities(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_identities_tenant_type ON identities(tenant_id, identity_type);

CREATE TRIGGER update_identities_updated_at BEFORE UPDATE ON identities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

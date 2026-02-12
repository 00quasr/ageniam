-- Roles table for RBAC

CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    parent_role_id UUID REFERENCES roles(id) ON DELETE SET NULL,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_role_per_tenant UNIQUE (tenant_id, name)
);

CREATE INDEX idx_roles_tenant ON roles(tenant_id);
CREATE INDEX idx_roles_parent ON roles(parent_role_id) WHERE parent_role_id IS NOT NULL;

-- Permissions table

CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    resource_type VARCHAR(100) NOT NULL,
    action VARCHAR(100) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_resource_action UNIQUE (resource_type, action)
);

CREATE INDEX idx_permissions_resource_type ON permissions(resource_type);

-- Role-Permission mapping

CREATE TABLE role_permissions (
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    constraints JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id)
);

CREATE INDEX idx_role_permissions_role ON role_permissions(role_id);
CREATE INDEX idx_role_permissions_permission ON role_permissions(permission_id);

-- Identity-Role assignments

CREATE TABLE identity_roles (
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    valid_from TIMESTAMPTZ,
    valid_until TIMESTAMPTZ,
    granted_by UUID REFERENCES identities(id),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (identity_id, role_id)
);

CREATE INDEX idx_identity_roles_identity ON identity_roles(identity_id);
CREATE INDEX idx_identity_roles_role ON identity_roles(role_id);
CREATE INDEX idx_identity_roles_valid ON identity_roles(identity_id, role_id, valid_from, valid_until);

-- Insert default permissions

INSERT INTO permissions (name, resource_type, action, description) VALUES
    ('agent:create', 'agent', 'create', 'Create new agent identities'),
    ('agent:read', 'agent', 'read', 'Read agent information'),
    ('agent:update', 'agent', 'update', 'Update agent information'),
    ('agent:delete', 'agent', 'delete', 'Delete agent identities'),
    ('agent:execute', 'agent', 'execute', 'Execute agent tasks'),
    ('task:create', 'task', 'create', 'Create new tasks'),
    ('task:read', 'task', 'read', 'Read task information'),
    ('task:execute', 'task', 'execute', 'Execute tasks'),
    ('task:cancel', 'task', 'cancel', 'Cancel running tasks'),
    ('secret:create', 'secret', 'create', 'Create secrets'),
    ('secret:read', 'secret', 'read', 'Read secrets'),
    ('secret:update', 'secret', 'update', 'Update secrets'),
    ('secret:delete', 'secret', 'delete', 'Delete secrets'),
    ('policy:create', 'policy', 'create', 'Create policies'),
    ('policy:read', 'policy', 'read', 'Read policies'),
    ('policy:update', 'policy', 'update', 'Update policies'),
    ('policy:delete', 'policy', 'delete', 'Delete policies');

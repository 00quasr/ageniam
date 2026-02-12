-- Rate limits table for limit definitions

CREATE TABLE rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    target_type VARCHAR(50) NOT NULL,
    target_id UUID NOT NULL,
    limit_type VARCHAR(100) NOT NULL,
    max_count INTEGER NOT NULL,
    window_seconds INTEGER NOT NULL,
    resource_type VARCHAR(100),
    action VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_target_type CHECK (target_type IN ('tenant', 'identity', 'role')),
    CONSTRAINT positive_max_count CHECK (max_count > 0),
    CONSTRAINT positive_window CHECK (window_seconds > 0)
);

CREATE INDEX idx_rate_limits_tenant ON rate_limits(tenant_id);
CREATE INDEX idx_rate_limits_target ON rate_limits(target_type, target_id);
CREATE INDEX idx_rate_limits_resource ON rate_limits(resource_type, action);

-- Helper function to get applicable rate limits for an identity
CREATE OR REPLACE FUNCTION get_rate_limits_for_identity(
    p_identity_id UUID,
    p_tenant_id UUID,
    p_resource_type VARCHAR DEFAULT NULL,
    p_action VARCHAR DEFAULT NULL
)
RETURNS TABLE (
    limit_type VARCHAR,
    max_count INTEGER,
    window_seconds INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        rl.limit_type,
        rl.max_count,
        rl.window_seconds
    FROM rate_limits rl
    WHERE
        (
            -- Tenant-level limits
            (rl.target_type = 'tenant' AND rl.target_id = p_tenant_id)
            OR
            -- Identity-level limits
            (rl.target_type = 'identity' AND rl.target_id = p_identity_id)
            OR
            -- Role-level limits
            (rl.target_type = 'role' AND rl.target_id IN (
                SELECT role_id FROM identity_roles
                WHERE identity_id = p_identity_id
                AND (valid_from IS NULL OR valid_from <= NOW())
                AND (valid_until IS NULL OR valid_until >= NOW())
            ))
        )
        AND (p_resource_type IS NULL OR rl.resource_type IS NULL OR rl.resource_type = p_resource_type)
        AND (p_action IS NULL OR rl.action IS NULL OR rl.action = p_action)
    ORDER BY
        -- More specific limits take precedence
        CASE
            WHEN rl.target_type = 'identity' THEN 1
            WHEN rl.target_type = 'role' THEN 2
            WHEN rl.target_type = 'tenant' THEN 3
        END,
        rl.max_count ASC;
END;
$$ LANGUAGE plpgsql;

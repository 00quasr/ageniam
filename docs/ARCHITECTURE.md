# Agent IAM Architecture

## Overview

Agent IAM is designed as a foundational identity and access management layer for AI agents. This document describes the system architecture, design decisions, and implementation details.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Client Layer                          │
│  (Users, Services, Agents via HTTP/JSON)                   │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    API Layer (Axum)                         │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐  │
│  │  Auth    │Identity  │  Authz   │ Policies │  Audit   │  │
│  │Endpoints │Endpoints │Endpoints │Endpoints │Endpoints │  │
│  └──────────┴──────────┴──────────┴──────────┴──────────┘  │
└────────────────────────┬────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ Middleware  │  │ Middleware  │  │ Middleware  │
│   Layer     │  │   Layer     │  │   Layer     │
├─────────────┤  ├─────────────┤  ├─────────────┤
│- Auth       │  │- Rate Limit │  │- Tracing    │
│- Authz      │  │- CORS       │  │- Metrics    │
│- Audit Log  │  │- Compression│  │             │
└─────────────┘  └─────────────┘  └─────────────┘
         │               │               │
         └───────────────┼───────────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Business   │  │  Business   │  │  Business   │
│   Logic     │  │   Logic     │  │   Logic     │
├─────────────┤  ├─────────────┤  ├─────────────┤
│- Identity   │  │- Session    │  │- Cedar      │
│  Manager    │  │  Manager    │  │  Engine     │
│- JIT Agent  │  │- Token Gen  │  │- Policy     │
│  Provision  │  │- Token Val  │  │  Eval       │
└─────────────┘  └─────────────┘  └─────────────┘
         │               │               │
         └───────────────┼───────────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ PostgreSQL  │  │    Redis    │  │     S3      │
│             │  │             │  │  (Future)   │
├─────────────┤  ├─────────────┤  ├─────────────┤
│- Identities │  │- Revocation │  │- Audit Logs │
│- Sessions   │  │- Rate Limits│  │  (Archive)  │
│- Policies   │  │- Counters   │  │             │
│- Audit Logs │  │             │  │             │
└─────────────┘  └─────────────┘  └─────────────┘
```

## Technology Stack

### Core Framework
- **Language**: Rust 1.75+
- **Web Framework**: Axum 0.7 (built on Tokio, Tower)
- **Async Runtime**: Tokio 1.36

### Data Storage
- **Primary Database**: PostgreSQL 14+ with SQLx
- **Cache/State**: Redis 7+
- **Object Storage**: S3-compatible (future)

### Security
- **User Tokens**: JWT with RS256
- **Agent Tokens**: Biscuit with Ed25519
- **Policy Engine**: Cedar
- **Password Hash**: Argon2id
- **TLS**: rustls (TLS 1.3)

### Observability
- **Logging**: tracing + tracing-subscriber
- **Metrics**: Prometheus
- **Tracing**: OpenTelemetry (planned)

## Core Components

### 1. Identity Manager

**Responsibility**: Manage all identity types (Users, Services, Agents)

**Key Features**:
- Multi-tenant isolation
- JIT agent provisioning
- Delegation chain tracking
- Identity lifecycle management

**Database Tables**:
- `identities` - All identity records
- `tenants` - Multi-tenant isolation

### 2. Session Manager

**Responsibility**: Token lifecycle and session management

**Key Features**:
- JWT generation for users
- Biscuit generation for agents
- Token validation and parsing
- Session revocation
- Refresh token rotation

**Database Tables**:
- `sessions` - Active sessions

**Redis Keys**:
- `revoked:<token_id>` - Revocation list

### 3. Authorization Engine

**Responsibility**: Policy-based access control

**Key Features**:
- Cedar policy evaluation
- Sub-millisecond decisions
- ABAC support (time, tenant, resource attributes)
- Policy caching

**Database Tables**:
- `policies` - Cedar policy storage
- `roles` - RBAC roles
- `permissions` - Permission catalog
- `role_permissions` - Role-permission mappings
- `identity_roles` - Identity-role assignments

### 4. Rate Limiter

**Responsibility**: Prevent abuse and resource exhaustion

**Key Features**:
- Sliding window algorithm
- Multi-dimensional limits (per-tenant, per-identity, per-resource)
- Redis-backed counters
- Graceful degradation

**Database Tables**:
- `rate_limits` - Limit definitions

**Redis Keys**:
- `ratelimit:<key>` - Sorted sets for sliding window

### 5. Audit Logger

**Responsibility**: Comprehensive event logging

**Key Features**:
- Async logging (non-blocking)
- Tamper-proof (hash chains, signatures)
- Multi-storage backend
- Query API

**Database Tables**:
- `audit_logs` - Event records

## Request Flow

### Authentication Flow (User Login)

```
1. POST /v1/auth/login
   ├─ Extract credentials
   ├─ Validate password (Argon2id)
   ├─ Check account status
   ├─ Check rate limit
   ├─ Generate JWT (RS256)
   ├─ Generate refresh token
   ├─ Store session
   ├─ Log auth event
   └─ Return tokens
```

### JIT Agent Provisioning Flow

```
1. POST /v1/identities (with parent auth)
   ├─ Validate parent token
   ├─ Check parent permissions
   ├─ Create agent identity
   │  ├─ Set parent_identity_id
   │  ├─ Set task_id
   │  ├─ Set expires_at
   │  └─ Set delegation_depth
   ├─ Generate Biscuit token
   │  ├─ Add authority block
   │  ├─ Add attenuation constraints
   │  └─ Sign with root key
   ├─ Store session
   ├─ Log identity creation
   └─ Return agent identity + token
```

### Authorization Check Flow

```
1. POST /v1/authz/check
   ├─ Extract token from header
   ├─ Validate token signature
   ├─ Check revocation (Redis)
   ├─ Check rate limit (Redis)
   ├─ Load Cedar policies
   ├─ Build evaluation context
   │  ├─ Principal (identity)
   │  ├─ Action
   │  ├─ Resource
   │  └─ Context (time, IP, etc.)
   ├─ Evaluate policies (Cedar)
   ├─ Log decision (async)
   └─ Return allow/deny + reason
```

## Database Schema Design

### Multi-Tenancy

All tenant-scoped tables include `tenant_id`:

```sql
tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE
```

Row-level security can be added later:

```sql
ALTER TABLE identities ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON identities
    USING (tenant_id = current_setting('app.current_tenant')::uuid);
```

### Delegation Chains

Agents reference their parent:

```sql
parent_identity_id UUID REFERENCES identities(id) ON DELETE CASCADE
```

Recursive query to get full chain:

```sql
WITH RECURSIVE chain AS (
    SELECT id, parent_identity_id, 0 as depth
    FROM identities WHERE id = $1
    UNION ALL
    SELECT i.id, i.parent_identity_id, c.depth + 1
    FROM identities i JOIN chain c ON i.id = c.parent_identity_id
)
SELECT * FROM chain;
```

### Audit Trail Integrity

Hash chain to detect tampering:

```sql
previous_event_hash VARCHAR(64)  -- SHA-256 of previous event
signature VARCHAR(255)            -- Ed25519 signature
```

Verification:

```python
def verify_audit_chain(events):
    for i, event in enumerate(events):
        if i > 0:
            expected = sha256(events[i-1])
            if event.previous_event_hash != expected:
                return False
        if not verify_signature(event):
            return False
    return True
```

## Security Considerations

### Token Security

**JWT (Users)**:
- RS256 (asymmetric) - public key distributed via JWKS
- Short-lived (15 min)
- Refresh token rotation (one-time use)
- Revocation via Redis

**Biscuit (Agents)**:
- Ed25519 signatures
- Native attenuation (child tokens cannot escalate)
- Task-scoped lifetime
- Offline verification possible

### Password Security

- Argon2id (OWASP recommended)
- Parameters: m=19456 KiB, t=2, p=1
- Account lockout: 5 failures, 15 min lockout
- Leaked password check (HaveIBeenPwned API - planned)

### Rate Limiting

- Authentication: 10 attempts/min per IP
- API requests: Per-tenant, per-identity limits
- Sliding window (more accurate than fixed window)

### TLS

- TLS 1.3 only
- rustls (memory-safe alternative to OpenSSL)
- HSTS headers
- mTLS for service-to-service (planned)

## Performance Targets

- Authorization decision: <10ms p99
- Token validation: <2ms p99
- Rate limit check: <1ms p99
- Audit log write: <50ms p99 (async)

## Deployment

### Single Binary

```bash
cargo build --release
./target/release/agent-iam
```

### Docker

```bash
docker build -t agent-iam .
docker run -p 8080:8080 agent-iam
```

### Docker Compose (Dev)

```bash
docker-compose up -d
```

### Health Checks

- `/health/live` - Always returns 200 if service is running
- `/health/ready` - Returns 200 if dependencies are healthy
- `/health/startup` - One-time check during initialization

## Monitoring

### Metrics

Prometheus metrics at `/metrics`:

```
http_requests_total{method,path,status}
http_request_duration_seconds{method,path}
authz_requests_total{decision,resource_type}
authz_latency_seconds{decision}
authz_policy_evaluation_errors_total{error_type}
active_sessions
rate_limit_exceeded_total{tenant_id,limit_type}
```

### Logging

Structured JSON logs:

```json
{
  "timestamp": "2026-02-12T15:00:00Z",
  "level": "INFO",
  "message": "Authorization decision",
  "fields": {
    "decision": "allow",
    "identity_id": "...",
    "resource_type": "task",
    "latency_ms": 2.3
  }
}
```

## Future Enhancements

1. **Agent Vault Integration**: Secrets, spend controls, ZK credentials
2. **Multi-region**: Active-active replication
3. **Audit Archive**: S3 + Parquet for long-term storage
4. **Advanced Policies**: ML-based anomaly detection
5. **WebAuthn/FIDO2**: Passwordless authentication
6. **Blockchain Audit**: Immutable on-chain audit trail

## References

- [Cedar Policy Language](https://github.com/cedar-policy/cedar)
- [Biscuit Tokens](https://github.com/biscuit-auth/biscuit)
- [Axum Framework](https://github.com/tokio-rs/axum)
- [SQLx](https://github.com/launchbadge/sqlx)

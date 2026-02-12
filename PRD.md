# Agent IAM - Product Requirements Document

## Project Vision

Build a production-ready identity and access management system specifically designed for AI agents, providing JIT provisioning, delegation chains, policy-based authorization, and comprehensive audit logging.

## Timeline

- **Total Duration**: 12 weeks
- **Start Date**: 2026-02-12
- **Target Completion**: 2026-05-06

## Task List (100 Tasks)

### Phase 1: Foundation (Weeks 1-2) ✅ COMPLETED

#### Infrastructure Setup (Tasks 1-10)

- [x] 1. Create Rust workspace with Cargo.toml and dependencies
- [x] 2. Set up project directory structure
- [x] 3. Create configuration system (TOML + env vars)
- [x] 4. Implement error types and Result alias
- [x] 5. Create .env.example and .gitignore
- [x] 6. Set up tracing/logging infrastructure
- [x] 7. Create Prometheus metrics registry
- [x] 8. Implement health check system
- [x] 9. Create Docker Compose setup (Postgres + Redis)
- [x] 10. Create multi-stage Dockerfile

#### Database Layer (Tasks 11-19)

- [x] 11. Create database connection pool with SQLx
- [x] 12. Implement migration runner
- [x] 13. Create tenants table migration
- [x] 14. Create identities table migration
- [x] 15. Create roles and permissions tables migration
- [x] 16. Create policies table migration
- [x] 17. Create sessions table migration
- [x] 18. Create audit_logs table migration
- [x] 19. Create rate_limits table migration

### Phase 2: Authentication (Weeks 3-4)

#### Password System (Tasks 20-25)

- [x] 20. Implement Argon2id password hashing with OWASP parameters
- [x] 21. Implement password verification with constant-time comparison
- [x] 22. Create password strength validator
- [ ] 23. Implement password policy enforcement (length, complexity)
- [ ] 24. Add password change functionality
- [ ] 25. Write unit tests for password hashing

#### JWT System (Tasks 26-35)

- [ ] 26. Generate RSA key pair for JWT signing (RS256)
- [ ] 27. Implement JWT token generation with claims
- [ ] 28. Implement JWT token validation and parsing
- [ ] 29. Create JWKS endpoint for public key distribution
- [ ] 30. Implement JWT key rotation logic
- [ ] 31. Create refresh token generation
- [ ] 32. Implement refresh token validation
- [ ] 33. Add token revocation to Redis
- [ ] 34. Create authentication middleware (extract token from header)
- [ ] 35. Write integration tests for JWT flow

#### Authentication Endpoints (Tasks 36-43)

- [ ] 36. Implement POST /v1/auth/login endpoint
- [ ] 37. Add rate limiting to login endpoint (10 attempts/min per IP)
- [ ] 38. Implement account lockout logic (5 failures, 15 min)
- [ ] 39. Implement POST /v1/auth/logout endpoint
- [ ] 40. Implement POST /v1/auth/refresh endpoint
- [ ] 41. Implement POST /v1/auth/token (service account auth)
- [ ] 42. Add comprehensive logging to auth endpoints
- [ ] 43. Write integration tests for all auth endpoints

### Phase 3: Identity Management (Weeks 5-6)

#### Identity Domain (Tasks 44-52)

- [ ] 44. Create Identity domain model with validation
- [ ] 45. Implement identity creation logic with multi-tenant isolation
- [ ] 46. Create database queries for identity CRUD
- [ ] 47. Implement JIT agent provisioning logic
- [ ] 48. Add delegation chain tracking
- [ ] 49. Implement identity expiration logic
- [ ] 50. Create background job for expired identity cleanup
- [ ] 51. Add identity status transitions (active, suspended, deleted)
- [ ] 52. Write unit tests for identity domain logic

#### Identity Endpoints (Tasks 53-60)

- [ ] 53. Implement POST /v1/identities (create identity)
- [ ] 54. Implement GET /v1/identities/:id (get identity)
- [ ] 55. Implement PATCH /v1/identities/:id (update identity)
- [ ] 56. Implement DELETE /v1/identities/:id (soft delete)
- [ ] 57. Implement GET /v1/identities (list with filters)
- [ ] 58. Add pagination to identity listing
- [ ] 59. Implement GET /v1/identities/:id/delegation-chain
- [ ] 60. Write integration tests for identity endpoints

#### Role & Permission System (Tasks 61-65)

- [ ] 61. Create role domain model
- [ ] 62. Implement role assignment to identities
- [ ] 63. Create database queries for role operations
- [ ] 64. Implement role hierarchy resolution
- [ ] 65. Add role-based permission resolution

### Phase 4: Authorization (Weeks 7-8)

#### Cedar Integration (Tasks 66-73)

- [ ] 66. Initialize Cedar policy engine
- [ ] 67. Create Cedar entity types (Principal, Action, Resource)
- [ ] 68. Implement policy loading from database
- [ ] 69. Create policy validation logic
- [ ] 70. Implement policy evaluation with context
- [ ] 71. Add policy decision caching (LRU)
- [ ] 72. Create policy compilation and hot-reload
- [ ] 73. Write unit tests for Cedar integration

#### Authorization Endpoints (Tasks 74-78)

- [ ] 74. Implement POST /v1/authz/check endpoint
- [ ] 75. Implement POST /v1/authz/bulk-check (batch authorization)
- [ ] 76. Create authorization middleware for protecting endpoints
- [ ] 77. Add detailed decision reasoning to responses
- [ ] 78. Write integration tests for authz endpoints

#### Biscuit Tokens (Tasks 79-82)

- [ ] 79. Generate Ed25519 key pair for Biscuit signing
- [ ] 80. Implement Biscuit token generation for agents
- [ ] 81. Implement Biscuit token validation
- [ ] 82. Implement token attenuation (child token constraints)

#### Policy Management (Tasks 83-85)

- [ ] 83. Implement POST /v1/policies (create policy)
- [ ] 84. Implement GET /v1/policies (list policies)
- [ ] 85. Implement PUT /v1/policies/:id and DELETE /v1/policies/:id

### Phase 5: Rate Limiting & Audit (Weeks 9-10)

#### Rate Limiting (Tasks 86-90)

- [ ] 86. Implement sliding window rate limiter with Redis
- [ ] 87. Create rate limit configuration loading
- [ ] 88. Implement multi-dimensional rate limiting (tenant, identity, resource)
- [ ] 89. Create rate limiting middleware
- [ ] 90. Add rate limit exceeded metrics and logging

#### Audit Logging (Tasks 91-96)

- [ ] 91. Create audit event types and domain model
- [ ] 92. Implement async audit logger with batching
- [ ] 93. Add hash chain implementation for tamper-proofing
- [ ] 94. Implement Ed25519 signature for audit logs
- [ ] 95. Create audit log query API (GET /v1/audit/logs)
- [ ] 96. Add S3 storage backend for audit log archival

### Phase 6: Production Hardening (Weeks 11-12)

#### Testing & Quality (Tasks 97-100)

- [ ] 97. Write comprehensive integration tests for all flows
- [ ] 98. Perform security audit and penetration testing
- [ ] 99. Create deployment documentation and runbooks
- [ ] 100. Conduct performance testing and optimization (load testing with 1000 concurrent agents)

---

## Detailed Task Descriptions

### Task 20: Implement Argon2id Password Hashing

**File**: `src/auth/password.rs`

**Requirements**:
- Use `argon2` crate
- Parameters: m=19456 KiB, t=2, p=1
- Generate random salt (16 bytes)
- Return PHC string format

**Acceptance Criteria**:
- Hashes are non-deterministic (different salt each time)
- Verification works correctly
- Timing attack resistant

---

### Task 26: Generate RSA Key Pair for JWT

**File**: `src/crypto/keys.rs`

**Requirements**:
- Generate 2048-bit RSA key pair
- Store private key securely (env var or KMS)
- Expose public key via JWKS endpoint
- Support key rotation (multiple active keys)

**Acceptance Criteria**:
- Keys generated on first startup
- Private key never logged or exposed
- JWKS endpoint returns valid JSON

---

### Task 36: Implement POST /v1/auth/login

**File**: `src/api/auth.rs`

**Requirements**:
- Accept email + password + tenant_slug
- Validate credentials against database
- Check account status (not suspended/deleted)
- Generate JWT + refresh token
- Store session in database
- Log successful/failed login attempts

**Acceptance Criteria**:
- Returns 200 + tokens on success
- Returns 401 on invalid credentials
- Returns 429 if rate limited
- Audit log created for all attempts

---

### Task 44: Create Identity Domain Model

**File**: `src/domain/identity.rs`

**Requirements**:
- Define Identity struct with all fields
- Implement validation (email format, tenant exists)
- Add builder pattern for creating identities
- Implement business logic for state transitions

**Acceptance Criteria**:
- All fields validated on construction
- Unit tests for validation logic
- Clear error messages

---

### Task 66: Initialize Cedar Policy Engine

**File**: `src/authz/engine.rs`

**Requirements**:
- Create Cedar Authorizer instance
- Load entity types from schema
- Implement policy loading from database
- Add policy validation

**Acceptance Criteria**:
- Policies parse correctly
- Invalid policies rejected with clear errors
- Policies cached in memory

---

### Task 86: Implement Sliding Window Rate Limiter

**File**: `src/rate_limit/limiter.rs`

**Requirements**:
- Use Redis sorted sets for sliding window
- Atomic Lua script for check + increment
- Support multiple time windows (per minute, hour, day)
- Return remaining quota in response

**Acceptance Criteria**:
- Rate limits enforced accurately
- No race conditions
- Works across multiple service instances

---

### Task 91: Create Audit Event Types

**File**: `src/domain/audit.rs`

**Requirements**:
- Define event types (auth, authz, policy, identity)
- Include all required context (actor, resource, timestamp)
- Support delegation chain serialization

**Acceptance Criteria**:
- All security events covered
- JSON serialization works
- Events are immutable after creation

---

## Success Metrics

### Functional Requirements

- [ ] Users can authenticate and receive JWT tokens
- [ ] Agents can be provisioned JIT with Biscuit tokens
- [ ] Cedar policies correctly authorize/deny requests
- [ ] Rate limits enforced across all dimensions
- [ ] Audit logs capture all critical events
- [ ] Multi-tenant isolation prevents cross-tenant access
- [ ] Delegation chains correctly attenuate permissions

### Performance Requirements

- [ ] Authorization decisions <10ms p99
- [ ] Token validation <2ms p99
- [ ] Rate limit checks <1ms p99
- [ ] Audit log writes <50ms p99
- [ ] System handles 1000 concurrent agents
- [ ] System handles 10,000 requests/sec

### Security Requirements

- [ ] No SQL injection vulnerabilities
- [ ] Tokens cannot be forged or replayed
- [ ] Audit logs are tamper-proof
- [ ] Secrets encrypted at rest
- [ ] TLS 1.3 enforced in production
- [ ] Rate limiting prevents brute force
- [ ] All sensitive operations logged

### Operational Requirements

- [ ] Single binary deployment
- [ ] Database migrations automatic
- [ ] Health checks for all dependencies
- [ ] Prometheus metrics exported
- [ ] Structured JSON logs
- [ ] Graceful shutdown
- [ ] Docker Compose for dev
- [ ] systemd service for production

## Dependencies Between Tasks

### Critical Path

```
Foundation (1-19)
    ↓
Password Hashing (20-25)
    ↓
JWT System (26-35)
    ↓
Auth Endpoints (36-43)
    ↓
Identity Management (44-60)
    ↓
Roles & Permissions (61-65)
    ↓
Cedar Integration (66-73)
    ↓
Authorization Endpoints (74-78)
    ↓
Biscuit Tokens (79-82)
    ↓
Rate Limiting (86-90)
    ↓
Audit Logging (91-96)
    ↓
Production Hardening (97-100)
```

### Parallel Tracks

These can be implemented in parallel once dependencies are met:

- **Track A**: Auth (20-43)
- **Track B**: Identity (44-60) - requires Auth
- **Track C**: Authz (66-78) - requires Identity
- **Track D**: Biscuit (79-82) - can start after JWT (26-35)
- **Track E**: Rate Limit (86-90) - can start anytime
- **Track F**: Audit (91-96) - can start anytime
- **Track G**: Policy Mgmt (83-85) - requires Cedar (66-73)

## Non-Functional Requirements

### Scalability

- Stateless service (scale horizontally)
- Redis for shared state
- Database connection pooling
- Efficient policy caching

### Reliability

- Graceful error handling
- Circuit breakers for external dependencies
- Retry logic for transient failures
- Health checks for auto-recovery

### Maintainability

- Clear module boundaries
- Comprehensive logging
- Documentation for all public APIs
- Integration tests for critical flows

### Security

- Defense in depth
- Least privilege principle
- Audit all security events
- Regular security reviews

## Out of Scope (Future Work)

- WebAuthn/FIDO2 authentication
- OAuth2/OIDC provider
- SAML integration
- Multi-region replication
- Blockchain audit trail
- Machine learning anomaly detection
- GraphQL API
- Agent Vault integration (secrets, spend controls, ZK proofs)

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Cedar policy performance | High | Medium | Implement aggressive caching |
| Database bottleneck | High | Low | Use connection pooling, read replicas |
| Redis single point of failure | Medium | Medium | Redis Sentinel or cluster mode |
| Key compromise | Critical | Low | Key rotation, HSM integration |
| Audit log tampering | High | Low | Hash chains + digital signatures |

## Approval & Sign-off

- [ ] Architecture reviewed
- [ ] Security requirements approved
- [ ] Performance targets validated
- [ ] Timeline feasible
- [ ] Resources allocated

---

**Document Version**: 1.0
**Last Updated**: 2026-02-12
**Status**: In Progress (Tasks 1-19 completed)

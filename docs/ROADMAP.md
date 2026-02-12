# Agent IAM Roadmap

## Project Timeline

**Duration**: 12 weeks (2026-02-12 to 2026-05-06)

```
Week 1-2   Week 3-4      Week 5-6        Week 7-8       Week 9-10      Week 11-12
   ██         ░░░░          ░░░░            ░░░░           ░░░░           ░░░░
Foundation  Auth       Identity Mgmt   Authorization   Rate Limit &   Production
                                                         Audit          Hardening
  DONE      NEXT
```

## Milestones

### ✅ Milestone 1: Foundation (Weeks 1-2) - COMPLETED

**Status**: 19/19 tasks complete (100%)

**Deliverables**:
- [x] Rust project structure
- [x] Database schema (PostgreSQL)
- [x] Redis integration
- [x] Axum web server
- [x] Health & metrics endpoints
- [x] Docker setup
- [x] Observability infrastructure

**Key Achievements**:
- Service starts and connects to dependencies
- Health checks verify DB and Redis
- Metrics exported via Prometheus
- Complete database schema with 8 migrations
- Docker Compose for local development

---

### 🔜 Milestone 2: Authentication (Weeks 3-4) - NEXT

**Status**: 0/24 tasks complete (0%)

**Deliverables**:
- [ ] Password hashing (Argon2id)
- [ ] JWT token system (RS256)
- [ ] Authentication endpoints
- [ ] Session management
- [ ] Token revocation
- [ ] Rate limiting for auth

**Success Criteria**:
- Users can log in with email/password
- JWT tokens generated and validated
- Refresh token flow works
- Account lockout after failed attempts
- All auth events logged

**Dependencies**: None (can start immediately)

**Risk**: Medium - Critical security component

---

### Milestone 3: Identity Management (Weeks 5-6)

**Status**: 0/22 tasks complete (0%)

**Deliverables**:
- [ ] Identity CRUD operations
- [ ] JIT agent provisioning
- [ ] Delegation chain tracking
- [ ] Role & permission system
- [ ] Identity lifecycle management

**Success Criteria**:
- Agents can be created on-demand
- Parent-child relationships tracked
- Multi-tenant isolation enforced
- Role-based permissions work
- Expired agents auto-cleanup

**Dependencies**: Requires Milestone 2 (Authentication)

**Risk**: Medium - Complex delegation logic

---

### Milestone 4: Authorization (Weeks 7-8)

**Status**: 0/20 tasks complete (0%)

**Deliverables**:
- [ ] Cedar policy engine integration
- [ ] Authorization endpoints
- [ ] Biscuit token support
- [ ] Policy management API
- [ ] Authorization middleware

**Success Criteria**:
- Policy decisions <10ms p99
- Cedar policies correctly evaluated
- Biscuit tokens for agents work
- Token attenuation functions
- Authorization denials logged

**Dependencies**: Requires Milestone 3 (Identity Management)

**Risk**: High - New technology (Cedar), performance critical

---

### Milestone 5: Rate Limiting & Audit (Weeks 9-10)

**Status**: 0/11 tasks complete (0%)

**Deliverables**:
- [ ] Sliding window rate limiter
- [ ] Multi-dimensional limits
- [ ] Audit event system
- [ ] Tamper-proof logging
- [ ] Audit query API

**Success Criteria**:
- Rate limits enforced accurately
- No race conditions
- All security events logged
- Hash chain prevents tampering
- Audit logs queryable

**Dependencies**: Can start after Milestone 2

**Risk**: Low - Well-understood patterns

---

### Milestone 6: Production Hardening (Weeks 11-12)

**Status**: 0/4 tasks complete (0%)

**Deliverables**:
- [ ] Comprehensive tests
- [ ] Security audit
- [ ] Performance optimization
- [ ] Deployment documentation

**Success Criteria**:
- >80% test coverage
- No critical security issues
- Performance targets met
- Production-ready deployment

**Dependencies**: Requires all previous milestones

**Risk**: Low - Validation and polish

---

## Feature Rollout Plan

### Phase 1: MVP (End of Week 8)

**What's included**:
- User authentication (email/password)
- JWT tokens
- Basic identity management
- Cedar authorization
- Health checks & metrics

**What's NOT included**:
- Biscuit tokens (agents use JWT)
- Advanced rate limiting
- Audit logging
- Production hardening

**Target Users**: Internal testing

---

### Phase 2: Beta (End of Week 10)

**What's included**:
- Everything from MVP
- Biscuit tokens for agents
- JIT agent provisioning
- Rate limiting
- Basic audit logging

**What's NOT included**:
- Tamper-proof audit
- S3 archival
- Advanced policies
- Production deployment

**Target Users**: Early adopters

---

### Phase 3: GA (End of Week 12)

**What's included**:
- Everything from Beta
- Tamper-proof audit trail
- Comprehensive tests
- Security audit complete
- Production deployment ready

**Target Users**: General availability

---

## Technical Debt & Future Work

### Known Technical Debt

1. **No key rotation implemented yet**
   - Manual key rotation required
   - Will automate in v1.1

2. **Single Redis instance**
   - No HA setup
   - Plan: Redis Sentinel in v1.2

3. **Basic policy caching**
   - Simple in-memory LRU
   - Plan: Distributed cache in v2.0

4. **No multi-region support**
   - Single datacenter only
   - Plan: Active-active in v2.0

### Future Enhancements (Post-GA)

#### v1.1 (Q2 2026)
- Automated key rotation
- WebAuthn/FIDO2 support
- OAuth2/OIDC provider
- Advanced policy templates

#### v1.2 (Q3 2026)
- Redis HA (Sentinel)
- Database read replicas
- Horizontal scaling guide
- Kubernetes deployment

#### v2.0 (Q4 2026)
- Multi-region support
- Blockchain audit trail
- ML anomaly detection
- GraphQL API

#### v3.0 (2027)
- Agent Vault integration
- ZK credentials (Midnight)
- Spend controls
- Tool permissions

---

## Progress Tracking

### Completion by Phase

| Phase | Tasks | Complete | Progress |
|-------|-------|----------|----------|
| Phase 1: Foundation | 19 | 19 | ████████████████████ 100% |
| Phase 2: Authentication | 24 | 0 | ░░░░░░░░░░░░░░░░░░░░ 0% |
| Phase 3: Identity | 22 | 0 | ░░░░░░░░░░░░░░░░░░░░ 0% |
| Phase 4: Authorization | 20 | 0 | ░░░░░░░░░░░░░░░░░░░░ 0% |
| Phase 5: Rate/Audit | 11 | 0 | ░░░░░░░░░░░░░░░░░░░░ 0% |
| Phase 6: Production | 4 | 0 | ░░░░░░░░░░░░░░░░░░░░ 0% |
| **TOTAL** | **100** | **19** | ███░░░░░░░░░░░░░░░░░ 19% |

### Velocity Tracking

| Week | Tasks Planned | Tasks Completed | Notes |
|------|---------------|-----------------|-------|
| 1 | 10 | 10 | Setup and infrastructure |
| 2 | 9 | 9 | Database and observability |
| 3 | 12 | - | Password & JWT (planned) |
| 4 | 12 | - | Auth endpoints (planned) |
| 5-12 | 58 | - | Remaining features |

**Current Velocity**: 9.5 tasks/week
**Required Velocity**: 8.1 tasks/week
**Status**: ✅ On track

---

## Risk Register

| Risk | Impact | Likelihood | Mitigation | Owner |
|------|--------|------------|------------|-------|
| Cedar performance issues | High | Medium | Aggressive caching, benchmarking | Tech Lead |
| Biscuit learning curve | Medium | High | Early prototype, docs review | Security |
| Database bottleneck | High | Low | Connection pooling, indexing | Backend |
| Key compromise | Critical | Low | Key rotation, HSM integration | Security |
| Scope creep | Medium | Medium | Strict PRD adherence | PM |

---

## Success Metrics

### Engineering Metrics

- **Code Quality**:
  - [ ] 0 compiler warnings
  - [ ] 0 Clippy warnings
  - [ ] >80% test coverage
  - [ ] <10 critical issues (Sonar)

- **Performance**:
  - [ ] Authorization <10ms p99
  - [ ] Token validation <2ms p99
  - [ ] Rate limit check <1ms p99
  - [ ] 1000 concurrent agents supported

- **Security**:
  - [ ] 0 critical vulnerabilities
  - [ ] All OWASP Top 10 addressed
  - [ ] Security audit passed
  - [ ] Penetration test passed

### Product Metrics (Post-GA)

- **Adoption**:
  - Target: 100 agents provisioned/day
  - Target: 1M auth requests/day
  - Target: 99.9% uptime

- **Developer Experience**:
  - Target: <5 min to first auth
  - Target: Complete docs
  - Target: Active community

---

## Decision Log

### Key Architectural Decisions

| Date | Decision | Rationale | Alternatives Considered |
|------|----------|-----------|------------------------|
| 2026-02-12 | Use Rust | Memory safety, performance | Go, Java |
| 2026-02-12 | PostgreSQL for primary DB | ACID, JSON support | MySQL, MongoDB |
| 2026-02-12 | Redis for cache/state | Speed, pub/sub | Memcached, Hazelcast |
| 2026-02-12 | Cedar for policies | Formal verification | OPA, Casbin |
| 2026-02-12 | Biscuit for agents | Native delegation | Macaroons, custom |
| 2026-02-12 | JWT for users | Broad compatibility | PASETO, sessions |

---

## Communication Plan

### Weekly Updates

- **Monday**: Week planning, task assignment
- **Wednesday**: Mid-week check-in, blocker review
- **Friday**: Week review, demo, retrospective

### Reporting

- **Daily**: Update PRD.md task checkboxes
- **Weekly**: Update IMPLEMENTATION_STATUS.md
- **Monthly**: Update ROADMAP.md (this file)

### Stakeholders

- **Engineering**: Daily Slack updates
- **Security**: Weekly security review
- **Product**: Bi-weekly feature demos
- **Management**: Monthly progress reports

---

**Last Updated**: 2026-02-12
**Next Review**: 2026-02-19
**Status**: ✅ On Track

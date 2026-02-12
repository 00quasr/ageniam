# Claude Code Instructions for Agent IAM

## Project Overview

**Agent IAM** is an identity and access management system specifically designed for AI agents. You are working on a Rust-based service that provides authentication, authorization, identity management, rate limiting, and audit logging for AI agents.

## Current Status

- **Phase 1 (Foundation)**: ✅ COMPLETED
- **Phase 2 (Authentication)**: 🔄 IN PROGRESS
- **Overall Progress**: ~17% (2 of 12 weeks)

## Project Structure

```
/ageniam/
├── src/
│   ├── main.rs              - Application entry point
│   ├── lib.rs               - Library exports
│   ├── config.rs            - Configuration management
│   ├── errors.rs            - Error types
│   ├── api/                 - HTTP endpoints
│   ├── auth/                - Authentication (JWT, Biscuit, passwords)
│   ├── authz/               - Authorization (Cedar engine)
│   ├── audit/               - Audit logging
│   ├── crypto/              - Cryptographic operations
│   ├── db/                  - Database layer
│   ├── domain/              - Business logic
│   ├── observability/       - Logging, metrics, health
│   ├── rate_limit/          - Rate limiting
│   └── redis/               - Redis operations
├── config/                  - TOML configuration files
├── docs/                    - Documentation
├── tests/                   - Integration tests
└── deploy/                  - Deployment artifacts
```

## Technology Stack

- **Language**: Rust 1.75+
- **Web Framework**: Axum 0.7
- **Database**: PostgreSQL 14+ with SQLx
- **Cache**: Redis 7+
- **Security**: JWT (RS256), Biscuit (Ed25519), Argon2id, Cedar policies

## Working with This Codebase

### Before You Start

1. **Read the docs**:
   - `docs/ARCHITECTURE.md` - System architecture and design
   - `docs/IMPLEMENTATION_STATUS.md` - Current progress
   - `PRD.md` - Detailed task list (100 tasks)

2. **Check task status**:
   - Open `PRD.md` in the main directory
   - Tasks are numbered 1-100 with checkboxes
   - Update checkboxes as you complete tasks

3. **Understand the context**:
   - This is a security-critical system
   - Code quality and correctness are paramount
   - All database queries use SQLx (compile-time checked)
   - All crypto operations must be secure

### Development Workflow

1. **Pick a task** from PRD.md (start with unchecked tasks in order)

2. **Before implementing**:
   - Read related code in the module
   - Check if there are existing patterns to follow
   - Review the database schema if touching data layer
   - Consider error handling from the start

3. **Implementation guidelines**:
   - Use existing error types from `errors.rs`
   - Add tracing/logging for important operations
   - Write tests for business logic
   - Update metrics where appropriate
   - Follow Rust best practices (clippy, rustfmt)

4. **After implementing**:
   - Update the checkbox in PRD.md
   - Test your changes (manual + automated)
   - Update documentation if needed
   - Commit with descriptive message

### Code Style

- **Error Handling**: Always use `Result<T>` (alias in errors.rs)
- **Logging**: Use `tracing::info!`, `tracing::error!`, etc.
- **Async**: All I/O operations are async (use `async fn` and `.await`)
- **Database**: Use SQLx macros for compile-time checking
- **Naming**:
  - Functions: `snake_case`
  - Types: `PascalCase`
  - Constants: `SCREAMING_SNAKE_CASE`

### Testing

Run tests:
```bash
cargo test
```

Run with Docker:
```bash
docker-compose up -d
curl http://localhost:8080/health/ready
```

### Common Tasks

#### Adding a new endpoint

1. Define handler in `src/api/<module>.rs`
2. Add route in `src/api/routes.rs`
3. Add request/response types
4. Implement business logic in `src/domain/`
5. Add database queries if needed
6. Add tests in `tests/`
7. Update PRD.md checkbox

#### Adding database operations

1. Check `src/db/migrations/` for schema
2. Add query in appropriate `src/db/<table>.rs` file
3. Use SQLx macros: `sqlx::query!()` or `sqlx::query_as!()`
4. Handle errors properly
5. Add tracing

#### Adding metrics

1. Define metric in `src/observability/metrics.rs`
2. Record metric where appropriate
3. Test by calling `/metrics` endpoint

### Security Considerations

⚠️ **CRITICAL - Always follow these rules**:

1. **Never log sensitive data**: passwords, tokens, API keys
2. **Always use parameterized queries**: prevent SQL injection
3. **Validate all inputs**: check types, ranges, formats
4. **Use constant-time comparisons**: for passwords, tokens
5. **Rate limit authentication**: prevent brute force
6. **Audit all security events**: log auth, authz, policy changes
7. **Encrypt secrets at rest**: use proper key management
8. **Use TLS in production**: never send credentials over HTTP

### Database Migrations

To add a migration:

```bash
# Create new file in src/db/migrations/
# Name it: 009_description.sql
# SQLx will run migrations in order
```

Migrations are embedded in the binary and run on startup.

### Configuration

Config precedence (highest to lowest):
1. Environment variables (`AGENT_IAM__<SECTION>__<KEY>`)
2. Environment-specific file (`config/production.toml`)
3. Default file (`config/default.toml`)

Example:
```bash
AGENT_IAM__SERVER__PORT=9090 cargo run
```

### Debugging

1. **Enable debug logging**:
   ```bash
   RUST_LOG=debug cargo run
   ```

2. **Check database**:
   ```bash
   docker-compose exec postgres psql -U postgres -d agent_iam_dev
   ```

3. **Check Redis**:
   ```bash
   docker-compose exec redis redis-cli
   ```

4. **View metrics**:
   ```bash
   curl http://localhost:8080/metrics
   ```

### Module-Specific Notes

#### `src/auth/` - Authentication

- **jwt.rs**: User tokens (RS256, 15 min expiry)
- **biscuit.rs**: Agent tokens (Ed25519, task-scoped)
- **password.rs**: Argon2id hashing (OWASP params)
- **middleware.rs**: Extract and validate tokens from headers

#### `src/authz/` - Authorization

- **engine.rs**: Cedar policy engine wrapper
- **evaluator.rs**: Build context and evaluate policies
- **cache.rs**: Cache policy decisions (LRU)
- **middleware.rs**: Protect endpoints with authz checks

#### `src/db/` - Database

- All queries use SQLx for type safety
- Migrations in `migrations/` directory
- Schema types in `schema.rs`
- Connection pool in `pool.rs`

#### `src/redis/` - Redis

- **revocation.rs**: Token revocation list
- **counters.rs**: Rate limit sliding window implementation
- Use connection manager for all operations

#### `src/audit/` - Audit Logging

- **logger.rs**: Async event logger
- **tamper_proof.rs**: Hash chains and signatures
- **storage.rs**: Multi-backend (Postgres, S3)
- **query.rs**: Query interface for audit logs

### Performance Targets

- Authorization: <10ms p99
- Token validation: <2ms p99
- Rate limit check: <1ms p99
- Audit log write: <50ms p99 (async)

### When You're Stuck

1. Check `docs/ARCHITECTURE.md` for design decisions
2. Look for similar code in the codebase
3. Check the original plan in the conversation history
4. Review Rust docs for libraries (docs.rs)
5. Ask for clarification if requirements are unclear

### What NOT to Do

❌ Don't create new error types (use existing in `errors.rs`)
❌ Don't bypass SQLx type checking (no raw SQL strings)
❌ Don't block async functions with `std::thread::sleep`
❌ Don't use `unwrap()` or `expect()` in production code
❌ Don't commit secrets or credentials
❌ Don't skip writing tests for security-critical code
❌ Don't modify the database schema without migrations
❌ Don't change API contracts without updating docs

### Useful Commands

```bash
# Build
cargo build

# Run locally
cargo run

# Run with Docker
docker-compose up -d

# Check code
cargo clippy

# Format code
cargo fmt

# Run tests
cargo test

# Check health
curl http://localhost:8080/health/ready

# View logs
docker-compose logs -f agent-iam

# Database migrations (auto-run on startup)
# Manual: sqlx migrate run

# Clean up
docker-compose down -v
cargo clean
```

### Before Marking a Task Complete

✅ Code compiles without warnings
✅ Tests pass
✅ Logging/metrics added where appropriate
✅ Error handling is proper
✅ Documentation updated if needed
✅ PRD.md checkbox marked
✅ Security considerations addressed

### Current Priority

**Next up: Authentication (Tasks #20-35)**

Focus on implementing:
1. Password hashing (Argon2id)
2. JWT token generation/validation
3. Authentication endpoints (login, logout, refresh)

These are blocking tasks for all subsequent features.

### Questions?

- Architecture questions → See `docs/ARCHITECTURE.md`
- Task questions → See `docs/PRD.md`
- Progress questions → See `docs/IMPLEMENTATION_STATUS.md`
- Implementation questions → Check existing code patterns

---

**Remember**: This is a security-critical system. When in doubt, be more cautious, not less. Quality over speed. 🔒

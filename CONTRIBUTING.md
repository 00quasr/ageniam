# Contributing to Agent IAM

Thank you for contributing to Agent IAM! This document provides guidelines for development.

## Getting Started

1. **Read the documentation**:
   - `CLAUDE.md` - Instructions for AI assistants working on this codebase
   - `PRD.md` - Product requirements and task list (100 tasks)
   - `docs/ARCHITECTURE.md` - System architecture
   - `docs/IMPLEMENTATION_STATUS.md` - Current progress

2. **Set up your development environment**:
   ```bash
   # Clone the repository
   cd /ageniam

   # Copy environment file
   cp .env.example .env

   # Start dependencies
   docker-compose up -d postgres redis

   # Run the service
   cargo run
   ```

## Development Workflow

### Picking a Task

1. Open `PRD.md` in the main directory
2. Find the next unchecked task in sequence
3. Read the task description and acceptance criteria
4. Check dependencies (some tasks require others to be completed first)

### Before You Code

1. **Understand the context**:
   - Read related code in the module
   - Check existing patterns
   - Review database schema if touching data layer

2. **Plan your approach**:
   - Identify files that need to be modified
   - Consider error handling
   - Think about testing

3. **Create a branch** (if using git):
   ```bash
   git checkout -b task-XX-description
   ```

### While Coding

1. **Follow Rust conventions**:
   - Run `cargo fmt` to format code
   - Run `cargo clippy` to check for issues
   - Use meaningful variable names
   - Add comments for complex logic

2. **Error handling**:
   - Use `Result<T>` from `errors.rs`
   - Never use `unwrap()` or `expect()` in production code
   - Provide meaningful error messages

3. **Logging**:
   - Use `tracing::info!()` for important events
   - Use `tracing::debug!()` for detailed information
   - Use `tracing::error!()` for errors
   - Never log sensitive data (passwords, tokens)

4. **Testing**:
   - Write unit tests for business logic
   - Write integration tests for endpoints
   - Test error cases, not just happy path

### After Coding

1. **Test your changes**:
   ```bash
   # Run tests
   cargo test

   # Run the service
   cargo run

   # Test manually
   curl http://localhost:8080/health/ready
   ```

2. **Check code quality**:
   ```bash
   # Format code
   cargo fmt

   # Check for issues
   cargo clippy

   # Ensure it compiles without warnings
   cargo build --all-targets
   ```

3. **Update documentation**:
   - Update `PRD.md` - check the task box
   - Update `docs/IMPLEMENTATION_STATUS.md` if completing a phase
   - Add comments to complex code
   - Update API docs if adding endpoints

4. **Commit your changes**:
   ```bash
   git add .
   git commit -m "Task #XX: Brief description"
   ```

## Code Style

### Naming Conventions

- **Functions**: `snake_case`
  ```rust
  async fn create_identity(...)
  ```

- **Types**: `PascalCase`
  ```rust
  struct Identity { ... }
  enum IdentityType { ... }
  ```

- **Constants**: `SCREAMING_SNAKE_CASE`
  ```rust
  const MAX_LOGIN_ATTEMPTS: u32 = 5;
  ```

- **Modules**: `snake_case`
  ```rust
  mod auth_middleware;
  ```

### File Organization

```rust
// Imports
use crate::errors::Result;
use sqlx::PgPool;

// Constants
const DEFAULT_TIMEOUT: u64 = 30;

// Types
pub struct MyStruct { ... }

// Implementation
impl MyStruct {
    pub fn new() -> Self { ... }

    pub async fn do_something(&self) -> Result<()> { ... }
}

// Helper functions
fn internal_helper() -> bool { ... }

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() { ... }
}
```

### Error Handling

Always use `Result<T>`:

```rust
// Good
pub async fn create_user(email: &str) -> Result<User> {
    if email.is_empty() {
        return Err(AppError::ValidationError("Email is required".into()));
    }
    // ...
    Ok(user)
}

// Bad
pub async fn create_user(email: &str) -> User {
    assert!(!email.is_empty());  // DON'T DO THIS
    // ...
}
```

### Async Functions

All I/O operations should be async:

```rust
// Good
pub async fn save_to_db(pool: &PgPool, data: &str) -> Result<()> {
    sqlx::query!("INSERT INTO table (data) VALUES ($1)", data)
        .execute(pool)
        .await?;
    Ok(())
}

// Bad - blocking
pub fn save_to_db(pool: &PgPool, data: &str) -> Result<()> {
    // Blocks the async runtime
    std::thread::sleep(Duration::from_secs(1));
    Ok(())
}
```

### Database Queries

Always use SQLx macros for type safety:

```rust
// Good - compile-time checked
let user = sqlx::query_as!(
    User,
    "SELECT * FROM identities WHERE id = $1",
    user_id
)
.fetch_one(pool)
.await?;

// Bad - no type checking
let user = sqlx::query("SELECT * FROM identities WHERE id = ?")
    .bind(user_id)
    .fetch_one(pool)
    .await?;
```

### Logging

Use structured logging:

```rust
// Good
tracing::info!(
    user_id = %user.id,
    email = %user.email,
    "User logged in successfully"
);

// Bad
println!("User {} logged in", user.email);  // DON'T DO THIS
```

## Testing Guidelines

### Unit Tests

Test business logic in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        let result = validate_password("weak");
        assert!(result.is_err());

        let result = validate_password("StrongP@ssw0rd!");
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Test full request/response cycle:

```rust
#[tokio::test]
async fn test_login_endpoint() {
    let app = create_test_app().await;

    let response = app
        .post("/v1/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let body: LoginResponse = response.json().await;
    assert!(!body.access_token.is_empty());
}
```

### Test Database

Use transactions for test isolation:

```rust
#[sqlx::test]
async fn test_create_user(pool: PgPool) -> sqlx::Result<()> {
    let user = create_user(&pool, "test@example.com").await?;
    assert_eq!(user.email, "test@example.com");
    Ok(())
    // Transaction auto-rolls back
}
```

## Security Checklist

Before submitting security-critical code:

- [ ] No secrets in logs
- [ ] Input validation implemented
- [ ] SQL injection prevented (using SQLx macros)
- [ ] XSS prevented (proper escaping)
- [ ] CSRF protection (for state-changing operations)
- [ ] Rate limiting added (for auth endpoints)
- [ ] Audit logging added
- [ ] Constant-time comparisons (for passwords/tokens)
- [ ] TLS required in production
- [ ] Error messages don't leak sensitive info

## Performance Checklist

- [ ] Database queries use indexes
- [ ] N+1 queries avoided
- [ ] Caching used where appropriate
- [ ] Async operations don't block
- [ ] Connection pools configured properly
- [ ] Large responses paginated
- [ ] Metrics added for monitoring

## Documentation

### Code Comments

Add comments for:
- Complex algorithms
- Non-obvious design decisions
- Security considerations
- Performance optimizations

```rust
/// Creates a new agent identity with JIT provisioning.
///
/// # Arguments
/// * `parent_id` - The identity ID of the parent (user or agent)
/// * `task_id` - The task this agent is scoped to
/// * `ttl` - Time-to-live in seconds
///
/// # Returns
/// A tuple of (Identity, BiscuitToken)
///
/// # Errors
/// Returns `AppError::Forbidden` if parent lacks permissions
pub async fn provision_agent(...) -> Result<(Identity, String)> {
    // ...
}
```

### API Documentation

Document all endpoints:

```rust
/// POST /v1/auth/login
///
/// Authenticates a user and returns JWT tokens.
///
/// # Request Body
/// ```json
/// {
///   "email": "user@example.com",
///   "password": "secret",
///   "tenant_slug": "acme-corp"
/// }
/// ```
///
/// # Response
/// ```json
/// {
///   "access_token": "eyJhbGc...",
///   "token_type": "Bearer",
///   "expires_in": 900
/// }
/// ```
```

## Common Patterns

### Extracting Auth from Request

```rust
use axum::extract::Extension;

async fn protected_handler(
    Extension(identity): Extension<Identity>,
) -> Result<Json<Response>> {
    // identity is already validated by middleware
    Ok(Json(Response { ... }))
}
```

### Database Transactions

```rust
let mut tx = pool.begin().await?;

// Do multiple operations
sqlx::query!("INSERT INTO table1 ...").execute(&mut *tx).await?;
sqlx::query!("INSERT INTO table2 ...").execute(&mut *tx).await?;

// Commit or rollback
tx.commit().await?;
```

### Async Logging

```rust
// Don't await audit logging (fire and forget)
tokio::spawn(async move {
    if let Err(e) = audit_logger.log_event(event).await {
        tracing::error!(error = ?e, "Failed to write audit log");
    }
});
```

## Getting Help

- **Architecture questions**: See `docs/ARCHITECTURE.md`
- **Task questions**: See `docs/PRD.md`
- **Implementation questions**: Check existing code patterns
- **Rust questions**: Check [Rust docs](https://doc.rust-lang.org/)
- **Library questions**: Check [docs.rs](https://docs.rs/)

## What to Avoid

❌ Don't:
- Use `unwrap()` or `expect()` in production
- Block async functions with sync I/O
- Log sensitive data (passwords, tokens)
- Bypass type safety (raw SQL, `as` casts)
- Commit secrets or credentials
- Skip tests for critical code
- Modify schema without migrations
- Break backward compatibility

✅ Do:
- Use proper error handling
- Add logging and metrics
- Write tests
- Follow existing patterns
- Document complex logic
- Consider security implications
- Update task checkboxes in PRD.md

## License

MIT License - See LICENSE file for details.

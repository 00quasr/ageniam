# Quick Start Guide

Get Agent IAM running in 5 minutes.

## Prerequisites

Choose one of the following setups:

### Option A: Docker (Recommended for Development)

- Docker 20.10+
- Docker Compose 1.29+

### Option B: Local Development

- Rust 1.75+
- PostgreSQL 14+
- Redis 7+

## Quick Start (Docker)

### 1. Clone and Navigate

```bash
cd /ageniam
```

### 2. Start All Services

```bash
docker-compose up -d
```

This starts:
- PostgreSQL (port 5432)
- Redis (port 6379)
- Agent IAM (port 8080)

### 3. Check Health

```bash
curl http://localhost:8080/health/ready
```

Expected response:
```json
{
  "status": "ok",
  "version": "0.1.0",
  "checks": {
    "database": {
      "status": "ok"
    },
    "redis": {
      "status": "ok"
    }
  }
}
```

### 4. View Logs

```bash
docker-compose logs -f agent-iam
```

### 5. View Metrics

```bash
curl http://localhost:8080/metrics
```

**Done!** 🎉 Agent IAM is running.

---

## Quick Start (Local)

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Start PostgreSQL

```bash
# macOS (Homebrew)
brew install postgresql@14
brew services start postgresql@14
createdb agent_iam_dev

# Linux (Ubuntu/Debian)
sudo apt install postgresql-14
sudo systemctl start postgresql
sudo -u postgres createdb agent_iam_dev

# Or use Docker
docker run -d \
  --name postgres \
  -e POSTGRES_DB=agent_iam_dev \
  -e POSTGRES_PASSWORD=postgres \
  -p 5432:5432 \
  postgres:16-alpine
```

### 3. Start Redis

```bash
# macOS (Homebrew)
brew install redis
brew services start redis

# Linux (Ubuntu/Debian)
sudo apt install redis-server
sudo systemctl start redis

# Or use Docker
docker run -d \
  --name redis \
  -p 6379:6379 \
  redis:7-alpine
```

### 4. Configure Environment

```bash
cp .env.example .env
```

Edit `.env`:
```bash
AGENT_IAM__DATABASE__URL=postgresql://postgres:postgres@localhost:5432/agent_iam_dev
AGENT_IAM__REDIS__URL=redis://localhost:6379
RUST_LOG=info,agent_iam=debug
```

### 5. Run Migrations and Start

```bash
cargo run
```

Migrations run automatically on startup.

### 6. Verify

```bash
curl http://localhost:8080/health/ready
```

**Done!** 🎉 Agent IAM is running locally.

---

## Testing the API

### Health Check

```bash
curl http://localhost:8080/health/live
```

### Readiness Check

```bash
curl http://localhost:8080/health/ready
```

### Metrics

```bash
curl http://localhost:8080/metrics | grep -E "^http_|^authz_"
```

### Placeholder Endpoints

Currently, these return placeholder responses:

```bash
# Auth endpoints (not implemented yet)
curl -X POST http://localhost:8080/v1/auth/login

# Identity endpoints (not implemented yet)
curl -X POST http://localhost:8080/v1/identities

# Authorization endpoints (not implemented yet)
curl -X POST http://localhost:8080/v1/authz/check
```

---

## Exploring the Database

### Connect to PostgreSQL

```bash
# Docker
docker-compose exec postgres psql -U postgres -d agent_iam_dev

# Local
psql -U postgres -d agent_iam_dev
```

### View Tables

```sql
\dt

-- You should see:
-- tenants, identities, roles, permissions, role_permissions,
-- identity_roles, policies, sessions, audit_logs, rate_limits
```

### View Schema

```sql
\d identities
```

### Sample Queries

```sql
-- Check migrations
SELECT * FROM _sqlx_migrations;

-- View tables
SELECT tablename FROM pg_tables WHERE schemaname = 'public';

-- Count rows (should be 0 initially)
SELECT
  (SELECT COUNT(*) FROM tenants) as tenants,
  (SELECT COUNT(*) FROM identities) as identities,
  (SELECT COUNT(*) FROM sessions) as sessions;
```

---

## Exploring Redis

### Connect to Redis

```bash
# Docker
docker-compose exec redis redis-cli

# Local
redis-cli
```

### Sample Commands

```bash
# Check connection
PING
# Returns: PONG

# List all keys (should be empty initially)
KEYS *

# Check info
INFO server
```

---

## Development Workflow

### 1. Pick a Task

Open `PRD.md` (in main directory) and find the next unchecked task.

### 2. Make Changes

Edit files in `src/`:

```bash
# Example: Implement password hashing
vim src/auth/password.rs
```

### 3. Test Locally

```bash
# Build
cargo build

# Run tests
cargo test

# Run the service
cargo run
```

### 4. Hot Reload (Optional)

Install cargo-watch:

```bash
cargo install cargo-watch
```

Run with auto-reload:

```bash
cargo watch -x run
```

### 5. Check Code Quality

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Check it compiles
cargo check
```

### 6. Update Documentation

Mark the task complete in `PRD.md`:

```markdown
- [x] 20. Implement Argon2id password hashing
```

---

## Common Commands

### Docker Compose

```bash
# Start all services
docker-compose up -d

# Stop all services
docker-compose down

# View logs
docker-compose logs -f

# Restart a service
docker-compose restart agent-iam

# Rebuild and restart
docker-compose up -d --build

# Clean up (remove volumes)
docker-compose down -v
```

### Cargo

```bash
# Build
cargo build

# Build release
cargo build --release

# Run
cargo run

# Test
cargo test

# Test a specific module
cargo test auth::

# Format code
cargo fmt

# Check for warnings
cargo clippy

# Clean build artifacts
cargo clean
```

### Database

```bash
# Connect to PostgreSQL (Docker)
docker-compose exec postgres psql -U postgres -d agent_iam_dev

# Export schema
pg_dump -U postgres -d agent_iam_dev -s > schema.sql

# Reset database (DANGER!)
docker-compose down -v
docker-compose up -d postgres
# Migrations will run on next startup
```

### Debugging

```bash
# Run with debug logs
RUST_LOG=debug cargo run

# Run with trace logs
RUST_LOG=trace cargo run

# Check environment
env | grep AGENT_IAM

# Test database connection
psql postgresql://postgres:postgres@localhost:5432/agent_iam_dev -c "SELECT 1"

# Test Redis connection
redis-cli -u redis://localhost:6379 PING
```

---

## Next Steps

Now that Agent IAM is running:

1. **Read the architecture**: `docs/ARCHITECTURE.md`
2. **Check the task list**: `PRD.md` (in main directory)
3. **Start contributing**: `CONTRIBUTING.md`
4. **Understand the codebase**: `CLAUDE.md`

### Current Priority Tasks

The next tasks to implement are:

- [ ] **Task #20**: Password hashing with Argon2id
- [ ] **Task #26**: JWT token generation
- [ ] **Task #36**: Login endpoint

See `PRD.md` (in main directory) for full details.

---

## Troubleshooting

### "Connection refused" errors

**Problem**: Can't connect to PostgreSQL or Redis

**Solution**:
```bash
# Check if services are running
docker-compose ps

# Restart services
docker-compose restart postgres redis

# Check logs
docker-compose logs postgres
docker-compose logs redis
```

### "Migration failed" errors

**Problem**: Database migrations won't run

**Solution**:
```bash
# Reset the database
docker-compose down -v
docker-compose up -d postgres

# Migrations will run automatically on next startup
cargo run
```

### "Port already in use" errors

**Problem**: Port 8080, 5432, or 6379 already in use

**Solution**:
```bash
# Find what's using the port
lsof -i :8080

# Kill the process or change the port
AGENT_IAM__SERVER__PORT=9090 cargo run
```

### Build errors

**Problem**: Cargo build fails

**Solution**:
```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update

# Check Rust version
rustc --version
# Should be 1.75 or later
```

### "Access denied" database errors

**Problem**: Can't connect to database

**Solution**:
```bash
# Check connection string in .env
cat .env | grep DATABASE

# Test connection manually
psql postgresql://postgres:postgres@localhost:5432/agent_iam_dev
```

---

## Environment Variables Reference

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENT_IAM_ENV` | `development` | Environment (development, production) |
| `AGENT_IAM__SERVER__PORT` | `8080` | HTTP server port |
| `AGENT_IAM__DATABASE__URL` | - | PostgreSQL connection string |
| `AGENT_IAM__REDIS__URL` | - | Redis connection string |
| `RUST_LOG` | `info` | Log level (error, warn, info, debug, trace) |

See `config/default.toml` for all configuration options.

---

## Getting Help

- **Documentation**: Check `docs/` directory
- **Issues**: See existing code patterns
- **Questions**: Review `CLAUDE.md` and `CONTRIBUTING.md`

---

**Happy coding!** 🚀

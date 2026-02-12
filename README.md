# Agent IAM

**Identity and Access Management for AI Agents**

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-in%20development-yellow.svg)](docs/PRD.md)

## Overview

Agent IAM is a foundational identity and access management layer specifically designed for AI agents. It provides:

- **Identity Management**: JIT provisioning, delegation chains, ephemeral identities
- **Authentication**: JWT tokens for users, Biscuit tokens for agents
- **Authorization**: Cedar policy engine with ABAC support
- **Rate Limiting**: Multi-dimensional limits with Redis sliding windows
- **Audit Logging**: Comprehensive, tamper-proof audit trail

## 📚 Documentation

### Getting Started
- **[docs/QUICKSTART.md](docs/QUICKSTART.md)** - Get up and running in 5 minutes
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development guidelines and workflow

### Project Management
- **[PRD.md](PRD.md)** - Product Requirements Document with 100-task breakdown ⭐
- **[docs/ROADMAP.md](docs/ROADMAP.md)** - Timeline, milestones, and progress tracking
- **[docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md)** - Current implementation status

### Technical Documentation
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** - Detailed system architecture and design
- **[CLAUDE.md](CLAUDE.md)** - Instructions for AI assistants working on this codebase

## Quick Start

### Prerequisites

- Rust 1.75 or later
- PostgreSQL 14 or later
- Redis 7 or later
- Docker and Docker Compose (optional)

### Local Development with Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f agent-iam

# Stop all services
docker-compose down
```

### Local Development without Docker

1. **Start PostgreSQL and Redis:**

```bash
# PostgreSQL
createdb agent_iam_dev

# Redis
redis-server
```

2. **Set up environment:**

```bash
cp .env.example .env
# Edit .env with your database and Redis URLs
```

3. **Run migrations and start server:**

```bash
cargo run
```

The service will be available at `http://localhost:8080`.

## API Endpoints

### Health Checks

- `GET /health/live` - Liveness probe
- `GET /health/ready` - Readiness probe
- `GET /health/startup` - Startup probe
- `GET /metrics` - Prometheus metrics

### Authentication (Coming Soon)

- `POST /v1/auth/login` - User login
- `POST /v1/auth/logout` - Logout
- `POST /v1/auth/refresh` - Refresh token

### Identities (Coming Soon)

- `POST /v1/identities` - Create identity (JIT agent provisioning)
- `GET /v1/identities/:id` - Get identity
- `PATCH /v1/identities/:id` - Update identity
- `DELETE /v1/identities/:id` - Delete identity

### Authorization (Coming Soon)

- `POST /v1/authz/check` - Check authorization

### Policies (Coming Soon)

- `GET /v1/policies` - List policies
- `POST /v1/policies` - Create policy
- `PUT /v1/policies/:id` - Update policy
- `DELETE /v1/policies/:id` - Delete policy

## Configuration

Configuration is managed through TOML files in the `config/` directory and environment variables.

See `config/default.toml` for all available options.

Environment variables use the prefix `AGENT_IAM__` with double underscores separating nested keys:

```bash
AGENT_IAM__SERVER__PORT=8080
AGENT_IAM__DATABASE__URL=postgresql://...
```

## Development

### Build

```bash
cargo build
```

### Run tests

```bash
cargo test
```

### Run with hot reload

```bash
cargo watch -x run
```

## Architecture

See `docs/ARCHITECTURE.md` for detailed architecture documentation.

## License

MIT

## 📊 Project Status

**Progress**: 19/100 tasks completed (~19%)

🚧 **Phase 1 (Foundation)** - ✅ COMPLETED (Tasks 1-19)
🔜 **Phase 2 (Authentication)** - NEXT (Tasks 20-43)

See [PRD.md](PRD.md) for the complete 100-task breakdown.

### Recent Completions

- ✅ Rust project structure and workspace
- ✅ Database schema (8 migrations)
- ✅ PostgreSQL + Redis integration
- ✅ Axum web server with middleware
- ✅ Health checks and Prometheus metrics
- ✅ Docker setup (Compose + Dockerfile)
- ✅ Observability (logging, tracing, metrics)

### Next Up (Phase 2)

- 🔜 Task #20-25: Password hashing with Argon2id
- 🔜 Task #26-35: JWT token system (RS256)
- 🔜 Task #36-43: Authentication endpoints (login, logout, refresh)

### Timeline

- **Start**: 2026-02-12
- **Target**: 2026-05-06 (12 weeks)
- **Current**: Week 2 of 12

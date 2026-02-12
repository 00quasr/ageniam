# Agent IAM Makefile

.PHONY: help build run test docker-up docker-down clean

help:
	@echo "Agent IAM - Development Commands"
	@echo ""
	@echo "  make build         - Build the project"
	@echo "  make run           - Run the service locally"
	@echo "  make test          - Run tests"
	@echo "  make docker-up     - Start Docker Compose services"
	@echo "  make docker-down   - Stop Docker Compose services"
	@echo "  make docker-logs   - View Docker logs"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make check-health  - Check service health"

build:
	cargo build

build-release:
	cargo build --release

run:
	cargo run

test:
	cargo test

docker-up:
	docker-compose up -d

docker-down:
	docker-compose down

docker-logs:
	docker-compose logs -f

docker-rebuild:
	docker-compose up -d --build

clean:
	cargo clean
	docker-compose down -v

check-health:
	@echo "Checking liveness..."
	@curl -s http://localhost:8080/health/live | jq .
	@echo ""
	@echo "Checking readiness..."
	@curl -s http://localhost:8080/health/ready | jq .
	@echo ""
	@echo "Checking metrics..."
	@curl -s http://localhost:8080/metrics | head -n 10

check-db:
	docker-compose exec postgres psql -U postgres -d agent_iam_dev -c "\dt"

check-redis:
	docker-compose exec redis redis-cli PING

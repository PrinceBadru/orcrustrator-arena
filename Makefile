.PHONY: up down bench audit check ui clean help

help:
	@echo "Orcrustrator Arena Task Runner"
	@echo "  make up      - Build and spin up all containers with Docker Compose"
	@echo "  make down    - Stop and remove all containers"
	@echo "  make bench   - Execute constant-arrival-rate benchmark harness"
	@echo "  make audit   - Run consistency auditor across all 6 microservices"
	@echo "  make check   - Conformance check & DAG architecture linter"
	@echo "  make ui      - Run UI dashboard in development mode"

up:
	docker compose -f compose/docker-compose.yml up --build -d

down:
	docker compose -f compose/docker-compose.yml down -v

bench:
	docker run --rm -i --network compose_arena-net loadimpact/k6 run - < bench/load-test.js

audit:
	curl -s http://localhost:8090/audit | jq .

check:
	cargo check --workspace

ui:
	cd ui && npm install && npm run dev

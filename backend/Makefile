.PHONY: dev keys down test test-integration test-all fmt clippy check clean

ENV_FILE := .env.local

keys:
	@if [ ! -f $(ENV_FILE) ]; then \
		echo "generating Ed25519 dev keys → $(ENV_FILE)"; \
		openssl genpkey -algorithm Ed25519 -out /tmp/lrs_priv.pem && \
		openssl pkey -in /tmp/lrs_priv.pem -pubout -out /tmp/lrs_pub.pem && \
		printf 'APP_JWT_PRIVATE_KEY="%s"\nAPP_JWT_PUBLIC_KEY="%s"\n' \
			"$$(cat /tmp/lrs_priv.pem)" "$$(cat /tmp/lrs_pub.pem)" > $(ENV_FILE) && \
		rm /tmp/lrs_priv.pem /tmp/lrs_pub.pem; \
	else \
		echo "$(ENV_FILE) already exists — skipping key generation"; \
	fi

dev: keys
	docker compose --env-file $(ENV_FILE) up --build

down:
	docker compose down

# Unit tests only — no containers, no network. Fast and CI-safe by default.
test:
	SQLX_OFFLINE=true cargo test --lib

# Integration tests — spins up Postgres + Redis via testcontainers. Requires Docker.
test-integration:
	SQLX_OFFLINE=true cargo test --tests

# Everything.
test-all: test test-integration

fmt:
	cargo fmt

clippy:
	SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings

# Same gate CI runs.
check: fmt clippy test

clean:
	cargo clean
	rm -f $(ENV_FILE)

.PHONY: dev keys clean test fmt clippy

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

test:
	SQLX_OFFLINE=true cargo test --lib

fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
	rm -f $(ENV_FILE)

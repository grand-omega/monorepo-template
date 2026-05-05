# syntax=docker/dockerfile:1.7
ARG RUST_VERSION=1.95

FROM lukemathwalker/cargo-chef:latest-rust-${RUST_VERSION}-slim AS chef
# `curl` is required at build time by utoipa-swagger-ui's build.rs to fetch
# Swagger UI assets. `pkg-config` + libssl-dev are kept in case any future
# transitive dep needs them.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin lab-rust-server
RUN strip target/release/lab-rust-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tini libssl3 \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -r -u 10001 -g nogroup app
WORKDIR /app
COPY --from=builder /app/target/release/lab-rust-server /usr/local/bin/lab-rust-server
COPY --from=builder /app/migrations ./migrations
USER 10001
EXPOSE 8080 9090
ENTRYPOINT ["/usr/bin/tini", "--", "lab-rust-server"]
CMD ["serve"]

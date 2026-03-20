# syntax=docker/dockerfile:1

# --- Build stage --------------------------------------------------------------
FROM rust:latest AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      build-essential \
      pkg-config \
      libssl-dev \
      ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/redeye_auth

# Copy manifest and source
COPY redeye_auth/Cargo.toml ./
COPY redeye_auth/src ./src
COPY redeye_auth/migrations ./migrations

# Build release binary
RUN cargo build --release

# --- Runtime stage ------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/redeye_auth/target/release/redeye_auth ./redeye_auth
COPY --from=builder /usr/src/redeye_auth/migrations ./migrations

EXPOSE 8084

CMD ["./redeye_auth"]

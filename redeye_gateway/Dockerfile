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

WORKDIR /usr/src/redeye_gateway

# Copy only the manifest first to leverage Docker layer caching.
COPY redeye_gateway/Cargo.toml ./

# Copy source code.
COPY redeye_gateway/src ./src

# Build release binary
RUN cargo build --release

# --- Runtime stage ------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/redeye_gateway/target/release/redeye_gateway ./redeye_gateway

EXPOSE 8080

CMD ["./redeye_gateway"]

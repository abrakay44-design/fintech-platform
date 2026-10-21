# syntax=docker/dockerfile:1
FROM rust:1.83-slim AS builder

WORKDIR /app

# protobuf compiler is needed for tonic-build
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY rust-core/common ./rust-core/common
COPY rust-core/wallet-service ./rust-core/wallet-service
COPY rust-core/transaction-engine ./rust-core/transaction-engine
COPY rust-core/policy-engine ./rust-core/policy-engine
COPY rust-core/ledger ./rust-core/ledger
COPY rust-core/orchestrator ./rust-core/orchestrator
COPY rust-core/grpc-server ./rust-core/grpc-server

RUN cargo build --release --bin grpc-server

# ── Runtime stage ──
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/grpc-server /usr/local/bin/grpc-server

ENV GRPC_ADDR=0.0.0.0:50051
EXPOSE 50051
CMD ["grpc-server"]

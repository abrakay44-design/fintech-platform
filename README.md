# Fintech Investment Platform

A security-first, multi-chain investment platform for crypto, tokenized ETFs, and stocks.

## Architecture

- **rust-core/** — Core transaction engine, wallet/key management, policy engine, ledger (Rust)
- **go-gateway/** — API gateway, auth, market data, WebSockets (Go)
- **ts-frontend/** — Web and mobile clients (TypeScript / Next.js / React Native)
- **infra/** — Terraform and Kubernetes manifests
- **docs/** — Threat model, architecture decision records

## Security Principles

1. Private keys never leave the Rust wallet service.
2. All sensitive operations require policy approval before signing.
3. Every state change is recorded in a double-entry ledger.
4. All inter-service communication uses mTLS.

## Development

See `docs/` for setup instructions and architecture decisions.

## Status

Phase 0 — Foundations.

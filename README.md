# Fintech Platform — Non-Custodial Web3 Investment App

A full-stack, security-first platform for non-custodial crypto swaps, featuring passwordless biometric authentication (Passkeys/WebAuthn) and live Ethereum (Sepolia) transaction execution.

## Architecture

- **Frontend** (`ts-frontend/web`): Next.js 14 App Router + Tailwind CSS. Uses wagmi and viem.
- **API Gateway** (`go-gateway`): Go (Chi). Handles REST, WebAuthn ceremonies, JWT issuance, gRPC translation.
- **Core Engine** (`rust-core`): Rust (Tokio/Tonic). EVM engine via alloy, double-entry ledger, gRPC server.
- **Database**: PostgreSQL 16 (users, credentials, ledger).
- **Cache**: Redis 7.

## System Diagram
Browser (Next.js) --REST+JWT--> Go Gateway --gRPC--> Rust Core --Alloy--> Alchemy/Sepolia
|
v
Postgres (ledger)


## Prerequisites

- Windows with WSL2 (Ubuntu 22.04+) and Docker Desktop with WSL Integration enabled.
- Rust 1.93+, Go 1.26+, Node.js 22+ / pnpm installed inside WSL2.
- An Alchemy account (or other EVM RPC provider) for a Sepolia endpoint.
- A test wallet funded with Sepolia ETH from a faucet.

## Environment Setup

### Root .env (backend)
JWT_SECRET=<generate with: openssl rand -base64 32>
SEPOLIA_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_KEY
EVM_CHAIN_ID=11155111


### Frontend .env.local (inside ts-frontend/web/)
NEXT_PUBLIC_API_URL=http://localhost:8080
NEXT_PUBLIC_TEST_PRIVATE_KEY=0xYOUR_TEST_WALLET_PRIVATE_KEY


Warning: `NEXT_PUBLIC_TEST_PRIVATE_KEY` is for local development only. Never put a mainnet key here.

## Running

### Backend
docker compose up -d --build
docker compose ps


The Rust server runs SQLx migrations on startup.

### Frontend
cd ts-frontend/web
pnpm install
pnpm dev


Open http://localhost:3000

## Core Workflows

### Authentication (Passkeys)

1. Enter email, click Register.
2. OS prompts for biometrics (Windows Hello / TouchID / FaceID).
3. Go backend validates the WebAuthn signature, creates the user, issues a JWT.
4. Session lasts 24 hours.

### Swap and Broadcast

1. Enter amount, click Swap ETH for USDC.
2. Quote: Rust fetches a live quote from Sepolia.
3. Build: Rust builds an EIP-1559 tx with live nonce and gas price.
4. Simulate: Rust runs eth_call to check for reverts.
5. Click Broadcast to Sepolia.
6. Frontend signs with viem, sends signed hex to the Go gateway.
7. Rust broadcasts via Alloy to Sepolia.
8. Etherscan link appears.

## Security Notes

- Non-Custodial: private keys never sent to the backend; only signed transactions.
- Test Only: `NEXT_PUBLIC_TEST_PRIVATE_KEY` is for local dev. Production should use MetaMask or an MPC wallet.
- Ledger: all value movements recorded in an immutable PostgreSQL double-entry ledger.
- JWT: signed with JWT_SECRET, 24h expiry. Forged tokens return 401.

## Roadmap

- [ ] Replace mock policy-engine with real AML/sanctions screening.
- [ ] Integrate a DEX aggregator (1inch, 0x) for real swap routes.
- [ ] Add Redis-backed rate limiting to the Go gateway.
- [ ] Support multiple EVM chains (Polygon, Arbitrum).
- [ ] Add transaction receipt polling on the frontend after broadcast.
# Deployment

This covers deploying the `stream` contract to Stellar testnet. Mainnet is the
same sequence with `--network mainnet`, but do not deploy to mainnet while the
contract is unaudited (see `SECURITY.md`).

## Prerequisites

- Rust toolchain (auto-applied via `rust-toolchain.toml`, target
  `wasm32v1-none`)
- Stellar CLI **v25.2.0 or newer** (v28.x recommended). `soroban-sdk` v28
  refuses a plain `cargo build`, so `stellar contract build` is required. The
  prebuilt binary from the [stellar-cli releases](https://github.com/stellar/stellar-cli/releases)
  is faster than `cargo install --locked stellar-cli`.
- A funded testnet identity

```bash
stellar keys generate --global alice --network testnet --fund
stellar keys address alice
```

## Build

```bash
make build
# → target/wasm32v1-none/release/stellar_stream.wasm
```

## Deploy and initialize

The contract has no constructor and no dependencies, so the sequence is short:
deploy, then `init` once. Run these in order and keep the printed contract ID.

```bash
#!/usr/bin/env bash
set -euo pipefail

NETWORK=testnet
SOURCE=alice
WASM=target/wasm32v1-none/release/stellar_stream.wasm

# 1. Deploy the wasm. The contract ID is printed and is the only output we keep.
echo "Deploying..."
CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM" \
  --source "$SOURCE" \
  --network "$NETWORK")
echo "STREAM_CONTRACT_ID=$CONTRACT_ID"

# 2. Initialize with the admin address (the deployer, here).
ADMIN=$(stellar keys address "$SOURCE")
echo "Initializing with admin $ADMIN ..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- init \
  --admin "$ADMIN"

echo
echo "================= COPY THESE ================="
echo "STREAM_CONTRACT_ID=$CONTRACT_ID"
echo "VITE_STREAM_CONTRACT_ID=$CONTRACT_ID"
echo "=============================================="
```

The final block is the single source of truth for the address. Paste
`STREAM_CONTRACT_ID` into the application repository's `.env.local`, and into
the hosting platform's environment variables. Both repositories must refer to
the same ID for the same network.

## Verify

```bash
stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
  -- get_stream --stream_id 0
# Expected: an error (StreamNotFound) before any stream is created.
```

Create a stream on testnet to confirm end to end, then check the events in a
block explorer (e.g. `https://stellar.expert/explorer/testnet/contract/$CONTRACT_ID`).

## Recording the deployment

After a successful deploy:

1. Add the contract ID to the release notes for the matching version tag.
2. Add the explorer link to the README's "Deployed contracts" section.
3. Update the application repo's environment variables.

## Redeploying

The contract is immutable and `init` can run only once. A behavioral change
requires a new deployment with a new contract ID; existing streams stay on the
old contract. Migrating live streams is not supported and is out of scope.

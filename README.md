<div align="center">

# Perenna Contracts

![Stellar](https://img.shields.io/badge/Stellar-Soroban-7D00FF?logo=stellar&logoColor=white)

**Continuous, linear payment streams on Stellar — built with Soroban.**

Lock funds once. They unlock to the recipient every second until the stream ends.

[![CI](https://github.com/Perenna-Labs/perenna-contracts/actions/workflows/ci.yml/badge.svg)](https://github.com/Perenna-Labs/perenna-contracts/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![soroban-sdk](https://img.shields.io/badge/soroban--sdk-28.x-blue.svg)](https://crates.io/crates/soroban-sdk)
[![Stellar Wave](https://img.shields.io/badge/Stellar-Wave-7B3FE4.svg)](https://www.drips.network/wave/stellar)

</div>

Perenna is a protocol for streaming payments on Stellar. A sender locks a
token deposit into a Soroban contract together with a start and stop time. From
that point the balance accrues to the recipient continuously, and the recipient
can withdraw whatever has accrued at any moment. If the stream is cancellable,
the sender can stop it early — the recipient keeps everything already earned and
the sender is refunded the rest.

This repository holds the contracts. The application layer (web app, SDK,
indexer) lives in a separate repository.

## Table of Contents

- [How it uses Stellar](#how-it-uses-stellar)
- [Why](#why)
- [Architecture](#architecture)
- [Maintainers](#maintainers)
- [Community](#community)
- [Prerequisites](#prerequisites)
- [Quick start](#quick-start)
- [Contract interface (summary)](#contract-interface-summary)
- [Security](#security)
- [Contributing](#contributing)
- [Changelog](CHANGELOG.md)
- [Contributors](#contributors)

## How it uses Stellar

Perenna is a **Soroban** streaming-payment protocol on **Stellar**: a sender locks a token deposit into the contract with a start and stop time, and the balance accrues to the recipient continuously so it can be withdrawn at any moment. Accrual is exact integer math against the ledger timestamp — no drift, no floating point.

## Why

Recurring payouts on a blockchain are usually either lump-sum transfers or
off-chain bookkeeping. Neither is transparent to the party receiving the money.
Streaming makes the balance visible and claimable at every second, which fits
salary, vesting, subscriptions, and grant disbursement. Stellar is a good fit
because transfers settle fast and cheaply, and Soroban lets the accrual rule
live on-chain instead of in a spreadsheet.

## Architecture

```
perenna-contracts/
├── contracts/
│   └── stream/                 # The streaming contract (single responsibility)
│       └── src/
│           ├── lib.rs          # Contract interface: init, create, balance, withdraw, cancel
│           ├── types.rs        # Stream struct + DataKey storage keys
│           ├── error.rs        # StreamError (contracterror)
│           ├── events.rs       # StreamCreated / TokensWithdrawn / StreamCanceled
│           ├── storage.rs      # Storage access + persistent TTL management
│           ├── admin.rs        # One-time initialization
│           └── test.rs         # Unit + mock-token integration tests
├── docs/
│   ├── CONTRACT_SPEC.md        # Function-by-function contract specification
│   └── DEPLOYMENT.md           # Deploy + initialize sequence
└── scripts/
    └── create-issues.sh        # Seeds the Drips Wave issue backlog
```

There is one contract. It holds two kinds of storage:

- **Instance:** `Admin`, `NextStreamId` (small, shared state)
- **Persistent:** `Stream(id)` for each stream, with TTL extended on every access

Arithmetic is exact integer math. Accrued amount is
`deposit_amount * elapsed / duration`, evaluated against the ledger timestamp,
so there is no drift and no floating point.

## Maintainers

| Name | Role | Contact |
| --- | --- | --- |
| Perenna-Labs | Maintainer | GitHub [@Perenna-Labs](https://github.com/Perenna-Labs) · |

## Community

Questions, ideas, and Wave discussion happen in the project's GitHub
[Discussions](https://github.com/Perenna-Labs/perenna-contracts/discussions)
and Issues. For anything security-related, read `SECURITY.md` first.

## Prerequisites

| Tool | Notes |
| --- | --- |
| **Rust** | stable (pinned in `rust-toolchain.toml`) |
| **Stellar CLI** | build/deploy helper |
| **Make** | convenience targets (`make test/build/clippy/fmt`) |

## Quick start

Prerequisites: Rust (the pinned toolchain in `rust-toolchain.toml` is applied
automatically by `rustup`) and the Stellar CLI for deployment.

```bash
git clone https://github.com/Perenna-Labs/perenna-contracts.git
cd perenna-contracts

make test      # run the test suite
make build     # build the release wasm artifact
make clippy    # lint with warnings as errors
make fmt       # format
make ci        # run all local checks (fmt, clippy, test, check)
```

The release artifact is written to
`target/wasm32v1-none/release/perenna.wasm`. See
`docs/DEPLOYMENT.md` to deploy it and `docs/CONTRACT_SPEC.md` for the full
interface.

## Contract interface (summary)

| Function | Caller | Purpose |
| --- | --- | --- |
| `init(admin)` | deployer | One-time initialization |
| `create_stream(sender, recipient, token, deposit, start, stop, cancelable)` | sender | Lock funds and open a stream; returns the stream id |
| `balance_of(stream_id, address)` | anyone | Claimable amount for the recipient, or refundable amount for the sender |
| `get_stream(stream_id)` | anyone | Full stream state |
| `withdraw(stream_id, amount)` | recipient | Claim accrued funds |
| `cancel(stream_id)` | sender | Stop a cancellable stream; pays the recipient what was earned, refunds the rest |

Full parameters, return types, auth, and events: `docs/CONTRACT_SPEC.md`.

## Security

- **Never commit secrets** — keep keys, seed phrases, and `.env` files out of source control.
- **Testnet values have no real-world value**; treat testnet deployments as experimental.
- **Keys never leave the wallet** — signing is delegated to the user's Stellar wallet; the app does not store secret keys.
- Report vulnerabilities per `SECURITY.md` where present rather than opening a public issue.

## Contributing

Read `CONTRIBUTING.md`. Issues labelled `good first issue` and `Stellar Wave`
are the best entry points. One logical change per pull request, Conventional
Commits, green CI, one maintainer review.

## License

MIT — see `LICENSE`.

## Contributors

Thanks to everyone who has contributed.

<a href="https://github.com/Perenna-Labs/perenna-contracts/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=Perenna-Labs/perenna-contracts" alt="Contributors" />
</a>

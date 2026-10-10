# Security Policy

## Audit status

The StellarStream contracts are **unaudited**. They are testnet software and
must not be used to custody funds of significant value. Treat the code as a
work in progress: peer-reviewed, but never reviewed by an external security
firm.

## Scope

In scope:

- `contracts/stream` — the streaming contract (Soroban/Rust)
- Storage, arithmetic, authorization, and cancellation logic
- TTL / state-archival handling

Out of scope:

- The `stellar-stream-app` repository (report issues there instead)
- The Stellar network, Soroban runtime, or `soroban-sdk` itself — report those
  upstream to the Stellar Development Foundation
- Anything requiring a compromised admin key or a malicious token contract

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability.

- Preferred: open a private report via GitHub Security Advisories —
  https://github.com/Deyanju23/stellar-stream-contract/security/advisories/new

Include, where possible:

- A description of the issue and its impact
- The affected function(s) and a minimal reproduction
- Whether it has been disclosed elsewhere

## What to expect

- Acknowledgement within 72 hours
- An assessment and remediation plan within 7 days
- Credit in the fix's release notes unless you ask to stay anonymous

## Known non-issues

The following are understood and documented, not vulnerabilities:

- There is no upgrade path. The contract is immutable once deployed.
- Anyone may extend the TTL of contract entries; this is how Soroban works.
- `balance_of` returns `StreamCanceled` for a cancelled stream rather than a
  balance. Use `get_stream` for the final state.

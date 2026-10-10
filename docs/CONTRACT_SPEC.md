# StellarStream — Contract Specification

This document is the source of truth for the `stream` contract. If code and
spec disagree, the spec wins until it is deliberately changed. Any change to
behavior must update this file in the same pull request.

- Contract crate: `contracts/stream` (`stellar-stream`)
- Language: Rust, edition 2021
- SDK: `soroban-sdk` 28.x, target `wasm32v1-none`
- Number of contracts: **one**. It has a single responsibility: custody and
  linear accrual of a timed token deposit.

---

## 1. Data model

### `DataKey` (`types.rs`)

| Variant | Storage class | Meaning |
| --- | --- | --- |
| `Admin` | instance | The address set at initialization |
| `NextStreamId` | instance | Monotonic counter for stream ids |
| `Stream(u64)` | persistent | One `Stream` struct per id |

Instance entries share one TTL — the contract instance's TTL — which is **not**
extended automatically. Every instance write calls
`extend_instance_ttl()`. Persistent entries carry their own TTL and are
extended on every access.

### `Stream`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u64` | Stream identifier, assigned sequentially from 0 |
| `sender` | `Address` | Funded the stream; may cancel if allowed |
| `recipient` | `Address` | Accrues and withdraws |
| `token` | `Address` | Token contract address (SAC or Soroban token) |
| `deposit_amount` | `i128` | Total locked at creation, in base units |
| `start_time` | `u64` | Unix seconds when accrual begins |
| `stop_time` | `u64` | Unix seconds when accrual completes |
| `rate_per_second` | `i128` | `deposit_amount / duration`, integer division (informational) |
| `remaining_balance` | `i128` | Tokens still held by the contract for this stream |
| `recipient_withdrawn` | `i128` | Cumulative amount withdrawn by the recipient |
| `is_canceled` | `bool` | Set once cancellation completes |
| `cancelable` | `bool` | Whether the sender may cancel before `stop_time` |

**Accrual rule.** For `current_time = env.ledger().timestamp()`:

```
elapsed = clamp(current_time - start_time, 0, duration)
earned  = deposit_amount * elapsed / duration        // i128, integer division
```

`earned` is monotonic in time and reaches `deposit_amount` exactly at
`stop_time`. There is no floating point anywhere.

---

## 2. Errors (`error.rs`)

| Code | Variant | Raised when |
| --- | --- | --- |
| 1 | `NotInitialized` | Admin read before `init` |
| 2 | `AlreadyInitialized` | `init` called twice |
| 3 | `Unauthorized` | Caller is neither recipient nor sender where one is required |
| 4 | `StreamNotFound` | No `Stream` under the given id |
| 5 | `StreamEnded` | Reserved for ABI stability (unused in contract logic) |
| 6 | `StreamCanceled` | Operation on an already-cancelled stream |
| 7 | `InvalidTimeRange` | `start_time >= stop_time` |
| 8 | `ZeroDeposit` | `deposit_amount <= 0` |
| 9 | `AmountMismatch` | Reserved for ABI stability (unused in contract logic) |
| 10 | `WithdrawAmountTooHigh` | `amount > available` |
| 11 | `NotCancelable` | `cancel` on a stream with `cancelable = false` |
| 12 | `MathOverflow` | Any checked arithmetic overflows |

Codes are stable and part of the ABI. Do not renumber. Removing a variant is a
breaking change.

---

## 3. Events (`events.rs`)

All emitted via `#[contractevent]`.

| Event | Fields | Emitted by |
| --- | --- | --- |
| `StreamCreated` | `sender, recipient, stream_id, token, deposit_amount, start_time, stop_time` | `create_stream` |
| `TokensWithdrawn` | `recipient, stream_id, amount, remaining_balance` | `withdraw` |
| `StreamCanceled` | `stream_id, sender_refund, recipient_payout` | `cancel` |

Every state-changing function emits exactly one event.

---

## 4. Functions

### `init(admin: Address) -> Result<(), StreamError>`

- **Auth:** `admin.require_auth()`. The admin must authorize its own
  appointment.
- **Effect:** stores `Admin`, sets `NextStreamId = 0`. Fails with
  `AlreadyInitialized` if `Admin` already set.
- **Events:** none.
- **Called once** after deployment.

### `create_stream(...) -> Result<u64, StreamError>`

```
create_stream(
  sender: Address,
  recipient: Address,
  token_addr: Address,
  deposit_amount: i128,
  start_time: u64,
  stop_time: u64,
  cancelable: bool,
) -> u64
```

- **Auth:** `sender.require_auth()`.
- **Validation:** `deposit_amount > 0`, else `ZeroDeposit`;
  `start_time < stop_time`, else `InvalidTimeRange`.
- **Effects:** transfers `deposit_amount` from `sender` to the contract,
  assigns the next id, stores the `Stream`, increments `NextStreamId`.
  Returns the new id.
- **Events:** `StreamCreated`.
- **Notes:** `start_time` may be in the past (retroactive vesting is allowed).

### `balance_of(stream_id: u64, target: Address) -> Result<i128, StreamError>`

- **Auth:** none. Read-only; any address may query.
- **Returns:** for `recipient`: `earned - recipient_withdrawn`; for `sender`:
  `deposit_amount - earned`. Any other address → `Unauthorized`.
- **Failure:** `StreamNotFound`; `StreamCanceled` if the stream was cancelled.
- **Side effect:** extends the persistent TTL of the stream entry.

### `get_stream(stream_id: u64) -> Result<Stream, StreamError>`

- **Auth:** none.
- **Returns:** the full `Stream`. Extends the persistent TTL.

### `withdraw(stream_id: u64, amount: i128) -> Result<(), StreamError>`

- **Auth:** `recipient.require_auth()`.
- **Validation:** `amount <= earned - recipient_withdrawn`, else
  `WithdrawAmountTooHigh`; `StreamCanceled` if already cancelled;
  `StreamNotFound` if missing.
- **Effects:** increments `recipient_withdrawn`, decrements
  `remaining_balance`, transfers `amount` from the contract to the recipient.
  State is written before the token transfer (checks-effects-interactions).
- **Events:** `TokensWithdrawn`.

### `cancel(stream_id: u64) -> Result<(), StreamError>`

- **Auth:** `sender.require_auth()`.
- **Validation:** `cancelable` must be true, else `NotCancelable`;
  not already cancelled, else `StreamCanceled`.
- **Effects:** computes `earned`; pays the recipient
  `earned - recipient_withdrawn` and refunds the sender
  `deposit_amount - earned`; sets `is_canceled = true` and
  `remaining_balance = 0`.
- **Events:** `StreamCanceled`.

---

## 5. Invariants

1. `contract token balance >= Σ remaining_balance` over all live streams.
2. For any live stream and any timestamp:
   `balance_of(recipient) + balance_of(sender) == deposit_amount`.
3. `earned` is non-decreasing in time and equals `deposit_amount` at
   `stop_time` and after.
4. `recipient_withdrawn` never exceeds `earned`.
5. After `cancel` or a full withdrawal, the contract holds 0 tokens for that
   stream.

---

## 6. Hardening backlog

These are known gaps. Each becomes an issue (see `scripts/create-issues.sh`).
Fixed in the initial hardening pass: instance-storage TTL (was H1) and
unauthenticated `init` (was H2). The unused `admin::require_admin` was deleted.
`Stream.rate_per_second` is **retained**: the contract does not read it, but it
is part of the public `get_stream` ABI consumed by the application SDK, so
removing it is a breaking change across repos and must be coordinated.

### H1 — Auth-failure tests missing (MEDIUM)

Most tests call `env.mock_all_auths()`. `init` now has a rejection test, but no
test proves that a non-recipient cannot `withdraw` or a non-sender cannot
`cancel`. Add rejection tests.

### H2 — Events are never asserted (LOW)

Add tests that check `StreamCreated`, `TokensWithdrawn`, and `StreamCanceled`
are published with the expected payload.

### H3 — Redundant state (`remaining_balance`) (LOW)

`remaining_balance == deposit_amount - recipient_withdrawn` while a stream is
live, so it is derivable. Kept for event payloads and readability; document the
invariant or remove the field.

### H4 — `StreamEnded` and `AmountMismatch` are reserved (LOW)

Unused error codes. Keep them for ABI stability or remove before v1.0.

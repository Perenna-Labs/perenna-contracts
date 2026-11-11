# Contributing to StellarStream Contracts

Thanks for your interest. This repository is part of the StellarStream project
and takes part in the Stellar Drips Wave program. Issues are labelled so new
contributors can find work at a comfortable level.

## Before you start

Read `docs/CONTRACT_SPEC.md`. It defines the intended behavior of every public
function. If your change alters that behavior, update the spec in the same pull
request.

## Workflow

1. Open or claim an issue before writing code. Comment on the issue to avoid
   duplicate work.
2. Branch from `main`: `git checkout -b feat/withdraw-all` (or `fix/…`,
   `docs/…`, `test/…`).
3. Make small, focused commits using Conventional Commits:
   `type(scope): description`
   - types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `ci`, `style`
   - scope: `stream` for contract code
   - example: `fix(stream): require admin auth in init`
4. Push early and often. Do not batch a week of work into one push.
5. Open a pull request against `main`. At least one maintainer review and green
   CI are required before merge. Do not merge your own PR.

## Standards

- Rust edition 2021, `soroban-sdk` 28.x, target `wasm32v1-none`.
- No floating-point math. All amounts are `i128` in the token's base unit.
- No `unwrap()` or `expect()` outside of `#[cfg(test)]` code. Return a
  `StreamError` instead. Descriptive defaults like `unwrap_or` are permitted
  for fallback values (e.g. initial counter state) when justified by a comment
  at the call site.
- Use checked arithmetic (`checked_add`, `checked_mul`, …) and map failure to
  `StreamError::MathOverflow`.
- Every state change emits an event. Every public function has a doc comment.
- Storage keys live in `DataKey`. Extend TTL on every persistent read/write.
- Keep one contract per responsibility. Do not add speculative functions.

Run the full check locally before opening a PR:

```bash
make ci
```

## Tests

New behavior needs tests. Prefer testing the invariant, not just the happy
path: e.g. `recipient_balance + sender_balance == deposit` at every timestamp.
Auth paths must be tested — a call that should be rejected must be shown to be
rejected.

## Reporting bugs and security issues

Ordinary bugs: open an issue using the bug template.

Security issues: follow `SECURITY.md`. Do not open a public issue.

## License

By contributing you agree that your contributions are licensed under the MIT
License in `LICENSE`.

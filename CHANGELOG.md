# Changelog

All notable changes to the Perenna streaming contracts will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Contract balance invariant validation across full stream lifecycle.
- Regression tests for boundary conditions at `start_time` and `stop_time`.
- Validation rejecting `create_stream` when sender equals recipient.
- Validation rejecting zero, negative, and non-positive withdrawal requests.
- Initialization checks preventing stream operations prior to contract initialization.

### Changed
- Event emission payloads expanded with complete contextual stream parameters.
- Re-landing and conflict resolution on streaming token accounting.

## [0.1.0] - 2026-10-09

### Added
- Core streaming contract implementation on Soroban (`contracts/stream`).
- Functions for `create_stream`, `withdraw`, `cancel`, and `get_stream`.
- Pro-rata token distribution calculation across configured start and stop timestamps.
- Admin initialization with secure parameter authentication.
- Comprehensive unit test suite with mock auth and token balances.

# Contract Structure Refactor Design

## Goal

Improve the code structure of contracts, shared libraries, and tests without changing protocol behavior, error codes, binary names, or public test scenarios.

The refactor should make module ownership clear:

- `run.rs` expresses transaction-level flow.
- Parser modules decode and validate raw cell data.
- Cell scanner modules find CKB cells and enforce uniqueness/location rules.
- Authority modules evaluate authorization.
- Access-list modules handle shard semantics and range coverage.

## Scope

Refactor only code that is already part of the current mainline contracts:

- `contracts/sudt`
- `contracts/xudt`
- `contracts/sudt-meta`
- `contracts/xudt-meta`
- `contracts/access-list`
- `lib/script-utils`
- `lib/types`
- `tests`

This design does not introduce new protocol features, new molecule schemas, or new cross-contract runtime dependencies.

## Approach

Use conservative in-place decomposition instead of moving shared parsing into `lib/script-utils`.

Rationale: some contract parsers intentionally avoid linking the generated molecule bindings in RISC-V builds because of ckb-std/version/linking constraints. Keeping parsing inside each contract crate preserves current binary behavior while still making files smaller and interfaces clearer.

## Contract Refactor

### `contracts/xudt-meta`

Split `src/meta_cell.rs` into a `src/meta_cell/` module:

- `mod.rs`: public API used by `run.rs` and `update.rs`.
- `parser.rs`: `XudtMeta`, `ScriptAttr`, strict table parsing, field length validation.
- `config.rs`: config flag constants and helpers.
- `token.rs`: xUDT script matching, initial supply summing, same-token-cell checks.
- `access_list.rs`: access-list shard parsing, shard ordering, full-domain checks.
- `cells.rs`: group meta loading and meta lock whitelist validation.

### `contracts/sudt`

Split `src/meta.rs` into a `src/meta/` module:

- `mod.rs`: public API used by `run.rs`.
- `parser.rs`: `SudtMeta`, `ScriptAttr`, strict table parsing.
- `cells.rs`: meta hash arg loading, amount collection, visible meta lookup, lock whitelist.
- `authority.rs`: authority evaluation and required-authority helper.
- `supply.rs`: mint/burn supply-state validation.

### `contracts/access-list`

Split `src/meta.rs` into a `src/meta/` module:

- `mod.rs`: public `MetaContext`, config helpers, and orchestration.
- `parser.rs`: minimal xUDT meta parser needed by access-list.
- `cells.rs`: meta lookup and lock whitelist validation.
- `authority.rs`: access authority evaluation.

### Other Contracts

Keep `sudt-meta` and `xudt` mostly as-is unless imports need minor cleanup. They are already split into clearer files than the modules above.

## Libraries

Do not move contract-specific parsers into `lib/types` or `lib/script-utils` in this pass.

Allowed cleanup:

- Clarify names or comments where helper semantics are currently ambiguous.
- Keep `ScriptError::SyscallUnknown` unchanged, since `script-utils` directly receives `ckb_std::error::SysError` in scanner helpers.

## Tests

Keep existing test behavior and names stable unless a helper moves.

Allowed cleanup:

- Move repeated builder/fixture helpers only if it reduces coupling without rewriting scenarios.
- Avoid large test-file churn during contract-module refactoring.

## Verification

Required checks:

- `RUSTUP_TOOLCHAIN=1.92.0 cargo fmt --check`
- `RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="-- --nocapture"`
- `RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug`
- `git diff --check`

## Non-Goals

- No behavior changes.
- No error code changes.
- No new access-list semantics.
- No new supply semantics.
- No switch to generated molecule bindings inside RISC-V contract parser code.

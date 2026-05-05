# standard-udt-contracts

Enhanced sUDT, Enhanced xUDT, and AccessList contracts for CKB.

## Contracts

- `enhanced-sudt-meta`: validates sUDT Meta creation and updates.
- `enhanced-sudt`: validates sUDT transfer, mint, protocol burn, and user destruction.
- `enhanced-xudt-meta`: validates xUDT Meta creation, updates, access mode governance, and extension configuration.
- `enhanced-xudt`: validates xUDT transfer, mint, protocol burn, user destruction, paused mode, AccessList, and extensions.
- `access-list`: validates xUDT AccessList shard updates.

## Build

```bash
make build MODE=debug
```

Build output is written to `build/debug/`. Use `MODE=release` for release binaries.

## Test

```bash
MODE=debug make test CARGO_ARGS="-- --nocapture"
```

Tests use `ckb-testtool` and expect the matching contract binaries under `build/{debug,release}`. Run `make build` first if a binary is missing.

## Supply Tracking

Supply tracking is optional and configured at token creation with `config_flags.bit0`.

- `0`: untracked supply. `current_supply` must be zero.
- `1`: tracked supply. `current_supply` is updated only by authorized mint and protocol burn.

User destruction is allowed without `mint_authority` and does not reduce `current_supply`. Fixed supply is achieved by minting the target amount and irreversibly removing `mint_authority`.

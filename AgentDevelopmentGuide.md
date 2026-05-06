# Agent Development Guide

This guide is for coding agents working on this repository. It explains how to
change the project without breaking contract boundaries, test assumptions, or
CKB-specific build behavior.

Read this together with:

- `agents.md` for hard repository constraints;
- `README.md` for project overview and commands;
- `Architecture.md` for contract responsibility boundaries.

## First Principles

The repository implements multiple cooperating CKB type scripts. Most bugs come
from putting a check in the wrong script or duplicating another script's
responsibility.

Use these rules before editing:

- A type script validates the state it owns.
- Consumers validate only the fact or proof they need from another type script.
- Metadata scripts validate metadata state.
- Token scripts validate token movement.
- AccessList validates AccessList shard lifecycle.
- Shared libraries provide parsing and mechanics, not business-rule ownership.

When in doubt, identify the owner of the invariant before writing code.

## Working With The Worktree

The worktree may already be dirty.

Before editing:

```bash
git status --short
```

Do not revert changes you did not make. If unrelated files are dirty, leave
them alone. If a dirty file overlaps with your task, inspect it and work with
the existing changes.

Prefer focused commits:

- one behavior change per commit;
- documentation-only changes in their own commit when practical;
- avoid mixing refactors with semantic changes unless the refactor is required.

## Repository Map

```text
contracts/
  sudt/            sUDT token type script
  sudt-meta/       sUDT metadata type script
  xudt/            xUDT token type script
  xudt-meta/       xUDT metadata type script
  access-list/     AccessList shard type script
lib/
  types/           Molecule-backed parsed metadata and AccessList types
  script-utils/    shared no-std script helpers
tests/
  src/             ckb-testtool integration tests
  plugins/         test extension and authority scripts
docs/superpowers/
  specs/           design notes
  plans/           implementation plans
```

New on-chain script crates should be generated through the repository template
flow. Do not hand-roll contract crate skeletons.

## Script Template Basics

On-chain script crates in this repository are based on
`ckb-script-templates`. Use the root Makefile wrapper instead of calling
`cargo generate` manually unless you have a concrete reason.

Generate a new contract under `contracts/`:

```bash
make generate CRATE=my-contract
```

What the wrapper does:

- calls `cargo generate` with the `contract` template from
  `https://github.com/cryptape/ckb-script-templates`;
- writes the generated crate under `contracts/<CRATE>`;
- appends the generated crate to the workspace `Cargo.toml`;
- moves generated template tests into the repository test layout when present;
- removes `.cargo-generate` template metadata after extraction.

The root Makefile exposes these knobs:

```text
CRATE        generated crate name
TEMPLATE     template name, default: contract
DESTINATION  output parent directory, default: contracts
```

Examples:

```bash
make generate CRATE=my-contract
make generate CRATE=my-plugin DESTINATION=tests/plugins
```

Only put production on-chain contracts under `contracts/`. Test-only scripts
belong under `tests/plugins/`, but they should still use the template flow so
their `Cargo.toml`, `Makefile`, `main.rs`, allocator setup, and target build
settings match the rest of the repository.

Generated contract crates usually contain:

- `Cargo.toml`;
- `Makefile`;
- `src/main.rs`;
- `src/lib.rs`;
- `src/entry.rs`;
- `src/error.rs`;
- a generated README stub.

Keep the template plumbing intact:

- do not replace `src/main.rs` unless the entry/allocator contract changes;
- put validation behavior in `src/entry.rs` and focused modules;
- add contract-local modules under `src/`;
- add shared parsing to `lib/types`;
- add shared CKB helper logic to `lib/script-utils`;
- update the root `Makefile` dependency ordering only when the new script needs
  another script's code hash at build time.

The per-contract template Makefile builds RISC-V binaries with:

```text
cargo build --target=riscv64imac-unknown-none-elf
```

The root Makefile is responsible for copying built binaries into
`build/<MODE>/` and passing dependent code hashes such as `SUDT_CODE_HASH`,
`XUDT_CODE_HASH`, and `ACCESS_LIST_CODE_HASH`.

Native simulators can be generated with:

```bash
make generate-native-simulator CRATE=my-contract
```

Do this only when the project needs a simulator package. Most contract changes
should be covered by script unit tests and `ckb-testtool` integration tests.

## Contract Responsibility Checklist

Use this checklist before moving validation logic.

### `sudt`

Owns:

- sUDT amount conservation;
- mint detection;
- protocol burn detection;
- user destruction detection;
- mint authority for mint and protocol burn;
- supply coordination with visible sUDT metadata.

Does not own:

- metadata cell shape;
- metadata type-id creation;
- metadata authority;
- metadata output lock policy.

### `sudt-meta`

Owns:

- sUDT metadata type-id creation;
- metadata group uniqueness;
- metadata decoding and size limits;
- output metadata lock whitelist;
- immutable supply-tracking mode;
- tracked supply consistency;
- metadata field changes;
- metadata and mint authority checks for metadata updates.

Does not own:

- token amount conservation;
- holder transfer rules;
- user destruction classification.

### `xudt`

Owns:

- xUDT amount conservation;
- mint detection;
- protocol burn detection;
- user destruction detection;
- paused-mode enforcement;
- whitelist and blacklist proof checks;
- extension execution;
- supply coordination with visible xUDT metadata.

Does not own:

- AccessList shard lifecycle;
- access-mode transition governance;
- metadata type-id creation;
- extension list governance.

### `xudt-meta`

Owns:

- xUDT metadata type-id creation;
- metadata group uniqueness;
- metadata decoding and size limits;
- output metadata lock whitelist;
- immutable supply-tracking mode;
- tracked supply consistency;
- pause-state governance;
- access-mode transition governance;
- access authority changes;
- extension list changes.

Does not own:

- individual transfer proof checks;
- AccessList shard insert/delete/split/merge details;
- token amount conservation.

### `access-list`

Owns:

- shard range validity;
- nibble alignment;
- entry sorting and uniqueness;
- entry range containment;
- create, update, split, merge, and destroy flows;
- local replacement consistency;
- full-domain create/replacement/destroy requirements;
- AccessList update authority.

Does not own:

- token transfer policy;
- whitelist vs blacklist interpretation during transfer;
- metadata supply state.

## Shared Library Rules

### `lib/types`

Put shared data interpretation here when both contracts and tests need it.

Good fits:

- Molecule schema wrappers;
- strict parsers;
- metadata size checks;
- config flag validation;
- authority shape validation;
- AccessList range and entry shape validation.

Avoid:

- transaction-specific business rules;
- CKB cell scans;
- contract-local error-code policy.

### `lib/script-utils`

Put shared script mechanics here.

Good fits:

- amount decoding;
- token delta scanning;
- authority checking mechanics;
- script hash matching;
- syscall error normalization.

Avoid:

- deciding when authority is required;
- deciding which contract owns an invariant;
- AccessList lifecycle policy;
- metadata update policy.

## Metadata Parsing

Use parsed types from `lib/types`.

Do not hand-parse Molecule tables in contracts or tests when a shared parser is
available. Hand parsing is acceptable only for deliberately malformed test data
where the point of the test is to bypass normal builders.

Preferred pattern:

```rust
let meta = XudtMeta::from_slice(&data)?;
```

Avoid duplicating parsed structs inside contract-local utility modules. If a
parsed representation is generally useful, move it to `lib/types`.

## Authority Rules

Authority descriptors are data. Authority requirement is policy.

Shared utilities can answer:

- is the authority shape valid?
- is the referenced lock/type present?
- did the dynamic-linking or spawn authority approve?

Contracts must decide:

- whether authority is needed for this operation;
- which authority field is primary;
- whether `mint_authority` is allowed as fallback.

Current policy:

- `mint_authority` controls mint, protocol burn, supply changes, mint authority
  changes, and xUDT extension list changes;
- `mint_authority` can also authorize metadata and access-state updates as a
  fallback;
- user destruction does not require mint authority because it does not consume
  the metadata input cell.

## Supply Rules

Supply tracking is optional and immutable after token creation.

When `CONFIG_SUPPLY_TRACKED` is unset:

- metadata `current_supply` must be zero;
- metadata updates must not create nonzero supply state.

When `CONFIG_SUPPLY_TRACKED` is set:

- mint must increase metadata supply by the token delta;
- protocol burn must decrease metadata supply by the token delta;
- user destruction must not decrease metadata supply;
- metadata-only transactions must not change supply unless matching token cells
  are present.

Negative token deltas are classified by metadata participation. If the
transaction consumes the token's metadata input cell, the token script must
treat the negative delta as protocol burn and require `mint_authority`. If no
metadata input cell is consumed, the negative delta is user destruction.

Both token and metadata scripts participate. Do not try to make only one side
responsible for tracked supply.

## AccessList Rules

Whitelist and blacklist use the same shard structure. They differ only at
interpretation time:

- whitelist requires membership proof;
- blacklist requires non-membership proof.

Access-mode transitions belong to `xudt-meta`.

Shard lifecycle belongs to `access-list`.

Ordinary shard updates can be local. Do not require full-domain inputs and
outputs for normal insert/delete/split/merge updates. Full-domain requirements
are reserved for global access-mode changes and active AccessList destruction.

## Extension Rules

xUDT extension configuration belongs to `xudt-meta`.

xUDT extension execution belongs to `xudt`.

When changing extensions:

- preserve sorted and unique extension metadata;
- require mint authority for extension list changes;
- keep extension execution fail-closed;
- pass operation context consistently to dynamic-linking and spawn extensions.

## Error Code Rules

Keep error names specific enough to explain the failing invariant.

Guidelines:

- syscall mappings should preserve syscall category where possible;
- argument/group-shape failures can use argument errors;
- duplicate cells should use duplicate-specific errors;
- metadata decoding failures should not be reported as authority or supply
  failures;
- supply mismatch and amount overflow should map to supply errors;
- authority denial and invalid authority shape should remain distinguishable
  when the contract error surface supports it.

When removing an error variant, check numeric codes. Either renumber within a
clear category or leave intentional gaps for future variants.

## Build And Test Workflow

Testing in this repository has two layers:

- normal Rust unit tests for shared libraries and host-buildable code;
- `ckb-testtool` integration tests that execute compiled RISC-V script
  binaries from `build/<MODE>/`.

The integration tests do not automatically rebuild on-chain binaries. If a
contract changes, rebuild first.

### Prepare The Toolchain

Install the RISC-V target once:

```bash
make prepare
```

This runs:

```bash
rustup target add riscv64imac-unknown-none-elf
```

The build also needs a Clang toolchain. The Makefile discovers it through
`scripts/find_clang`.

### Fast Host Checks

Use host checks when changing shared libraries or when you only need compiler
feedback:

```bash
make check
cargo test -p standard-udt-types
cargo test -p standard-udt-script-utils
```

These do not prove the RISC-V contract binaries used by integration tests are
up to date.

### Contract Integration Test Loop

For contract changes, build RISC-V binaries before integration tests:

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test
```

For focused tests:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug cargo test -p tests xudt_meta_ -- --nocapture
```

Other useful focused filters:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug cargo test -p tests sudt_meta_ -- --nocapture
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug cargo test -p tests access_list_ -- --nocapture
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug cargo test -p tests xudt_ -- --nocapture
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug cargo test -p tests sudt_ -- --nocapture
```

Use `MODE=debug` when tests rely on debug-only allowances such as the
ckb-testtool always-success lock. Use `MODE=release` when validating release
binary behavior.

The `MODE` environment variable used by tests selects which build directory the
test loader reads from:

```text
MODE=debug    tests load build/debug/*
MODE=release  tests load build/release/*
```

If tests fail with missing binaries, run the matching build first:

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
```

If a test keeps returning an old error code after code changes, suspect stale
RISC-V binaries and rebuild with `make build MODE=debug`.

### Release Checks

Use release mode when validating production-like binaries:

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=release
RUSTUP_TOOLCHAIN=1.92.0 MODE=release make test
```

Release binaries do not include `#[cfg(debug_assertions)]` branches. If behavior
depends on debug-only test locks, it should not be expected to pass in release
mode.

### Formatting And Static Checks

Run whitespace checks before committing:

```bash
git diff --check
```

Format Rust code when source files changed:

```bash
cargo fmt
```

Run Clippy when touching shared libraries or broad contract logic:

```bash
make clippy
```

For documentation-only changes, `git diff --check` is usually enough unless the
docs describe behavior that should be verified against tests or code.

## Test Writing Guidelines

Integration tests should describe one invariant per test.

Prefer:

- focused transaction builders;
- shared helpers under `tests/src/test_helpers` for reusable construction;
- explicit expected error codes for stable contract errors;
- pass tests for accepted flows and fail tests for rejected flows.

Avoid:

- copying large helper blocks between test files;
- relying on whichever script happens to fail first when two scripts could
  reject the transaction;
- asserting only generic failure when the error code is part of the contract
  surface;
- building malformed Molecule bytes by hand when normal builders can express
  the state.

When a transaction involves multiple scripts, isolate the intended failure.
For example, if testing metadata supply mismatch, make sure the token type
script itself would otherwise accept the token delta.

## Refactoring Guidelines

Rename modules when the name hides the responsibility boundary. Good names in
this repo describe owned state or protocol role, not just data shape.

Examples:

- `state` for metadata state loading and creation validation;
- `meta` for read-only current metadata context in token scripts;
- `access` for transfer-time access checks;
- `shards` for AccessList shard lifecycle logic.

Keep mechanical renames separate from semantic changes when practical.

## Documentation Guidelines

Use root-level docs for broad project orientation:

- `README.md`: overview, commands, high-level usage;
- `Architecture.md`: responsibility boundaries and flows;
- `AgentDevelopmentGuide.md`: agent workflow and development practices.

Use `docs/superpowers/specs` and `docs/superpowers/plans` for design and plan
artifacts tied to a specific feature or refactor.

Keep docs factual. Avoid documenting intended future behavior as current
behavior unless the text clearly says it is planned.

## Before Finishing A Task

Check:

- Did you preserve script responsibility boundaries?
- Did you avoid duplicating another script's invariant?
- Did you use shared parsers instead of local ad hoc parsing?
- Did you update tests for behavior changes?
- Did you run the narrowest meaningful test?
- Did you run broader tests for shared behavior?
- Did you run `git diff --check`?
- Did you leave unrelated dirty worktree changes untouched?

Report exactly what was changed and exactly what was verified.

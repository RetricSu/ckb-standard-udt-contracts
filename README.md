# standard-udt-contracts

Rust implementations of sUDT, sUDT metadata, xUDT, xUDT metadata, and xUDT
AccessList contracts for CKB.

This repository contains the on-chain scripts, shared metadata types, shared
script utilities, and ckb-testtool integration tests used to validate the
contracts as a coordinated token system.

## What Is Included

- `sudt`: sUDT type script.
- `sudt-meta`: metadata type script for sUDT.
- `xudt`: xUDT type script.
- `xudt-meta`: metadata type script for xUDT.
- `access-list`: access-list shard type script used by xUDT.
- `lib/types`: shared Molecule-backed metadata and access-list parsers.
- `lib/script-utils`: shared script-side helpers for authorities, supply
  deltas, token cell scanning, and error conversion.
- `tests`: integration tests and helper scripts using `ckb-testtool`.

## Repository Layout

```text
contracts/
  access-list/     AccessList shard lifecycle and full-domain validation
  sudt/            sUDT token type script
  sudt-meta/       sUDT metadata type script
  xudt/            xUDT token type script
  xudt-meta/       xUDT metadata type script
lib/
  script-utils/    Shared no-std script helpers
  types/           Shared metadata schemas, generated Molecule types, parsers
tests/
  plugins/         Dynamic-linking and spawn test fixtures
  src/             ckb-testtool integration tests and builders
udtx/              UDT token operations CLI (issue, transfer, chain management)
scripts/
  ckb-data-hash    Helper used by the Makefile to pass dependent code hashes
  find_clang       Helper used for C/RISC-V plugin builds
```

## Contract Responsibilities

### `sudt-meta`

`sudt-meta` owns the lifecycle and invariants of an sUDT metadata cell.

It validates:

- type-id creation for new metadata cells;
- metadata uniqueness inside the type script group;
- output metadata lock whitelist;
- sUDT metadata decoding and size limits;
- immutable supply-tracking mode;
- initial tracked supply on creation;
- supply delta consistency on update;
- zero-supply metadata destruction in tracked-supply mode;
- mint authority changes;
- metadata field changes;
- metadata authority checks, with `mint_authority` as a fallback authority.

It does not validate transfers or token ownership. Those are handled by `sudt`.

### `sudt`

`sudt` owns token cell movement for a single sUDT type.

It validates:

- ordinary transfers;
- mint classification and untracked-supply mint authority;
- protocol-burn classification;
- user destruction;
- tracked supply coordination with current metadata context.

Supply is changed by token cell deltas, but the tracked `current_supply` value
is stored in the metadata cell. When tracked supply changes, `sudt` and
`sudt-meta` cooperate: the token type script validates token movement and
requires the tracked metadata input context, while the metadata type script
validates that the metadata supply matches the transaction token delta and that
`mint_authority` authorizes the supply change.

An sUDT metadata cell can be destroyed to reclaim CKB only when supply tracking
is enabled, `current_supply` is zero, and `mint_authority` authorizes the
transaction.

### `xudt-meta`

`xudt-meta` owns xUDT metadata, access-mode configuration, pause state, and
extension configuration.

It validates:

- type-id creation for new metadata cells;
- metadata uniqueness inside the type script group;
- output metadata lock whitelist;
- xUDT metadata decoding and size limits;
- immutable supply-tracking mode;
- initial tracked supply on creation;
- supply delta consistency on update;
- zero-supply metadata destruction in tracked-supply mode;
- access mode transitions;
- pause-state changes;
- access authority changes;
- extension list changes;
- mint authority and metadata authority checks;
- full-domain AccessList requirements when enabling, disabling, or replacing
  access mode.
- full-domain AccessList input and empty bound AccessList output requirements
  when destroying active-access metadata.

It does not validate individual transfer access proofs. Those are handled by
`xudt` and `access-list`.

### `xudt`

`xudt` owns token cell movement for a single xUDT type.

It validates:

- ordinary transfers;
- mint classification and untracked-supply mint authority;
- protocol-burn classification;
- user destruction;
- paused mode;
- whitelist and blacklist checks;
- access-list inclusion and non-inclusion proofs;
- extension execution;
- tracked supply coordination with current metadata context.

For user destruction, xUDT can allow holders to destroy their own token cells
without mint authority. A negative token delta is a protocol burn when the same
transaction consumes the token's metadata input cell; without a metadata input
cell, the negative delta is user destruction. Protocol burns, minting, and
supply-changing metadata updates still require the proper authority path.

An xUDT metadata cell can be destroyed to reclaim CKB only when supply tracking
is enabled, `current_supply` is zero, and `mint_authority` authorizes the
transaction. If access mode is enabled, the transaction must also consume
full-domain AccessList inputs and leave no AccessList outputs bound to the
destroyed metadata cell.

### `access-list`

`access-list` owns AccessList shard structure and shard lifecycle.

It validates:

- AccessList shard decoding;
- shard range validity and nibble alignment;
- sorted and unique entries;
- entries staying inside the shard range;
- create, update, split, merge, and destroy lifecycle operations;
- same-contiguous-coverage consistency for partial updates;
- full-domain requirements for AccessList creation, mode replacement, and
  active-mode destruction;
- access authority checks, with `mint_authority` as a fallback authority.

The token and metadata scripts use AccessList cells, but they do not own the
linked-list/shard invariants. Those invariants belong to the `access-list` type
script.

For ordinary AccessList updates, input shards and output shards must cover the
same continuous range. Within that range, entries may be inserted or removed
while shards are split or merged in the same transaction. The parsed
`AccessListShard` type still enforces that every shard's entries are inside the
range, sorted, and unique.

## Metadata Model

Metadata is encoded with Molecule schemas under `lib/types/src/schemas`.
The Rust-facing parsed types live under `lib/types/src/metadata`.

### sUDT Metadata Fields

`SudtMeta` contains:

- `config_flags`;
- `current_supply`;
- `decimals`;
- `name`;
- `symbol`;
- `uri`;
- `extra_data`;
- `mint_authority`;
- `metadata_authority`.

### xUDT Metadata Fields

`XudtMeta` contains all sUDT metadata fields plus:

- `access_authority`;
- `extensions`.

### Size Limits

The shared type parser enforces these limits:

- metadata name: 1024 bytes;
- metadata symbol: 128 bytes;
- metadata URI: 2048 bytes;
- metadata extra data: 16 KiB;
- extensions: 16 entries;
- access-list shard entries: 4096 entries.

The extension limit is intentionally small. Each xUDT extension is an
additional script execution on every covered token operation, so token designs
that need many independent checks should normally aggregate policy inside one
extension script instead of appending many small extensions.

## Config Flags

Config flags are defined in `lib/types/src/metadata/config.rs`.

| Bit | Constant | Applies To | Meaning |
| --- | --- | --- | --- |
| `0b0000_0001` | `CONFIG_SUPPLY_TRACKED` | sUDT, xUDT | Track total supply in metadata. |
| `0b0000_0010` | `CONFIG_ACCESS_ENABLED` | xUDT | Enable AccessList checks. |
| `0b0000_0100` | `CONFIG_ACCESS_WHITELIST` | xUDT | Interpret AccessList as a whitelist. Without this bit, enabled access mode is blacklist mode. |
| `0b0000_1000` | `CONFIG_PAUSED` | xUDT | Reject transfer and mint flows while allowing user destruction. |

sUDT only accepts `CONFIG_SUPPLY_TRACKED`. xUDT accepts all four flags. xUDT
rejects whitelist mode unless access mode is enabled.

## xUDT Extensions

xUDT extensions are optional validation hooks stored in xUDT metadata. They are
configured through `xudt-meta` and executed by `xudt` during token operations.

The metadata extension list contains up to 16 entries. Each entry has:

- an extension type, using the same ordering as authority locations:
  - `0 = InputLock`;
  - `1 = InputType`;
  - `2 = OutputType`;
  - `3 = DynamicLinking`;
  - `4 = Spawn`;
- a CKB script. For presence extensions, the script hash is the required
input/output script hash. For executable extensions, the script locates and
  configures the extension code, and its args are passed as extension data.

The extension list must be sorted by `(extension_type, script_hash)` and must
not contain duplicates. Changing the extension list requires mint authority.

Presence extensions do not execute extra code. `xudt` only requires that the
transaction contains the configured script hash in the matching source:

- `InputLock`: input lock scripts;
- `InputType`: input type scripts;
- `OutputType`: output type scripts.

The matching input or output script is then executed by normal CKB transaction
validation. Cell dep scripts and output lock scripts are not presence extension
types because they are not executed by CKB as part of the transaction's script
groups.

Executable extensions are `DynamicLinking` and `Spawn`. `xudt` executes all
configured extensions in order for:

- transfer: `operation = 0`, `mint_authority_checked = none`;
- mint: `operation = 1`, `mint_authority_checked = checked`;
- protocol burn: `operation = 2`, `mint_authority_checked = none`.

Pure user destruction skips extension execution, matching the holder-owned
destruction path.

Dynamic-linking extensions must export:

```c
int udt_validate(
    const unsigned char *script_hash,
    unsigned char operation,
    unsigned char extension_index,
    const unsigned char *extension_data,
    unsigned long extension_data_len,
    unsigned char mint_authority_checked
);
```

The return value must be `0` to accept the transaction. Any non-zero return
value, missing extension code, load failure, spawn failure, or malformed
extension metadata rejects the xUDT transaction.

Spawn extensions receive four argv values:

```text
operation extension_index hex(extension_data) mint_authority_checked
```

`mint_authority_checked` is encoded as:

- `1`: mint authority was checked for the current mint path;
- `0`: reserved for an explicitly unchecked mint-authority context;
- `2`: no mint-authority context is available.

## Supply Tracking

Supply tracking is optional and fixed at token creation time.

- Untracked supply: `CONFIG_SUPPLY_TRACKED` is unset and `current_supply` must
  be zero.
- Tracked supply: `CONFIG_SUPPLY_TRACKED` is set and `current_supply` must match
  the token delta validated in the transaction.

For tracked supply:

- mint increases supply and requires `mint_authority` through the matching
  metadata update;
- protocol burn decreases supply and requires `mint_authority` through the
  matching metadata update;
- user destruction does not reduce `current_supply`;
- metadata destruction requires tracked supply, zero `current_supply`, and
  `mint_authority`;
- changing `current_supply` directly without matching UDT cell deltas is
  rejected by the metadata script.

Protocol burn and user destruction are distinguished by metadata participation:
if the transaction consumes the token's metadata input cell, a negative token
delta is a protocol burn and tracked supply must decrease by that delta. If no
metadata input cell is consumed, holders may destroy token cells without
decreasing tracked supply.

Fixed supply can be represented by minting the target amount and then removing
`mint_authority`. After that, no further authorized mint or protocol burn can be
performed.

## Access Modes

xUDT access mode is controlled by `xudt-meta` metadata flags and enforced by the
combination of `xudt` and `access-list`.

Supported states:

- disabled: no AccessList checks;
- blacklist: access enabled, whitelist bit unset;
- whitelist: access enabled, whitelist bit set.

Whitelist and blacklist use the same AccessList shard structure. They differ
only in how `xudt` interprets holder membership:

- whitelist: checked holder locks must be included in the AccessList;
- blacklist: checked holder locks must not be included in the AccessList.

xUDT access control is holder-based. Transfers and protocol burns check both
input and output xUDT holder locks; mint checks output holder locks. Pure user
destruction with no xUDT outputs remains available even if the input lock is not
currently allowed, so users can reclaim CKB from their own token cells.

Creating an active access mode, destroying an active mode, and switching between
whitelist and blacklist require full-domain AccessList coverage where relevant.
Ordinary AccessList updates can touch only the shard nodes being updated; the
`access-list` type script validates that the consumed and produced shard sets
cover the same continuous range and prevents overlap or ordering violations.
Destroying xUDT metadata while access mode is enabled additionally requires
full-domain AccessList inputs and forbids bound AccessList outputs.

xUDT token movement reads AccessList proofs from CellDeps only. Same-meta
AccessList inputs or outputs are not proof sources; they may appear in the same
transaction when their own AccessList state transition is valid. Matching
CellDep proof shards must be ordered by range and non-overlapping. The xUDT
script indexes proof shards by `start`, `end`, and `dep_index`, then loads full
shard entries only for shards that cover checked holder locks.

## Authorities

Authority values are encoded in metadata and interpreted by shared script
utilities.

Supported authority types:

- `InputLock`: the referenced script hash must appear as an input lock;
- `InputType`: the referenced script hash must appear as an input type script;
- `OutputType`: the referenced script hash must appear as an output type script;
- `DynamicLinking`: the referenced executable script is loaded through dynamic
  linking and asked to authorize;
- `Spawn`: the referenced executable script is spawned and asked to authorize.

`mint_authority` is the strongest metadata authority. It can authorize supply
changes and also acts as a fallback for metadata/access authority controlled
updates when the narrower authority is absent or does not authorize.

Contracts use the shared `AuthorityVerifier` from `lib/script-utils` to perform
authority checks. A validation path reuses one verifier so repeated authority
requirements are cached instead of re-scanning cells or re-running executable
authority code. Consuming a metadata or AccessList state cell requires an
applicable authority even for a no-op update.

## Build Prerequisites

The project uses the Rust toolchain pinned in `rust-toolchain.toml`.

Install the RISC-V target before building contracts:

```bash
rustup target add riscv64imac-unknown-none-elf
```

The Makefile also expects a usable Clang toolchain for C-based test plugins.
`scripts/find_clang` is used to locate the available Clang binary.

## Build

Build all contracts and test plugins in debug mode:

```bash
make build MODE=debug
```

Build all release binaries:

```bash
make build MODE=release
```

`MODE=release` is the default, so this is equivalent:

```bash
make build
```

Build output is written to:

- `build/debug/` for debug mode;
- `build/release/` for release mode.

The top-level Makefile builds dependent contracts in order and passes code
hashes automatically:

- `sudt-meta` receives `SUDT_CODE_HASH`;
- `xudt` receives `ACCESS_LIST_CODE_HASH`;
- `xudt-meta` receives `XUDT_CODE_HASH` and `ACCESS_LIST_CODE_HASH`.

To build a single contract and its required dependencies:

```bash
make build MODE=debug CONTRACT=xudt-meta
```

## Debug And Release Differences

When `MODE=debug`, the Makefile adds:

```text
-C debug-assertions
```

This enables Rust `#[cfg(debug_assertions)]` code inside the contract binaries.
The contracts use this only for test-oriented allowances such as accepting the
ckb-testtool always-success lock code hash in output metadata or AccessList
locks.

Release builds do not include those debug-only branches.

## Test

Build the matching contract binaries before running integration tests:

```bash
make build MODE=debug
MODE=debug make test
```

To show test output:

```bash
MODE=debug make test CARGO_ARGS="-- --nocapture"
```

The test target checks for required binaries under `build/{debug,release}` and
prints a hint if they are missing. The `MODE` value used for tests must match
the build output directory you want the tests to load.

Run Cargo checks directly when you do not need on-chain binaries:

```bash
make check
make clippy
make fmt
```

## UDTX CLI

The workspace includes `udtx`, a CLI tool for token operations and local devnet management.

### Build

Build the CLI via Cargo:

```bash
cargo build --bin udtx
```

Or build the release binary:

```bash
cargo build --bin udtx --release
```

### Commands

| Command | Description |
| --- | --- |
| `udtx init [ --name <name> ]` | Initialize a new UDTX project with `udtx.yaml` and profile files. |
| `udtx doctor` | Check the environment, configuration, and dependencies. |
| `udtx chain up [ --background ]` | Start a local offckb devnet (requires `@offckb/cli`). |
| `udtx chain down` | Stop the local devnet. |
| `udtx chain reset [ --yes ]` | Reset devnet data. |
| `udtx chain status` | Show whether the devnet is running. |
| `udtx token issue` | Issue a new sUDT or xUDT token. |
| `udtx token transfer` | Transfer tokens (planned). |
| `udtx token mint` | Mint additional tokens (planned). |
| `udtx token burn` | Burn tokens (planned). |
| `udtx token info` | Show token information (planned). |

### Token Issue

Issue a new token with optional CLI overrides:

```bash
udtx token issue \
  --token-type sudt \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 8 \
  --supply 1000000 \
  --owner owner \
  --dry-run
```

Parameters:

- `--token-type`: `sudt` or `xudt` (default: `sudt`);
- `--name`, `--symbol`, `--decimals`: token metadata;
- `--supply`: initial supply as an integer;
- `--owner`: account name from the config;
- `--dry-run`: preview the transaction without submitting.

### Configuration

`udtx init` creates two files:

- `udtx.yaml` — project-level settings (network profile, accounts, token defaults, contract sources);
- `profiles/devnet.yaml` — devnet-specific RPC and contract references.

Accounts can be defined as environment-variable-backed private keys or as plain addresses:

```yaml
accounts:
  owner:
    private_key_env: OWNER_PRIVATE_KEY
  alice:
    address: "ckt1..."
```

### Devnet Dependency

Chain commands require `@offckb/cli` to be installed globally:

```bash
npm install -g @offckb/cli
```

## Reproducible Builds And Checksums

Generate native SHA-256 checksums for the selected build mode:

```bash
make checksum MODE=release
```

The checksum file is written to:

```text
build/checksums-release.txt
```

Use `MODE=debug` to generate debug checksums instead.

For release verification in a pinned Docker environment, use:

```bash
./scripts/reproducible_build_docker --update
./scripts/reproducible_build_docker
```

The first command rebuilds release artifacts in the pinned
`cryptape/llvm-n-rust` image and updates the root `checksums.txt`. The second
command rebuilds in the same environment and verifies `checksums.txt`.

The Docker image is pinned by digest in `scripts/reproducible_build_docker`, so
LLVM tools such as `clang`, `llvm-ar`, and `llvm-objcopy` come from an
immutable image. Rust toolchain components and Cargo dependencies are still
resolved by `rustup` and Cargo inside the container, using `rust-toolchain.toml`
and `Cargo.lock`; this is a reproducible Docker build path, not a fully
hermetic offline build.

The reproducible build runs the same top-level checksum target. The build still
compiles test plugin fixtures for integration-test compatibility, but
`checksums.txt` records only release contract binaries. `CLANG` and `OBJCOPY`
can be overridden when the local LLVM tools use different names.

## Development Notes

- Contract code is `no_std` for on-chain builds.
- Shared parsers should live in `lib/types` when both contracts and tests need
  the same metadata interpretation.
- Shared script-side logic should live in `lib/script-utils` when multiple
  contracts need the same CKB cell-scanning or authority behavior.
- Type scripts should validate their own state invariants. Consumers should
  check only the proofs or facts they need from another type script.
- Keep debug-only test allowances behind `#[cfg(debug_assertions)]` so they are
  not included in release binaries.

## License

No license file is currently included in this repository.

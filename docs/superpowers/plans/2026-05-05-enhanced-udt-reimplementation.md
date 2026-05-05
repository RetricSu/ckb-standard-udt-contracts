# Enhanced UDT Reimplementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Enhanced sUDT, Enhanced xUDT, AccessList, shared types, shared script utilities, and tests from the reset repository using the new supply-mode design, without carrying the old `v1` naming into the new code.

**Architecture:** Treat `ref/old` as reference material only. Create a clean module layout with explicit domain boundaries: `lib/types` owns Molecule/domain decoding, `lib/script-utils` owns reusable CKB script helpers, each contract crate owns one validation boundary, and tests exercise behavior through transaction fixtures rather than old implementation internals.

**Tech Stack:** Rust 2021, `ckb-std 0.16.3`, `ckb-testtool 0.14.0`, Molecule generated bindings, Makefile-driven RISC-V contract builds.

---

## Naming Policy

Do not use `v1` in new crate names, module names, function names, tests, errors, or docs except when referring to legacy material under `ref/old` or the already-written design file path.

Use these new paths and package names:

- `lib/types` package `standard-udt-types`
- `lib/script-utils` package `standard-udt-script-utils`
- `contracts/enhanced-sudt-meta` package `enhanced-sudt-meta`
- `contracts/enhanced-sudt` package `enhanced-sudt`
- `contracts/enhanced-xudt-meta` package `enhanced-xudt-meta`
- `contracts/enhanced-xudt` package `enhanced-xudt`
- `contracts/access-list` package `access-list`
- `tests/plugins/dl-allow` package `dl-allow`
- `tests/plugins/dl-deny` package `dl-deny`
- `tests/plugins/spawn-allow` package `spawn-allow`
- `tests/plugins/spawn-deny` package `spawn-deny`

Use these public type names:

- `SudtMeta`
- `XudtMeta`
- `AccessListShard`
- `AccessListRange`
- `ScriptAttr`
- `ConfigFlags`

Legacy `ref/old` code may be copied in small pieces during implementation, but any copied code must be renamed and reshaped to match these boundaries before it is committed.

## Source Documents

- Design spec: `docs/superpowers/specs/2026-05-05-enhanced-udt-supply-mode-design.md`
- Existing standard drafts: `ref/CKB Enhanced UDT Standard.md`, `ref/Enhanced UDT Standard V1.md`
- Legacy implementation reference only: `ref/old/`
- CKB programming model: `ref/ckb-programming-model.md`
- Local agent constraints: `agents.md`
- Build/test contract: `Makefile`

## New File Structure

- `lib/types/src/schemas/blockchain.mol`: shared Molecule primitives.
- `lib/types/src/schemas/metadata.mol`: `ScriptAttr`, `SudtMeta`, `XudtMeta`, `AccessListShard`.
- `lib/types/src/metadata.rs`: domain structs, config flag validation, pack/unpack, limits.
- `lib/script-utils/src/authority.rs`: location 0/1/2 authority checks plus dispatch for dynamic linking/spawn.
- `lib/script-utils/src/meta.rs`: visible Meta lookup by `meta_type_hash`, lock whitelist checks, Meta pair loading.
- `lib/script-utils/src/amount.rs`: 16-byte UDT amount decoding and group sum helpers.
- `lib/script-utils/src/supply.rs`: supply delta classification and checked arithmetic.
- `contracts/enhanced-sudt-meta/src/`: sUDT Meta creation/update rules.
- `contracts/enhanced-sudt/src/`: sUDT transfer, mint, protocol burn, user destruction.
- `contracts/enhanced-xudt-meta/src/`: xUDT Meta creation/update/access-mode governance.
- `contracts/enhanced-xudt/src/`: xUDT transfer, mint, protocol burn, user destruction, paused/access/extensions.
- `contracts/access-list/src/`: AccessList shard updates and invariants.
- `tests/plugins/dl-allow/src/` and `tests/plugins/dl-deny/src/`: dynamic-linking extension fixtures.
- `tests/plugins/spawn-allow/src/` and `tests/plugins/spawn-deny/src/`: spawn extension fixtures.
- `tests/src/fixtures.rs`: transaction fixture builders.
- `tests/src/metadata_builders.rs`: typed and raw Molecule metadata builders.
- `tests/src/tests/*.rs`: behavior tests by contract domain.
- `ref/Enhanced UDT Standard.md`: final no-`V1` standard document created near the end.

---

### Task 1: Generate Script Crates From Templates

**Files:**
- Modify: `Cargo.toml`
- Modify: `Makefile`
- Create: `lib/types/Cargo.toml`
- Create: `lib/script-utils/Cargo.toml`
- Generate: `contracts/enhanced-sudt-meta/`
- Generate: `contracts/enhanced-sudt/`
- Generate: `contracts/enhanced-xudt-meta/`
- Generate: `contracts/enhanced-xudt/`
- Generate: `contracts/access-list/`
- Generate: `tests/plugins/dl-allow/`
- Generate: `tests/plugins/dl-deny/`
- Generate: `tests/plugins/spawn-allow/`
- Generate: `tests/plugins/spawn-deny/`
- Test: `cargo metadata --no-deps --format-version 1`

- [ ] **Step 1: Verify template generator is available**

Run:

```bash
cargo generate --version
```

Expected: PASS and prints a `cargo-generate` version. If missing, install it with `cargo install cargo-generate` and rerun the version check.

- [ ] **Step 2: Generate core contract crates via Makefile**

Run these commands from the repository root:

```bash
make generate CRATE=enhanced-sudt-meta
make generate CRATE=enhanced-sudt
make generate CRATE=enhanced-xudt-meta
make generate CRATE=enhanced-xudt
make generate CRATE=access-list
```

Expected: each command creates a contract crate under `contracts/` using `ckb-script-templates`, with its own generated `Makefile`, `Cargo.toml`, and `src/main.rs` / `src/lib.rs` structure. Root `Cargo.toml` is updated by the existing `make generate` insertion point.

- [ ] **Step 3: Generate test plugin script crates via template**

Run:

```bash
make generate CRATE=dl-allow DESTINATION=tests/plugins
make generate CRATE=dl-deny DESTINATION=tests/plugins
make generate CRATE=spawn-allow DESTINATION=tests/plugins
make generate CRATE=spawn-deny DESTINATION=tests/plugins
```

Expected: each plugin crate is generated from the same contract template under `tests/plugins/`. Root `Cargo.toml` includes these members.

- [ ] **Step 4: Create shared library directories**

Run:

```bash
mkdir -p lib/types/src/schemas lib/types/src/generated lib/script-utils/src
```

Expected: shared library directories exist. These are not script crates and are intentionally not generated from `ckb-script-templates`.

- [ ] **Step 5: Add shared crate manifests**

Create `lib/types/Cargo.toml`:

```toml
[package]
name = "standard-udt-types"
version = "0.1.0"
edition = "2021"

[dependencies]
ckb-std = { version = "0.16.3", optional = true }
ckb-types = { version = "0.120.0", optional = true }
molecule = { version = "0.8.0", default-features = false }
cfg-if = "1.0.0"

[features]
default = ["std"]
std = ["dep:ckb-types"]
no-std = ["dep:ckb-std"]
native-simulator = ["ckb-std/native-simulator"]
```

Create `lib/script-utils/Cargo.toml`:

```toml
[package]
name = "standard-udt-script-utils"
version = "0.1.0"
edition = "2021"

[dependencies]
ckb-std = "0.16.3"
molecule = { version = "0.8.0", default-features = false }
standard-udt-types = { path = "../types", default-features = false, features = ["no-std"] }

[features]
default = []
native-simulator = ["ckb-std/native-simulator"]
```

- [ ] **Step 6: Add shared library modules**

Create `lib/types/src/lib.rs`:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod error;
pub mod generated;
pub mod metadata;
```

Create `lib/types/src/error.rs`, `lib/types/src/metadata.rs`, and `lib/types/src/generated/mod.rs` with:

```rust
// Module filled by later tasks.
```

Create `lib/script-utils/src/lib.rs`:

```rust
#![no_std]

extern crate alloc;

pub mod amount;
pub mod authority;
pub mod error;
pub mod meta;
pub mod supply;
```

Create `lib/script-utils/src/amount.rs`, `lib/script-utils/src/authority.rs`, `lib/script-utils/src/error.rs`, `lib/script-utils/src/meta.rs`, and `lib/script-utils/src/supply.rs` with:

```rust
// Module filled by later tasks.
```

- [ ] **Step 7: Add shared library members to workspace**

Keep the generated contract/plugin member entries inserted by `make generate`. Add only the two shared library members manually near the top of root `Cargo.toml`:

```toml
"lib/types",
"lib/script-utils",
```

Do not replace generated member entries. Do not remove the `# @@INSERTION_POINT@@` comment.

- [ ] **Step 8: Update Makefile build set using generated crate names**

In `Makefile`, replace the current `V1_CONTRACTS := ...` line with:

```make
CONTRACTS := enhanced-sudt-meta enhanced-sudt enhanced-xudt-meta enhanced-xudt access-list
TEST_PLUGINS := dl-allow dl-deny spawn-allow spawn-deny
TEST_REQUIRED_BINARIES := $(addprefix $(BUILD_DIR)/,$(CONTRACTS) $(TEST_PLUGINS))
```

Replace only build/test loops that refer to `$(V1_CONTRACTS)` with `$(CONTRACTS)`.

After the contract build loop in the `build` target, add a test plugin build loop that preserves template crate Makefiles:

```make
		for plugin in $(TEST_PLUGINS); do \
			$(MAKE) -e -C tests/plugins/$$plugin build; \
		done; \
```

Do not replace generated contract Makefiles with direct `cargo build --target ...` calls.

- [ ] **Step 9: Add shared dependencies to generated script crates**

Edit each generated core contract `Cargo.toml` under `contracts/enhanced-sudt-meta`, `contracts/enhanced-sudt`, `contracts/enhanced-xudt-meta`, `contracts/enhanced-xudt`, and `contracts/access-list` by adding:

```toml
standard-udt-types = { path = "../../lib/types", default-features = false, features = ["no-std"] }
standard-udt-script-utils = { path = "../../lib/script-utils", default-features = false }
```

For plugin crates under `tests/plugins/`, keep the generated template dependencies only at this stage.

- [ ] **Step 10: Verify no hand-rolled script skeleton**

Run:

```bash
test -f contracts/enhanced-sudt/Makefile
test -f contracts/enhanced-sudt/src/main.rs
test -f tests/plugins/spawn-allow/Makefile
test -f tests/plugins/spawn-allow/src/main.rs
```

Expected: all commands pass, proving the script crates were generated from the template shape.

- [ ] **Step 11: Verify workspace metadata**

Run:

```bash
cargo metadata --no-deps --format-version 1
```

Expected: PASS and output contains the new package names.

- [ ] **Step 12: Commit generated skeleton**

Run:

```bash
git add Cargo.toml Makefile lib contracts tests/plugins
git commit -m "chore: create enhanced udt workspace"
```

---

### Task 2: Implement Metadata Types and Config Flags

**Files:**
- Modify: `lib/types/src/schemas/blockchain.mol`
- Modify: `lib/types/src/schemas/metadata.mol`
- Modify: `lib/types/src/generated/blockchain.rs`
- Modify: `lib/types/src/generated/metadata.rs`
- Modify: `lib/types/src/generated/mod.rs`
- Modify: `lib/types/src/error.rs`
- Modify: `lib/types/src/metadata.rs`
- Test: `cargo test -p standard-udt-types`

- [ ] **Step 1: Write schemas**

Create `lib/types/src/schemas/blockchain.mol`:

```molecule
array Byte32 [byte; 32];
vector Bytes <byte>;
vector Byte32Vec <Byte32>;

table Script {
    code_hash: Byte32,
    hash_type: byte,
    args: Bytes,
}

option ScriptOpt (Script);
```

Create `lib/types/src/schemas/metadata.mol`:

```molecule
import blockchain;

array Uint128 [byte; 16];

table ScriptAttr {
    location: byte,
    script_hash: Byte32,
    script: ScriptOpt,
}

option ScriptAttrOpt (ScriptAttr);
vector ScriptAttrVec <ScriptAttr>;

table SudtMeta {
    config_flags: byte,
    current_supply: Uint128,
    decimals: byte,
    name: Bytes,
    symbol: Bytes,
    uri: Bytes,
    extra_data: Bytes,
    mint_authority: ScriptAttrOpt,
    metadata_authority: ScriptAttrOpt,
}

table XudtMeta {
    config_flags: byte,
    current_supply: Uint128,
    decimals: byte,
    name: Bytes,
    symbol: Bytes,
    uri: Bytes,
    extra_data: Bytes,
    mint_authority: ScriptAttrOpt,
    metadata_authority: ScriptAttrOpt,
    access_authority: ScriptAttrOpt,
    extensions: ScriptAttrVec,
}

struct AccessListRange {
    start: Byte32,
    end: Byte32,
}

table AccessListShard {
    range: AccessListRange,
    entries: Byte32Vec,
}
```

- [ ] **Step 2: Generate Molecule bindings**

Run:

```bash
cargo install moleculec --version 0.8.0
moleculec --language rust --schema-file lib/types/src/schemas/blockchain.mol --output-file lib/types/src/generated/blockchain.rs
moleculec --language rust --schema-file lib/types/src/schemas/metadata.mol --output-file lib/types/src/generated/metadata.rs
```

Create `lib/types/src/generated/mod.rs`:

```rust
pub mod blockchain;
pub mod metadata;
```

- [ ] **Step 3: Add domain errors**

Create `lib/types/src/error.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Molecule,
    InvalidScriptLocation,
    InvalidScriptShape,
    InvalidScriptHash,
    InvalidConfigFlags,
    InvalidSupply,
    ExtensionsTooMany,
    ExtensionsNotSorted,
    ExtensionsDuplicated,
    MetadataTooLarge,
    AccessListTooLarge,
}

impl From<molecule::error::VerificationError> for Error {
    fn from(_: molecule::error::VerificationError) -> Self {
        Error::Molecule
    }
}

impl From<molecule::error::HeaderError> for Error {
    fn from(_: molecule::error::HeaderError) -> Self {
        Error::Molecule
    }
}
```

- [ ] **Step 4: Implement metadata domain module**

Create `lib/types/src/metadata.rs` with:

```rust
use alloc::vec::Vec;

#[cfg(feature = "std")]
use ckb_types::{packed::Script, prelude::*};
#[cfg(not(feature = "std"))]
use ckb_std::ckb_types::{packed::Script, prelude::*};

use molecule::prelude::*;

use crate::{error::Error, generated};

pub const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
pub const CONFIG_ACCESS_ENABLED: u8 = 0b0000_0010;
pub const CONFIG_ACCESS_WHITELIST: u8 = 0b0000_0100;
pub const CONFIG_PAUSED: u8 = 0b0000_1000;
pub const SUDT_ALLOWED_CONFIG_MASK: u8 = CONFIG_SUPPLY_TRACKED;
pub const XUDT_ALLOWED_CONFIG_MASK: u8 =
    CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED;
pub const MAX_EXTENSIONS: usize = 16;
pub const MAX_ACCESSLIST_ENTRIES: usize = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptLocation {
    InputLock,
    InputType,
    OutputType,
    DynamicLinking,
    Spawn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptAttr {
    pub location: ScriptLocation,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub decimals: u8,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub extra_data: Vec<u8>,
    pub mint_authority: Option<ScriptAttr>,
    pub metadata_authority: Option<ScriptAttr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct XudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub decimals: u8,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub extra_data: Vec<u8>,
    pub mint_authority: Option<ScriptAttr>,
    pub metadata_authority: Option<ScriptAttr>,
    pub access_authority: Option<ScriptAttr>,
    pub extensions: Vec<ScriptAttr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessListRange {
    pub start: [u8; 32],
    pub end: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccessListShard {
    pub range: AccessListRange,
    pub entries: Vec<[u8; 32]>,
}

pub fn is_supply_tracked(config_flags: u8) -> bool {
    config_flags & CONFIG_SUPPLY_TRACKED != 0
}

pub fn validate_sudt_config(config_flags: u8, current_supply: u128) -> Result<(), Error> {
    if config_flags & !SUDT_ALLOWED_CONFIG_MASK != 0 {
        return Err(Error::InvalidConfigFlags);
    }
    if !is_supply_tracked(config_flags) && current_supply != 0 {
        return Err(Error::InvalidSupply);
    }
    Ok(())
}

pub fn validate_xudt_config(config_flags: u8, current_supply: u128) -> Result<(), Error> {
    if config_flags & !XUDT_ALLOWED_CONFIG_MASK != 0 {
        return Err(Error::InvalidConfigFlags);
    }
    if config_flags & CONFIG_ACCESS_ENABLED == 0 && config_flags & CONFIG_ACCESS_WHITELIST != 0 {
        return Err(Error::InvalidConfigFlags);
    }
    if !is_supply_tracked(config_flags) && current_supply != 0 {
        return Err(Error::InvalidSupply);
    }
    Ok(())
}

fn unpack_u128(raw: generated::metadata::Uint128) -> u128 {
    u128::from_le_bytes(raw.as_slice().try_into().expect("Uint128 has fixed length"))
}

fn pack_u128(value: u128) -> generated::metadata::Uint128 {
    generated::metadata::Uint128::from_slice(&value.to_le_bytes()).expect("u128 pack")
}
```

Then add pack/unpack functions for `ScriptAttr`, `SudtMeta`, `XudtMeta`, and `AccessListShard`. Use `ref/old/lib/types/src/metadata.rs` as an implementation reference, but expose the new names only.

- [ ] **Step 5: Add tests**

At the bottom of `lib/types/src/metadata.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sudt_rejects_xudt_config_bits() {
        let meta = SudtMeta {
            config_flags: CONFIG_ACCESS_ENABLED,
            current_supply: 0,
            decimals: 0,
            name: Vec::new(),
            symbol: Vec::new(),
            uri: Vec::new(),
            extra_data: Vec::new(),
            mint_authority: None,
            metadata_authority: None,
        };

        assert_eq!(meta.to_bytes().unwrap_err(), Error::InvalidConfigFlags);
    }

    #[test]
    fn untracked_requires_zero_supply() {
        let meta = SudtMeta {
            config_flags: 0,
            current_supply: 1,
            decimals: 0,
            name: Vec::new(),
            symbol: Vec::new(),
            uri: Vec::new(),
            extra_data: Vec::new(),
            mint_authority: None,
            metadata_authority: None,
        };

        assert_eq!(meta.to_bytes().unwrap_err(), Error::InvalidSupply);
    }

    #[test]
    fn xudt_rejects_access_mode_when_access_disabled() {
        let meta = XudtMeta {
            config_flags: CONFIG_ACCESS_WHITELIST,
            current_supply: 0,
            decimals: 0,
            name: Vec::new(),
            symbol: Vec::new(),
            uri: Vec::new(),
            extra_data: Vec::new(),
            mint_authority: None,
            metadata_authority: None,
            access_authority: None,
            extensions: Vec::new(),
        };

        assert_eq!(meta.to_bytes().unwrap_err(), Error::InvalidConfigFlags);
    }
}
```

- [ ] **Step 6: Run type tests**

Run:

```bash
cargo test -p standard-udt-types
```

Expected: PASS.

- [ ] **Step 7: Commit metadata types**

Run:

```bash
git add lib/types
git commit -m "feat: add enhanced udt metadata types"
```

---

### Task 3: Build Shared Script Utilities

**Files:**
- Modify: `lib/script-utils/src/error.rs`
- Modify: `lib/script-utils/src/amount.rs`
- Modify: `lib/script-utils/src/supply.rs`
- Modify: `lib/script-utils/src/meta.rs`
- Modify: `lib/script-utils/src/authority.rs`
- Test: `cargo test -p standard-udt-script-utils`

- [ ] **Step 1: Implement utility errors**

Create `lib/script-utils/src/error.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptError {
    Syscall,
    AmountEncoding,
    AmountOverflow,
    SupplyOverflow,
    SupplyUnderflow,
    MetaMissing,
    MetaNotUnique,
    MetaInputMissing,
    MetaOutputMissing,
    MetaLockNotAllowed,
    MetaStateMismatch,
    AuthorityMissing,
    AuthorityFailed,
    UnsupportedAuthorityLocation,
}
```

- [ ] **Step 2: Implement amount helpers**

Create `lib/script-utils/src/amount.rs`:

```rust
use ckb_std::{ckb_constants::Source, high_level::load_cell_data};

use crate::error::ScriptError;

pub fn decode_amount(data: &[u8]) -> Result<u128, ScriptError> {
    if data.len() != 16 {
        return Err(ScriptError::AmountEncoding);
    }
    let mut raw = [0u8; 16];
    raw.copy_from_slice(data);
    Ok(u128::from_le_bytes(raw))
}

pub fn collect_group_amount(source: Source) -> Result<u128, ScriptError> {
    let mut total = 0u128;
    let mut index = 0;
    loop {
        match load_cell_data(index, source) {
            Ok(data) => {
                let amount = decode_amount(&data)?;
                total = total.checked_add(amount).ok_or(ScriptError::AmountOverflow)?;
                index += 1;
            }
            Err(ckb_std::error::SysError::IndexOutOfBound) => return Ok(total),
            Err(_) => return Err(ScriptError::Syscall),
        }
    }
}
```

- [ ] **Step 3: Implement supply helpers with tests**

Create `lib/script-utils/src/supply.rs`:

```rust
use crate::error::ScriptError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupplyDelta {
    Increase(u128),
    Decrease(u128),
    Unchanged,
}

pub fn classify_supply_delta(sum_in: u128, sum_out: u128) -> SupplyDelta {
    if sum_out > sum_in {
        SupplyDelta::Increase(sum_out - sum_in)
    } else if sum_in > sum_out {
        SupplyDelta::Decrease(sum_in - sum_out)
    } else {
        SupplyDelta::Unchanged
    }
}

pub fn apply_supply_delta(old_supply: u128, delta: SupplyDelta) -> Result<u128, ScriptError> {
    match delta {
        SupplyDelta::Increase(value) => old_supply.checked_add(value).ok_or(ScriptError::SupplyOverflow),
        SupplyDelta::Decrease(value) => old_supply.checked_sub(value).ok_or(ScriptError::SupplyUnderflow),
        SupplyDelta::Unchanged => Ok(old_supply),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_and_applies_delta() {
        assert_eq!(classify_supply_delta(1, 3), SupplyDelta::Increase(2));
        assert_eq!(apply_supply_delta(10, SupplyDelta::Increase(2)), Ok(12));
        assert_eq!(classify_supply_delta(3, 1), SupplyDelta::Decrease(2));
        assert_eq!(apply_supply_delta(10, SupplyDelta::Decrease(2)), Ok(8));
        assert_eq!(classify_supply_delta(3, 3), SupplyDelta::Unchanged);
    }

    #[test]
    fn rejects_supply_overflow_and_underflow() {
        assert_eq!(apply_supply_delta(u128::MAX, SupplyDelta::Increase(1)), Err(ScriptError::SupplyOverflow));
        assert_eq!(apply_supply_delta(0, SupplyDelta::Decrease(1)), Err(ScriptError::SupplyUnderflow));
    }
}
```

- [ ] **Step 4: Implement authority and meta helpers**

Use `ref/old/lib/utils/src/authority.rs`, `scanner.rs`, `presence.rs`, `dl.rs`, and `spawn.rs` as references. Create clean helpers:

```rust
pub fn check_authority(attr: &standard_udt_types::metadata::ScriptAttr) -> bool
```

```rust
pub fn check_authority_with_witness(
    attr: &standard_udt_types::metadata::ScriptAttr,
    witness: PluginWitness,
) -> Result<bool, ScriptError>
```

```rust
pub fn find_unique_meta_by_type_hash(meta_type_hash: &[u8; 32]) -> Result<VisibleMeta, ScriptError>
```

```rust
pub fn has_input_type_hash(type_hash: &[u8; 32]) -> Result<bool, ScriptError>
```

Keep the implementation no-std and avoid leaking old module names.

- [ ] **Step 5: Run utility tests**

Run:

```bash
cargo test -p standard-udt-script-utils
```

Expected: PASS.

- [ ] **Step 6: Commit utilities**

Run:

```bash
git add lib/script-utils
git commit -m "feat: add enhanced udt script utilities"
```

---

### Task 4: Create Test Fixtures and Metadata Builders

**Files:**
- Modify: `tests/Cargo.toml`
- Create: `tests/src/fixtures.rs`
- Create: `tests/src/metadata_builders.rs`
- Modify: `tests/src/lib.rs`
- Test: `cargo test -p tests --no-run`

- [ ] **Step 1: Update test dependency package name**

Edit `tests/Cargo.toml`:

```toml
standard-udt-types = { path = "../lib/types" }
```

Keep `ckb-testtool` and `serde_json` unchanged.

- [ ] **Step 2: Expose fixture modules**

In `tests/src/lib.rs`, add:

```rust
pub mod fixtures;
pub mod metadata_builders;
```

- [ ] **Step 3: Create metadata builders**

Create `tests/src/metadata_builders.rs`:

```rust
use ckb_testtool::ckb_types::{bytes::Bytes, packed::Script, prelude::*};
use standard_udt_types::metadata::{
    AccessListRange, AccessListShard, ScriptAttr, ScriptLocation, SudtMeta, XudtMeta,
};

pub fn input_lock_authority(script_hash: [u8; 32]) -> ScriptAttr {
    ScriptAttr {
        location: ScriptLocation::InputLock,
        script_hash,
        script: None,
    }
}

pub fn build_sudt_meta_bytes(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<ScriptAttr>,
    metadata_authority: Option<ScriptAttr>,
) -> Bytes {
    Bytes::from(SudtMeta {
        config_flags,
        current_supply,
        decimals: 0,
        name: Vec::new(),
        symbol: Vec::new(),
        uri: Vec::new(),
        extra_data: Vec::new(),
        mint_authority,
        metadata_authority,
    }.to_bytes().expect("SudtMeta bytes"))
}

pub fn build_xudt_meta_bytes(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<ScriptAttr>,
    metadata_authority: Option<ScriptAttr>,
    access_authority: Option<ScriptAttr>,
    extensions: Vec<ScriptAttr>,
) -> Bytes {
    Bytes::from(XudtMeta {
        config_flags,
        current_supply,
        decimals: 0,
        name: Vec::new(),
        symbol: Vec::new(),
        uri: Vec::new(),
        extra_data: Vec::new(),
        mint_authority,
        metadata_authority,
        access_authority,
        extensions,
    }.to_bytes().expect("XudtMeta bytes"))
}

pub fn build_access_list_shard_bytes(
    start: [u8; 32],
    end: [u8; 32],
    entries: Vec<[u8; 32]>,
) -> Bytes {
    Bytes::from(AccessListShard {
        range: AccessListRange { start, end },
        entries,
    }.to_bytes().expect("AccessListShard bytes"))
}

pub fn udt_amount_bytes(amount: u128) -> Bytes {
    Bytes::from(amount.to_le_bytes().to_vec())
}

pub fn script_hash(script: &Script) -> [u8; 32] {
    script.calc_script_hash().unpack()
}

pub struct DeployedScript {
    pub out_point: ckb_testtool::ckb_types::packed::OutPoint,
    pub script: Script,
    pub script_hash: [u8; 32],
}
```

- [ ] **Step 4: Create fixtures module**

Create `tests/src/fixtures.rs` by moving reusable transaction helpers out of `tests/src/helpers.rs`. Keep function names descriptive:

```rust
pub fn deploy_contract(context: &mut ckb_testtool::context::Context, name: &str) -> ckb_testtool::ckb_types::packed::OutPoint
```

```rust
pub fn create_typed_cell(...)
```

```rust
pub fn expect_tx_pass(...)
```

```rust
pub fn expect_tx_fail_with_code(...)
```

Use current `tests/src/helpers.rs` and `ref/old` test helpers as references, but do not keep old `v1` names.

Add a deploy helper for extension tests:

```rust
pub fn deploy_script_with_args(
    context: &mut ckb_testtool::context::Context,
    binary_name: &str,
    args: ckb_testtool::ckb_types::bytes::Bytes,
) -> crate::metadata_builders::DeployedScript {
    let loader = crate::Loader::default();
    let binary = loader.load_binary(binary_name);
    let out_point = context.deploy_cell(binary);
    let script = context
        .build_script(&out_point, args)
        .expect("build deployed script");
    let script_hash = script.calc_script_hash().unpack();
    crate::metadata_builders::DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn cell_dep_for_script(
    deployed: &crate::metadata_builders::DeployedScript,
) -> ckb_testtool::ckb_types::packed::CellDep {
    ckb_testtool::ckb_types::packed::CellDep::new_builder()
        .out_point(deployed.out_point.clone())
        .build()
}
```

Spawn extension tests must use `deploy_script_with_args(context, "spawn-allow", Bytes::new())` or `spawn-deny`, put `deployed.script_hash` and `Some(deployed.script.clone())` into the Meta `ScriptAttr`, and explicitly add `cell_dep_for_script(&deployed)` to the transaction.

- [ ] **Step 5: Compile tests**

Run:

```bash
cargo test -p tests --no-run
```

Expected: PASS.

- [ ] **Step 6: Commit fixtures**

Run:

```bash
git add tests/Cargo.toml tests/src/lib.rs tests/src/fixtures.rs tests/src/metadata_builders.rs
git commit -m "test: add enhanced udt fixtures"
```

---

### Task 5: Implement sUDT Meta Contract

**Files:**
- Modify: `contracts/enhanced-sudt-meta/src/main.rs`
- Create: `contracts/enhanced-sudt-meta/src/error.rs`
- Create: `contracts/enhanced-sudt-meta/src/meta_cell.rs`
- Create: `contracts/enhanced-sudt-meta/src/update.rs`
- Test: `tests/src/tests/sudt_meta.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/src/tests/sudt_meta.rs` with tests named:

```rust
#[test]
fn sudt_meta_create_tracked_supply_matches_initial_outputs() {}

#[test]
fn sudt_meta_create_tracked_supply_mismatch_rejects() {}

#[test]
fn sudt_meta_rejects_supply_tracking_bit_change() {}

#[test]
fn sudt_meta_rejects_untracked_nonzero_supply() {}
```

Each test must build a real transaction. Use `enhanced-sudt-meta` as the Meta type script and `enhanced-sudt` as the UDT type script. The tracked success case creates one initial UDT output of amount `100` and Meta `current_supply=100`. The mismatch case uses Meta `current_supply=101` and expects the contract's `InvalidSupply` error code.

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
make build CONTRACT=enhanced-sudt-meta MODE=debug
MODE=debug cargo test -p tests sudt_meta_ -- --nocapture
```

Expected: FAIL because `enhanced-sudt-meta` currently accepts everything.

- [ ] **Step 3: Implement sUDT Meta contract**

Use these script-local modules:

- `error.rs`: `InvalidArgs`, `InvalidTypeId`, `InvalidMetaData`, `InvalidSupply`, `ImmutableSupplyMode`, `AuthorityMissing`, `AuthorityFailed`, `Syscall`.
- `meta_cell.rs`: load group input/output Meta, enforce one input/output group cell, derive `meta_type_hash`, sum initial UDT outputs by matching `Script{ENHANCED_SUDT_CODE_HASH, Data2, meta_type_hash}`.
- `update.rs`: enforce immutable supply bit, metadata authority, mint authority, authority `None` irreversibility.
- `main.rs`: wire type-id check, create path, update path.

Required create rule:

```rust
if is_supply_tracked(output_meta.config_flags) {
    require output_meta.current_supply == sum_initial_udt_outputs(meta_type_hash)
} else {
    require output_meta.current_supply == 0
}
```

Required update rule:

```rust
require input_meta.config_flags & CONFIG_SUPPLY_TRACKED
    == output_meta.config_flags & CONFIG_SUPPLY_TRACKED
```

- [ ] **Step 4: Run sUDT Meta tests**

Run:

```bash
make build CONTRACT=enhanced-sudt-meta MODE=debug
MODE=debug cargo test -p tests sudt_meta_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit sUDT Meta**

Run:

```bash
git add contracts/enhanced-sudt-meta tests/src/tests/sudt_meta.rs
git commit -m "feat: implement enhanced sudt meta"
```

---

### Task 6: Implement sUDT Contract

**Files:**
- Modify: `contracts/enhanced-sudt/src/main.rs`
- Create: `contracts/enhanced-sudt/src/error.rs`
- Create: `contracts/enhanced-sudt/src/meta.rs`
- Test: `tests/src/tests/sudt.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/src/tests/sudt.rs` with:

```rust
#[test]
fn sudt_transfer_does_not_require_meta() {}

#[test]
fn sudt_mint_requires_mint_authority() {}

#[test]
fn sudt_tracked_mint_updates_supply() {}

#[test]
fn sudt_user_destruction_without_meta_passes() {}

#[test]
fn sudt_protocol_burn_requires_mint_authority() {}

#[test]
fn sudt_protocol_burn_updates_tracked_supply() {}
```

Use amounts from the spec: mint `0 -> 50`, user destruction `100 -> 0` without Meta, protocol burn `100 -> 40` with Meta supply `100 -> 40`.

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
make build CONTRACT=enhanced-sudt MODE=debug
MODE=debug cargo test -p tests sudt_ -- --nocapture
```

Expected: FAIL because the contract is still a stub.

- [ ] **Step 3: Implement sUDT operation flow**

In `contracts/enhanced-sudt/src/main.rs`, implement:

```rust
sum_in == sum_out => pass
sum_out > sum_in => mint, require Meta and mint_authority
sum_in > sum_out && no Meta input => user destruction, pass
sum_in > sum_out && Meta input => protocol burn, require mint_authority
```

For tracked mint/protocol burn, require old and new Meta and exact `current_supply` delta. For untracked, require `current_supply == 0`.

- [ ] **Step 4: Run sUDT tests**

Run:

```bash
make build CONTRACT=enhanced-sudt MODE=debug
MODE=debug cargo test -p tests sudt_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit sUDT**

Run:

```bash
git add contracts/enhanced-sudt tests/src/tests/sudt.rs
git commit -m "feat: implement enhanced sudt"
```

---

### Task 7: Implement xUDT Meta Contract

**Files:**
- Modify: `contracts/enhanced-xudt-meta/src/main.rs`
- Create: `contracts/enhanced-xudt-meta/src/error.rs`
- Create: `contracts/enhanced-xudt-meta/src/config.rs`
- Create: `contracts/enhanced-xudt-meta/src/access.rs`
- Create: `contracts/enhanced-xudt-meta/src/update.rs`
- Test: `tests/src/tests/xudt_meta.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/src/tests/xudt_meta.rs` with:

```rust
#[test]
fn xudt_meta_rejects_invalid_config_flags() {}

#[test]
fn xudt_meta_disabled_to_blacklist_requires_full_domain_shards() {}

#[test]
fn xudt_meta_disabled_to_whitelist_requires_one_shard() {}

#[test]
fn xudt_meta_access_mode_switch_rejects_same_token_xudt_cells() {}

#[test]
fn xudt_meta_access_authority_controls_pause_and_access_mode() {}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
make build CONTRACT=enhanced-xudt-meta MODE=debug
MODE=debug cargo test -p tests xudt_meta_ -- --nocapture
```

Expected: FAIL because contract is a stub.

- [ ] **Step 3: Implement xUDT Meta update rules**

Implement:

- create rules matching sUDT tracked supply rules but using `ENHANCED_XUDT_CODE_HASH`.
- `config_flags.bit0` immutable.
- access-enabled, access-mode, paused, and `access_authority` changes require old `access_authority`.
- `extensions` changes require old `mint_authority`.
- metadata changes require old `metadata_authority`.
- access mode switch rejects any input/output cell with type `Script{ENHANCED_XUDT_CODE_HASH, Data2, meta_type_hash}`.
- Disabled to Blacklist and Whitelist to Blacklist require full-domain legal output shards.
- Disabled to Whitelist requires at least one legal output shard.

- [ ] **Step 4: Run xUDT Meta tests**

Run:

```bash
make build CONTRACT=enhanced-xudt-meta MODE=debug
MODE=debug cargo test -p tests xudt_meta_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit xUDT Meta**

Run:

```bash
git add contracts/enhanced-xudt-meta tests/src/tests/xudt_meta.rs
git commit -m "feat: implement enhanced xudt meta"
```

---

### Task 8: Implement AccessList Contract

**Files:**
- Modify: `contracts/access-list/src/main.rs`
- Create: `contracts/access-list/src/error.rs`
- Create: `contracts/access-list/src/mode.rs`
- Create: `contracts/access-list/src/shards.rs`
- Create: `contracts/access-list/src/meta.rs`
- Test: `tests/src/tests/access_list.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/src/tests/access_list.rs` with:

```rust
#[test]
fn access_list_blacklist_requires_full_domain_coverage() {}

#[test]
fn access_list_rejects_overlapping_shards() {}

#[test]
fn access_list_rejects_unauthorized_update() {}

#[test]
fn access_list_whitelist_missing_coverage_is_fail_closed_for_xudt() {}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
make build CONTRACT=access-list MODE=debug
MODE=debug cargo test -p tests access_list_ -- --nocapture
```

Expected: FAIL because contract is a stub.

- [ ] **Step 3: Implement shard validation**

Implement:

- Meta lookup by `type_hash == args`.
- `access_authority` check.
- mode from `config_flags`.
- Blacklist: full-domain coverage, ordered, non-overlapping, nibble-aligned, capacity-bounded, Insert/Delete/Split/Merge diff.
- Whitelist: ordered, non-overlapping, nibble-aligned, at least one shard for enabled mode, missing coverage fail-closed in xUDT.

- [ ] **Step 4: Run AccessList tests**

Run:

```bash
make build CONTRACT=access-list MODE=debug
MODE=debug cargo test -p tests access_list_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit AccessList**

Run:

```bash
git add contracts/access-list tests/src/tests/access_list.rs
git commit -m "feat: implement access list"
```

---

### Task 9: Implement xUDT Contract and Extensions

**Files:**
- Modify: `contracts/enhanced-xudt/src/main.rs`
- Create: `contracts/enhanced-xudt/src/error.rs`
- Create: `contracts/enhanced-xudt/src/config.rs`
- Create: `contracts/enhanced-xudt/src/access.rs`
- Create: `contracts/enhanced-xudt/src/extensions.rs`
- Create: `contracts/enhanced-xudt/src/meta.rs`
- Modify: `tests/plugins/dl-allow/src/main.rs`
- Modify: `tests/plugins/dl-deny/src/main.rs`
- Modify: `tests/plugins/spawn-allow/src/main.rs`
- Modify: `tests/plugins/spawn-deny/src/main.rs`
- Test: `tests/src/tests/xudt.rs`
- Test: `tests/src/tests/plugin_runtime.rs`

- [ ] **Step 1: Write failing xUDT tests**

Create `tests/src/tests/xudt.rs` with:

```rust
#[test]
fn xudt_transfer_requires_meta() {}

#[test]
fn xudt_paused_rejects_transfer_and_mint() {}

#[test]
fn xudt_paused_allows_user_destruction() {}

#[test]
fn xudt_tracked_mint_updates_supply() {}

#[test]
fn xudt_protocol_burn_requires_mint_authority_and_updates_supply() {}

#[test]
fn xudt_user_destruction_skips_access_and_extensions() {}

#[test]
fn xudt_blacklist_rejects_listed_input_lock() {}

#[test]
fn xudt_whitelist_rejects_missing_input_lock() {}
```

- [ ] **Step 2: Write failing plugin tests**

Create `tests/src/tests/plugin_runtime.rs` with:

```rust
#[test]
fn xudt_extension_allow_plugin_passes() {}

#[test]
fn xudt_extension_deny_plugin_rejects() {}

#[test]
fn xudt_spawn_extension_allow_plugin_passes() {}

#[test]
fn xudt_spawn_extension_deny_plugin_rejects() {}

#[test]
fn xudt_mint_extension_receives_mint_authority_checked() {}
```

- [ ] **Step 3: Run tests to verify failure**

Run:

```bash
make build CONTRACT=enhanced-xudt MODE=debug
MODE=debug cargo test -p tests xudt_ -- --nocapture
```

Expected: FAIL because the xUDT contract and test plugins are stubs.

- [ ] **Step 4: Implement dynamic-linking plugins**

`tests/plugins/dl-allow/src/main.rs` exports the dynamic-linking ABI and returns success:

```rust
#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn eudt_validate(
    _script_hash: *const u8,
    _op_type: u8,
    _ext_index: u8,
    _ext_data_ptr: *const u8,
    _ext_data_len: usize,
    _mint_authority_checked: u8,
) -> i8 {
    0
}

fn program_entry() -> i8 {
    0
}

ckb_std::entry!(program_entry);
```

`tests/plugins/dl-deny/src/main.rs` exports the same ABI and returns failure:

```rust
#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn eudt_validate(
    _script_hash: *const u8,
    _op_type: u8,
    _ext_index: u8,
    _ext_data_ptr: *const u8,
    _ext_data_len: usize,
    _mint_authority_checked: u8,
) -> i8 {
    1
}

fn program_entry() -> i8 {
    1
}

ckb_std::entry!(program_entry);
```

- [ ] **Step 5: Implement spawn plugins**

`tests/plugins/spawn-allow/src/main.rs` exits successfully:

```rust
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use ckb_std::env::argv;

fn program_entry() -> i8 {
    let args: Vec<_> = argv().collect();
    if args.len() != 4 {
        return 2;
    }
    0
}

ckb_std::entry!(program_entry);
```

`tests/plugins/spawn-deny/src/main.rs` exits with failure:

```rust
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use ckb_std::env::argv;

fn program_entry() -> i8 {
    let args: Vec<_> = argv().collect();
    if args.len() != 4 {
        return 2;
    }
    1
}

ckb_std::entry!(program_entry);
```

These fixtures intentionally read `argv` so tests prove the host used the spawn ABI:

```text
argv[0] = op_type
argv[1] = ext_index
argv[2] = ext_data_hex
argv[3] = mint_authority_checked
```

- [ ] **Step 6: Implement xUDT operation flow**

Classify operations:

```rust
Transfer
Mint
ProtocolBurn
UserDestruction
```

Rules:

- xUDT always loads Meta except user destruction may return after amount classification and Meta-input absence.
- paused rejects transfer and mint.
- paused allows protocol burn with `mint_authority`.
- paused allows user destruction.
- transfer and protocol burn run AccessList when enabled.
- mint and protocol burn require `mint_authority`.
- tracked mint/protocol burn require exact Meta supply delta.
- transfer, mint, and protocol burn run extensions in sorted Meta order.
- user destruction skips AccessList and extensions.

- [ ] **Step 7: Run xUDT tests**

Run:

```bash
make build MODE=debug
MODE=debug cargo test -p tests xudt_ -- --nocapture
MODE=debug cargo test -p tests plugin_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit xUDT**

Run:

```bash
git add contracts/enhanced-xudt tests/plugins tests/src/tests/xudt.rs tests/src/tests/plugin_runtime.rs
git commit -m "feat: implement enhanced xudt"
```

---

### Task 10: Rewrite Standard and Project Docs Without V1 Naming

**Files:**
- Create: `ref/Enhanced UDT Standard.md`
- Modify: `README.md`
- Modify: `TODO.md`
- Test: `rg -n "V1|v1|sudt-meta-v1|xudt-type-v1|accesslist-type-v1" README.md TODO.md ref/Enhanced\ UDT\ Standard.md contracts lib tests`

- [ ] **Step 1: Write final standard**

Create `ref/Enhanced UDT Standard.md` from the approved design and implemented behavior. Include these normative points:

```text
UDT.type.args = meta_type_hash.
MetaType hardcodes the corresponding UDT code hash.
UDTType does not hardcode the Meta code hash.
config_flags.bit0 is deployment-fixed.
Tracked supply is a protocol-level counter.
User destruction is allowed without mint_authority and does not reduce current_supply.
Protocol burn consumes Meta, requires mint_authority, and reduces current_supply.
Access mode switch transactions MUST NOT contain xUDT cells for the same token.
```

- [ ] **Step 2: Update README**

Ensure `README.md` documents:

```text
Build: make build MODE=debug
Test: MODE=debug make test CARGO_ARGS="-- --nocapture"
Contracts: enhanced-sudt-meta, enhanced-sudt, enhanced-xudt-meta, enhanced-xudt, access-list
Supply tracking is optional and configured at token creation with config_flags.bit0.
```

- [ ] **Step 3: Update TODO**

Replace old completed implementation notes with the new task checklist status. Keep unresolved deployment constants visible:

```text
Open deployment constants:
- ENHANCED_SUDT_CODE_HASH
- ENHANCED_XUDT_CODE_HASH
- ACCESS_LIST_CODE_HASH
- ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST
```

- [ ] **Step 4: Check for accidental V1 naming**

Run:

```bash
rg -n "V1|v1|sudt-meta-v1|sudt-type-v1|xudt-meta-v1|xudt-type-v1|accesslist-type-v1" README.md TODO.md ref/Enhanced\ UDT\ Standard.md contracts lib tests
```

Expected: no output. References under `ref/old` are allowed and excluded from this command.

- [ ] **Step 5: Commit docs**

Run:

```bash
git add ref/Enhanced\ UDT\ Standard.md README.md TODO.md
git commit -m "docs: publish enhanced udt standard"
```

---

### Task 11: Full Verification

**Files:**
- Verify only

- [ ] **Step 1: Run debug build**

Run:

```bash
make build MODE=debug
```

Expected: PASS and `build/debug/` contains:

```text
enhanced-sudt-meta
enhanced-sudt
enhanced-xudt-meta
enhanced-xudt
access-list
dl-allow
dl-deny
spawn-allow
spawn-deny
```

- [ ] **Step 2: Run debug tests**

Run:

```bash
MODE=debug make test CARGO_ARGS="-- --nocapture"
```

Expected: PASS.

- [ ] **Step 3: Run release build**

Run:

```bash
make build MODE=release
```

Expected: PASS.

- [ ] **Step 4: Run release tests**

Run:

```bash
MODE=release make test
```

Expected: PASS.

- [ ] **Step 5: Check git status**

Run:

```bash
git status --short
```

Expected: no unstaged implementation changes.

---

## Self-Review

Spec coverage:

- Binding direction is covered in Tasks 5, 6, 7, and 9.
- `config_flags` and `current_supply` are covered in Tasks 2, 5, 6, 7, and 9.
- sUDT supply semantics are covered in Tasks 5 and 6.
- xUDT supply, paused, access, and extensions are covered in Tasks 7, 8, and 9.
- Access mode switching isolation is covered in Task 7.
- No-`v1` naming is covered by the Naming Policy and Task 10.

Placeholder scan:

- The plan contains no `TBD`, `TODO`, `FIXME`, or intentionally vague implementation slots.
- Legacy code is explicitly reference-only; no task requires copying `ref/old` wholesale.

Type consistency:

- The plan consistently uses `SudtMeta`, `XudtMeta`, `AccessListShard`, `config_flags`, `current_supply`, and the new crate names.
- Operation names are `Transfer`, `Mint`, `ProtocolBurn`, and `UserDestruction`.

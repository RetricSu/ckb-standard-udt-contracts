# Contract Structure Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split oversized contract modules into clear parser, cell scanner, authority, config, and access-list units without changing behavior.

**Architecture:** Keep decomposition inside each contract crate to avoid new RISC-V linking risk. Preserve public entry points used by `run.rs` and tests, and verify with the existing build/test suite after each slice.

**Tech Stack:** Rust 2024, `ckb-std`, `ckb-testtool`, `make build`, `cargo test`.

---

## File Structure

- `contracts/sudt/src/meta/mod.rs`: public API for sUDT runtime validation.
- `contracts/sudt/src/meta/parser.rs`: sUDT meta and script attr parser.
- `contracts/sudt/src/meta/cells.rs`: script args, amount collection, meta lookup, meta lock whitelist.
- `contracts/sudt/src/meta/authority.rs`: authority checks.
- `contracts/sudt/src/meta/supply.rs`: mint/burn supply checks.
- `contracts/access-list/src/meta/mod.rs`: public meta context API for access-list.
- `contracts/access-list/src/meta/parser.rs`: minimal xUDT meta parser.
- `contracts/access-list/src/meta/cells.rs`: meta lookup and lock whitelist.
- `contracts/access-list/src/meta/authority.rs`: authority checks.
- `contracts/xudt-meta/src/meta_cell/mod.rs`: public xUDT meta-cell API.
- `contracts/xudt-meta/src/meta_cell/parser.rs`: xUDT meta and script attr parser.
- `contracts/xudt-meta/src/meta_cell/config.rs`: xUDT config constants and helpers.
- `contracts/xudt-meta/src/meta_cell/cells.rs`: group meta loading and lock whitelist.
- `contracts/xudt-meta/src/meta_cell/token.rs`: xUDT script matching and supply scans.
- `contracts/xudt-meta/src/meta_cell/access_list.rs`: access-list shard parsing and coverage checks.

## Task 1: Split `contracts/sudt/src/meta.rs`

**Files:**
- Delete: `contracts/sudt/src/meta.rs`
- Create: `contracts/sudt/src/meta/mod.rs`
- Create: `contracts/sudt/src/meta/parser.rs`
- Create: `contracts/sudt/src/meta/cells.rs`
- Create: `contracts/sudt/src/meta/authority.rs`
- Create: `contracts/sudt/src/meta/supply.rs`
- Modify: `contracts/sudt/src/lib.rs`

- [ ] **Step 1: Preserve public API**

Keep these functions available through `crate::meta`:

```rust
pub fn load_meta_type_hash_arg() -> Result<[u8; 32], Error>;
pub fn collect_group_amount(source: Source) -> Result<u128, Error>;
pub fn validate_mint(meta_type_hash: &[u8; 32], delta: u128) -> Result<(), Error>;
pub fn validate_burn_or_destruction(meta_type_hash: &[u8; 32], delta: u128) -> Result<(), Error>;
```

- [ ] **Step 2: Move parsing into `parser.rs`**

Move `SudtMeta`, `ScriptAttr`, `parse_meta`, `parse_script_attr_opt`, `parse_script_attr`, `table_offsets`, `single_byte_field`, `u128_field`, `byte32_field`, and `read_u32`.

Expose only:

```rust
pub(crate) struct SudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub mint_authority: Option<ScriptAttr>,
}

pub(crate) struct ScriptAttr {
    pub location: u8,
    pub script_hash: [u8; 32],
}

pub(crate) fn parse_meta(data: &[u8]) -> Result<SudtMeta, Error>;
pub(crate) fn is_supply_tracked(config_flags: u8) -> bool;
```

- [ ] **Step 3: Move cell scanning into `cells.rs`**

Move `load_meta_type_hash_arg`, `collect_group_amount`, `find_unique_visible_meta`, `find_meta_in_source`, `decode_amount`, and `validate_meta_lock`.

Expose:

```rust
pub(crate) fn find_unique_visible_meta(meta_type_hash: &[u8; 32]) -> Result<Option<SudtMeta>, Error>;
pub(crate) fn find_meta_in_source(meta_type_hash: &[u8; 32], source: Source) -> Result<Option<SudtMeta>, Error>;
```

- [ ] **Step 4: Move authority into `authority.rs`**

Move `require_authority`, `check_authority`, `has_input_lock_hash`, `has_type_hash`, and related hash scanning helpers.

Expose:

```rust
pub(crate) fn require_authority(authority: Option<&ScriptAttr>) -> Result<(), Error>;
```

- [ ] **Step 5: Move supply validation into `supply.rs`**

Move `validate_mint`, `validate_initial_create_mint`, and `validate_burn_or_destruction`.

- [ ] **Step 6: Verify sUDT slice**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p sudt -- --nocapture
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p tests tests::sudt:: -- --nocapture
```

Expected: all selected tests pass.

- [ ] **Step 7: Commit**

```bash
git add contracts/sudt/src
git commit -m "refactor: split sudt meta module"
```

## Task 2: Split `contracts/access-list/src/meta.rs`

**Files:**
- Delete: `contracts/access-list/src/meta.rs`
- Create: `contracts/access-list/src/meta/mod.rs`
- Create: `contracts/access-list/src/meta/parser.rs`
- Create: `contracts/access-list/src/meta/cells.rs`
- Create: `contracts/access-list/src/meta/authority.rs`
- Modify: `contracts/access-list/src/lib.rs`

- [ ] **Step 1: Preserve public API**

Keep these items available through `crate::meta`:

```rust
pub struct ScriptAttr {
    pub location: u8,
    pub script_hash: [u8; 32],
}

pub struct MetaContext {
    pub output_config_flags: u8,
    pub access_authority: Option<ScriptAttr>,
}

pub fn load_meta_type_hash_arg() -> Result<[u8; 32], Error>;
pub fn load_meta_context(meta_type_hash: &[u8; 32]) -> Result<MetaContext, Error>;
pub fn check_authority(authority: &ScriptAttr) -> Result<bool, Error>;
```

- [ ] **Step 2: Move parsing into `parser.rs`**

Move `XudtMeta`, `parse_meta`, script attr parsing, config validation, and strict table helpers.

- [ ] **Step 3: Move cell loading into `cells.rs`**

Move `load_meta_type_hash_arg`, `load_meta_context`, `find_meta_in_source`, and `validate_meta_lock`.

- [ ] **Step 4: Move authority into `authority.rs`**

Move `check_authority`, `has_input_lock_hash`, `has_type_hash`, and scanner helpers.

- [ ] **Step 5: Verify access-list slice**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p access-list -- --nocapture
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p tests tests::access_list:: -- --nocapture
```

Expected: all selected tests pass.

- [ ] **Step 6: Commit**

```bash
git add contracts/access-list/src
git commit -m "refactor: split access-list meta module"
```

## Task 3: Split `contracts/xudt-meta/src/meta_cell.rs`

**Files:**
- Delete: `contracts/xudt-meta/src/meta_cell.rs`
- Create: `contracts/xudt-meta/src/meta_cell/mod.rs`
- Create: `contracts/xudt-meta/src/meta_cell/parser.rs`
- Create: `contracts/xudt-meta/src/meta_cell/config.rs`
- Create: `contracts/xudt-meta/src/meta_cell/cells.rs`
- Create: `contracts/xudt-meta/src/meta_cell/token.rs`
- Create: `contracts/xudt-meta/src/meta_cell/access_list.rs`
- Modify: `contracts/xudt-meta/src/lib.rs`

- [ ] **Step 1: Preserve public API**

Keep these functions and types available through `crate::meta_cell`:

```rust
pub struct XudtMeta;
pub struct ScriptAttr;
pub struct MetaGroup;
pub fn load_meta_group() -> Result<MetaGroup, Error>;
pub fn validate_type_args() -> Result<(), Error>;
pub fn validate_create_type_id() -> Result<(), Error>;
pub fn validate_create(output_meta: &XudtMeta, meta_type_hash: &[u8; 32]) -> Result<(), Error>;
pub fn sum_initial_udt_outputs(meta_type_hash: &[u8; 32], udt_code_hash: &[u8; 32]) -> Result<u128, Error>;
pub fn has_same_token_cells(meta_type_hash: &[u8; 32]) -> Result<bool, Error>;
pub fn has_legal_access_list_shard(meta_type_hash: &[u8; 32]) -> Result<bool, Error>;
pub fn has_full_domain_access_list_shards(meta_type_hash: &[u8; 32]) -> Result<bool, Error>;
pub fn is_supply_tracked(config_flags: u8) -> bool;
pub fn access_enabled(config_flags: u8) -> bool;
pub fn whitelist_mode(config_flags: u8) -> bool;
pub fn paused(config_flags: u8) -> bool;
```

- [ ] **Step 2: Move config into `config.rs`**

Move config constants, allowed mask, and helpers: `is_supply_tracked`, `access_enabled`, `whitelist_mode`, `paused`, `validate_config`.

- [ ] **Step 3: Move parser into `parser.rs`**

Move `XudtMeta`, `ScriptAttr`, metadata parser, script attr parser, bytes/table helpers, and extension validation.

- [ ] **Step 4: Move cell group loading into `cells.rs`**

Move `MetaGroup`, `load_meta_group`, `validate_type_args`, `validate_create_type_id`, `load_group_meta`, and `validate_meta_lock`.

- [ ] **Step 5: Move token scanning into `token.rs`**

Move `validate_create`, `sum_initial_udt_outputs`, `has_same_token_cells`, `decode_amount`, and token script matching.

- [ ] **Step 6: Move access-list shard checks into `access_list.rs`**

Move `has_legal_access_list_shard`, `has_full_domain_access_list_shards`, shard parser, byte32 vector parser, shard ordering, nibble alignment, and byte32 increment.

- [ ] **Step 7: Verify xUDT meta slice**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p xudt-meta -- --nocapture
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p tests tests::xudt_meta:: -- --nocapture
```

Expected: all selected tests pass.

- [ ] **Step 8: Commit**

```bash
git add contracts/xudt-meta/src
git commit -m "refactor: split xudt-meta meta_cell module"
```

## Task 4: Final Verification

**Files:**
- Review: all modified files.

- [ ] **Step 1: Run full format check**

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt --check
```

Expected: exit code 0.

- [ ] **Step 2: Run full test suite**

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="-- --nocapture"
```

Expected: exit code 0, including 64 integration tests.

- [ ] **Step 3: Run RISC-V debug build**

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
```

Expected: exit code 0 and binaries in `build/debug`.

- [ ] **Step 4: Check whitespace and status**

```bash
git diff --check
git status --short
```

Expected: no whitespace errors; only intentional committed changes or clean worktree.

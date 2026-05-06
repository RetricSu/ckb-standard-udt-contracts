# Authority Type Schema Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Rename `ScriptAttr` / `ScriptLocation` to `Authority` / `AuthorityType`, split xUDT extensions into `Extension` / `ExtensionType`, and make every authority field support input hash scanning, dynamic linking, and spawn.

**Architecture:** First change the shared Molecule schema and host types, then adapt test builders and contract parsers to retain `script: Option<Script>` for authorities and parse executable-only extensions separately. Add a shared contract-side authority runtime in `lib/script-utils`, wire every contract authority check through it, and keep xUDT extensions on their existing extension ABI.

**Tech Stack:** Rust no_std contracts, Molecule schema/generated Rust bindings, `ckb-std` high-level cell scanning, `CKBDLContext`, `spawn_cell`, `ckb-testtool` integration tests, existing Makefile build/test flow.

---

## File Structure

- Modify `lib/types/src/schemas/metadata.mol`: rename schema types and fields.
- Regenerate `lib/types/src/generated/metadata.rs` using the repository's existing Molecule generation flow.
- Modify `lib/types/src/metadata/script_attr.rs`: split into `authority.rs` and `extension.rs`, exposing `Authority` / `AuthorityType` and `Extension` / `ExtensionType`.
- Modify `lib/types/src/metadata/token.rs`, `access_list.rs`, `mod.rs`, `tests.rs`: update host names and generated type names.
- Modify `lib/script-utils/src/authority.rs`: implement shared authority runtime for authority types `0..=4`.
- Modify `lib/script-utils/src/lib.rs`: export the updated authority API if needed by contracts.
- Modify contract-local metadata parsers:
  - `contracts/sudt/src/meta/parser.rs`
  - `contracts/xudt/src/meta.rs`
  - `contracts/sudt-meta/src/meta_cell.rs`
  - `contracts/xudt-meta/src/meta_cell/parser.rs`
  - `contracts/access-list/src/meta/parser.rs`
- Modify contract authority callers:
  - `contracts/sudt/src/meta/authority.rs`
  - `contracts/xudt/src/meta.rs`
  - `contracts/sudt-meta/src/update.rs`
  - `contracts/xudt-meta/src/update.rs`
  - `contracts/access-list/src/meta/authority.rs`
- Modify xUDT extension code:
  - `contracts/xudt/src/extensions.rs`, change from authority-shaped items to `Extension`.
- Modify tests and fixtures:
  - `tests/src/metadata_builders.rs`
  - `tests/src/tests/plugin_runtime.rs`
  - `tests/src/tests/sudt.rs`
  - `tests/src/tests/sudt_meta.rs`
  - `tests/src/tests/xudt.rs`
  - `tests/src/tests/xudt_meta.rs`
  - `tests/src/tests/access_list.rs`
  - add authority plugin fixtures under `tests/plugins/authority-dl-allow`, `authority-dl-deny`, `authority-spawn-allow`, `authority-spawn-deny` or reuse existing plugin crates with a new entrypoint.

---

### Task 1: Rename Molecule Schema and Host Types

**Files:**
- Modify: `lib/types/src/schemas/metadata.mol`
- Modify: `lib/types/src/metadata/mod.rs`
- Move/Modify: `lib/types/src/metadata/script_attr.rs` -> `lib/types/src/metadata/authority.rs`
- Modify: `lib/types/src/metadata/token.rs`
- Modify: `lib/types/src/metadata/tests.rs`
- Modify generated: `lib/types/src/generated/metadata.rs`
- Modify tests: `tests/src/metadata_builders.rs`, `tests/src/tests/plugin_runtime.rs`, `tests/src/tests/sudt_meta.rs`, `tests/src/tests/xudt.rs`, `tests/src/tests/xudt_meta.rs`

- [x] **Step 1: Update Molecule schema names**

Replace the authority section in `lib/types/src/schemas/metadata.mol` with:

```mol
table Authority {
    authority_type: byte,
    script_hash: Byte32,
    script: ScriptOpt,
}

option AuthorityOpt (Authority);

table Extension {
    extension_type: byte,
    script: Script,
}

vector ExtensionVec <Extension>;
```

Update metadata fields:

```mol
mint_authority: AuthorityOpt,
metadata_authority: AuthorityOpt,
access_authority: AuthorityOpt,
extensions: ExtensionVec,
```

- [x] **Step 2: Regenerate Molecule bindings**

Run the repository's existing generation command. If there is no Makefile target, inspect `build.rs`, `moleculec` usage, or previous generation script, then run the exact generator that updates `lib/types/src/generated/metadata.rs`.

Expected result: generated Rust contains `pub struct Authority`, `AuthorityOpt`, `Extension`, and `ExtensionVec`, and no generated `ScriptAttr` or unused `AuthorityVec` types.

- [x] **Step 3: Rename host authority module**

Move:

```bash
git mv lib/types/src/metadata/script_attr.rs lib/types/src/metadata/authority.rs
```

In `lib/types/src/metadata/mod.rs`, replace:

```rust
mod script_attr;
pub use script_attr::{ScriptAttr, ScriptLocation};
```

with:

```rust
mod authority;
pub use authority::{Authority, AuthorityType};
mod extension;
pub use extension::{Extension, ExtensionType};
```

- [x] **Step 4: Rewrite host authority types**

In `lib/types/src/metadata/authority.rs`, define:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityType {
    InputLock,
    InputType,
    OutputType,
    DynamicLinking,
    Spawn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Authority {
    pub authority_type: AuthorityType,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}
```

Rename helpers:

```rust
unpack_authority_opt
unpack_authority
pack_authority_opt
pack_authority
unpack_authority_vec
pack_authority_vec
validate_authorities
```

Use generated types:

```rust
generated::metadata::Authority
generated::metadata::AuthorityOpt
```

- [x] **Step 5: Add host extension types**

Create `lib/types/src/metadata/extension.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionType {
    DynamicLinking,
    Spawn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Extension {
    pub extension_type: ExtensionType,
    pub script: Script,
}
```

Implement conversion to/from generated `metadata::Extension` and `metadata::ExtensionVec`.

Validation rules:

- `extension_type` accepts only `0` and `1`.
- `extensions.len() <= MAX_EXTENSIONS`.
- Sorting and duplicate checks use `(u8::from(extension.extension_type), script.calc_script_hash())`.

Helper names:

```rust
unpack_extension_vec
pack_extension_vec
validate_extensions
```

- [x] **Step 6: Update token host types**

In `lib/types/src/metadata/token.rs`, replace `ScriptAttr` with `Authority` and helper calls with the renamed authority helpers. Final fields should be:

```rust
pub mint_authority: Option<Authority>,
pub metadata_authority: Option<Authority>,
pub access_authority: Option<Authority>,
pub extensions: Vec<Extension>,
```

- [x] **Step 7: Update host tests and builders**

Replace imports in tests:

```rust
use standard_udt_types::metadata::{Authority, AuthorityType, Extension, ExtensionType};
```

Replace builder construction:

```rust
Authority {
    authority_type: AuthorityType::InputLock,
    script_hash,
    script: None,
}
```

Dynamic linking and spawn fixture authorities must set:

```rust
Authority {
    authority_type: AuthorityType::DynamicLinking,
    script_hash: script.calc_script_hash().unpack(),
    script: Some(script),
}
```

Extension construction must be:

```rust
Extension {
    extension_type: ExtensionType::DynamicLinking,
    script,
}
```

- [x] **Step 8: Verify and commit**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt --check
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p standard-udt-types
```

Expected: all standard-udt-types tests pass.

Commit:

```bash
git add lib/types tests/src/metadata_builders.rs tests/src/tests
git commit -m "refactor: split authority and extension schema"
```

---

### Task 2: Add Shared Contract Authority Runtime

**Files:**
- Modify: `lib/script-utils/src/authority.rs`
- Modify: `lib/script-utils/src/error.rs`
- Modify: `lib/script-utils/src/lib.rs`
- Test: `lib/script-utils/src/authority.rs`

- [x] **Step 1: Define runtime input type**

Add a no_std-compatible contract runtime type:

```rust
pub struct ParsedAuthority {
    pub authority_type: u8,
    pub script_hash: [u8; 32],
    pub script: Option<ckb_std::ckb_types::packed::Script>,
}
```

Keep it independent from `standard_udt_types::metadata::Authority` to avoid linking host generated Molecule parsing into every contract.

- [x] **Step 2: Implement check dispatch**

Implement:

```rust
pub fn check_authority(authority: &ParsedAuthority) -> Result<bool, ScriptError> {
    match authority.authority_type {
        0 => has_input_lock_hash(&authority.script_hash),
        1 => has_input_type_hash(&authority.script_hash),
        2 => has_output_type_hash(&authority.script_hash),
        3 => run_dynamic_linking_authority(authority),
        4 => run_spawn_authority(authority),
        _ => Err(ScriptError::InvalidAuthority),
    }
}
```

If `ScriptError::InvalidAuthority` does not exist, add it at the end of `ScriptError` and assign the next sequential code. Keep existing error code order deterministic.

- [x] **Step 3: Implement dynamic linking authority**

Use `CKBDLContext` and load by `authority.script.code_hash().raw_data()`. Look up symbol:

```rust
type AuthorityFn = unsafe extern "C" fn(*const u8, *const u8, usize) -> i8;
let authorize: Symbol<AuthorityFn> = library
    .get(b"eudt_authorize")
    .ok_or(ScriptError::AuthorityFailed)?;
```

Call:

```rust
let args = script.args().raw_data();
let rc = authorize(
    authority.script_hash.as_ptr(),
    args.as_ptr(),
    args.len(),
);
```

Return `Ok(rc == 0)`.

- [x] **Step 4: Implement spawn authority**

Use:

```rust
spawn_cell(&code_hash, script_hash_type(script)?, &args, &[])
```

Build argv:

```text
argv[0] = hex(script_hash)
argv[1] = hex(script.args)
```

Use existing `wait(pid)` handling. Exit `0` returns `Ok(true)`, nonzero returns `Ok(false)`.

- [x] **Step 5: Validate authority shape before execution**

Add:

```rust
fn validate_authority_shape(authority: &ParsedAuthority) -> Result<(), ScriptError>
```

Rules:

- `0..=2`: `script.is_none()`
- `3..=4`: `script.is_some()` and `calc_script_hash == script_hash`

Invalid shape returns `ScriptError::InvalidAuthority`.

- [x] **Step 6: Unit test scan modes and invalid shapes**

Keep existing scan tests. Add tests that call `check_authority` for invalid shapes:

```rust
assert_eq!(
    check_authority(&ParsedAuthority {
        authority_type: 0,
        script_hash: [0; 32],
        script: Some(dummy_script()),
    }),
    Err(ScriptError::InvalidAuthority)
);
assert_eq!(
    check_authority(&ParsedAuthority {
        authority_type: 3,
        script_hash: [0; 32],
        script: None,
    }),
    Err(ScriptError::InvalidAuthority)
);
```

- [x] **Step 7: Verify and commit**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt --check
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p standard-udt-script-utils
```

Expected: script-utils tests pass.

Commit:

```bash
git add lib/script-utils
git commit -m "feat: add authority runtime for dl and spawn"
```

---

### Task 3: Update Contract Parsers to Preserve Authority Script and Parse Extensions

**Files:**
- Modify: `contracts/sudt/src/meta/parser.rs`
- Modify: `contracts/xudt/src/meta.rs`
- Modify: `contracts/sudt-meta/src/meta_cell.rs`
- Modify: `contracts/xudt-meta/src/meta_cell/parser.rs`
- Modify: `contracts/access-list/src/meta/parser.rs`

- [x] **Step 1: Rename parsed struct fields**

In each parser, replace local `ScriptAttr` with:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedAuthority {
    pub authority_type: u8,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}
```

If a module already has a public `Authority` name with no ambiguity, `Authority` is acceptable, but the field names must be `authority_type`, `script_hash`, and `script`.

- [x] **Step 2: Parse renamed schema field**

Update table parsing to read `authority_type` from field 0. The binary position remains field 0, so parsing logic is unchanged except variable names:

```rust
let authority_type = single_byte_field(data, offsets[0], offsets[1])?;
let script_hash = byte32_field(data, offsets[1], offsets[2])?;
let script_opt = &data[offsets[2]..offsets[3]];
```

- [x] **Step 3: Preserve script for 3/4**

Use:

```rust
let script = match authority_type {
    0..=2 if script_opt.is_empty() => None,
    3 | 4 if !script_opt.is_empty() => {
        let script = Script::from_slice(script_opt).map_err(|_| Error::InvalidMetaData)?;
        let parsed_hash: [u8; 32] = script.calc_script_hash().unpack();
        if parsed_hash != script_hash {
            return Err(Error::InvalidMetaData);
        }
        Some(script)
    }
    0..=4 => return Err(Error::InvalidMetaData),
    _ => return Err(Error::InvalidMetaData),
};
```

Return:

```rust
Ok(ParsedAuthority {
    authority_type,
    script_hash,
    script,
})
```

- [x] **Step 4: Parse xUDT extensions as dedicated executable plugins**

In xUDT and xUDT-meta parsers, define a separate parsed extension:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedExtension {
    pub extension_type: u8,
    pub script: Script,
}
```

Parse `ExtensionVec` entries as a table with 2 fields:

```rust
let extension_type = single_byte_field(data, offsets[0], offsets[1])?;
if extension_type > 1 {
    return Err(Error::InvalidMetaData);
}
let script = Script::from_slice(&data[offsets[1]..offsets[2]])
    .map_err(|_| Error::InvalidMetaData)?;
```

Sort and duplicate check with:

```rust
let script_hash: [u8; 32] = extension.script.calc_script_hash().unpack();
let key = (extension.extension_type, script_hash);
```

- [x] **Step 5: Verify parser compile**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p sudt -p sudt-meta -p xudt -p xudt-meta -p access-list
```

Expected: contracts compile; tests may still fail until authority callers are rewired in the next task.

Commit:

```bash
git add contracts
git commit -m "refactor: preserve parsed authority scripts"
```

---

### Task 4: Wire All Authority Checks to Shared Runtime

**Files:**
- Modify: `contracts/sudt/src/meta/authority.rs`
- Modify: `contracts/xudt/src/meta.rs`
- Modify: `contracts/sudt-meta/src/update.rs`
- Modify: `contracts/xudt-meta/src/update.rs`
- Modify: `contracts/access-list/src/meta/authority.rs`
- Modify: `contracts/xudt/src/extensions.rs`

- [x] **Step 1: Replace per-contract scan-only checks**

Each authority checker should convert local parsed authority into `standard_udt_script_utils::authority::ParsedAuthority` and call shared `check_authority`.

Example:

```rust
fn check_authority(authority: &ParsedAuthority) -> Result<bool, Error> {
    standard_udt_script_utils::authority::check_authority(
        &standard_udt_script_utils::authority::ParsedAuthority {
            authority_type: authority.authority_type,
            script_hash: authority.script_hash,
            script: authority.script.clone(),
        },
    )
    .map_err(map_script_error)
}
```

- [x] **Step 2: Map shared errors to contract errors**

For each contract, map:

```rust
ScriptError::AuthorityFailed => Error::AuthorityFailed
ScriptError::UnsupportedAuthorityLocation => Error::UnsupportedAuthorityLocation
ScriptError::InvalidAuthority => Error::InvalidMetaData
ScriptError::SyscallUnknown => Error::SyscallUnknown
other syscall-derived errors => matching Sys* variants where available
```

If `ScriptError` lacks enough detail for syscall variants, keep local scanning in shared runtime returning `ScriptError::SyscallUnknown` and map it to the contract's `SyscallUnknown`.

- [x] **Step 3: Remove direct scanning helpers**

Delete duplicated helpers where no longer used:

```rust
has_input_lock_hash
has_type_hash
load_cell_lock_hash imports
load_cell_type_hash imports
SysError imports
```

Keep parser helpers and meta lookup helpers unchanged.

- [x] **Step 4: Preserve xUDT extension ABI**

In `contracts/xudt/src/extensions.rs`, change local type references to `ParsedExtension`. Keep this behavior:

```rust
match extension.extension_type {
    0 => run_dynamic_linking_extension(...),
    1 => run_spawn_extension(...),
    _ => return Err(Error::InvalidMetaData),
}
```

Compute the extension script hash when calling `eudt_validate`:

```rust
let script_hash: [u8; 32] = extension.script.calc_script_hash().unpack();
```

Do not use the authority runtime for xUDT extensions, because extensions use `eudt_validate` with operation context, while authority uses `eudt_authorize`.

- [x] **Step 5: Verify contract packages**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt --check
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p sudt -p sudt-meta -p xudt -p xudt-meta -p access-list
```

Expected: all listed packages pass.

Commit:

```bash
git add contracts lib/script-utils
git commit -m "feat: use shared authority runtime in contracts"
```

---

### Task 5: Add Authority Plugin Fixtures and Integration Tests

**Files:**
- Create or Modify: `tests/plugins/authority-dl-allow`
- Create or Modify: `tests/plugins/authority-dl-deny`
- Create or Modify: `tests/plugins/authority-spawn-allow`
- Create or Modify: `tests/plugins/authority-spawn-deny`
- Modify: root `Makefile` plugin build list
- Modify: `tests/src/fixtures.rs` if plugin deployment helpers need names
- Modify: `tests/src/metadata_builders.rs`
- Modify: `tests/src/tests/sudt.rs`
- Modify: `tests/src/tests/sudt_meta.rs`
- Modify: `tests/src/tests/xudt.rs`
- Modify: `tests/src/tests/xudt_meta.rs`
- Modify: `tests/src/tests/access_list.rs`

- [x] **Step 1: Add dynamic linking authority fixtures**

Create C fixtures exporting:

```c
__attribute__((visibility("default"))) int eudt_authorize(
    const unsigned char *script_hash,
    const unsigned char *args,
    unsigned long args_len) {
  (void)script_hash;
  if (args_len == 5 &&
      args[0] == 'a' &&
      args[1] == 'l' &&
      args[2] == 'l' &&
      args[3] == 'o' &&
      args[4] == 'w') {
    return 0;
  }
  return 1;
}
```

The deny fixture returns `1`.

- [x] **Step 2: Add spawn authority fixtures**

Create no_std Rust fixtures where `program_entry()` returns:

```rust
0
```

for allow, and:

```rust
1
```

for deny. They should accept exactly two argv items and return a distinct nonzero code if argv count is wrong, so ABI mistakes are caught.

- [x] **Step 3: Add metadata builder helpers**

In `tests/src/metadata_builders.rs`, add:

```rust
pub fn dynamic_linking_authority(deployed: &DeployedScript) -> Authority
pub fn spawn_authority(deployed: &DeployedScript) -> Authority
pub fn dynamic_linking_extension(deployed: &DeployedScript) -> Extension
pub fn spawn_extension(deployed: &DeployedScript) -> Extension
```

Each helper sets:

```rust
authority_type: AuthorityType::DynamicLinking // or Spawn
script_hash: deployed.script_hash
script: Some(deployed.script.clone())
```

Extension helpers set:

```rust
extension_type: ExtensionType::DynamicLinking // or Spawn
script: deployed.script.clone()
```

- [x] **Step 4: Add sUDT authority tests**

Add tests:

```rust
sudt_mint_with_dynamic_linking_authority_passes
sudt_mint_with_dynamic_linking_authority_denies
sudt_mint_with_spawn_authority_passes
sudt_mint_with_spawn_authority_denies
```

Each test should mint by including the meta dep and using the plugin authority as `mint_authority`.

- [x] **Step 5: Add sUDT meta authority tests**

Replace the old `sudt_meta_update_rejects_dynamic_linking_authority_for_now` with pass/deny tests:

```rust
sudt_meta_update_metadata_change_with_dynamic_linking_authority_passes
sudt_meta_update_metadata_change_with_spawn_authority_passes
sudt_meta_update_metadata_change_with_dynamic_linking_authority_denies
sudt_meta_update_metadata_change_with_spawn_authority_denies
```

- [x] **Step 6: Add xUDT authority tests**

Add mint authority tests equivalent to sUDT:

```rust
xudt_mint_with_dynamic_linking_authority_passes
xudt_mint_with_spawn_authority_passes
xudt_mint_with_dynamic_linking_authority_denies
xudt_mint_with_spawn_authority_denies
```

Keep existing xUDT extension tests; they verify a different ABI.

- [x] **Step 7: Add xUDT meta access authority tests**

Add tests proving access mode or paused updates can use dynamic linking and spawn authorities:

```rust
xudt_meta_access_update_with_dynamic_linking_authority_passes
xudt_meta_access_update_with_spawn_authority_passes
xudt_meta_access_update_with_dynamic_linking_authority_denies
xudt_meta_access_update_with_spawn_authority_denies
```

- [x] **Step 8: Add AccessList authority tests**

Add:

```rust
access_list_update_with_dynamic_linking_authority_passes
access_list_update_with_spawn_authority_passes
access_list_update_with_dynamic_linking_authority_denies
access_list_update_with_spawn_authority_denies
```

- [x] **Step 9: Verify full test suite**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="-- --nocapture"
```

Expected: all integration tests pass, including the new authority dl/spawn cases.

Commit:

```bash
git add tests Makefile
git commit -m "test: cover dynamic and spawn authorities"
```

---

### Task 6: Update Spec Text and Final Verification

**Files:**
- Modify: `ref/Enhanced UDT Standard V1.md`
- Modify: `TODO.md`
- Review: `AGENTS.md`

- [x] **Step 1: Update standard terminology**

In `ref/Enhanced UDT Standard V1.md`, replace the authority model section with `Authority` / `AuthorityType`. Remove the old implementation note that says no-witness authority path fails closed for 3/4.

Use:

```text
所有权限字段使用 AuthorityOpt 表达。
AuthorityType=3/4 MUST be executable in every authority field, not only xUDT extensions.
```

- [x] **Step 2: Update schema section**

Replace:

```mol
option ScriptAttrOpt (ScriptAttr);
vector ScriptAttrVec <ScriptAttr>;
```

with:

```mol
option AuthorityOpt (Authority);
table Extension { extension_type: byte, script: Script }
vector ExtensionVec <Extension>;
```

- [x] **Step 3: Update TODO status**

In `TODO.md`, mark the authority rename and runtime unification as completed or add a completed note if the TODO format is free-form.

- [x] **Step 4: Run release/debug safety checks**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt --check
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p standard-udt-types -p standard-udt-script-utils -p sudt -p sudt-meta -p xudt -p xudt-meta -p access-list
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=release
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="-- --nocapture"
git diff --check
```

Expected:

- all tests pass
- release build succeeds
- debug build succeeds
- no whitespace errors

- [x] **Step 5: Commit docs and final cleanup**

Run:

```bash
git status --short
git add ref/Enhanced\ UDT\ Standard\ V1.md TODO.md
git commit -m "docs: update authority type standard"
```

If code changes remain uncommitted, stop and inspect them before committing.

---

## Self-Review

Spec coverage:

- Schema rename is covered by Task 1.
- Host type rename is covered by Task 1.
- Contract parser preservation of `script` is covered by Task 3.
- Shared authority runtime for all five authority types is covered by Task 2 and Task 4.
- xUDT extension schema split and ABI separation are covered by Task 1, Task 3, and Task 4.
- Integration tests for dynamic linking and spawn authority domains are covered by Task 5.
- Spec and TODO updates are covered by Task 6.

Placeholder scan:

- No `TBD`, `TODO`, or open implementation placeholders are intentionally left in this plan.

Type consistency:

- Public names are consistently `Authority`, `AuthorityType`, `Extension`, and `ExtensionType`.
- Binary schema field is consistently `authority_type`.
- Contract-local parsed type is consistently `ParsedAuthority` unless a module-local `Authority` name is chosen during implementation.

# AccessList State Machine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make whitelist and blacklist share one full-domain AccessList lifecycle while keeping mode interpretation isolated in xUDT.

**Architecture:** `xudt-meta` classifies meta access mode transitions and requires full-domain AccessList presence for create, destroy, and replace. `access-list` validates the concrete shard group lifecycle and all shard invariants. `xudt` remains a proof consumer and does not validate lifecycle or full-domain structure.

**Tech Stack:** Rust no-std CKB scripts, ckb-std, ckb-testtool integration tests, existing Molecule metadata builders.

---

## File Structure

- Modify `contracts/access-list/src/meta/mod.rs`: expose both input and output access modes to `entry`.
- Modify `contracts/access-list/src/meta/cells.rs`: keep meta discovery focused on parsing meta; do not validate meta lock.
- Modify `contracts/access-list/src/shards.rs`: replace mode-only validation with lifecycle validation using old mode and new mode.
- Modify `contracts/access-list/src/entry.rs`: pass old and new access modes into shard validation.
- Modify `contracts/xudt-meta/src/meta_cell/access_list.rs`: add full-domain output scanner and reuse full-domain input scanner.
- Modify `contracts/xudt-meta/src/update.rs`: require full-domain inputs/outputs for mode create/destroy/replace.
- Modify `tests/src/tests/access_list.rs`: add lifecycle tests for whitelist and repeated create/fork prevention.
- Modify `tests/src/tests/xudt_meta.rs`: add transition tests for full-domain create/destroy/replace.
- Modify `docs/superpowers/specs/2026-05-06-access-list-state-machine-design.md`: adjust if implementation finds a necessary clarification.

---

### Task 1: Add AccessList Lifecycle Test Coverage

**Files:**
- Modify: `tests/src/tests/access_list.rs`

- [x] **Step 1: Add a helper that builds different input and output meta modes**

Add this helper near `access_list_update_tx`. It intentionally uses an always-success fake meta type script carrying valid xUDT meta data, so these tests isolate `access-list` behavior and are not intercepted by `xudt-meta` transition checks.

```rust
fn access_list_transition_tx(
    input_config_flags: u8,
    output_config_flags: u8,
    include_authority_input: bool,
    input_shards: Vec<Bytes>,
    output_shards: Vec<Bytes>,
) -> AccessListCase {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let meta = always_success_lock(&mut context, Bytes::from(vec![3u8; 32]));
    let access_list = access_list_script(&mut context, meta.script_hash);

    let meta_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &meta.script,
        100_000_000_000,
        xudt_meta_data(input_config_flags, &authority),
    );
    let mut builder = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(meta_out_point)
                .build(),
        )
        .output(typed_output(
            &cell_lock.script,
            &meta.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(output_config_flags, &authority).pack())
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&authority))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list));

    if include_authority_input {
        let out_point = context.create_cell(
            ckb_testtool::ckb_types::packed::CellOutput::new_builder()
                .capacity(100_000_000_000u64.pack())
                .lock(authority.script.clone())
                .build(),
            Bytes::new(),
        );
        builder = builder.input(CellInput::new_builder().previous_output(out_point).build());
    }

    for data in input_shards {
        let out_point = create_typed_cell(
            &mut context,
            &cell_lock.script,
            &access_list.script,
            100_000_000_000,
            data,
        );
        builder = builder.input(CellInput::new_builder().previous_output(out_point).build());
    }

    for data in output_shards {
        builder = builder
            .output(typed_output(
                &cell_lock.script,
                &access_list.script,
                100_000_000_000,
            ))
            .output_data(data.pack());
    }

    let tx = context.complete_tx(builder.build());
    AccessListCase { context, tx }
}
```

- [x] **Step 2: Add failing tests for whitelist full-domain lifecycle**

Append these tests:

```rust
#[test]
fn access_list_disabled_to_disabled_rejects_access_list_inputs_or_outputs() {
    let with_input = access_list_transition_tx(
        0,
        0,
        true,
        vec![full_domain_shard(Vec::new())],
        Vec::new(),
    );
    expect_tx_fail_with_code(&with_input.context, &with_input.tx, "error code 61");

    let with_output = access_list_transition_tx(
        0,
        0,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );
    expect_tx_fail_with_code(&with_output.context, &with_output.tx, "error code 61");
}

#[test]
fn access_list_whitelist_create_requires_full_domain_outputs() {
    let partial = access_list_transition_tx(
        0,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![custom_shard([0u8; 32], prefix_end(0x7f), Vec::new())],
    );
    expect_tx_fail_with_code(&partial.context, &partial.tx, "error code 61");

    let full = access_list_transition_tx(
        0,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );
    expect_tx_pass(&full.context, &full.tx);
}

#[test]
fn access_list_whitelist_rejects_repeated_create_from_empty_inputs() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}

#[test]
fn access_list_blacklist_rejects_repeated_create_from_empty_inputs() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}
```

- [x] **Step 3: Add failing tests for whitelist update/split/merge parity**

Append:

```rust
#[test]
fn access_list_whitelist_allows_same_range_insert_delete() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x0f), vec![entry(0x10)])],
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x0f),
            vec![entry(0x10), entry(0x20)],
        )],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_whitelist_allows_split_preserving_entries() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![
                prefix_entry(0x08),
                prefix_entry(0x20),
            ],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(prefix_start(0x10), prefix_end(0x2f), vec![prefix_entry(0x20)]),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_whitelist_rejects_split_that_changes_entries() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![prefix_entry(0x08)],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(prefix_start(0x10), prefix_end(0x2f), vec![prefix_entry(0x20)]),
        ],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}
```

- [x] **Step 4: Add failing tests for blacklist local update parity**

Append:

```rust
#[test]
fn access_list_blacklist_allows_local_same_range_insert_delete() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x0f), vec![entry(0x10)])],
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x0f),
            vec![entry(0x10), entry(0x20)],
        )],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_local_split_preserving_entries() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![
                prefix_entry(0x08),
                prefix_entry(0x20),
            ],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(prefix_start(0x10), prefix_end(0x2f), vec![prefix_entry(0x20)]),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}
```

- [x] **Step 5: Add failing tests for destroy and replace**

Append:

```rust
#[test]
fn access_list_active_destroy_requires_full_domain_inputs_and_empty_outputs() {
    let partial = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x7f), Vec::new())],
        Vec::new(),
    );
    expect_tx_fail_with_code(&partial.context, &partial.tx, "error code 61");

    let with_output = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![full_domain_shard(Vec::new())],
    );
    expect_tx_fail_with_code(&with_output.context, &with_output.tx, "error code 61");

    let full_destroy = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![full_domain_shard(Vec::new())],
        Vec::new(),
    );
    expect_tx_pass(&full_destroy.context, &full_destroy.tx);
}

#[test]
fn access_list_mode_replace_requires_full_domain_inputs_and_outputs_but_allows_entry_reset() {
    let missing_input = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(vec![entry(0x20)])],
    );
    expect_tx_fail_with_code(&missing_input.context, &missing_input.tx, "error code 61");

    let full_replace = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
        vec![full_domain_shard(vec![entry(0x20)])],
    );
    expect_tx_pass(&full_replace.context, &full_replace.tx);
}
```

- [x] **Step 6: Run tests and verify failures**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="access_list_ -- --nocapture"
```

Expected before implementation: the new disabled-to-disabled, repeated-create, whitelist full-domain, destroy, replace, and local update tests expose current behavior gaps.

---

### Task 2: Refactor AccessList Meta Context

**Files:**
- Modify: `contracts/access-list/src/meta/mod.rs`

- [x] **Step 1: Replace `MetaContext` fields**

Change `MetaContext` to include old and new config flags:

```rust
pub struct MetaContext {
    pub input_config_flags: Option<u8>,
    pub output_config_flags: Option<u8>,
    pub access_authority: Option<ParsedAuthority>,
}
```

- [x] **Step 2: Update `load_meta_context`**

Replace the returned context construction with:

```rust
let authority_meta = input
    .as_ref()
    .or(cell_dep.as_ref())
    .or(output.as_ref())
    .ok_or(Error::MetaMissing)?;

Ok(MetaContext {
    input_config_flags: input
        .as_ref()
        .or(cell_dep.as_ref())
        .map(|meta| meta.config_flags),
    output_config_flags: output
        .as_ref()
        .or(cell_dep.as_ref())
        .map(|meta| meta.config_flags),
    access_authority: authority_meta.access_authority.clone(),
})
```

`CellDep` meta is the fallback for a side of the transition only when that side has no meta input/output. Do not reject a transaction merely because both `CellDep` meta and input/output meta are visible; meta cell uniqueness is owned by the meta type script and its type-id/type-group checks, not by `access-list`.

- [x] **Step 3: Continue without a standalone build**

Do not run `make build` after only this task. `MetaContext` now has a new shape and `access-list/src/entry.rs` is intentionally updated in Task 3 before the next compile check.

Expected: no command is run in this step.

---

### Task 3: Implement Lifecycle Classification in AccessList

**Files:**
- Modify: `contracts/access-list/src/shards.rs`
- Modify: `contracts/access-list/src/entry.rs`

- [x] **Step 1: Add lifecycle enum and active helper**

In `contracts/access-list/src/shards.rs`, add near `AccessListShard`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccessListLifecycle {
    Create,
    Update,
    Destroy,
    Replace,
}

fn is_active(mode: AccessMode) -> bool {
    mode != AccessMode::Disabled
}
```

- [x] **Step 2: Replace `validate_shards_for_mode` signature**

Replace:

```rust
pub fn validate_shards_for_mode(
    mode: AccessMode,
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
```

with:

```rust
pub fn validate_shards_for_modes(
    input_mode: AccessMode,
    output_mode: AccessMode,
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
```

- [x] **Step 3: Implement lifecycle rules**

Use this implementation body:

```rust
    validate_ordered_non_overlapping(input_shards)?;
    validate_ordered_non_overlapping(output_shards)?;

    match classify_lifecycle(input_mode, output_mode)? {
        AccessListLifecycle::Create => {
            if !input_shards.is_empty() {
                return Err(Error::InvalidShardSet);
            }
            validate_full_domain(output_shards)
        }
        AccessListLifecycle::Destroy => {
            validate_full_domain(input_shards)?;
            if output_shards.is_empty() {
                Ok(())
            } else {
                Err(Error::InvalidShardSet)
            }
        }
        AccessListLifecycle::Replace => {
            validate_full_domain(input_shards)?;
            validate_full_domain(output_shards)
        }
        AccessListLifecycle::Update => {
            validate_local_replacement_range(input_shards, output_shards)?;
            validate_update_diff(input_shards, output_shards)
        }
    }
```

- [x] **Step 4: Add lifecycle classifier**

Add below `validate_shards_for_modes`:

```rust
fn classify_lifecycle(
    input_mode: AccessMode,
    output_mode: AccessMode,
) -> Result<AccessListLifecycle, Error> {
    match (is_active(input_mode), is_active(output_mode)) {
        (false, false) => Err(Error::InvalidShardSet),
        (false, true) => Ok(AccessListLifecycle::Create),
        (true, false) => Ok(AccessListLifecycle::Destroy),
        (true, true) if input_mode == output_mode => Ok(AccessListLifecycle::Update),
        (true, true) => Ok(AccessListLifecycle::Replace),
    }
}
```

- [x] **Step 5: Rename blacklist diff to update diff**

Rename:

```rust
fn validate_blacklist_diff(
```

to:

```rust
fn validate_update_diff(
```

Keep its body unchanged. This makes same-mode whitelist and blacklist updates share the same insert/delete/split/merge rules over the touched local range.

- [x] **Step 6: Add local replacement range validation**

Add below `validate_update_diff`:

```rust
fn validate_local_replacement_range(
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
    validate_contiguous_local_range(input_shards)?;
    validate_contiguous_local_range(output_shards)?;

    let input_start = input_shards
        .first()
        .ok_or(Error::InvalidShardSet)?
        .start;
    let input_end = input_shards.last().ok_or(Error::InvalidShardSet)?.end;
    let output_start = output_shards
        .first()
        .ok_or(Error::InvalidShardSet)?
        .start;
    let output_end = output_shards.last().ok_or(Error::InvalidShardSet)?.end;

    if input_start == output_start && input_end == output_end {
        Ok(())
    } else {
        Err(Error::InvalidShardSet)
    }
}

fn validate_contiguous_local_range(shards: &[AccessListShard]) -> Result<(), Error> {
    if shards.is_empty() {
        return Err(Error::InvalidShardSet);
    }

    let mut expected_start = shards[0].start;
    for shard in shards {
        if shard.start != expected_start {
            return Err(Error::InvalidShardSet);
        }

        let Some(next_start) = increment_byte32(&shard.end) else {
            return Ok(());
        };
        expected_start = next_start;
    }

    Ok(())
}
```

- [x] **Step 7: Update entry mode handling**

In `contracts/access-list/src/entry.rs`, replace:

```rust
let mode = AccessMode::from_flags(meta_context.output_config_flags)?;
validate_shards_for_mode(mode, &input_shards, &output_shards)?;
```

with:

```rust
let input_mode = match meta_context.input_config_flags {
    Some(flags) => AccessMode::from_flags(flags)?,
    None => AccessMode::Disabled,
};
let output_mode = match meta_context.output_config_flags {
    Some(flags) => AccessMode::from_flags(flags)?,
    None => AccessMode::Disabled,
};
validate_shards_for_modes(input_mode, output_mode, &input_shards, &output_shards)?;
```

- [x] **Step 8: Remove disabled-mode authority bypass**

Delete this block from `contracts/access-list/src/entry.rs`:

```rust
if mode == AccessMode::Disabled && output_shards.is_empty() {
    return Ok(());
}
```

Do not replace it with another disabled-mode bypass. In the tightened state machine, `Disabled -> Disabled` is not an AccessList lifecycle operation. If no AccessList cells exist, this script is not invoked; if it is invoked, the transaction contains AccessList group inputs or outputs under disabled mode and `validate_shards_for_modes` rejects it.

- [x] **Step 9: Update import**

Change the import in `entry.rs`:

```rust
shards::{collect_group_shards, validate_shards_for_mode},
```

to:

```rust
shards::{collect_group_shards, validate_shards_for_modes},
```

- [x] **Step 10: Verify access-list tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="access_list_ -- --nocapture"
```

Expected: access-list lifecycle tests pass.


---

### Task 4: Strengthen xUDT Meta Transition Checks

**Files:**
- Modify: `contracts/xudt-meta/src/meta_cell/access_list.rs`
- Modify: `contracts/xudt-meta/src/meta_cell/mod.rs`
- Modify: `contracts/xudt-meta/src/update.rs`
- Modify: `tests/src/tests/xudt_meta.rs`

- [x] **Step 1: Add full-domain output scanner**

In `contracts/xudt-meta/src/meta_cell/access_list.rs`, add:

```rust
pub fn has_full_domain_access_list_outputs(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    has_full_domain_access_list_cells(meta_type_hash, Source::Output)
}
```

Replace the current `has_full_domain_access_list_inputs` body with:

```rust
pub fn has_full_domain_access_list_inputs(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    has_full_domain_access_list_cells(meta_type_hash, Source::Input)
}
```

Then add:

```rust
fn has_full_domain_access_list_cells(
    meta_type_hash: &[u8; 32],
    source: Source,
) -> Result<bool, Error> {
    let mut ranges = alloc::vec::Vec::new();
    let mut index = 0;

    loop {
        match load_cell_type(index, source) {
            Ok(Some(script)) if is_access_list_script(&script, meta_type_hash) => {
                let data = load_cell_data(index, source)?;
                ranges.push(parse_access_list_range(&data)?);
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => break,
            Err(error) => return Err(error.into()),
        }
    }

    ranges.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    Ok(covers_full_domain(&ranges))
}
```

- [x] **Step 2: Export full-domain output scanner**

In `contracts/xudt-meta/src/meta_cell/mod.rs`, update the export:

```rust
pub use access_list::{
    has_full_domain_access_list_inputs, has_full_domain_access_list_outputs,
};
```

- [x] **Step 3: Replace presence checks in update transition**

In `contracts/xudt-meta/src/update.rs`, import `has_full_domain_access_list_outputs`.

Replace `validate_access_mode_transition` match body with:

```rust
match (
    input_enabled,
    input_whitelist,
    output_enabled,
    output_whitelist,
) {
    (false, false, true, false) | (false, false, true, true) => {
        if !has_full_domain_access_list_outputs(meta_type_hash)? {
            return Err(Error::AccessListRequired);
        }
    }
    (true, false, false, false) | (true, true, false, false) => {
        if !has_full_domain_access_list_inputs(meta_type_hash)? {
            return Err(Error::AccessListRequired);
        }
    }
    (true, false, true, true) | (true, true, true, false) => {
        if !has_full_domain_access_list_inputs(meta_type_hash)?
            || !has_full_domain_access_list_outputs(meta_type_hash)?
        {
            return Err(Error::AccessListRequired);
        }
    }
    _ => {}
}
```

Remove the old `has_access_list_output` import and any call sites in `validate_access_mode_transition`; the transition now uses only full-domain input/output scanners.

- [x] **Step 4: Add xudt-meta transition helpers**

In `tests/src/tests/xudt_meta.rs`, add this helper near `full_domain_shard()`:

```rust
fn half_domain_shard() -> Bytes {
    build_access_list_shard_bytes([0u8; 32], [0x7fu8; 32], Vec::new())
}
```

Add this helper near `update_meta_tx`:

```rust
fn access_mode_transition_tx(
    input_flags: u8,
    output_flags: u8,
    include_full_input: bool,
    include_full_output: bool,
) -> UpdateCase {
    update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let mut extra_cells = Vec::new();

        if include_full_input {
            let access_list = access_list_script(context, meta.script_hash);
            extra_cells.push(ExtraCell::Input {
                previous_output: create_typed_cell(
                    context,
                    &lock.script,
                    &access_list.script,
                    100_000_000_000,
                    full_domain_shard(),
                ),
                cell_dep: access_list,
            });
        }

        if include_full_output {
            let access_list = access_list_script(context, meta.script_hash);
            extra_cells.push(ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: full_domain_shard(),
                cell_dep: access_list,
            });
        }

        (
            xudt_meta_data(input_flags, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(output_flags, 0, None, None, Some(authority), Vec::new()),
            extra_cells,
        )
    })
}
```

- [x] **Step 5: Add xudt-meta full-domain transition tests**

Add `expect_tx_fail` to the `tests/src/tests/xudt_meta.rs` fixtures import if it is not already imported.

Append:

```rust
#[test]
fn xudt_meta_blacklist_to_whitelist_requires_full_domain_inputs_and_outputs() {
    let missing_input = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        false,
        true,
    );
    expect_tx_fail(&missing_input.context, &missing_input.tx);

    let missing_output = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        false,
    );
    expect_tx_fail(&missing_output.context, &missing_output.tx);

    let full_replace = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        true,
    );
    expect_tx_pass(&full_replace.context, &full_replace.tx);
}

#[test]
fn xudt_meta_whitelist_to_blacklist_requires_full_domain_inputs_and_outputs() {
    let missing_input = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED,
        false,
        true,
    );
    expect_tx_fail(&missing_input.context, &missing_input.tx);

    let missing_output = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED,
        true,
        false,
    );
    expect_tx_fail(&missing_output.context, &missing_output.tx);

    let full_replace = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED,
        true,
        true,
    );
    expect_tx_pass(&full_replace.context, &full_replace.tx);
}

#[test]
fn xudt_meta_whitelist_to_disabled_requires_full_domain_inputs() {
    let missing_input = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        false,
        false,
    );
    expect_tx_fail_with_code(&missing_input.context, &missing_input.tx, "error code 60");

    let full_input = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        false,
    );
    expect_tx_pass(&full_input.context, &full_input.tx);
}
```

- [x] **Step 6: Add xudt-meta partial full-domain rejection test**

Append:

```rust
#[test]
fn xudt_meta_active_transition_rejects_partial_access_list_domain() {
    let partial_input = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let input_access_list = access_list_script(context, meta.script_hash);
        let output_access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(CONFIG_ACCESS_ENABLED, 0, None, None, Some(authority), Vec::new()),
            vec![
                ExtraCell::Input {
                    previous_output: create_typed_cell(
                        context,
                        &lock.script,
                        &input_access_list.script,
                        100_000_000_000,
                        half_domain_shard(),
                    ),
                    cell_dep: input_access_list,
                },
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: output_access_list.script.clone(),
                    data: full_domain_shard(),
                    cell_dep: output_access_list,
                },
            ],
        )
    });

    expect_tx_fail_with_code(&partial_input.context, &partial_input.tx, "error code 60");
}
```

Use the existing `update_meta_tx`, `access_list_script`, `ExtraCell::Input`, `ExtraCell::Output`, `create_typed_cell`, and `full_domain_shard()` helpers.

- [x] **Step 7: Verify xudt-meta tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="xudt_meta_ -- --nocapture"
```

Expected: xudt-meta transition tests pass.

---

### Task 5: Preserve xUDT Proof Consumer Boundary

**Files:**
- Review only: `contracts/xudt/src/access.rs`
- Modify only if tests expose a regression: `tests/src/tests/xudt.rs`

- [x] **Step 1: Confirm no full-domain lifecycle checks were added to xUDT**

Run:

```bash
rg -n "full_domain|validate_full_domain|AccessListLifecycle|validate_shards_for" contracts/xudt/src
```

Expected: no matches.

- [x] **Step 2: Run xUDT proof tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="xudt_ -- --nocapture"
```

Expected: existing xUDT proof tests pass. If any fail, fix only proof collection or test setup; do not move lifecycle checks into xUDT.

---

### Task 6: Update Documentation and Run Full Verification

**Files:**
- Modify: `docs/superpowers/specs/2026-05-06-cell-model-invariants-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-cell-model-invariants.md`
- Verify: all touched Rust files and tests

- [x] **Step 1: Update existing cell-model spec**

In `docs/superpowers/specs/2026-05-06-cell-model-invariants-design.md`, update AccessList ownership language to say:

```markdown
Whitelist and blacklist share the same full-domain AccessList shard structure and lifecycle rules. They differ only in xUDT proof interpretation. Switching between whitelist and blacklist consumes the old full-domain list and creates a new full-domain list.
```

- [x] **Step 2: Update existing plan notes**

In `docs/superpowers/plans/2026-05-06-cell-model-invariants.md`, add a short follow-up note:

```markdown
Follow-up AccessList state-machine tightening is specified in `docs/superpowers/specs/2026-05-06-access-list-state-machine-design.md` and planned in `docs/superpowers/plans/2026-05-06-access-list-state-machine.md`.
```

- [x] **Step 3: Run full verification**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt --check
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test
git diff --check
```

Expected:

- fmt check passes.
- debug RISC-V build passes.
- full test suite passes.
- diff check reports no whitespace errors.

---

## Self-Review

- Spec coverage: covered unified whitelist/blacklist structure, mode replace semantics, create/update/destroy, script boundaries, disabled-mode rejection, and tests.
- Placeholder scan: no incomplete sections remain; tests and helper shapes are written directly in the plan.
- Boundary check: lifecycle/full-domain checks live in `xudt-meta` and `access-list`; `xudt` remains proof-only.

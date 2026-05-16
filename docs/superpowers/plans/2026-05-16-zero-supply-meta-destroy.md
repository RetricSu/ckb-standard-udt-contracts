# Zero Supply Meta Destroy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow tracked-supply sUDT and xUDT metadata cells to be destroyed when tracked supply is zero, and require active xUDT AccessList cells to be destroyed with the xUDT metadata cell.

**Architecture:** Add destroy-specific integration tests first, then add `(Some(input), None)` entrypoint branches in `sudt-meta` and `xudt-meta`. Keep lifecycle ownership in metadata scripts, reuse existing authority helpers, and use existing xUDT full-domain AccessList scanning for active access retirement.

**Tech Stack:** Rust no_std CKB contracts, `ckb-std`, `standard-udt-types`, `ckb-testtool`, repository Makefile/Cargo test flow.

---

## File Structure

- Modify `tests/src/tests/sudt_meta/fixture.rs`: add a `destroy_meta_tx_with_data` fixture that creates one sUDT metadata input and no metadata output.
- Modify `tests/src/tests/sudt_meta/supply.rs`: add zero-supply destroy pass and rejection tests.
- Modify `tests/src/tests/xudt_meta/fixture.rs`: add a `destroy_meta_tx` fixture that supports optional AccessList inputs.
- Modify `tests/src/tests/xudt_meta/access_transitions.rs`: add xUDT metadata destroy tests because they exercise access-mode retirement.
- Modify `contracts/sudt-meta/src/entry.rs`: route `(Some(input), None)` to a new destroy validator.
- Modify `contracts/sudt-meta/src/update.rs`: add `validate_destroy` using existing supply-mode checks and `mint_authority`.
- Modify `contracts/xudt-meta/src/entry.rs`: route `(Some(input), None)` to a new destroy validator.
- Modify `contracts/xudt-meta/src/update.rs`: add `validate_destroy` using existing supply, `mint_authority`, and full-domain AccessList helpers.

---

### Task 1: sUDT Meta Destroy Tests

**Files:**
- Modify: `tests/src/tests/sudt_meta/fixture.rs`
- Modify: `tests/src/tests/sudt_meta/supply.rs`

- [ ] **Step 1: Add a destroy fixture**

In `tests/src/tests/sudt_meta/fixture.rs`, add this helper after `update_meta_tx_with_data`:

```rust
pub(super) fn destroy_meta_tx_with_data<F>(build_data: F) -> (Context, TransactionView)
where
    F: FnOnce([u8; 32]) -> Bytes,
{
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let input_meta_data = build_data(lock.script_hash);
    let meta = meta_script(&mut context, Bytes::from(vec![2u8; 32]));
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);
    (context, tx)
}
```

- [ ] **Step 2: Add failing sUDT destroy tests**

In `tests/src/tests/sudt_meta/supply.rs`, add:

```rust
#[test]
fn sudt_meta_destroy_accepts_tracked_zero_supply() {
    let (context, tx) = destroy_meta_tx_with_data(|lock_hash| {
        sudt_meta_data(
            CONFIG_SUPPLY_TRACKED,
            0,
            None,
            Some(input_lock_authority(lock_hash)),
            Vec::new(),
            Vec::new(),
        )
    });

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_destroy_rejects_tracked_nonzero_supply() {
    let (context, tx) = destroy_meta_tx_with_data(|lock_hash| {
        sudt_meta_data(
            CONFIG_SUPPLY_TRACKED,
            1,
            None,
            Some(input_lock_authority(lock_hash)),
            Vec::new(),
            Vec::new(),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_destroy_rejects_untracked_zero_supply() {
    let (context, tx) = destroy_meta_tx_with_data(|lock_hash| {
        sudt_meta_data(
            0,
            0,
            None,
            Some(input_lock_authority(lock_hash)),
            Vec::new(),
            Vec::new(),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}
```

- [ ] **Step 3: Run the first sUDT destroy test and verify RED**

Run:

```bash
cargo test -p tests sudt_meta_destroy_accepts_tracked_zero_supply -- --nocapture
```

Expected: FAIL with the transaction rejected by `sudt-meta` because `(Some(input), None)` still returns `InvalidArgs`.

---

### Task 2: Implement sUDT Meta Destroy

**Files:**
- Modify: `contracts/sudt-meta/src/entry.rs`
- Modify: `contracts/sudt-meta/src/update.rs`

- [ ] **Step 1: Add the entrypoint branch**

In `contracts/sudt-meta/src/entry.rs`, replace the final match arm with:

```rust
        (Some(input), Some(output)) => {
            crate::update::validate_update(input, output, &group.meta_type_hash)
        }
        (Some(input), None) => crate::update::validate_destroy(input),
        _ => Err(Error::InvalidArgs),
```

- [ ] **Step 2: Add the destroy validator**

In `contracts/sudt-meta/src/update.rs`, add this public function after
`validate_update`:

```rust
pub fn validate_destroy(input: &SudtMeta) -> Result<(), Error> {
    if !is_supply_tracked(input.config_flags) || input.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    require_authority(input.mint_authority.as_ref())
}
```

- [ ] **Step 3: Run sUDT destroy tests and verify GREEN**

Run:

```bash
cargo test -p tests sudt_meta_destroy_ -- --nocapture
```

Expected: all `sudt_meta_destroy_*` tests pass.

---

### Task 3: xUDT Meta Destroy Tests

**Files:**
- Modify: `tests/src/tests/xudt_meta/fixture.rs`
- Modify: `tests/src/tests/xudt_meta/access_transitions.rs`

- [ ] **Step 1: Add an xUDT destroy fixture**

In `tests/src/tests/xudt_meta/fixture.rs`, add this helper after
`access_mode_transition_tx`:

```rust
pub(super) fn destroy_meta_tx(
    input_flags: u8,
    input_supply: u128,
    include_full_access_list_input: bool,
) -> UpdateCase {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context);
    let authority = input_lock_authority(lock.script_hash);
    let input_meta_data = xudt_meta_data(
        input_flags,
        input_supply,
        Some(authority.clone()),
        Some(authority.clone()),
        Some(authority),
        Vec::new(),
    );
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );

    let mut builder = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta));

    if include_full_access_list_input {
        let access_list = access_list_script(&mut context, meta.script_hash);
        let access_out_point = create_typed_cell(
            &mut context,
            &lock.script,
            &access_list.script,
            100_000_000_000,
            full_domain_shard(),
        );
        builder = builder
            .input(
                CellInput::new_builder()
                    .previous_output(access_out_point)
                    .build(),
            )
            .cell_dep(cell_dep_for_script(&access_list));
    }

    let tx = context.complete_tx(builder.build());
    UpdateCase { context, tx }
}
```

- [ ] **Step 2: Add xUDT destroy tests**

In `tests/src/tests/xudt_meta/access_transitions.rs`, add:

```rust
#[test]
fn xudt_meta_destroy_accepts_tracked_zero_supply_when_access_disabled() {
    let case = destroy_meta_tx(CONFIG_SUPPLY_TRACKED, 0, false);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_destroy_rejects_tracked_nonzero_supply() {
    let case = destroy_meta_tx(CONFIG_SUPPLY_TRACKED, 1, false);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_destroy_rejects_active_access_without_full_domain_inputs() {
    let case = destroy_meta_tx(CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED, 0, false);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn xudt_meta_destroy_accepts_active_access_with_full_domain_inputs() {
    let case = destroy_meta_tx(CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED, 0, true);

    expect_tx_pass(&case.context, &case.tx);
}
```

- [ ] **Step 3: Run the first xUDT destroy test and verify RED**

Run:

```bash
cargo test -p tests xudt_meta_destroy_accepts_tracked_zero_supply_when_access_disabled -- --nocapture
```

Expected: FAIL with the transaction rejected by `xudt-meta` because `(Some(input), None)` still returns `InvalidArgs`.

---

### Task 4: Implement xUDT Meta Destroy

**Files:**
- Modify: `contracts/xudt-meta/src/entry.rs`
- Modify: `contracts/xudt-meta/src/update.rs`

- [ ] **Step 1: Add the entrypoint branch**

In `contracts/xudt-meta/src/entry.rs`, replace the final match arm with:

```rust
        (Some(input), Some(output)) => {
            crate::update::validate_update(input, output, &group.meta_type_hash)
        }
        (Some(input), None) => crate::update::validate_destroy(input, &group.meta_type_hash),
        _ => Err(Error::InvalidArgs),
```

- [ ] **Step 2: Add the destroy validator**

In `contracts/xudt-meta/src/update.rs`, add this public function after
`validate_update`:

```rust
pub fn validate_destroy(input: &XudtMeta, meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    if !is_supply_tracked(input.config_flags) || input.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if access_enabled(input.config_flags)
        && !has_full_domain_access_list_inputs(meta_type_hash)?
    {
        return Err(Error::AccessListRequired);
    }

    require_authority(input.mint_authority.as_ref())
}
```

- [ ] **Step 3: Run xUDT destroy tests and verify GREEN**

Run:

```bash
cargo test -p tests xudt_meta_destroy_ -- --nocapture
```

Expected: all `xudt_meta_destroy_*` tests pass.

---

### Task 5: Regression Verification

**Files:**
- No code changes.

- [ ] **Step 1: Run focused metadata and AccessList tests**

Run:

```bash
cargo test -p tests sudt_meta -- --nocapture
cargo test -p tests xudt_meta -- --nocapture
cargo test -p tests access_list_active_destroy_requires_full_domain_inputs_and_empty_outputs -- --nocapture
```

Expected: all focused tests pass.

- [ ] **Step 2: Run the full test suite**

Run:

```bash
cargo test
```

Expected: the workspace test suite passes.

- [ ] **Step 3: Review the diff**

Run:

```bash
git diff -- tests/src/tests/sudt_meta/fixture.rs tests/src/tests/sudt_meta/supply.rs tests/src/tests/xudt_meta/fixture.rs tests/src/tests/xudt_meta/access_transitions.rs contracts/sudt-meta/src/entry.rs contracts/sudt-meta/src/update.rs contracts/xudt-meta/src/entry.rs contracts/xudt-meta/src/update.rs docs/superpowers/specs/2026-05-16-zero-supply-meta-destroy-design.md docs/superpowers/plans/2026-05-16-zero-supply-meta-destroy.md
```

Expected: diff contains only the zero-supply metadata destroy behavior, tests, spec, and plan.

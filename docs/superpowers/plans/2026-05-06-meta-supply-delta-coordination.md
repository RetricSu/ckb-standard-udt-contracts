# Meta Supply Delta Coordination Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Require tracked meta `current_supply` updates to match same-transaction UDT input/output amount deltas.

**Architecture:** Add failing integration tests for meta-only supply changes, then add a shared transaction-wide UDT amount scanner and call it from `sudt-meta` and `xudt-meta` update validation. Keep UDT type scripts' existing mint/protocol-burn supply checks so both token movement and meta field updates are protected.

**Tech Stack:** Rust no_std CKB contracts, `ckb-std` high-level cell APIs, `standard-udt-script-utils`, `ckb-testtool`, current Makefile build/test flow.

---

## File Structure

- Modify `tests/src/tests/sudt_meta.rs`: add update helpers that include same-token UDT input/output cells, and add failing tests for supply-delta mismatch and matching deltas.
- Modify `tests/src/tests/xudt_meta.rs`: add the same coverage for xUDT meta updates using an explicit transaction helper.
- Create `lib/script-utils/src/token.rs`: shared helpers for matching same-token UDT type scripts and summing UDT amounts from `Source::Input` / `Source::Output`.
- Modify `lib/script-utils/src/lib.rs`: export the new `token` module.
- Modify `contracts/sudt-meta/src/meta_cell.rs`: replace the local output-only UDT summation with the shared scanner.
- Modify `contracts/sudt-meta/src/update.rs`: enforce tracked supply update delta against transaction-wide same-token UDT sums.
- Modify `contracts/xudt-meta/src/meta_cell/token.rs`: replace local token matching/summing with the shared scanner where possible.
- Modify `contracts/xudt-meta/src/meta_cell/mod.rs`: remove re-export of the deleted output-only sum helper.
- Modify `contracts/xudt-meta/src/update.rs`: enforce tracked supply update delta against transaction-wide same-token UDT sums.

---

### Task 1: sUDT Meta Supply Delta Tests

**Files:**
- Modify: `tests/src/tests/sudt_meta.rs`

- [x] **Step 1: Add a helper that can include UDT inputs and outputs**

Add this helper near `update_meta_tx_with_data`:

```rust
fn update_meta_tx_with_udt_delta(
    input_supply: u128,
    output_supply: u128,
    input_udt_amount: Option<u128>,
    output_udt_amount: Option<u128>,
) -> (Context, TransactionView) {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let authority = input_lock_authority(lock.script_hash);
    let input_meta_data = sudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        input_supply,
        Some(authority.clone()),
        None,
        Vec::new(),
        Vec::new(),
    );
    let output_meta_data = sudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        output_supply,
        Some(authority),
        None,
        Vec::new(),
        Vec::new(),
    );
    let meta = meta_script(&mut context, Bytes::from(vec![2u8; 32]));
    let udt = udt_script(&mut context, meta.script_hash);
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
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&udt));

    if let Some(amount) = input_udt_amount {
        let out_point = create_typed_cell(
            &mut context,
            &lock.script,
            &udt.script,
            100_000_000_000,
            udt_amount_bytes(amount),
        );
        builder = builder.input(CellInput::new_builder().previous_output(out_point).build());
    }

    if let Some(amount) = output_udt_amount {
        builder = builder
            .output(typed_output(&lock.script, &udt.script, 100_000_000_000))
            .output_data(udt_amount_bytes(amount).pack());
    }

    let tx = context.complete_tx(builder.build());
    (context, tx)
}
```

- [x] **Step 2: Add failing tests for meta-only supply changes**

Add these tests near `sudt_meta_update_supply_change_with_input_lock_mint_authority_passes`:

```rust
#[test]
fn sudt_meta_rejects_supply_increase_without_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 101, None, None);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_rejects_supply_decrease_without_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 99, None, None);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}
```

- [x] **Step 3: Add matching-delta and mismatch tests**

Deferred from Task 1 red-stage commit: same-token SUDT input/output cells activate the SUDT type script before the planned `sudt-meta` fix, so these tests are not clean meta-side assertions yet.

Add these tests after the tests from Step 2:

```rust
#[test]
fn sudt_meta_accepts_supply_increase_matching_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 125, None, Some(25));

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_accepts_supply_decrease_matching_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 75, Some(25), None);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_rejects_supply_delta_mismatch() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 125, None, Some(24));

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}
```

- [x] **Step 4: Run the new sUDT meta tests and verify failure**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test TEST=sudt_meta_rejects_supply_increase_without_udt_delta
```

If the Makefile does not support `TEST=...`, run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p tests sudt_meta_rejects_supply_increase_without_udt_delta -- --nocapture
```

Expected: the test fails because the current code accepts the meta-only supply increase.

---

### Task 2: xUDT Meta Supply Delta Tests

**Files:**
- Modify: `tests/src/tests/xudt_meta.rs`

- [x] **Step 1: Add a helper for xUDT meta updates with UDT deltas**

Add this helper near `access_mode_transition_tx`:

```rust
fn update_meta_tx_with_udt_delta(
    input_supply: u128,
    output_supply: u128,
    input_udt_amount: Option<u128>,
    output_udt_amount: Option<u128>,
) -> UpdateCase {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context);
    let xudt = xudt_script(&mut context, meta.script_hash);
    let authority = input_lock_authority(lock.script_hash);
    let input_meta_data = xudt_meta_data(
        standard_udt_types::metadata::CONFIG_SUPPLY_TRACKED,
        input_supply,
        Some(authority.clone()),
        None,
        None,
        Vec::new(),
    );
    let output_meta_data = xudt_meta_data(
        standard_udt_types::metadata::CONFIG_SUPPLY_TRACKED,
        output_supply,
        Some(authority),
        None,
        None,
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
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&xudt));

    if let Some(amount) = input_udt_amount {
        let out_point = create_typed_cell(
            &mut context,
            &lock.script,
            &xudt.script,
            100_000_000_000,
            udt_amount_bytes(amount),
        );
        builder = builder.input(CellInput::new_builder().previous_output(out_point).build());
    }

    if let Some(amount) = output_udt_amount {
        builder = builder
            .output(typed_output(&lock.script, &xudt.script, 100_000_000_000))
            .output_data(udt_amount_bytes(amount).pack());
    }

    let tx = context.complete_tx(builder.build());
    UpdateCase { context, tx }
}
```

- [x] **Step 2: Add failing tests for meta-only xUDT supply changes**

Add these tests near the existing supply and mint-authority tests:

```rust
#[test]
fn xudt_meta_rejects_supply_increase_without_udt_delta() {
    let case = update_meta_tx_with_udt_delta(100, 101, None, None);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_rejects_supply_decrease_without_udt_delta() {
    let case = update_meta_tx_with_udt_delta(100, 99, None, None);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}
```

- [x] **Step 3: Add matching-delta and mismatch tests**

Deferred from Task 2 red stage: same-token xUDT input/output cells can activate the xUDT type script before the planned `xudt-meta` fix, so these tests are not clean meta-side assertions yet.

Add these tests after the tests from Step 2:

```rust
#[test]
fn xudt_meta_accepts_supply_increase_matching_udt_delta() {
    let case = update_meta_tx_with_udt_delta(100, 125, None, Some(25));

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_accepts_supply_decrease_matching_udt_delta() {
    let case = update_meta_tx_with_udt_delta(100, 75, Some(25), None);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_rejects_supply_delta_mismatch() {
    let case = update_meta_tx_with_udt_delta(100, 125, Some(24), Some(24));

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_ignores_fake_data2_udt_outputs() {
    let case = update_meta_tx_with_fake_udt_output(100, 125, 25);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}
```

- [x] **Step 4: Run one new xUDT meta test and verify failure**

Task 2 red run note: set `MODE=debug` so the test uses debug artifacts with the testtool always-success lock allowance. With `MODE=debug`, the transaction is accepted, so `expect_tx_fail_with_code(... "error code 31")` panics.

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug cargo test -p tests xudt_meta_rejects_supply_increase_without_udt_delta -- --nocapture
```

Expected: the test fails because the current code accepts the meta-only supply increase.

---

### Task 3: Shared Same-Token Amount Scanner

**Files:**
- Create: `lib/script-utils/src/token.rs`
- Modify: `lib/script-utils/src/lib.rs`

- [x] **Step 1: Add the token scanner module**

Create `lib/script-utils/src/token.rs`:

```rust
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, packed::Script, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_type},
};

use crate::{
    amount::decode_amount,
    error::ScriptError,
    supply::{classify_supply_delta, SupplyDelta},
};

pub fn is_token_script(
    type_script: &Script,
    meta_type_hash: &[u8; 32],
    code_hash: &[u8; 32],
) -> bool {
    if type_script.hash_type() != ScriptHashType::Data2.into() {
        return false;
    }
    if type_script.args().raw_data().as_ref() != meta_type_hash {
        return false;
    }

    let actual_code_hash: [u8; 32] = type_script.code_hash().unpack();
    &actual_code_hash == code_hash
}

pub fn sum_token_amount(
    source: Source,
    meta_type_hash: &[u8; 32],
    code_hash: &[u8; 32],
) -> Result<u128, ScriptError> {
    let mut total = 0u128;
    let mut index = 0;

    loop {
        let type_script = match load_cell_type(index, source) {
            Ok(Some(script)) => script,
            Ok(None) => {
                index += 1;
                continue;
            }
            Err(SysError::IndexOutOfBound) => return Ok(total),
            Err(_) => return Err(ScriptError::SyscallUnknown),
        };

        if is_token_script(&type_script, meta_type_hash, code_hash) {
            let data = load_cell_data(index, source).map_err(|_| ScriptError::SyscallUnknown)?;
            let amount = decode_amount(&data)?;
            total = total
                .checked_add(amount)
                .ok_or(ScriptError::AmountOverflow)?;
        }

        index += 1;
    }
}

pub fn transaction_token_delta(
    meta_type_hash: &[u8; 32],
    code_hash: &[u8; 32],
) -> Result<SupplyDelta, ScriptError> {
    let input = sum_token_amount(Source::Input, meta_type_hash, code_hash)?;
    let output = sum_token_amount(Source::Output, meta_type_hash, code_hash)?;
    classify_supply_delta(input, output)
}
```

- [x] **Step 2: Export the module**

Modify `lib/script-utils/src/lib.rs`:

```rust
pub mod amount;
pub mod authority;
pub mod error;
pub mod meta;
pub mod supply;
pub mod token;
```

- [x] **Step 3: Add focused unit tests**

In `lib/script-utils/src/token.rs`, append:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ckb_std::ckb_types::packed::ScriptBuilder;

    fn script(hash_type: ScriptHashType, code_hash: [u8; 32], args: [u8; 32]) -> Script {
        ScriptBuilder::default()
            .code_hash(code_hash.pack())
            .hash_type(hash_type.into())
            .args(args.to_vec().pack())
            .build()
    }

    #[test]
    fn token_script_requires_data2_code_hash_and_meta_args() {
        let code_hash = [1u8; 32];
        let meta_hash = [2u8; 32];

        assert!(is_token_script(
            &script(ScriptHashType::Data2, code_hash, meta_hash),
            &meta_hash,
            &code_hash,
        ));
        assert!(!is_token_script(
            &script(ScriptHashType::Data, code_hash, meta_hash),
            &meta_hash,
            &code_hash,
        ));
        assert!(!is_token_script(
            &script(ScriptHashType::Data2, [3u8; 32], meta_hash),
            &meta_hash,
            &code_hash,
        ));
        assert!(!is_token_script(
            &script(ScriptHashType::Data2, code_hash, [4u8; 32]),
            &meta_hash,
            &code_hash,
        ));
    }
}
```

- [x] **Step 4: Run script-utils tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p standard-udt-script-utils --lib token::tests -- --nocapture
```

Expected: PASS.

---

### Task 4: Enforce sUDT Meta Supply Delta

**Files:**
- Modify: `contracts/sudt-meta/src/meta_cell.rs`
- Modify: `contracts/sudt-meta/src/update.rs`

- [x] **Step 1: Reuse the shared output sum in create validation**

In `contracts/sudt-meta/src/meta_cell.rs`, import:

```rust
use standard_udt_script_utils::token::sum_token_amount;
```

Replace the tracked create sum in `validate_create` with:

```rust
let initial_supply = sum_token_amount(Source::Output, meta_type_hash, &SUDT_CODE_HASH)
    .map_err(map_supply_scan_error)?;
```

Add this mapper near `decode_amount` or near `validate_create`:

```rust
fn map_supply_scan_error(error: standard_udt_script_utils::error::ScriptError) -> Error {
    match error {
        standard_udt_script_utils::error::ScriptError::AmountEncoding
        | standard_udt_script_utils::error::ScriptError::AmountOverflow => Error::InvalidSupply,
        standard_udt_script_utils::error::ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}
```

After this replacement, remove the local `sum_initial_udt_outputs`, `decode_amount`, and `is_initial_udt_script` functions when `rg` shows no remaining references in `contracts/sudt-meta/src`.

- [x] **Step 2: Pass `meta_type_hash` into update validation**

Find the call site in `contracts/sudt-meta/src/entry.rs` that currently calls:

```rust
update::validate_update(input, output)
```

Change it to:

```rust
crate::update::validate_update(input, output, &group.meta_type_hash)
```

Change the signature in `contracts/sudt-meta/src/update.rs`:

```rust
pub fn validate_update(
    input: &SudtMeta,
    output: &SudtMeta,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
```

- [x] **Step 3: Add tracked supply delta validation**

In `contracts/sudt-meta/src/update.rs`, add imports:

```rust
use crate::constants::SUDT_CODE_HASH;
use standard_udt_script_utils::{
    authority::check_authority as check_runtime_authority,
    error::ScriptError,
    supply::apply_supply_delta,
    token::transaction_token_delta,
};
```

Add this after the untracked supply check and before authority checks:

```rust
if is_supply_tracked(output.config_flags) {
    validate_supply_delta(input.current_supply, output.current_supply, meta_type_hash)?;
}
```

Add helper functions near the authority helpers:

```rust
fn validate_supply_delta(
    input_supply: u128,
    output_supply: u128,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    let delta = transaction_token_delta(meta_type_hash, &SUDT_CODE_HASH).map_err(map_supply_error)?;
    let expected = apply_supply_delta(input_supply, delta).map_err(map_supply_error)?;
    if output_supply == expected {
        Ok(())
    } else {
        Err(Error::InvalidSupply)
    }
}

fn map_supply_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AmountEncoding
        | ScriptError::AmountOverflow
        | ScriptError::SupplyOverflow
        | ScriptError::SupplyUnderflow => Error::InvalidSupply,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}
```

- [x] **Step 4: Run sUDT meta tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p tests sudt_meta_ -- --nocapture
```

Expected: all sUDT meta tests pass, including the new supply delta tests.

---

### Task 5: Enforce xUDT Meta Supply Delta

**Files:**
- Modify: `contracts/xudt-meta/src/meta_cell/token.rs`
- Modify: `contracts/xudt-meta/src/meta_cell/mod.rs`
- Modify: `contracts/xudt-meta/src/update.rs`

- [x] **Step 1: Reuse the shared output sum in xUDT meta create validation**

In `contracts/xudt-meta/src/meta_cell/token.rs`, add imports:

```rust
use standard_udt_script_utils::{
    error::ScriptError,
    token::{is_token_script, sum_token_amount},
};
```

Replace the tracked create sum in `validate_create` with:

```rust
let initial_supply = sum_token_amount(Source::Output, meta_type_hash, &XUDT_CODE_HASH)
    .map_err(map_supply_error)?;
```

Add:

```rust
fn map_supply_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AmountEncoding | ScriptError::AmountOverflow => Error::InvalidSupply,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}
```

Remove the local `sum_initial_udt_outputs`, local `is_token_script`, and local `decode_amount` functions after `validate_create` and `has_same_token_cells` call the shared scanner helpers.

Keep `has_same_token_cells` and make it call the imported `is_token_script`.

Update `contracts/xudt-meta/src/meta_cell/mod.rs` so the public re-export no longer mentions the removed function:

```rust
pub use token::{has_same_token_cells, validate_create};
```

- [x] **Step 2: Add tracked supply delta validation in update**

In `contracts/xudt-meta/src/update.rs`, add imports:

```rust
use crate::constants::XUDT_CODE_HASH;
use standard_udt_script_utils::{
    authority::check_authority as check_runtime_authority,
    error::ScriptError,
    supply::apply_supply_delta,
    token::transaction_token_delta,
};
```

Add this after the untracked supply check and before access/metadata authority checks:

```rust
if is_supply_tracked(output.config_flags) {
    validate_supply_delta(input.current_supply, output.current_supply, meta_type_hash)?;
}
```

Add helper functions near `validate_access_mode_transition`:

```rust
fn validate_supply_delta(
    input_supply: u128,
    output_supply: u128,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    let delta = transaction_token_delta(meta_type_hash, &XUDT_CODE_HASH).map_err(map_supply_error)?;
    let expected = apply_supply_delta(input_supply, delta).map_err(map_supply_error)?;
    if output_supply == expected {
        Ok(())
    } else {
        Err(Error::InvalidSupply)
    }
}

fn map_supply_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AmountEncoding
        | ScriptError::AmountOverflow
        | ScriptError::SupplyOverflow
        | ScriptError::SupplyUnderflow => Error::InvalidSupply,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}
```

- [x] **Step 3: Run xUDT meta tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug cargo test -p tests xudt_meta_ -- --nocapture
```

Expected: all xUDT meta tests pass, including the new supply delta tests.

---

### Task 6: Full Verification and Cleanup

**Files:**
- Review all modified files.

- [x] **Step 1: Search for duplicated token scan helpers**

Run:

```bash
rg "sum_initial_udt_outputs|fn decode_amount|fn is_initial_udt_script|pub\\(crate\\) fn is_token_script" contracts/sudt-meta contracts/xudt-meta lib/script-utils -n
```

Expected: no local `decode_amount`, `sum_initial_udt_outputs`, or token-script matcher remains in `contracts/sudt-meta` or `contracts/xudt-meta`. The remaining amount decoder should be `lib/script-utils/src/amount.rs`, and the remaining token matcher should be `lib/script-utils/src/token.rs`.

- [x] **Step 2: Format**

Run:

```bash
cargo fmt
```

Expected: no output.

- [x] **Step 3: Run unit tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p standard-udt-script-utils --lib
RUSTUP_TOOLCHAIN=1.92.0 cargo test -p standard-udt-types --lib
```

Expected: both pass.

- [x] **Step 4: Run contract build**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
```

Expected: all contracts and test plugins build.

- [x] **Step 5: Run integration tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test
```

Expected: all integration tests pass.

- [x] **Step 6: Check diff hygiene**

Run:

```bash
git diff --check
git status --short
```

Expected: `git diff --check` prints nothing. `git status --short` lists only intentional source, test, and docs changes.

- [x] **Step 7: Commit**

Run:

```bash
git add lib/script-utils/src/lib.rs lib/script-utils/src/token.rs contracts/sudt-meta/src contracts/xudt-meta/src tests/src/tests/sudt_meta.rs tests/src/tests/xudt_meta.rs tests/src/tests/xudt.rs docs/superpowers/specs/2026-05-06-meta-supply-delta-coordination-design.md docs/superpowers/plans/2026-05-06-meta-supply-delta-coordination.md
git commit -m "Enforce meta supply deltas against UDT amounts"
```

Expected: commit succeeds after all verification passes.

---

## Self-Review

Spec coverage:

- Meta-only supply increase/decrease rejection is covered by Tasks 1, 2, 4, and 5.
- Matching mint/burn deltas are covered by Tasks 1, 2, 4, and 5.
- Same-token definition is implemented in Task 3 and reused in Tasks 4 and 5.
- Existing create behavior is preserved while deduplicating output summation in Tasks 4 and 5.
- UDT type forward checks remain untouched.

Placeholder scan:

- No placeholder markers or unspecified test steps are present.

Type consistency:

- Shared scanner returns `ScriptError`; each meta contract maps it to its own `Error`.
- `transaction_token_delta` returns `SupplyDelta`; meta update applies it through `apply_supply_delta`.
- Test helpers use existing `Bytes`, `CellInput`, `TransactionBuilder`, and `ExtraCell` types already imported in the target files.

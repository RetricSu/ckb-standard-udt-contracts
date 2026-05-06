# Cell Model Invariants Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move lock ownership checks to MetaType/AccessListType and make xUDT access control require explicit membership or non-membership proofs for checked lock hashes.

**Architecture:** First add failing tests that encode the cell-model responsibility boundary. Then remove consumer-side meta lock checks, add owner-side output lock checks, keep blacklist shard chain validation owned by AccessListType, and make xUDT's access reader consume shard proofs only. Keep the existing schema and authority runtime unchanged.

Follow-up decisions:
- xUDT treats `output_amount < input_amount` with a visible `CellDep` meta as user destruction, even if a meta input is also present for an accompanying meta update.
- `xudt-meta` requires blacklist -> disabled transitions to consume AccessList input shards covering the full lock-hash domain.

**Tech Stack:** Rust no_std CKB contracts, `ckb-std` high-level cell APIs, `ckb-testtool` integration tests, current Makefile build/test flow.

---

## File Structure

- Modify `contracts/sudt/src/meta/cells.rs`: remove meta lock whitelist code from consumer meta lookup.
- Modify `contracts/xudt/src/meta.rs`: remove meta lock whitelist code from consumer meta lookup.
- Modify `contracts/access-list/src/meta/cells.rs`: remove meta lock whitelist code from consumer meta lookup.
- Modify `contracts/sudt-meta/src/meta_cell.rs`: add `GroupOutput` lock whitelist validation for meta outputs.
- Modify `contracts/xudt-meta/src/meta_cell/cells.rs`: validate only `GroupOutput` meta locks, not `GroupInput` locks.
- Modify `contracts/access-list/src/entry.rs`: validate `GroupOutput` shard locks before accepting shard outputs.
- Modify `contracts/access-list/src/shards.rs`: switch nibble alignment to prefix-bucket semantics, keep blacklist chain validation explicit, and set `MAX_ACCESSLIST_ENTRIES` to `4096`.
- Modify `contracts/xudt/src/access.rs`: require membership/non-membership shard proofs for checked lock hashes, remove any complete-chain or prefix-bucket validation from the access reader, and set `MAX_ACCESSLIST_ENTRIES` to `4096`.
- Modify `contracts/xudt-meta/src/meta_cell/access_list.rs`: set the access-list initialization parser limit to `4096`.
- Modify tests in `tests/src/tests/sudt.rs`, `tests/src/tests/xudt.rs`, `tests/src/tests/access_list.rs`, `tests/src/tests/sudt_meta.rs`, and `tests/src/tests/xudt_meta.rs`.

---

### Task 1: Lock Ownership Tests

**Files:**
- Modify: `tests/src/tests/sudt.rs`
- Modify: `tests/src/tests/xudt.rs`
- Modify: `tests/src/tests/access_list.rs`
- Modify: `tests/src/tests/sudt_meta.rs`
- Modify: `tests/src/tests/xudt_meta.rs`

- [ ] **Step 1: Change the sUDT consumer test expectation**

Find `sudt_mint_rejects_non_whitelisted_meta_lock` in `tests/src/tests/sudt.rs`.

Rename it to:

```rust
fn sudt_mint_allows_visible_meta_with_non_whitelisted_lock()
```

Keep the transaction shape with a non-whitelisted visible meta lock, but change the previous `MetaLockNotAllowed` failure expectation:

```rust
expect_tx_fail(&case.context, &case.tx);
```

to:

```rust
expect_tx_pass(&case.context, &case.tx);
```

- [ ] **Step 2: Add xUDT consumer coverage**

In `tests/src/tests/xudt.rs`, add this helper near `always_success_lock`:

```rust
fn non_whitelisted_lock(context: &mut Context) -> DeployedScript {
    let out_point = context.deploy_cell(Bytes::from(vec![1u8]));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, Bytes::new())
        .expect("build non-whitelisted lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}
```

Add this fixture method in `impl XudtFixture`:

```rust
fn live_meta_input_with_lock(
    &mut self,
    lock: &Script,
    config_flags: u8,
    supply: u128,
    mint_authority: Option<Authority>,
) -> CellInput {
    let out_point = create_typed_cell(
        &mut self.context,
        lock,
        &self.meta.script,
        100_000_000_000,
        xudt_meta_data(config_flags, supply, mint_authority, Vec::new()),
    );
    CellInput::new_builder().previous_output(out_point).build()
}
```

Add this test near the mint authority tests:

```rust
#[test]
fn xudt_mint_allows_visible_meta_with_non_whitelisted_lock() {
    let mut fixture = XudtFixture::new();
    let meta_lock = non_whitelisted_lock(&mut fixture.context);
    let authority = input_lock_authority(fixture.lock.script_hash);
    let meta_input =
        fixture.live_meta_input_with_lock(&meta_lock.script, CONFIG_SUPPLY_TRACKED, 0, Some(authority.clone()));
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 50, Some(authority), Vec::new()).pack())
        .output_data(udt_amount_bytes(50).pack())
        .cell_dep(cell_dep_for_script(&meta_lock))
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}
```

- [ ] **Step 3: Add AccessList consumer coverage**

In `tests/src/tests/access_list.rs`, add this helper near `always_success_lock`:

```rust
fn non_whitelisted_lock(context: &mut Context) -> DeployedScript {
    let out_point = context.deploy_cell(Bytes::from(vec![1u8]));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, Bytes::new())
        .expect("build non-whitelisted lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}
```

Add this transaction builder helper:

```rust
fn access_list_update_tx_with_non_whitelisted_meta_lock(
    config_flags: u8,
    input_shards: Vec<Bytes>,
    output_shards: Vec<Bytes>,
) -> AccessListCase {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let meta_lock = non_whitelisted_lock(&mut context);
    let meta = meta_script(&mut context);
    let access_list = access_list_script(&mut context, meta.script_hash);
    let meta_data = xudt_meta_data(config_flags, &authority);

    let meta_out_point = create_typed_cell(
        &mut context,
        &meta_lock,
        &meta.script,
        100_000_000_000,
        meta_data,
    );
    let mut builder = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(meta_out_point).build())
        .output(typed_output(&meta_lock.script, &meta.script, 100_000_000_000))
        .output_data(xudt_meta_data(config_flags, &authority).pack())
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&authority))
        .cell_dep(cell_dep_for_script(&meta_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list));

    let auth_out_point = context.create_cell(
        ckb_testtool::ckb_types::packed::CellOutput::new_builder()
            .capacity(100_000_000_000u64.pack())
            .lock(authority.script.clone())
            .build(),
        Bytes::new(),
    );
    builder = builder.input(CellInput::new_builder().previous_output(auth_out_point).build());

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
            .output(typed_output(&cell_lock.script, &access_list.script, 100_000_000_000))
            .output_data(data.pack());
    }

    let tx = context.complete_tx(builder.build());
    AccessListCase { context, tx }
}
```

Add this test:

```rust
#[test]
fn access_list_update_allows_visible_meta_with_non_whitelisted_lock() {
    let case = access_list_update_tx_with_non_whitelisted_meta_lock(
        CONFIG_ACCESS_ENABLED,
        vec![full_domain_shard(Vec::new())],
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}
```

- [ ] **Step 4: Add owner-side meta output lock rejection tests**

In `tests/src/tests/sudt_meta.rs`, add this helper near `always_success_lock`:

```rust
fn non_whitelisted_lock(context: &mut Context) -> DeployedScript {
    let out_point = context.deploy_cell(Bytes::from(vec![1u8]));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, Bytes::new())
        .expect("build non-whitelisted lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}
```

Add a sibling helper to `update_meta_tx_with_data` named `update_meta_tx_with_locks`:

```rust
fn update_meta_tx_with_locks<F>(build_data: F) -> (Context, TransactionView)
where
    F: FnOnce(&mut Context, [u8; 32], Script) -> (DeployedScript, Bytes, Bytes),
{
    let mut context = Context::default();
    let input_lock = always_success_lock(&mut context);
    let (output_lock, input_meta_data, output_meta_data) =
        build_data(&mut context, input_lock.script_hash, input_lock.script.clone());
    let meta = meta_script(&mut context, Bytes::from(vec![2u8; 32]));
    let input_out_point = create_typed_cell(
        &mut context,
        &input_lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_out_point).build())
        .output(typed_output(&output_lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&input_lock))
        .cell_dep(cell_dep_for_script(&output_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);
    (context, tx)
}
```

Then add:

```rust
#[test]
fn sudt_meta_rejects_non_whitelisted_output_lock() {
    let (context, tx) = update_meta_tx_with_locks(|context, lock_hash, _| {
        let output_lock = non_whitelisted_lock(context);
        let authority = input_lock_authority(lock_hash);
        (
            output_lock,
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority.clone()),
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority),
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail(&context, &tx);
}
```

In `tests/src/tests/xudt_meta.rs`, add this helper near `always_success_lock`:

```rust
fn non_whitelisted_lock(context: &mut Context) -> DeployedScript {
    let out_point = context.deploy_cell(Bytes::from(vec![1u8]));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, Bytes::new())
        .expect("build non-whitelisted lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}
```

Add this sibling to `update_meta_tx`:

```rust
fn update_meta_tx_with_output_lock<F>(build_lock: F) -> UpdateCase
where
    F: FnOnce(&mut Context) -> DeployedScript,
{
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let output_lock = build_lock(&mut context);
    let meta = meta_script(&mut context);
    let input = xudt_meta_data(0, 0, None, Some(input_lock_authority(lock.script_hash)), None, Vec::new());
    let output = xudt_meta_data(
        0,
        0,
        None,
        Some(input_lock_authority(lock.script_hash)),
        None,
        Vec::new(),
    );

    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input,
    );
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_out_point).build())
        .output(typed_output(&output_lock.script, &meta.script, 100_000_000_000))
        .output_data(output.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&output_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);
    UpdateCase { context, tx }
}
```

Then add:

```rust
#[test]
fn xudt_meta_rejects_non_whitelisted_output_lock() {
    let case = update_meta_tx_with_output_lock(non_whitelisted_lock);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 20");
}
```

- [ ] **Step 5: Add AccessList output lock rejection test**

In `tests/src/tests/access_list.rs`, add this helper:

```rust
fn access_list_update_tx_with_non_whitelisted_output_lock(
    output_shards: Vec<Bytes>,
) -> AccessListCase {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let output_lock = non_whitelisted_lock(&mut context);
    let meta = meta_script(&mut context);
    let access_list = access_list_script(&mut context, meta.script_hash);
    let meta_data = xudt_meta_data(CONFIG_ACCESS_ENABLED, &authority);
    let meta_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &meta.script,
        100_000_000_000,
        meta_data,
    );
    let auth_out_point = context.create_cell(
        ckb_testtool::ckb_types::packed::CellOutput::new_builder()
            .capacity(100_000_000_000u64.pack())
            .lock(authority.script.clone())
            .build(),
        Bytes::new(),
    );
    let input_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &access_list.script,
        100_000_000_000,
        full_domain_shard(Vec::new()),
    );

    let mut builder = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(meta_out_point).build())
        .input(CellInput::new_builder().previous_output(auth_out_point).build())
        .input(CellInput::new_builder().previous_output(input_out_point).build())
        .output(typed_output(&cell_lock.script, &meta.script, 100_000_000_000))
        .output_data(xudt_meta_data(CONFIG_ACCESS_ENABLED, &authority).pack())
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&authority))
        .cell_dep(cell_dep_for_script(&output_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list));

    for data in output_shards {
        builder = builder
            .output(typed_output(&output_lock.script, &access_list.script, 100_000_000_000))
            .output_data(data.pack());
    }

    let tx = context.complete_tx(builder.build());
    AccessListCase { context, tx }
}
```

Add:

```rust
#[test]
fn access_list_rejects_non_whitelisted_output_lock() {
    let case = access_list_update_tx_with_non_whitelisted_output_lock(vec![
        full_domain_shard(vec![entry(0x10)]),
    ]);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 20");
}
```

- [ ] **Step 6: Run targeted tests and confirm failures**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="sudt_mint_allows_visible_meta_with_non_whitelisted_lock -- --nocapture"
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="xudt_mint_allows_visible_meta_with_non_whitelisted_lock -- --nocapture"
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="access_list_update_allows_visible_meta_with_non_whitelisted_lock -- --nocapture"
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="rejects_non_whitelisted_output_lock -- --nocapture"
```

Expected before implementation:

- Consumer tests fail because consumer scripts still reject meta locks.
- Owner tests fail because owner scripts do not yet consistently reject output locks.

- [ ] **Step 7: Commit failing tests**

```bash
git add tests/src/tests/sudt.rs tests/src/tests/xudt.rs tests/src/tests/access_list.rs tests/src/tests/sudt_meta.rs tests/src/tests/xudt_meta.rs tests/src
git commit -m "test: capture cell-owned lock invariants"
```

---

### Task 2: Move Meta Lock Checks to MetaType

**Files:**
- Modify: `contracts/sudt/src/meta/cells.rs`
- Modify: `contracts/xudt/src/meta.rs`
- Modify: `contracts/access-list/src/meta/cells.rs`
- Modify: `contracts/sudt-meta/src/meta_cell.rs`
- Modify: `contracts/xudt-meta/src/meta_cell/cells.rs`

- [ ] **Step 1: Remove consumer-side meta lock checks**

In `contracts/sudt/src/meta/cells.rs`, remove:

```rust
load_cell_lock
ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST
TESTTOOL_ALWAYS_SUCCESS_LOCK_CODE_HASH
is_allowed_always_success_lock_code_hash
validate_meta_lock
```

Remove this call inside `find_meta_in_source`:

```rust
validate_meta_lock(index, source)?;
```

Repeat the same removal in:

```text
contracts/xudt/src/meta.rs
contracts/access-list/src/meta/cells.rs
```

- [ ] **Step 2: Add sUDT meta output lock validation**

In `contracts/sudt-meta/src/meta_cell.rs`, import `load_cell_lock`:

```rust
use ckb_std::high_level::{load_cell_data, load_cell_lock, load_cell_type, load_script, load_script_hash};
```

Add whitelist constants matching the other meta contracts:

```rust
const ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST: [[u8; 32]; 1] = [[
    0x3b, 0x52, 0x1c, 0xc4, 0xb5, 0x52, 0xf1, 0x09, 0xd0, 0x92, 0xd8, 0xcc, 0x46, 0x8a, 0x80, 0x48,
    0xac, 0xb5, 0x3c, 0x59, 0x52, 0xdb, 0xe7, 0x69, 0xd2, 0xb2, 0xf9, 0xcf, 0x6e, 0x47, 0xf7, 0xf1,
]];
```

Use the existing debug-only testtool hash pattern from `contracts/xudt-meta/src/meta_cell/cells.rs`.

Add:

```rust
fn validate_output_meta_lock(index: usize) -> Result<(), Error> {
    let lock = load_cell_lock(index, Source::GroupOutput).map_err(Error::from)?;
    let code_hash: [u8; 32] = lock.code_hash().unpack();
    if is_allowed_always_success_lock_code_hash(&code_hash) {
        Ok(())
    } else {
        Err(Error::InvalidArgs)
    }
}
```

Inside `load_group_meta`, call it only for `Source::GroupOutput`:

```rust
if source == Source::GroupOutput {
    validate_output_meta_lock(index)?;
}
```

- [ ] **Step 3: Limit xUDT meta lock validation to outputs**

In `contracts/xudt-meta/src/meta_cell/cells.rs`, replace:

```rust
validate_meta_lock(index, source)?;
```

with:

```rust
if source == Source::GroupOutput {
    validate_meta_lock(index, source)?;
}
```

Keep the existing function body and error mapping unless tests require a clearer `MetaLockNotAllowed` variant.

- [ ] **Step 4: Run lock-boundary tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="non_whitelisted -- --nocapture"
```

Expected: consumer non-whitelisted meta-lock tests pass, owner output-lock rejection tests pass.

- [ ] **Step 5: Commit**

```bash
git add contracts/sudt/src/meta/cells.rs contracts/xudt/src/meta.rs contracts/access-list/src/meta/cells.rs contracts/sudt-meta/src/meta_cell.rs contracts/xudt-meta/src/meta_cell/cells.rs tests/src
git commit -m "fix: enforce meta lock at meta type boundary"
```

---

### Task 3: AccessList Output Lock Ownership

**Files:**
- Modify: `contracts/access-list/src/entry.rs`
- Modify: `contracts/access-list/src/error.rs` if a distinct error variant is needed.
- Modify: `tests/src/tests/access_list.rs`

- [ ] **Step 1: Add output shard lock validator**

In `contracts/access-list/src/entry.rs`, import:

```rust
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    error::SysError,
    high_level::load_cell_lock,
};
```

Add whitelist constants using the same values and debug gate as meta contracts.

Add:

```rust
fn validate_group_output_locks() -> Result<(), Error> {
    let mut index = 0;
    loop {
        match load_cell_lock(index, Source::GroupOutput) {
            Ok(lock) => {
                let code_hash: [u8; 32] = lock.code_hash().unpack();
                if !is_allowed_always_success_lock_code_hash(&code_hash) {
                    return Err(Error::InvalidArgs);
                }
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
}
```

Call it near the start of `main()` after group output collection is known to be in this script group:

```rust
validate_group_output_locks()?;
```

- [ ] **Step 2: Run AccessList output-lock test**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="access_list_rejects_non_whitelisted_output_lock -- --nocapture"
```

Expected: pass.

- [ ] **Step 3: Commit**

```bash
git add contracts/access-list/src/entry.rs contracts/access-list/src/error.rs tests/src/tests/access_list.rs
git commit -m "fix: enforce access list shard output locks"
```

---

### Task 4: Access Proof Tests

**Files:**
- Modify: `tests/src/tests/xudt.rs`
- Modify: `tests/src/tests/access_list.rs`

- [ ] **Step 1: Add xUDT missing blacklist proof rejection**

In `tests/src/tests/xudt.rs`, add this helper near `full_domain_shard`:

```rust
fn custom_shard(start: [u8; 32], end: [u8; 32], entries: Vec<[u8; 32]>) -> Bytes {
    build_access_list_shard_bytes(start, end, entries)
}
```

Add:

```rust
#[test]
fn xudt_blacklist_rejects_missing_non_membership_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let mut end = [0xff; 32];
    end[0] = 0x0f;
    let non_covering = fixture.live_access_list_input(custom_shard([0u8; 32], end, Vec::new()));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(non_covering.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);
    expect_tx_fail_with_code(&fixture.context, &tx, "error code 60");
}
```

- [ ] **Step 2: Add AccessList prefix-bucket nibble alignment test**

In `tests/src/tests/access_list.rs`, add a blacklist shard output whose range is suffix-aligned but not prefix-bucket aligned:

```rust
#[test]
fn access_list_blacklist_rejects_suffix_only_nibble_alignment() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![
            bounded_shard(0x00, 0x0f, Vec::new()),
            tail_shard(0x10, Vec::new()),
        ],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}
```

Do not add an xUDT test that rejects suffix-only alignment by revalidating the complete chain. xUDT is a consumer; AccessListType owns the shard-chain and prefix-bucket invariants.

- [ ] **Step 3: Add xUDT blacklist covering non-membership proof pass**

In `tests/src/tests/xudt.rs`, add a visible shard whose range covers the checked input lock hash but does not list it:

```rust
#[test]
fn xudt_blacklist_accepts_covering_non_membership_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof = fixture.live_access_list_input(full_domain_shard(Vec::new()));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}
```

- [ ] **Step 4: Confirm tests fail before implementation**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="blacklist_rejects_missing_non_membership_proof -- --nocapture"
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="access_list_blacklist_rejects_suffix_only_nibble_alignment -- --nocapture"
```

Expected: fail before implementation.

- [ ] **Step 5: Commit failing tests**

```bash
git add tests/src/tests/xudt.rs tests/src/tests/access_list.rs tests/src
git commit -m "test: require access list proofs"
```

---

### Task 5: Prefix-Bucket Chain Validation and xUDT Proofs

**Files:**
- Modify: `contracts/access-list/src/shards.rs`
- Modify: `contracts/xudt/src/access.rs`

- [ ] **Step 1: Update AccessList nibble alignment**

In `contracts/access-list/src/shards.rs`, replace:

```rust
fn is_nibble_aligned_range(start: &[u8; 32], end: &[u8; 32]) -> bool {
    start[31] & 0x0f == 0 && end[31] & 0x0f == 0x0f
}
```

with:

```rust
fn is_nibble_aligned_range(start: &[u8; 32], end: &[u8; 32]) -> bool {
    is_nibble_aligned_start(start) && is_nibble_aligned_end(end)
}

fn is_nibble_aligned_start(start: &[u8; 32]) -> bool {
    start[0] & 0x0f == 0x00 && start[1..].iter().all(|byte| *byte == 0x00)
}

fn is_nibble_aligned_end(end: &[u8; 32]) -> bool {
    end[0] & 0x0f == 0x0f && end[1..].iter().all(|byte| *byte == 0xff)
}
```

- [ ] **Step 2: Add xUDT proof-based access validation**

In `contracts/xudt/src/access.rs`, do not add complete-chain validation or prefix-bucket validation. If those checks already exist, remove them from xUDT. After collecting and parsing visible shards, validate each checked `GroupInput` lock hash against the visible proof shards:

```rust
fn find_covering_shard<'a>(
    shards: &'a [AccessListShard],
    lock_hash: &[u8; 32],
) -> Option<&'a AccessListShard> {
    shards
        .iter()
        .find(|shard| shard.start <= *lock_hash && *lock_hash <= shard.end)
}
```

For blacklist mode:

```rust
let Some(shard) = find_covering_shard(&shards, &lock_hash) else {
    return Err(Error::InvalidShardData);
};
if shard.entries.binary_search(&lock_hash).is_ok() {
    return Err(Error::AccessDenied);
}
```

For whitelist mode:

```rust
let Some(shard) = find_covering_shard(&shards, &lock_hash) else {
    return Err(Error::AccessDenied);
};
if shard.entries.binary_search(&lock_hash).is_err() {
    return Err(Error::AccessDenied);
}
```

Keep local shard parsing strict enough for proof validation: shard data decoding, `start <= end`, entry sorting, uniqueness, range containment, and `MAX_ACCESSLIST_ENTRIES`. Do not require visible shards to cover the full domain, form a complete chain, or use prefix-bucket alignment.

- [ ] **Step 3: Run blacklist tests**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="blacklist -- --nocapture"
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="proof -- --nocapture"
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test CARGO_ARGS="access_list_blacklist_rejects_suffix_only_nibble_alignment -- --nocapture"
```

Expected: AccessList blacklist chain tests pass, xUDT proof tests pass, and xUDT does not require complete visible chain coverage.

- [ ] **Step 4: Commit**

```bash
git add contracts/access-list/src/shards.rs contracts/xudt/src/access.rs tests/src/tests/xudt.rs tests/src/tests/access_list.rs
git commit -m "fix: require access list proofs"
```

---

### Task 6: Spec Sync and Full Verification

**Files:**
- Modify: `ref/Enhanced UDT Standard V1.md`

- [ ] **Step 1: Update V1 standard text**

In `ref/Enhanced UDT Standard V1.md`, change the dependency notes to:

```markdown
* UDTType 定位 Meta 时不依赖 MetaType code_hash，使用 type_hash==self.args 定位并解码；Meta.lock 由 MetaType 自己在 GroupOutput 上约束，consumer 不重复检查。
* AccessListType 不需要知道 MetaType code_hash，同样使用 args（meta_type_hash）定位 Meta；Meta.lock 不由 AccessListType 检查。
```

In the xUDT access section, replace "Blacklist must cover full domain" with:

```markdown
Blacklist 的完整 AccessList shard chain 由 AccessListType 在 shard 更新时验证；xUDT 作为使用者只要求每个被检查的 input lock hash 都有可见 covering shard 作为不包含证明，若 proof 缺失或 covering shard 中包含该 lock hash 则拒绝。Whitelist 要求可见 covering shard 且 entries 包含该 lock hash 作为包含证明。
```

- [ ] **Step 2: Run complete verification**

Run:

```bash
RUSTUP_TOOLCHAIN=1.92.0 cargo fmt --check
RUSTUP_TOOLCHAIN=1.92.0 make build MODE=debug
RUSTUP_TOOLCHAIN=1.92.0 MODE=debug make test
git diff --check
```

Expected: all commands pass.

- [ ] **Step 3: Commit docs and final fixes**

```bash
git add ref/Enhanced\ UDT\ Standard\ V1.md contracts tests
git commit -m "docs: clarify cell model invariant ownership"
```

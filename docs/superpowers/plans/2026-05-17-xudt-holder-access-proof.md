# xUDT Holder Access Proof Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make xUDT access control holder-based, CellDep-only, ordered, and memory-bounded.

**Architecture:** xUDT will choose checked lock sources per operation, read AccessList proofs from ordered CellDeps only, build a lightweight ordered CellDep shard index, and validate lock hashes in bounded batches. AccessList entries remain owned by the AccessList type script; xUDT parses full shard data only for shards used as proof for checked locks.

**Tech Stack:** Rust no-std CKB contracts, `ckb-std`, `standard_udt_types::metadata::AccessListShard`, `ckb-testtool` integration tests, `make build MODE=debug`, `MODE=debug make test`.

---

## File Map

- Modify `contracts/xudt/src/access.rs`: replace full-shard collection with operation-aware source checking, CellDep-only proof indexing, ordered proof validation, and lock batching.
- Modify `contracts/xudt/src/entry.rs`: call access validation with operation-specific checked sources and treat partial user destruction with remaining outputs as holder movement.
- Reuse existing `InvalidShardData` / `AccessDenied` errors; no new xUDT error is needed.
- Modify `tests/src/tests/xudt/access.rs`: add access behavior tests for output holders, mint, ordered CellDep proofs, Input proof rejection, and same-transaction AccessList state updates with explicit CellDep proofs.
- Modify `tests/src/tests/xudt/mod.rs`: add fixtures for custom output locks, AccessList output cells, and ordered/unordered CellDep proof shards if missing.
- Modify `Architecture.md` and `README.md`: document holder-based access semantics and CellDep-only ordered proofs after implementation.

## Task 1: Add Holder-Side Access Tests

**Files:**
- Modify: `tests/src/tests/xudt/access.rs`
- Modify: `tests/src/tests/xudt/mod.rs`

- [x] **Step 1: Add a whitelist output-lock rejection test**

Add a test named `xudt_whitelist_rejects_non_whitelisted_output_lock` in `tests/src/tests/xudt/access.rs`.

Test shape:

```rust
#[test]
fn xudt_whitelist_rejects_non_whitelisted_output_lock() {
    let mut fixture = XudtFixture::new();
    let allowed_lock = fixture.always_success_lock_with_args(Bytes::from(vec![1u8]));
    let denied_lock = fixture.always_success_lock_with_args(Bytes::from(vec![2u8]));
    let meta = fixture.live_meta_dep(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
    );
    let access_list = fixture.live_access_list_dep(full_domain_shard(vec![
        allowed_lock.script_hash,
    ]));

    let tx = fixture
        .tx_builder()
        .input(fixture.live_udt_input_with_lock(&allowed_lock.script, 100))
        .output(fixture.udt_output_with_lock(&denied_lock.script, 100))
        .output_data(amount_bytes(100).pack())
        .cell_dep(cell_dep(meta.previous_output()))
        .cell_dep(cell_dep(access_list.previous_output()))
        .build();

    expect_tx_fail_with_code(&fixture.context, &fixture.complete_tx(tx), "error code 61");
}
```

If helper names differ, add small fixture wrappers with the same meaning rather than changing the assertion.

- [x] **Step 2: Add a blacklist output-lock rejection test**

Add `xudt_blacklist_rejects_blacklisted_output_lock` in `tests/src/tests/xudt/access.rs`.

Use access flags `CONFIG_ACCESS_ENABLED`, build an AccessList shard containing the output lock hash, and expect xUDT error code 61.

- [x] **Step 3: Add mint output-lock access tests**

Add:

```rust
#[test]
fn xudt_whitelist_mint_rejects_non_whitelisted_output_lock() { /* output lock absent from whitelist */ }

#[test]
fn xudt_blacklist_mint_rejects_blacklisted_output_lock() { /* output lock present in blacklist */ }
```

Both tests must mint positive xUDT amount, include visible metadata, satisfy mint authority, and fail because output holder access is invalid.

- [x] **Step 4: Run tests and verify red**

Run:

```bash
MODE=debug cargo test -p tests xudt_ -- --nocapture
```

Expected: the new holder-output and mint-output tests fail because current xUDT access only checks `GroupInput` and mint does not run access checks.

## Task 2: Add Proof Source And Ordering Tests

**Files:**
- Modify: `tests/src/tests/xudt/access.rs`
- Modify: `tests/src/tests/xudt/mod.rs`

- [x] **Step 1: Add Input-proof rejection test**

Add `xudt_whitelist_rejects_input_access_list_as_transfer_proof`.

Build a whitelist transfer where:

- the required AccessList shard is present as an input cell;
- no matching AccessList CellDep proof is provided;
- the input lock would be allowed if input proof were accepted.

Expected after implementation: fail because xUDT proof source is CellDep-only.

- [x] **Step 2: Add unordered CellDep proof rejection test**

Add `xudt_rejects_unordered_access_list_cell_dep_proofs`.

Build two AccessList CellDep shards bound to the same meta:

```text
Shard B: start = prefix_start(0x10), end = prefix_end(0x1f)
Shard A: start = prefix_start(0x00), end = prefix_end(0x0f)
```

Provide them in CellDep order B then A. Expected after implementation: fail as invalid proof.

- [x] **Step 3: Add overlapping CellDep proof rejection test**

Add `xudt_rejects_overlapping_access_list_cell_dep_proofs`.

Provide two matching AccessList CellDeps where the second starts before or at the previous end. Expected: fail as invalid proof.

- [x] **Step 4: Add same-transaction AccessList state update tests**

Add:

```rust
#[test]
fn xudt_transfer_allows_same_meta_access_list_update_with_cell_dep_proof() { /* AccessList update plus explicit CellDep proof */ }

#[test]
fn xudt_pure_user_destruction_allows_same_meta_access_list_update() { /* pure destruction plus valid AccessList update */ }
```

The transfer test should include otherwise sufficient CellDep proof and pass because AccessList input/output cells are not proof sources. The pure destruction test should pass because pure user destruction does not depend on holder access.

- [x] **Step 5: Run tests and verify red**

Run:

```bash
MODE=debug cargo test -p tests xudt_ -- --nocapture
```

Expected: new tests fail under current implementation because `Source::Input` proofs are accepted, CellDep proofs are sorted in memory instead of requiring order, and same-transaction AccessList state updates are still rejected by xUDT.

## Task 3: Refactor xUDT Access API By Operation

**Files:**
- Modify: `contracts/xudt/src/access.rs`
- Modify: `contracts/xudt/src/entry.rs`

- [x] **Step 1: Introduce checked-source API**

In `contracts/xudt/src/access.rs`, replace `validate_if_enabled(meta_type_hash, meta_data)` with:

```rust
pub enum CheckedLocks {
    Inputs,
    Outputs,
    InputsAndOutputs,
    None,
}

pub fn validate_if_enabled(
    meta_type_hash: &[u8; 32],
    meta_data: &XudtMeta,
    checked_locks: CheckedLocks,
) -> Result<(), Error> {
    if !meta::is_access_enabled(meta_data) || matches!(checked_locks, CheckedLocks::None) {
        return Ok(());
    }
    validate_checked_locks(meta_type_hash, meta::is_whitelist(meta_data), checked_locks)
}
```

- [x] **Step 2: Wire operation-specific calls**

In `contracts/xudt/src/entry.rs`:

- transfer calls `CheckedLocks::InputsAndOutputs`;
- mint calls `CheckedLocks::Outputs` after mint authority and supply validation;
- protocol burn calls `CheckedLocks::InputsAndOutputs`;
- negative delta without metadata input:
  - if `output_amount == 0`, pure user destruction returns `Ok(())`;
  - if `output_amount > 0`, partial user destruction plus transfer calls `access::validate_if_enabled(&meta_type_hash, &visible_meta, CheckedLocks::InputsAndOutputs)` using current visible metadata from input or cell dep.

- [x] **Step 3: Add partial user destruction access tests**

Add tests in `tests/src/tests/xudt/access.rs`:

```rust
#[test]
fn xudt_partial_user_destruction_checks_blacklisted_input_lock() {
    // input amount 100, output amount 99, no metadata input.
    // blacklist shard contains input lock.
    // Expected: fail with access denied.
}

#[test]
fn xudt_partial_user_destruction_checks_whitelisted_output_lock() {
    // input amount 100, output amount 99, no metadata input.
    // whitelist shard contains input lock but not output lock.
    // Expected: fail with access denied.
}

#[test]
fn xudt_pure_user_destruction_skips_holder_access() {
    // input amount 100, output amount 0, no metadata input.
    // input lock is blacklisted or absent from whitelist.
    // Expected: pass, as pure destruction must remain available.
}
```

- [x] **Step 4: Run focused tests**

Run:

```bash
cargo test -p xudt --features library -- --nocapture
```

Expected: compile succeeds. If the package has no host tests, expect zero tests but no compile errors.

## Task 4: Implement CellDep Shard Index

**Files:**
- Modify: `contracts/xudt/src/access.rs`

- [x] **Step 1: Add index structures**

Add:

```rust
#[derive(Clone, Copy)]
struct ShardIndex {
    start: [u8; 32],
    end: [u8; 32],
    dep_index: usize,
}

#[derive(Clone, Copy)]
struct LockState {
    lock_hash: [u8; 32],
}
```

- [x] **Step 2: Build ordered CellDep index**

Add `build_shard_index(meta_type_hash) -> Result<Vec<ShardIndex>, Error>`.

Behavior:

- scan `Source::CellDep`;
- only consider type scripts matching AccessList code hash, Data2, and meta args;
- parse only range fields for the index;
- reject `next.start <= previous.end`;
- store `{ start, end, dep_index }`.

Use `AccessListShard::from_slice` temporarily if there is no range-only parser yet; then replace with range-only parsing in Task 5.

- [x] **Step 3: Remove CellDep/Input full-shard collection**

Delete `collect_visible_shards` and `collect_shards_from_source` after the new index path compiles.

- [x] **Step 4: Run focused xUDT access tests**

Run:

```bash
MODE=debug cargo test -p tests xudt_whitelist_rejects_input_access_list_as_transfer_proof -- --nocapture
MODE=debug cargo test -p tests xudt_rejects_unordered_access_list_cell_dep_proofs -- --nocapture
```

Expected: after rebuilding contracts in a later task these pass; before rebuild, stale debug binaries may still show old behavior.

## Task 5: Add Range-Only AccessList Parsing

**Files:**
- Modify: `contracts/xudt/src/access.rs`

- [x] **Step 1: Implement range-only parser**

Add a helper:

```rust
fn parse_access_list_range(data: &[u8]) -> Result<([u8; 32], [u8; 32]), Error>
```

It must read the AccessListShard molecule table enough to extract `range.start`
and `range.end`, reject malformed data, and validate:

```rust
start <= end
```

Do not parse entries in this helper.

- [x] **Step 2: Use range-only parser in index build**

Change `build_shard_index` to call `parse_access_list_range`.

- [x] **Step 3: Keep full parse for used shards**

When validating locks covered by a shard, load that shard data and call:

```rust
AccessListShard::from_slice(&data)
```

before binary searching entries.

- [x] **Step 4: Run type and xUDT compile checks**

Run:

```bash
cargo test -p standard-udt-types -- --nocapture
cargo test -p xudt --features library -- --nocapture
```

Expected: both commands pass.

## Task 6: Implement Lock Collection And Batch Validation

**Files:**
- Modify: `contracts/xudt/src/access.rs`

- [x] **Step 1: Add batch constants**

Add:

```rust
const SINGLE_BATCH_LOCK_LIMIT: usize = 64;
const LOCK_BATCH_SIZE: usize = 64;
```

These values are conservative and can be tuned after cycle measurements.

- [x] **Step 2: Count checked locks**

Add:

```rust
fn count_checked_locks(checked_locks: CheckedLocks) -> Result<usize, Error>
```

It scans the selected `GroupInput` and/or `GroupOutput` sources using
`load_cell_lock_hash` and counts cells.

- [x] **Step 3: Implement small transaction path**

If count <= `SINGLE_BATCH_LOCK_LIMIT`, collect all selected lock hashes into a
`Vec<[u8; 32]>`, sort, dedup, then call `validate_lock_batch`.

- [x] **Step 4: Implement large transaction path**

If count > `SINGLE_BATCH_LOCK_LIMIT`, collect lock hashes into a Vec until
`LOCK_BATCH_SIZE`, sort/dedup/validate the batch, clear it, and continue. After
the scan, validate the final partial batch.

- [x] **Step 5: Implement `validate_lock_batch`**

`validate_lock_batch` receives:

```rust
fn validate_lock_batch(
    whitelist: bool,
    locks: &mut Vec<[u8; 32]>,
    shard_index: &[ShardIndex],
) -> Result<(), Error>
```

It must:

- return `Ok(())` for an empty batch;
- sort and dedup locks;
- scan locks and shard indexes in order;
- fail if a lock has no covering shard;
- load each covering shard data at most once for consecutive locks covered by
  the same `dep_index`;
- full-parse the shard and binary-search entries;
- require membership in whitelist mode;
- require non-membership in blacklist mode.

- [x] **Step 6: Run focused access tests**

Run:

```bash
make build MODE=debug
MODE=debug cargo test -p tests xudt_ -- --nocapture
```

Expected: all xUDT tests pass, including new access tests.

## Task 7: Add Many-Lock Batch Coverage

**Files:**
- Modify: `tests/src/tests/xudt/access.rs`
- Modify: `tests/src/tests/xudt/mod.rs`

- [x] **Step 1: Add whitelist many-lock transfer test**

Add `xudt_whitelist_accepts_many_output_locks_in_batches`.

Build more than `SINGLE_BATCH_LOCK_LIMIT` unique output locks, include a full-domain whitelist shard containing all those output lock hashes, and expect pass.

- [x] **Step 2: Add blacklist many-lock transfer test**

Add `xudt_blacklist_accepts_many_unlisted_output_locks_in_batches`.

Build more than `SINGLE_BATCH_LOCK_LIMIT` unique output locks, include full-domain blacklist shard with empty entries, and expect pass.

- [x] **Step 3: Run targeted tests**

Run:

```bash
make build MODE=debug
MODE=debug cargo test -p tests xudt_whitelist_accepts_many_output_locks_in_batches -- --nocapture
MODE=debug cargo test -p tests xudt_blacklist_accepts_many_unlisted_output_locks_in_batches -- --nocapture
```

Expected: both pass.

## Task 8: Update Docs

**Files:**
- Modify: `Architecture.md`
- Modify: `README.md`

- [x] **Step 1: Update access semantics text**

Document that xUDT access mode is holder-based and validates relevant input and output token holders.

- [x] **Step 2: Update proof source text**

Document that xUDT movement uses CellDep-only AccessList proofs. Same-meta AccessList inputs or outputs are not proof sources, but may appear when their own state transition is valid.

- [x] **Step 3: Update performance text**

Document ordered proof shards, lightweight shard indexes, range-only indexing, and batched lock validation at a high level.

## Task 9: Full Verification And Commit

**Files:**
- All modified code, tests, and docs.

- [x] **Step 1: Format and diff check**

Run:

```bash
cargo fmt --check
git diff --check
```

Expected: both pass.

- [x] **Step 2: Host tests**

Run:

```bash
cargo test -p standard-udt-types -- --nocapture
cargo test -p xudt --features library -- --nocapture
```

Expected: both pass.

- [x] **Step 3: Debug build and integration tests**

Run:

```bash
make build MODE=debug
MODE=debug make test
```

Expected: build succeeds and all integration tests pass.

- [x] **Step 4: Review diff**

Run:

```bash
git diff --stat
git diff -- contracts/xudt/src/access.rs contracts/xudt/src/entry.rs tests/src/tests/xudt README.md Architecture.md
```

Expected: diff is limited to xUDT holder access proof behavior, tests, and docs.

- [x] **Step 5: Commit**

Run:

```bash
git add contracts/xudt/src/access.rs contracts/xudt/src/entry.rs tests/src/tests/xudt README.md Architecture.md docs/superpowers/specs/2026-05-17-xudt-holder-access-proof-design.md docs/superpowers/plans/2026-05-17-xudt-holder-access-proof.md
git commit -m "feat: make xudt access holder based"
```

Expected: commit succeeds.

## Self-Review

- Spec coverage: holder-based access, CellDep-only proofs, ordered proof shards, same-transaction AccessList update behavior, range-only indexing, lock batching, and docs are all mapped to tasks.
- Placeholder scan: no intentional placeholders remain; tests are named explicitly and commands include expected outcomes.
- Type consistency: `CheckedLocks`, `ShardIndex`, and batch helpers are introduced before use in later tasks.

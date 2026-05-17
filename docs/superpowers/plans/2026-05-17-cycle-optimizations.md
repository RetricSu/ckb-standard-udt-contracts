# CKB UDT Cycle Optimizations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce CKB contract cycles without changing UDT/meta/access-list protocol semantics.

**Architecture:** Keep validation ownership unchanged and optimize syscall-heavy scan paths. Measure cycles after each independent change using temporary ignored tests, then remove probe code before continuing.

**Tech Stack:** Rust no-std CKB scripts, `ckb-std` syscalls, `ckb-testtool` integration tests, root `make build MODE=debug`, `MODE=debug make test`.

---

### Task 1: Amount Prefix Reads

**Files:**
- Modify: `lib/script-utils/src/amount.rs`
- Modify: `lib/script-utils/src/token.rs`
- Test: `lib/script-utils/src/amount.rs`
- Temporary probe: `tests/src/tests/cycles_probe.rs`, `tests/src/lib.rs`

- [x] Add a failing unit test for amount prefix result handling in `lib/script-utils/src/amount.rs`.
- [x] Run `cargo test -p standard-udt-script-utils amount::tests::maps_length_not_enough_to_amount_prefix`.
- [x] Implement a 16-byte amount prefix reader using `ckb_std::syscalls::load_cell_data`.
- [x] Update `collect_group_amount` and token amount summing to use the prefix reader.
- [x] Run focused unit tests, build debug binaries, run cycles probe, then delete probe.

**Result:** Kept. This avoids loading whole UDT data when only the first 16 bytes are needed. It preserves the previous amount decoding semantics: data shorter than 16 bytes is invalid, trailing bytes are ignored, and overflow checks are unchanged.

### Task 2: Expected Type Hash Scans

**Files:**
- Modify: `lib/script-utils/src/token.rs`
- Modify: `contracts/sudt-meta/src/state.rs`
- Modify: `contracts/xudt-meta/src/state/token.rs`
- Modify: `contracts/xudt-meta/src/state/access_list.rs`
- Modify: `contracts/xudt/src/access.rs`
- Test: `lib/script-utils/src/token.rs`
- Temporary probe: `tests/src/tests/cycles_probe.rs`, `tests/src/lib.rs`

- [x] Add a unit test that expected bound type script construction matches the existing full-script matcher.
- [x] Implement shared expected script hash helper in `lib/script-utils/src/cells.rs`.
- [x] Replace full `load_cell_type` scans where the measured path benefits from exact type-hash identity.
- [x] Run focused tests, build debug binaries, run cycles probes, then delete probes.

**Result:** Partially kept.

- Kept `sum_token_amount` and `transaction_token_delta` on the type-hash path. This has a fixed hash cost but large savings for batched token-cell scans.
- Kept access-list cell scans and xUDT access-list shard index scans on the type-hash path.
- Rejected the attempted type-hash conversion for `has_bound_xudt_cells` / `has_bound_xudt_outputs`; cycles were worse on representative paths, so only their duplicated loop was merged.

**Measured tradeoffs:**

- `sum_token_amount` meta create, old full-script scan vs new type-hash scan:
  - sUDT 1 output: `963,467` -> `1,037,756` cycles
  - xUDT 1 output: `1,071,929` -> `1,146,218` cycles
  - sUDT 70 outputs: `4,315,868` -> `1,626,947` cycles
  - xUDT 70 outputs: `4,424,465` -> `1,735,547` cycles
- `transaction_token_delta` type-hash sharing:
  - 70 token-cell paths saved about `682k` cycles
  - 1-output tracked mint regressed by about `39k` cycles against the adaptive experiment
- `has_bound_xudt_*` type-hash attempt, rejected:
  - access update: `1,776,122` full-script vs `1,814,135` type-hash
  - destroy: `422,742` full-script vs `571,231` type-hash

### Task 3: xUDT Access Single-Pass Lock Collection

**Files:**
- Modify: `contracts/xudt/src/access.rs`
- Test: `tests/src/tests/xudt/access.rs`
- Temporary probe: `tests/src/tests/cycles_probe.rs`, `tests/src/lib.rs`

- [x] Add a boundary test for 65 checked locks to exercise batched validation.
- [x] Remove the pre-count pass and always collect/flush in a single pass.
- [x] Run xUDT access tests, build debug binaries, run cycles probe, then delete probe.

**Result:** Kept. Removing the count pass avoids scanning checked lock hashes twice and does not alter whitelist/blacklist membership validation.

### Task 4: Final Verification

**Files:**
- Modify: `contracts/sudt-meta/src/update.rs`

- [x] Confirm no function-order-only diff remains in `contracts/sudt-meta/src/update.rs`.
- [x] Run `cargo fmt`.
- [x] Run `make build MODE=debug`.
- [x] Run `MODE=debug make test`.
- [x] Run `git diff --check`.
- [x] Confirm no temporary cycles probe remains.

**Final verification:**

- `cargo test -p standard-udt-script-utils`
- `make build MODE=debug`
- `MODE=debug make test`
- `git diff --check`
- `rg -n "cycles_probe|probe_sudt|probe_xudt" tests/src -S`

All final verification commands passed. Temporary ignored cycles probes were removed before commit.

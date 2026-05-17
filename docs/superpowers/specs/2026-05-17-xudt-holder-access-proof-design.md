# xUDT Holder Access Proof Design

## Summary

xUDT access control should define who may hold xUDT cells, not only who may
spend existing xUDT cells. When access mode is enabled, xUDT must validate the
lock hashes of all relevant xUDT inputs and outputs. AccessList proof cells used
by xUDT token movement should be read-only `CellDep` cells, ordered by shard
range, and verified with bounded memory.

## Current Behavior

`contracts/xudt/src/access.rs` currently:

- loads AccessList proof shards from both `Source::CellDep` and `Source::Input`;
- collects all visible shards into memory;
- sorts the shards after collection;
- validates only `Source::GroupInput` lock hashes.

This behavior enforces "who may spend" more than "who may hold". It also keeps
full `AccessListShard` values in memory, including entries, even when many
shards are not relevant to the locks being checked.

## Desired Semantics

Access mode is holder-based:

- whitelist: every checked xUDT holder lock must appear in a provided
  AccessList shard entry;
- blacklist: every checked xUDT holder lock must be covered by a provided
  AccessList shard range and must not appear in that shard's entries.

Operation-specific checked lock sources:

- transfer: `GroupInput` and `GroupOutput`;
- mint: `GroupOutput`;
- protocol burn: `GroupInput` and `GroupOutput`;
- negative delta without metadata input:
  - if `GroupOutput` total amount is zero, this is pure user destruction and
    holder access is not checked;
  - if any xUDT amount remains in `GroupOutput`, this is partial user
    destruction plus transfer and must check `GroupInput` and `GroupOutput`.

## Proof Source Rule

xUDT token movement must read AccessList proofs from `Source::CellDep` only.
`Source::Input` AccessList cells are state-transition participants, not
read-only transfer proofs.

Same-meta AccessList cells in `Source::Input` or `Source::Output` do not act as
xUDT proofs. They may appear in the same transaction when their own AccessList
state transition is valid. xUDT validates holder access only against explicit
matching CellDep proof shards.

## Ordered CellDep Proofs

Among CellDeps bound to the current metadata type hash, AccessList proof shards
must appear in ascending range order:

```text
next.start > previous.end
```

This rejects duplicate, overlapping, and out-of-order proof shards. The ordering
rule applies only to matching AccessList proof CellDeps; unrelated CellDeps do
not participate in this ordering.

## Shard Indexing

xUDT should build a lightweight in-memory shard index from CellDep proof shards:

```rust
struct ShardIndex {
    start: [u8; 32],
    end: [u8; 32],
    dep_index: usize,
}
```

The first pass streams CellDeps and stores only `{ start, end, dep_index }` for
matching AccessList shards. It should parse only the shard range needed for
indexing. Entries for unrelated shards do not need to be decoded by xUDT because
the AccessList type script owns shard-state validity during creation and update.

For shards that actually cover checked locks, xUDT loads the shard data by
`dep_index` and parses the full `AccessListShard` before checking entries. This
keeps long-lived memory bounded while still validating the entry facts xUDT uses.

## Lock Batching

xUDT should validate locks with bounded memory:

- count the lock hashes for the operation's checked sources;
- if the count is small, collect all lock hashes, sort and dedup them, then
  validate in one batch;
- if the count is large, collect lock hashes in fixed-size batches, sort and
  dedup each batch, validate the batch, then clear it.

Cross-batch duplicate locks may be validated more than once. That is acceptable
because it only affects cycles, not safety.

## Batch Validation Algorithm

For each sorted lock batch:

1. Use the ordered `ShardIndex` list and sorted locks with a two-pointer scan.
2. For each lock, find the covering shard index.
3. If no covering shard exists:
   - whitelist: fail with access denied;
   - blacklist: fail as missing/invalid proof.
4. Group consecutive locks covered by the same `dep_index`.
5. Load and parse that shard data once for the group.
6. Use `binary_search` on shard entries for each grouped lock.
7. Apply the access-mode decision:
   - whitelist requires entry membership;
   - blacklist requires entry non-membership.

The parsed `AccessListShard` type guarantees entries are sorted, unique, and in
range, so `binary_search` remains valid.

## Error Semantics

The implementation should keep errors simple and compatible with existing xUDT
error categories:

- access denied for whitelist miss or blacklist hit;
- invalid shard/proof data for malformed proof, unordered proof, overlapping
  proof, or missing blacklist coverage.

New error variants are allowed only if existing variants make tests ambiguous.

## Out Of Scope

This change does not:

- change the AccessList cell state machine;
- change AccessList type args;
- introduce a chain-level shard index;
- require minimal proof shards;
- add a cross-batch seen cache;
- change xUDT extension behavior.

## Test Requirements

Add integration coverage for:

- whitelist transfer rejects non-whitelisted output lock;
- blacklist transfer rejects blacklisted output lock;
- whitelist mint rejects non-whitelisted output lock;
- blacklist mint rejects blacklisted output lock;
- protocol burn checks remaining outputs as holders;
- same-meta AccessList inputs/outputs are not treated as proof unless also
  provided as matching CellDeps;
- xUDT uses CellDep AccessList proof and ignores Input AccessList proof;
- CellDep proof shards must be ordered and non-overlapping;
- many-lock transfers can pass through the batch path.

Existing tests for input-side whitelist/blacklist behavior must continue to
pass.

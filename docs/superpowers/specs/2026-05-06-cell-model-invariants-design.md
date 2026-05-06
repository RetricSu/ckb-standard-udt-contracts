# Cell Model Invariants Design

## Goal

Move invariant checks to the script that owns the cell being constrained, and make xUDT access control fail closed by requiring membership or non-membership proofs for every checked lock hash.

This is a contract-boundary cleanup. It does not change Molecule schemas, ABI symbols, authority semantics, or UDT/meta binding.

## Problem

The current implementation checks `Meta.lock` from several consumer scripts:

- sUDT scans meta and checks its lock.
- xUDT scans meta and checks its lock.
- AccessList scans meta and checks its lock.

That duplicates a cell-owned invariant in scripts that merely consume meta state. In CKB's cell model, a type script should enforce invariants for its own live output cells. Consumers should only verify the cross-cell facts they depend on.

The current implementation also has an access-control proof gap. xUDT can only validate the AccessList shards supplied by the transaction. In blacklist mode, missing shards must not mean "not listed"; otherwise a transaction can omit the shard that would reject it. xUDT should therefore require a covering shard proof for each checked lock hash, while AccessListType remains responsible for the global shard chain invariant.

## Ownership Rules

### MetaType Owns Meta Cell Invariants

`sudt-meta` and `xudt-meta` own the following invariants:

- Meta type args shape and type-id creation.
- Meta data decoding and field constraints.
- Authority field mutation rules.
- Supply tracking mode immutability and meta supply field constraints.
- `GroupOutput` meta lock must use the always-success lock code hash whitelist.

`GroupInput` meta lock does not need to be rechecked during an update because the existing cell was already created under the same type script. If a transaction creates a replacement meta cell, the replacement appears in `GroupOutput` and is checked there.

### UDTType Consumes Meta State

`sudt` and `xudt` should not validate `Meta.lock`.

They must still validate the cross-cell facts they depend on:

- `type.args` is exactly 32 bytes and equals `meta_type_hash`.
- Amount decoding uses the first 16 bytes and rejects data shorter than 16 bytes.
- Amount summation detects overflow.
- Non-conservation requires the relevant visible meta.
- Visible meta must be unique in the script's chosen visible set.
- Meta data must decode successfully.
- Mint/protocol-burn uses the relevant authority.
- Tracked supply changes must match input/output meta state.
- xUDT transfer/mint/burn must enforce paused, access, and extension behavior.
- For xUDT `output_amount < input_amount`, a visible meta in `CellDep` marks the operation as user destruction. Protocol burn is selected only when no `CellDep` meta is supplied and an input meta is being consumed.

### AccessListType Owns Shard Cells

`access-list` owns the following invariants:

- Its own `GroupOutput` shard locks must use the always-success lock code hash whitelist.
- Shard data must decode strictly.
- Entries must be sorted, unique, within the shard range, and below the `4096` entries-per-shard limit.
- Ranges must be ordered and non-overlapping.
- Blacklist mode must satisfy the shard chain invariant.
- Blacklist updates may only insert/delete entries within equal ranges, or split/merge adjacent ranges without changing coverage or flattened entries.
- Disabled mode may remove stale shard cells but must not create replacement shard outputs.

`access-list` should not validate `Meta.lock`; it only reads meta to determine access mode and access authority.

`xudt-meta` owns access-mode transition completeness constraints that cannot be proven by an individual AccessList group alone. In particular, switching from blacklist mode to disabled mode must consume AccessList input shards covering the full lock-hash domain, so stale blacklist shards cannot remain live after access is disabled.

## Blacklist Shard Chain Invariant

Blacklist mode uses AccessList shard cells as an implicit linked list over the full `Byte32` lock-hash domain.

The chain is encoded by contiguous ranges, not by explicit `prev`/`next` fields:

- The first shard range must start at `[0x00; 32]`.
- The last shard range must end at `[0xff; 32]`.
- For every adjacent pair, `right.start == increment_byte32(left.end)`.
- Ranges must be strictly ordered and non-overlapping.
- Each range must be nibble-aligned using the old accesslist implementation's prefix-bucket rule:
  - `start[0] & 0x0f == 0x00`
  - `start[1..]` are all `0x00`
  - `end[0] & 0x0f == 0x0f`
  - `end[1..]` are all `0xff`

The current suffix-only check on `start[31]` / `end[31]` is not the intended bucket model and should be replaced.

## xUDT Access Reader Rules

xUDT's access reader is a consumer of AccessList state. It must not revalidate the complete blacklist shard chain. It only validates enough visible shard data to prove the access decision for the checked `GroupInput` lock hashes.

For blacklist mode:

- Collect visible AccessList shards from `CellDep` and `Input` whose type script is `ACCESS_LIST_CODE_HASH`, `Data2`, and `args == meta_type_hash`.
- Parse each shard strictly enough for proof validation: decode shard data, require `start <= end`, require entries sorted and unique, require entries to be within the shard range, and enforce the entries-per-shard limit.
- Sort by `(start, end)`.
- For each checked `GroupInput` lock hash, require at least one visible shard whose range covers the hash.
- That covering shard is the non-membership proof: if the hash appears in any covering shard's entries, reject; if no covering shard is visible, reject.

For whitelist mode:

- Collect visible AccessList shards from `CellDep` and `Input`.
- Parse each shard strictly enough for proof validation.
- Sort by `(start, end)`.
- For each checked `GroupInput` lock hash, require at least one visible shard whose range covers the hash and whose entries contain the hash.
- That covering entry is the membership proof; missing coverage or missing entry rejects.

The checked lock set remains `GroupInput` lock hashes.

xUDT must not reject a proof shard solely because visible shards do not form a complete chain, do not cover the full domain, or are not prefix-bucket aligned. Those are AccessListType-owned state invariants.

## Meta Discovery Rules

Consumer scripts still need meta discovery, but discovery should not include meta lock validation.

Expected behavior:

- `find_unique_visible_meta` may scan `CellDep` and `Input`.
- location-specific functions may scan `Input` or `Output` for supply/meta-update cross checks.
- duplicate meta in a scanned source remains an error.
- conflicting visible meta remains an error.
- meta data decode failure remains an error.
- meta lock code hash is ignored by consumers.

MetaType is the only place that rejects bad meta output locks.

## Tests

New or changed tests must prove:

- sUDT mint no longer rejects a visible meta solely because that meta cell uses a non-whitelisted lock.
- xUDT transfer/mint/protocol-burn no longer rejects a visible meta solely because that meta cell uses a non-whitelisted lock.
- AccessList update no longer rejects a visible meta solely because that meta cell uses a non-whitelisted lock.
- `sudt-meta` rejects a created or updated meta output with a non-whitelisted lock.
- `xudt-meta` rejects a created or updated meta output with a non-whitelisted lock.
- `access-list` rejects created or updated shard outputs with non-whitelisted locks.
- AccessListType rejects blacklist updates that break complete chain coverage or prefix-bucket nibble alignment.
- xUDT blacklist rejects when a checked input lock hash has no visible covering shard proof.
- xUDT blacklist rejects when a visible covering shard contains the checked input lock hash.
- xUDT whitelist still fails closed when the relevant lock hash is not covered/listed.

## Out of Scope

- No schema changes.
- No authority ABI changes.
- No extension ABI changes.
- No change to the `Authority` / `Extension` split.
- No change to sUDT transfer behavior.
- No change to tracked supply semantics except keeping existing UDT/meta supply cross checks.

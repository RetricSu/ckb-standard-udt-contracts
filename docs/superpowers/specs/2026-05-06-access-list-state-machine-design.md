# AccessList State Machine Design

## Goal

Make AccessList state transitions explicit, mode-independent, and enforceable without mixing responsibilities between `xudt-meta`, `access-list`, and `xudt`.

The core model is:

- Whitelist and blacklist use the same full-domain shard structure and the same lifecycle rules.
- Whitelist and blacklist differ only in how `xudt` interprets entries during proof validation.
- Switching between whitelist and blacklist changes entry semantics, so it must consume the old full-domain list and create a new full-domain list.

## Problem

The current AccessList rules are asymmetric:

- Blacklist requires full-domain shard coverage.
- Whitelist only requires at least one output shard.
- `xudt-meta` only checks for the presence of an AccessList output for several transitions.
- `access-list` allows creating a full-domain blacklist list from empty inputs whenever the output meta mode is blacklist.

This leaves unclear or unsafe cases:

- Whitelist shards are not globally provable because they do not have full-domain coverage.
- Existing active AccessList state can potentially be forked by creating another full-domain list from empty inputs.
- `whitelist <-> blacklist` could accidentally reuse old entries even though entry meaning is reversed.
- Destroy rules depend on the output mode but do not fully encode whether this transaction is a valid lifecycle transition.

## State Model

Access mode is one of:

- `Disabled`
- `Whitelist`
- `Blacklist`

`Whitelist` and `Blacklist` are active modes. An active AccessList is a full-domain shard set over the complete `Byte32` lock-hash domain:

- First shard starts at `[0x00; 32]`.
- Last shard ends at `[0xff; 32]`.
- Adjacent ranges are contiguous.
- Ranges are ordered and non-overlapping.
- Ranges use the existing prefix-bucket nibble alignment rule.
- Entries are sorted, unique, within range, and limited to 4096 per shard.

The shard structure is identical for whitelist and blacklist.

## Lifecycle Transitions

The AccessList lifecycle transition is derived from old meta mode, new meta mode, and AccessList group input/output shard sets.

| Meta transition | AccessList transition | Required shard shape |
| --- | --- | --- |
| Disabled -> Disabled | No AccessList lifecycle operation | AccessList type script is not expected to run; reject if invoked |
| Disabled -> Whitelist | Create | empty inputs, full-domain outputs |
| Disabled -> Blacklist | Create | empty inputs, full-domain outputs |
| Whitelist -> Whitelist | Update | one or more touched input shards and one or more replacement output shards covering exactly the same local range |
| Blacklist -> Blacklist | Update | one or more touched input shards and one or more replacement output shards covering exactly the same local range |
| Whitelist -> Disabled | Destroy | full-domain inputs, empty outputs |
| Blacklist -> Disabled | Destroy | full-domain inputs, empty outputs |
| Whitelist -> Blacklist | Replace | full-domain inputs, full-domain outputs |
| Blacklist -> Whitelist | Replace | full-domain inputs, full-domain outputs |

`Replace` is not a split/merge/update of the old entry set. It is a full consumption of the old list plus creation of a new list because the meaning of every entry flips.

`Update` is intentionally local. A transaction only needs to consume the shard nodes it is changing. Global consistency is preserved because the consumed input shard range must be replaced by output shards covering the same closed range, without gaps or overlap. The transaction cannot prove or require the untouched rest of the full-domain list, and it does not need to.

## Script Responsibilities

### `xudt-meta`

`xudt-meta` owns access mode transitions for the token meta cell.

It should:

- Require access authority when access enabled, whitelist bit, paused bit, or access authority changes.
- Reject access mode changes while same-token xUDT cells exist.
- Classify the access mode transition:
  - `Disabled -> Active`: requires AccessList full-domain outputs.
  - `Active -> Disabled`: requires AccessList full-domain inputs and no AccessList outputs.
  - `Active -> Active same mode`: requires no special meta-level lifecycle check; `access-list` validates group updates.
  - `Active -> Active different mode`: requires AccessList full-domain inputs and full-domain outputs.
- Not validate entry-level changes.
- Not validate split/merge legality.
- Not run access authority for AccessList cell updates; that remains `access-list` responsibility.

`xudt-meta` may implement full-domain input/output presence checks by scanning transaction inputs/outputs for AccessList type scripts matching `ACCESS_LIST_CODE_HASH`, `Data2`, and `args == meta_type_hash`.

### `access-list`

`access-list` owns AccessList shard cell lifecycle and shard state invariants.

It should:

- Read old and new meta modes from visible meta input/output/cell dep.
- Treat `CellDep` meta as mode fallback only; do not enforce meta cell uniqueness across sources.
- Classify the shard group operation using old mode, new mode, group inputs, and group outputs.
- Enforce full-domain structure for global lifecycle operations: create, destroy, and replace.
- Enforce local range continuity for same-mode updates.
- Enforce output shard lock whitelist.
- Validate shard data strictly.
- For `Create`:
  - inputs must be empty.
  - outputs must be full-domain.
- For `Destroy`:
  - inputs must be full-domain.
  - outputs must be empty.
- For `Replace`:
  - inputs must be full-domain.
  - outputs must be full-domain.
  - entries are not required to match because mode interpretation changed.
- For `Update`:
  - inputs must be non-empty.
  - outputs must be non-empty.
  - input shards must form one contiguous local range.
  - output shards must form one contiguous local range.
  - output local range must have the same start and end as the input local range.
  - same ranges may insert/delete entries.
  - split/merge may change ranges only if flattened entries are unchanged.
  - mixed split+merge rewrites are rejected.
- Require access authority when the shard set changes.

### `xudt`

`xudt` is only a consumer of the active list.

It should:

- Read current meta.
- Collect visible AccessList shards from `Input` and `CellDep`.
- Parse proof shards strictly enough to decide membership/non-membership.
- In whitelist mode, require a covering shard containing each checked `GroupInput` lock hash.
- In blacklist mode, require a covering shard not containing each checked `GroupInput` lock hash.
- Not validate full-domain chain structure.
- Not validate split/merge/update/destroy lifecycle.
- Not check whether a shard set belongs to an old mode beyond using current meta mode interpretation.
- Not enforce meta cell uniqueness across sources; meta type scripts own that invariant.

## No Disabled Cleanup Path

The final model does not include a compatibility cleanup path for AccessList cells while access mode is disabled:

- Active -> Disabled consumes the full-domain active list.
- Active -> Active different mode consumes the old full-domain list and creates a new full-domain list.
- Active -> Active same mode updates the same full-domain state.
- Disabled -> Disabled has no AccessList cells, so AccessList type script is normally not invoked. If it is invoked, the transaction contains AccessList cells under a disabled mode and must be rejected.

## Error Categories

Use existing error categories:

- `InvalidShardData` for malformed shard data.
- `InvalidShardSet` for structurally invalid lifecycle or coverage.
- `AccessListRequired` for `xudt-meta` mode transitions missing required full-domain AccessList presence.
- `AccessModeTokenCells` when access mode changes while same-token xUDT cells exist.
- `AuthorityMissing` / `AuthorityFailed` for missing or failed access authority.

## Tests

Add tests proving:

- Whitelist create requires full-domain outputs.
- Whitelist update only requires the touched local shard range and replacement local range.
- Whitelist split/merge follows the same rules as blacklist.
- Whitelist destroy requires full-domain inputs and empty outputs.
- Blacklist create cannot be repeated from empty inputs when the old mode is already blacklist.
- Whitelist create cannot be repeated from empty inputs when the old mode is already whitelist.
- Blacklist -> whitelist requires full-domain inputs and full-domain outputs.
- Whitelist -> blacklist requires full-domain inputs and full-domain outputs.
- Blacklist -> whitelist permits different entries in replacement outputs.
- Whitelist -> blacklist permits different entries in replacement outputs.
- Same-mode split/merge still rejects entry changes.
- `xudt` proof behavior is unchanged except it can now rely on whitelist full-domain state existing.

## Out of Scope

- No Molecule schema changes.
- No authority ABI changes.
- No xUDT extension ABI changes.
- No change to how xUDT interprets whitelist or blacklist entries.
- No change to meta lock ownership rules.

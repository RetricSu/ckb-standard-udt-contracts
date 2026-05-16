# Zero Supply Meta Destroy Design

## Goal

Allow tracked-supply `sudt-meta` and `xudt-meta` cells to be destroyed when
their tracked supply is zero, so the locked CKB capacity can be reclaimed. For
xUDT tokens with active access mode, all AccessList cells for the token must be
destroyed in the same transaction.

## Problem

The current metadata entrypoints accept only create and update shapes:

```text
(None, Some(output))   create
(Some(input), Some(output)) update
```

The destroy shape is rejected:

```text
(Some(input), None)    InvalidArgs
```

That makes a completed tracked-supply token impossible to retire even when the
tracked `current_supply` is zero. The metadata cell and any associated
AccessList cells continue to occupy live CKB state.

## Desired Behavior

### sUDT Metadata Destroy

`sudt-meta` accepts a transaction with exactly one group input metadata cell and
no group output metadata cell when all of these are true:

- supply tracking is enabled in the input metadata config;
- `input.current_supply == 0`;
- the transaction satisfies `mint_authority`.

The destroy is rejected when supply tracking is disabled, when tracked supply is
non-zero, or when the required authority is absent or fails.

### xUDT Metadata Destroy

`xudt-meta` accepts a transaction with exactly one group input metadata cell and
no group output metadata cell when all of these are true:

- supply tracking is enabled in the input metadata config;
- `input.current_supply == 0`;
- the transaction satisfies `mint_authority`;
- if input access mode is active, the transaction includes full-domain
  AccessList input cells for the same metadata type hash.

The destroy is rejected when supply tracking is disabled, when tracked supply is
non-zero, when authority is absent or fails, or when active access mode is
enabled without full-domain AccessList inputs.

## AccessList Interaction

The `access-list` script already validates active-to-disabled destruction:

```text
input mode:  blacklist or whitelist
output mode: disabled
inputs:      full-domain AccessList shards
outputs:     none
```

Destroying an active `xudt-meta` cell makes the output metadata context absent,
which maps to disabled access mode for AccessList validation. `xudt-meta` must
also require full-domain AccessList inputs in this case, so the metadata script
and AccessList script agree that an active access list is globally removed.

When xUDT access mode is already disabled, metadata destruction does not require
AccessList inputs.

## Ownership Rules

Metadata scripts own the metadata cell lifecycle. The zero-supply destroy check
belongs in `sudt-meta` and `xudt-meta`, not in the token scripts.

AccessList shard lifecycle stays in `access-list`. `xudt-meta` only checks that
full-domain AccessList inputs are present when active access state is being
retired.

Token scripts continue to own token movement and do not need new destroy logic.

## Authority Rules

Destroying a metadata cell consumes governed state, so zero supply alone is not
authorization.

The authority rule intentionally differs from no-op metadata updates:

- `sudt-meta`: only `mint_authority` can authorize metadata destruction.
- `xudt-meta`: only `mint_authority` can authorize metadata destruction.

This keeps state-cell unlock semantics consistent with the repository
architecture: metadata and AccessList cells use always-success locks, but their
type scripts enforce governance authority.

## Out of Scope

- No Molecule schema changes.
- No token amount semantics changes.
- No change to untracked supply semantics except that untracked metadata cannot
  be destroyed through this zero-supply retirement path.
- No change to AccessList shard validation rules.
- No change to metadata creation type-id rules.
- No new error codes unless existing codes cannot express the failure.

## Required Tests

The implementation must prove:

- `sudt-meta` currently rejects destroy before implementation.
- tracked zero-supply `sudt-meta` destroy with `mint_authority` passes.
- tracked zero-supply `sudt-meta` destroy with only `metadata_authority`
  fails.
- tracked non-zero-supply `sudt-meta` destroy fails with `InvalidSupply`.
- untracked zero-valued `sudt-meta` destroy fails with `InvalidSupply`.
- `xudt-meta` currently rejects destroy before implementation.
- tracked zero-supply `xudt-meta` destroy with one valid authority passes when
  that authority is `mint_authority` and access mode is disabled.
- tracked zero-supply `xudt-meta` destroy with only `metadata_authority` or
  only `access_authority` fails.
- tracked non-zero-supply `xudt-meta` destroy fails with `InvalidSupply`.
- active-access `xudt-meta` destroy without full-domain AccessList inputs fails
  with `AccessListRequired`.
- active-access `xudt-meta` destroy with full-domain AccessList inputs and no
  AccessList outputs passes.

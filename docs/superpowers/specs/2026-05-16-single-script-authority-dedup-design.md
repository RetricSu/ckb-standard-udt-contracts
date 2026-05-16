# Single Script Authority Dedup Design

## Goal

Reduce repeated authority checks inside a single type script invocation without
changing authorization semantics, error boundaries, or cross-script ownership.

The target scripts are:

- `sudt-meta`
- `xudt-meta`

## Problem

Metadata updates classify several independent state changes:

- supply or mint-authority changes;
- metadata field changes;
- xUDT access-state changes;
- xUDT extension-list changes;
- no-op governed state unlocks;
- metadata destruction.

The current implementation checks authority immediately inside each branch.
When one transaction changes multiple governed areas, the same authority can be
checked more than once in the same script invocation.

Examples:

- In `sudt-meta`, changing supply and metadata can check `mint_authority`, then
  check `metadata_authority` and possibly fall back to `mint_authority`.
- In `xudt-meta`, changing access state, extension list, metadata fields, and
  supply can independently trigger access, metadata, and mint checks. If the
  narrower authority is absent or fails, mint fallback can be checked repeatedly.

For simple hash-scan authorities, the cost is repeated input/output scans. For
dynamic-linking or spawn authorities, repeated checks can reload or execute
authority code more than once.

## Non-Goals

- Do not share authority results across different type scripts.
- Do not make `access-list` trust an `xudt-meta` authority decision.
- Do not change which authority is required for any state transition.
- Do not change authority ABI, metadata schema, or error codes.
- Do not add transaction-global witness state or cross-script coordination.

## Boundary Rule

Authority result reuse is only safe inside one script invocation.

CKB executes each script group independently. A result computed by
`xudt-meta` is not visible to `access-list`, and making one script rely on
another script's internal decision would blur ownership:

- metadata scripts own metadata governance;
- AccessList owns AccessList shard lifecycle;
- token scripts own token movement.

Therefore this design only deduplicates checks inside `sudt-meta` and
`xudt-meta`.

## Desired Behavior

The externally visible behavior must remain unchanged:

- `sudt-meta` supply or mint-authority changes still require
  `mint_authority`.
- `sudt-meta` metadata changes and no-op updates still require
  `metadata_authority`, with `mint_authority` fallback.
- `sudt-meta` destroy still requires `mint_authority`.
- `xudt-meta` access-state changes still require `access_authority`, with
  `mint_authority` fallback.
- `xudt-meta` extension-list changes still require `mint_authority`.
- `xudt-meta` metadata changes still require `metadata_authority`, with
  `mint_authority` fallback.
- `xudt-meta` supply or mint-authority changes still require
  `mint_authority`.
- `xudt-meta` no-op updates still require any of `metadata_authority`,
  `access_authority`, or `mint_authority`.
- `xudt-meta` destroy still requires `mint_authority`.

The internal behavior should change so that one script invocation executes
`check_authority` at most once for the same authority descriptor.

## Design

Add a small contract-local `AuthorityVerifier` helper to each metadata update
module, or to a shared script-utils module if both scripts can use it without
host-build side effects.

The helper stores results for authority descriptors already checked during the
current script invocation:

```rust
struct AuthorityVerifier {
    checked: Vec<(Authority, bool)>,
}
```

The helper exposes three operations:

- `require(authority)`: a specific authority must pass.
- `require_with_fallback(primary, fallback)`: primary may pass; if primary is
  missing or fails, fallback must pass.
- `require_any(authorities)`: at least one provided authority must pass.

Each operation preserves the current error behavior:

- missing required authority returns `AuthorityMissing`;
- provided but failing authority returns `AuthorityFailed`;
- invalid authority shape and syscall errors propagate as they do today.

The helper may cache only successful and false authority results. Errors that
represent invalid data or syscall failure should be returned immediately and do
not need caching.

## Implementation Notes

`Authority` implements `Clone` and `PartialEq`, so a small linear cache is
enough. Metadata has at most a few authority descriptors, making a hash map
unnecessary and unsuitable for no-std simplicity.

The update validators should create one verifier per invocation:

```rust
let mut verifier = AuthorityVerifier::new();
```

Then every authority branch should call the verifier instead of calling
`check_authority` directly.

## Testing Strategy

This is a behavior-preserving refactor, so integration tests remain the primary
regression guard:

- existing metadata authority tests must still pass;
- existing plugin authority tests must still pass;
- zero-supply destroy tests must still pass;
- full `MODE=debug cargo test -p tests -- --nocapture` must pass.

The duplicate-check reduction is mainly an internal cost optimization. If a
pure unit seam is introduced for the verifier, add unit coverage for:

- repeated `require` uses cached success;
- fallback does not re-check a previously successful fallback;
- `require_any` preserves missing-vs-failed behavior.

Do not add brittle cycle-count assertions to integration tests.

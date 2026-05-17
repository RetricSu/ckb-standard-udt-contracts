# Architecture

This document describes how the standard UDT contracts in this repository fit
together. It focuses on script boundaries, ownership of invariants, and the
transaction-level coordination between token cells, metadata cells, AccessList
cells, authorities, and extensions.

## Goals

The system is organized around five type scripts:

- `sudt`
- `sudt-meta`
- `xudt`
- `xudt-meta`
- `access-list`

The main architectural goals are:

- keep token movement checks inside token type scripts;
- keep metadata state checks inside metadata type scripts;
- keep AccessList shard invariants inside the AccessList type script;
- share parsing and low-level script utilities without sharing ownership of
  business rules;
- make mint, burn, user destruction, access control, and supply tracking
  explicit transaction-level protocols.

## Layering

```text
tests
  ckb-testtool fixtures, transaction builders, integration tests

contracts
  sudt        token movement for sUDT
  sudt-meta   metadata state for sUDT
  xudt        token movement, access checks, extensions for xUDT
  xudt-meta   metadata state, access-mode governance, extension config for xUDT
  access-list AccessList shard structure and lifecycle

lib/script-utils
  reusable no-std helpers for authorities, token deltas, cell scans, errors

lib/types
  Molecule schemas, generated types, strict parsed metadata structs
```

The lower libraries provide reusable mechanics. They do not decide which
contract owns a protocol invariant. Ownership stays in the contract type script.

## Cell Families

The system uses three important cell families.

### Token Cells

Token cells are typed by `sudt` or `xudt`.

Token type scripts own:

- input and output amount conservation;
- mint detection;
- protocol burn detection;
- user destruction detection;
- untracked-supply mint authority checks;
- tracked-supply metadata participation checks for mint and protocol burn;
- access checks for xUDT;
- extension execution for xUDT.

When token scripts or metadata scripts need to identify cells bound to a
metadata cell, they match the candidate type script by all binding fields:

- `hash_type = Data2`;
- type args equal to the metadata type hash;
- code hash equal to the expected token or AccessList script code hash.

### Metadata Cells

Metadata cells are typed by `sudt-meta` or `xudt-meta`.

Metadata type scripts own:

- metadata type-id creation;
- uniqueness inside the type script group;
- metadata decoding and size limits;
- output metadata lock restrictions;
- immutable supply-tracking mode;
- tracked supply consistency;
- metadata authority checks;
- mint authority changes;
- xUDT access-mode governance;
- xUDT pause-state governance;
- xUDT extension-list governance.

The metadata scripts do not validate token ownership or transfer semantics.

### AccessList Cells

AccessList cells are typed by `access-list`.

The AccessList type script owns:

- shard decoding;
- shard range validity;
- nibble alignment;
- sorted and unique entries;
- entry range containment;
- non-overlapping ordered shards;
- create, update, split, merge, and destroy lifecycle rules;
- output shard lock restrictions;
- full-domain requirements for global access-mode changes.

Consumers use AccessList facts and proofs. They do not revalidate the shard
lifecycle rules.

## State Cell Locks and Unlock Authority

Metadata cells and AccessList cells are state cells. Their locks are not the
authorization policy for state changes. Authorization is encoded in metadata
authority descriptors and enforced by the type scripts.

Consuming an existing metadata or AccessList cell requires authority even when
the output data is unchanged. A no-op state-cell update is still an unlock of
governed state. The scripts therefore do not rely on capacity preservation or
lock script ownership to protect the state cell.

To make that ownership boundary explicit, newly produced metadata and AccessList
state cells must use an always-success lock. The accepted production lock is
identified by:

- `hash_type = Data2`;
- `code_hash` equal to the whitelisted always-success code hash.

Lock args are not part of this restriction. Debug builds additionally whitelist
the ckb-testtool always-success code hash for integration tests; release builds
compile that allowance out.

## Metadata Types

The canonical parsed metadata structs live in `lib/types`.

`SudtMeta` contains:

- config flags;
- current supply;
- display metadata fields;
- mint authority;
- metadata authority.

`XudtMeta` extends this with:

- access authority;
- extension list.

The type layer validates shape-level constraints:

- allowed config bits;
- untracked supply must have zero `current_supply`;
- whitelist mode cannot be enabled without access mode;
- metadata byte field size limits;
- authority encoding shape;
- extension ordering and uniqueness;
- AccessList range and entry shape.

Contract scripts then validate transaction-level constraints using these parsed
types.

## Config State

The config flags are small, explicit state bits.

```text
CONFIG_SUPPLY_TRACKED    supply is tracked in metadata
CONFIG_ACCESS_ENABLED    xUDT AccessList checks are active
CONFIG_ACCESS_WHITELIST  active AccessList is interpreted as whitelist
CONFIG_PAUSED            xUDT transfer and mint flows are paused
```

sUDT accepts only supply tracking. xUDT accepts all four flags.

The access-state model is:

```text
disabled   access_enabled = false
blacklist  access_enabled = true, whitelist = false
whitelist  access_enabled = true, whitelist = true
```

Whitelist and blacklist use the same AccessList data structure. Only the
interpretation differs:

- whitelist requires membership proof for each checked holder lock;
- blacklist requires non-membership proof for each checked holder lock.

xUDT access control is holder-based. For token movement, xUDT checks the
relevant xUDT input and output holder locks: transfers and protocol burns check
both sides, mint checks outputs, and pure user destruction with no xUDT outputs
remains available without holder access.

## Script Responsibility Matrix

| Responsibility | Owner | Consumers |
| --- | --- | --- |
| Token amount conservation | `sudt`, `xudt` | metadata scripts observe deltas |
| Mint authority for untracked token creation | `sudt`, `xudt` | metadata scripts validate matching supply when tracked |
| Mint/protocol-burn authority for tracked supply changes | `sudt-meta`, `xudt-meta` | token scripts require metadata participation |
| User destruction classification | `sudt`, `xudt` | metadata scripts are not involved unless metadata is updated |
| Metadata creation type-id | `sudt-meta`, `xudt-meta` | token scripts may consume output metadata facts |
| Metadata uniqueness in group | `sudt-meta`, `xudt-meta` | token scripts look for current metadata context |
| Metadata field mutation | `sudt-meta`, `xudt-meta` | token scripts do not inspect display fields |
| Supply tracking mode immutability | `sudt-meta`, `xudt-meta` | token scripts use parsed mode |
| Tracked supply value | `sudt-meta`, `xudt-meta` | token scripts require matching metadata update for mint/burn |
| xUDT pause mode | `xudt-meta` configures, `xudt` enforces | extensions receive operation context |
| xUDT access mode | `xudt-meta` configures, `xudt` enforces | `access-list` validates shards |
| AccessList shard lifecycle | `access-list` | `xudt` and `xudt-meta` scan required shards |
| Extension list governance | `xudt-meta` | `xudt` executes configured extensions |
| Authority execution mechanics | `lib/script-utils` helper | contracts decide when authority is required |

## Supply Coordination

Supply coordination is deliberately split between token and metadata scripts.

The token type script sees token cells and classifies the operation:

```text
outputs > inputs  mint
outputs = inputs  transfer or metadata-only update
outputs < inputs  protocol burn or user destruction
```

Negative token deltas are classified by metadata participation. If the
transaction consumes the token's metadata input cell, the negative delta is a
protocol burn. If no metadata input cell is consumed, the negative delta is user
destruction.

The metadata type script sees metadata cells and validates `current_supply`.

For tracked supply updates, both sides must agree:

1. The token type script validates that the token delta is allowed.
2. For tracked mint or protocol burn, the token type script requires the
   metadata input context so the metadata script participates.
3. The metadata type script calculates the same transaction token delta.
4. The metadata output `current_supply` must equal input supply plus that delta,
   and `mint_authority` must authorize the supply change.

This prevents a metadata-only transaction from changing tracked supply without
matching token cell movement. It also prevents token mint or protocol burn from
claiming a supply change that the metadata cell does not record.

For untracked supply, metadata `current_supply` must remain zero.

User destruction is intentionally different from protocol burn. It allows a
holder to destroy their own token cells without mint authority. User destruction
does not reduce tracked `current_supply`; only protocol burn does.

Metadata cells themselves can be destroyed only in supply-tracked mode when
`current_supply == 0`, and only with `mint_authority`. This is a state-cell
cleanup path for reclaiming occupied CKB, not a token burn. For xUDT metadata,
if access mode is still enabled, the same transaction must consume full-domain
AccessList inputs and must leave no AccessList outputs bound to the destroyed
metadata cell.

## Mint Flow

Mint creates additional token amount.

For an existing token:

```text
transaction
  inputs:
    existing token cells, metadata input if tracked supply is updated
  outputs:
    increased token amount, metadata output if tracked supply is updated
```

Validation split:

- `sudt` or `xudt` detects the positive token delta;
- for untracked supply, the token script requires `mint_authority`;
- for tracked supply, the token script requires metadata input participation;
- if supply is tracked, the metadata output must increase by the same delta and
  the metadata script requires `mint_authority`;
- the metadata script independently verifies the delta and authority-sensitive
  metadata changes.

For initial create mint, the token script can use output metadata because no
input metadata exists yet.

For untracked supply, `current_supply` remains zero and mint does not need to
consume the metadata cell. The token script may load the current metadata from a
cell dep as read-only context, allowing independent mints without serializing on
the metadata cell.

## Protocol Burn Flow

Protocol burn reduces token amount and tracked supply. A negative token delta is
protocol burn only when the transaction consumes the token's metadata input
cell.

```text
transaction
  inputs:
    token cells, metadata input
  outputs:
    reduced token amount, metadata output with reduced current_supply
```

Validation split:

- the token script detects the negative token delta;
- the token script requires metadata input participation;
- the metadata script verifies `current_supply` decreases by the same amount
  and requires `mint_authority`.

If the negative token delta is user destruction rather than protocol burn, no
metadata input cell is consumed and metadata supply is not reduced.

## Transfer Flow

Transfer preserves token amount.

For sUDT:

- `sudt` checks amount conservation;
- no metadata is required for an ordinary transfer.

For xUDT:

- `xudt` checks amount conservation;
- `xudt` loads current metadata context;
- if paused, transfer is rejected;
- if access mode is enabled, `xudt` requires ordered CellDep AccessList proofs
  for input and output holder locks;
- configured extensions are executed for the transfer operation.

## Access Mode Transitions

`xudt-meta` owns access-mode state changes.

The important transition classes are:

```text
disabled -> blacklist
disabled -> whitelist
blacklist -> disabled
whitelist -> disabled
blacklist -> whitelist
whitelist -> blacklist
```

Rules:

- enabling access mode requires full-domain AccessList outputs;
- disabling active access mode requires full-domain AccessList inputs;
- switching between whitelist and blacklist requires full-domain inputs and
  outputs because the interpretation is inverted;
- active-mode transitions cannot be mixed with same-token xUDT cells;
- ordinary AccessList updates do not require full-domain consumption.
- destroying active xUDT metadata requires full-domain AccessList inputs and no
  bound AccessList outputs.

xUDT token movement uses AccessList proof shards from CellDeps only. Same-meta
AccessList inputs or outputs are not proof sources; they may appear in the same
transaction when their own AccessList state transition is valid. Matching
CellDep proof shards must be ordered by range and non-overlapping; xUDT builds
a lightweight `{start, end, dep_index}` index and loads full shard entries only
for shards covering checked holder locks.

The full-domain checks in `xudt-meta` confirm that the transaction presents a
global AccessList replacement or removal when the access-mode semantics change.

The detailed shard consistency rules still belong to `access-list`.

## AccessList Shard Lifecycle

AccessList shards divide the lock-hash domain into ranges. A shard stores sorted
entries inside its range.

The `access-list` script supports:

- create;
- insert/delete within a range;
- split or merge of the same covered range;
- simultaneous entry changes and split/merge inside the same covered range;
- destroy.

Local updates can consume only the affected shard nodes. This is enough because
the type script validates that input shards and output shards cover the same
continuous range, and that each side is ordered, non-overlapping, and locally
continuous. Entries may change freely inside that covered range. Per-shard entry
validity is guaranteed by the parsed `AccessListShard` type: entries must be
inside the shard range, sorted, and unique.

Full-domain consumption is reserved for global state changes:

- initial active AccessList creation;
- access mode replacement;
- active AccessList destruction.

## Authority Model

Metadata stores authority descriptors. Shared script utilities perform the
mechanical checks.

Authority kinds:

- `InputLock`: matching script hash appears as an input lock;
- `InputType`: matching script hash appears as an input type;
- `OutputType`: matching script hash appears as an output type;
- `DynamicLinking`: referenced executable is loaded and asked to authorize;
- `Spawn`: referenced executable is spawned and asked to authorize.

Contracts decide when an authority is required.

Authority checks are performed through the shared `AuthorityVerifier`. A
contract builds one verifier for a validation path and asks it for the
authorities required by the fields that changed. The verifier caches repeated
checks, so a fallback or multi-field update does not re-scan or re-execute the
same authority descriptor unnecessarily.

`mint_authority` is the strongest authority. It controls:

- mint;
- protocol burn;
- supply-changing metadata updates;
- mint authority changes;
- extension list changes.

It also acts as a fallback authority for metadata and access-state updates.
This keeps a token administrator from being locked out when narrower metadata or
access authorities are absent or fail to authorize.

Consuming an existing metadata or AccessList state cell requires an applicable
authority even if the transaction is a no-op update. For metadata updates this
base unlock check happens before field-specific checks; field-specific checks
then add stronger requirements such as mint authority for supply changes,
mint-authority changes, or extension-list changes.

## Extension Architecture

xUDT supports dynamic extensions configured in metadata.

`xudt-meta` owns the extension list:

- extension metadata must decode correctly;
- extensions must be sorted and unique;
- changing the extension list requires mint authority.

`xudt` owns extension execution:

- it loads the current metadata context;
- it runs configured extensions for the current operation;
- it passes operation context and mint-authority context to extension code;
- deny or failing extensions reject the transaction.

Extensions are therefore configured by metadata governance but enforced by the
token script during token operations.

## Visible Metadata

Token scripts use current metadata context to decide how ordinary token
operations should be validated.

Current metadata context may come from:

- metadata input cells;
- metadata cells referenced through cell deps.

The token scripts use this current metadata as read-only context. Metadata
output cells are read only in location-specific flows such as initial token
creation, where no metadata input exists yet. The metadata type scripts remain
responsible for validating actual metadata creation and updates.

This separation lets token scripts enforce transfer-time behavior without
duplicating metadata lifecycle rules.

## Debug-Only Test Allowances

Some output locks are accepted only in debug builds so integration tests can use
ckb-testtool always-success locks.

These allowances are guarded by:

```rust
#[cfg(debug_assertions)]
```

The top-level Makefile enables this cfg only for `MODE=debug` by passing:

```text
-C debug-assertions
```

Release builds do not include those branches.

## Error Boundaries

Each contract maps its own validation failures to contract-local error codes.

The codebase uses separate error categories for:

- syscall failures;
- argument and group-shape failures;
- duplicate metadata cells;
- metadata decoding failures;
- supply failures;
- authority failures;
- access-list failures;
- extension failures.

Shared libraries return structured helper errors where possible. Contracts map
those helper errors into the contract's public error surface.

## Design Principles

The architecture follows these rules:

- A type script validates the state it owns.
- A consumer validates only the proof or fact it needs from another type.
- Metadata scripts validate metadata state, not token ownership.
- Token scripts validate token movement, not AccessList shard lifecycle.
- AccessList scripts validate shard lifecycle, not token transfer policy.
- Shared libraries provide parsing and mechanics, not business-rule ownership.
- Debug-only behavior must be compiled out of release binaries.

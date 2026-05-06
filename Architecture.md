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
- authority requirements for mint and protocol burn;
- access checks for xUDT;
- extension execution for xUDT.

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
- create, update, split, merge, and destroy rules;
- full-domain requirements for global access-mode changes.

Consumers use AccessList facts and proofs. They do not revalidate the shard
lifecycle rules.

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

- whitelist requires membership proof;
- blacklist requires non-membership proof.

## Script Responsibility Matrix

| Responsibility | Owner | Consumers |
| --- | --- | --- |
| Token amount conservation | `sudt`, `xudt` | metadata scripts observe deltas |
| Mint authority for token creation | `sudt`, `xudt` | metadata scripts validate matching supply |
| Protocol burn authority | `sudt`, `xudt` | metadata scripts validate matching supply |
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
2. The metadata type script calculates the same transaction token delta.
3. The metadata output `current_supply` must equal input supply plus that delta.

This prevents a metadata-only transaction from changing tracked supply without
matching token cell movement. It also prevents token mint or protocol burn from
claiming a supply change that the metadata cell does not record.

For untracked supply, metadata `current_supply` must remain zero.

User destruction is intentionally different from protocol burn. It allows a
holder to destroy their own token cells without mint authority. User destruction
does not reduce tracked `current_supply`; only protocol burn does.

## Mint Flow

Mint creates additional token amount.

For an existing token:

```text
transaction
  inputs:
    existing token cells, optional metadata input
  outputs:
    increased token amount, metadata output if supply is tracked
```

Validation split:

- `sudt` or `xudt` detects the positive token delta;
- the token script requires `mint_authority`;
- if supply is tracked, the metadata output must increase by the same delta;
- the metadata script independently verifies the delta and authority-sensitive
  metadata changes.

For initial create mint, the token script can use output metadata because no
input metadata exists yet.

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
- the token script requires `mint_authority`;
- the metadata script verifies `current_supply` decreases by the same amount.

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
- if access mode is enabled, `xudt` requires the appropriate AccessList proof;
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

The full-domain checks in `xudt-meta` confirm that the transaction presents a
global AccessList replacement or removal when the access-mode semantics change.

The detailed shard consistency rules still belong to `access-list`.

## AccessList Shard Lifecycle

AccessList shards divide the lock-hash domain into ranges. A shard stores sorted
entries inside its range.

The `access-list` script supports:

- create;
- insert/delete within a range;
- split;
- merge;
- destroy.

Local updates can consume only the affected shard nodes. This is enough because
the type script validates local continuity, range boundaries, sorted entries,
and non-overlap for the consumed and produced group cells.

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

`mint_authority` is the strongest authority. It controls:

- mint;
- protocol burn;
- supply-changing metadata updates;
- mint authority changes;
- extension list changes.

It also acts as a fallback authority for metadata and access-state updates.
This keeps a token administrator from being locked out when narrower metadata or
access authorities are absent or fail to authorize.

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

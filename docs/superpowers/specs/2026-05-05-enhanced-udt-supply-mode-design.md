# Enhanced UDT Supply Mode Design

Date: 2026-05-05

## Goal

Rewrite the Enhanced UDT V1 design around two explicit supply modes:

- `Untracked`: keep the current lightweight model. Meta does not track supply.
- `Tracked`: Meta stores a protocol-level supply counter updated by authorized mint and protocol burn operations.

This design also fixes the script binding direction:

- `Meta.type.args = type_id`
- `UDT.type.args = meta_type_hash`
- `AccessList.type.args = meta_type_hash`
- `MetaType` hardcodes the corresponding `UDT_CODE_HASH_V1`
- `UDTType` does not hardcode `META_CODE_HASH_V1`; it locates Meta by `type_hash == self.args`

## Non-Goals

- No capped supply mode. Fixed supply is achieved by minting the desired amount and irreversibly removing `mint_authority`.
- No attempt to track user destruction. If a holder destroys their own UDT cells to reclaim CKB without consuming Meta, the protocol-level supply counter does not change.
- No legacy compatibility branch. This is the new V1 mainline design.

## Binding Model

The token identity anchor is `meta_type_hash`.

```text
Meta.type = Script {
    code_hash: META_CODE_HASH_V1,
    hash_type: Data2,
    args: type_id,
}

meta_type_hash = type_hash(Meta.type)

UDT.type = Script {
    code_hash: UDT_CODE_HASH_V1,
    hash_type: Data2,
    args: meta_type_hash,
}

AccessList.type = Script {
    code_hash: ACCESSLIST_CODE_HASH_V1,
    hash_type: Data2,
    args: meta_type_hash,
}
```

`UDTType` locates Meta by scanning transaction-visible cells for exactly one cell whose type hash equals `self.args`. It must still validate the Meta lock whitelist and strict Meta data decoding. It must not depend on the Meta type code hash.

`MetaType` hardcodes the matching UDT type code hash. It uses that constant to validate initial supply on creation and supply deltas on tracked mint/protocol burn updates.

## Meta Schema

Both sUDT and xUDT use a unified `config_flags` byte and `current_supply` counter.

```molecule
array Uint128 [byte; 16];

table SudtMetaV1 {
    config_flags: byte,
    current_supply: Uint128,
    decimals: byte,
    name: Bytes,
    symbol: Bytes,
    uri: Bytes,
    extra_data: Bytes,
    mint_authority: ScriptAttrOpt,
    metadata_authority: ScriptAttrOpt,
}

table XudtMetaV1 {
    config_flags: byte,
    current_supply: Uint128,
    decimals: byte,
    name: Bytes,
    symbol: Bytes,
    uri: Bytes,
    extra_data: Bytes,
    mint_authority: ScriptAttrOpt,
    metadata_authority: ScriptAttrOpt,
    access_authority: ScriptAttrOpt,
    extensions: ScriptAttrVec,
}
```

`config_flags` layout:

```text
bit0: supply tracking
  0 = Untracked
  1 = Tracked

bit1: xUDT access control enabled
  0 = disabled
  1 = enabled

bit2: xUDT access mode
  0 = Blacklist
  1 = Whitelist
  Valid only when bit1 = 1.

bit3: xUDT paused
  0 = active
  1 = paused

bit4..7: reserved, MUST be 0
```

sUDT only permits bit0. For sUDT, bits 1 through 7 must be zero.

xUDT permits bits 0 through 3. If bit1 is zero, bit2 must also be zero.

`config_flags.bit0` is fixed at deployment and must never change. When bit0 is zero, `current_supply` must be zero. When bit0 is one, `current_supply` is the protocol-level supply counter.

## Supply Semantics

Tracked supply is not the live total of all spendable UDT cells on chain. It tracks only protocol-recognized issuance and protocol burn:

- Mint increases `current_supply`.
- Protocol burn decreases `current_supply`.
- User destruction does not change `current_supply`.

User destruction means `sum_in > sum_out` without consuming the Meta Cell. This lets a holder destroy UDT cells to reclaim CKB without issuer permission. It is intentionally outside the supply counter.

Protocol burn means `sum_in > sum_out` while consuming and recreating the Meta Cell. It must pass `mint_authority`. In `Tracked` mode it decreases `current_supply`.

## Create Token

Meta creation validates:

- type-id rules for `Meta.type.args`
- Meta lock is in the always-success whitelist
- Meta data decodes strictly
- `config_flags` is legal for the token kind
- `current_supply` is zero when `Untracked`

For `Tracked`, `MetaType` scans transaction outputs for UDT cells with:

```text
type.code_hash == UDT_CODE_HASH_V1
type.hash_type == Data2
type.args == meta_type_hash
```

It sums those output amounts and requires:

```text
current_supply == sum(initial UDT outputs)
```

If no initial UDT cells are created, tracked initial supply must be zero.

## sUDT Type Rules

The sUDT script group is all input and output cells with the same type code hash and `type.args == self.args`.

All UDT cell data must be exactly 16 bytes of little-endian `u128`. Input and output sums must reject overflow.

Operation classification:

```text
sum_in == sum_out => transfer
sum_in <  sum_out => mint
sum_in >  sum_out => burn or user destruction
```

Rules:

- Transfer passes without requiring Meta.
- Mint must locate unique Meta, validate it, and pass `mint_authority`.
- In `Tracked` mint, the transaction must consume old Meta and create new Meta. The required delta is `new_supply = old_supply + (sum_out - sum_in)`.
- If `sum_in > sum_out` and no Meta is consumed, the operation is user destruction. It passes without `mint_authority`; tracked supply is unchanged.
- If `sum_in > sum_out` and Meta is consumed, the operation is protocol burn. It must pass `mint_authority`.
- In `Tracked` protocol burn, the transaction must create new Meta and satisfy `new_supply = old_supply - (sum_in - sum_out)`.
- Supply addition and subtraction must reject overflow and underflow.

## xUDT Type Rules

xUDT always locates and validates unique Meta because paused, access control, and extensions depend on it.

Operation classification is the same as sUDT. Mint and protocol burn use `mint_authority`; user destruction does not.

Paused behavior:

- When `config_flags.bit3 == 1`, transfer and mint are rejected.
- Protocol burn remains allowed, but still requires `mint_authority`.
- User destruction remains allowed and does not execute access checks or extensions.

Access control:

- If `config_flags.bit1 == 0`, no AccessList check is performed.
- If `config_flags.bit1 == 1`, transfer and protocol burn run AccessList checks.
- `bit2 == 0` means Blacklist: any checked lock hash present in entries rejects.
- `bit2 == 1` means Whitelist: any checked lock hash missing from entries rejects.
- Shards are accepted only when `type.code_hash == ACCESSLIST_CODE_HASH_V1 && type.args == self.args`.

Extensions:

- transfer, mint, and protocol burn execute extensions in index order.
- Any extension failure rejects the transaction.
- mint executes extensions only after `mint_authority` succeeds and passes `mint_authority_checked=1`.
- user destruction does not execute extensions.

Tracked supply deltas match sUDT.

## Meta Update Rules

Meta updates may combine multiple changes, but every changed field must satisfy its own old-authority check. Implementations should encourage separating governance updates from supply updates, but the standard permits combined transactions if all constraints pass.

Common rules:

- `config_flags.bit0` is immutable.
- `current_supply` is zero for `Untracked`.
- In `Tracked`, `current_supply` may change only as the validated result of mint or protocol burn.
- `mint_authority` changes require old `mint_authority`.
- `metadata_authority` and metadata field changes require old `metadata_authority`.
- Any authority set to `None` is irreversible.
- Meta lock must remain always-success whitelisted.

xUDT-specific rules:

- `access_authority` changes require old `access_authority`.
- Changes to access-enabled, access-mode, or paused bits require old `access_authority`.
- Changes to `extensions` require old `mint_authority`.
- `extensions` must remain unique, sorted, and under the maximum count.

Both `UDTType` and `MetaType` must validate tracked supply deltas consistently. This prevents a transaction from satisfying only one side of the state transition.

## AccessList State Machine

xUDT has three access states:

```text
Disabled:  bit1=0, bit2=0
Blacklist: bit1=1, bit2=0
Whitelist: bit1=1, bit2=1
```

`bit1=0, bit2=1` is invalid.

`access_authority` controls:

- enabling or disabling access control
- switching Blacklist and Whitelist modes
- toggling paused
- changing `access_authority`
- updating AccessList shards

If `access_authority=None`, these operations are no longer possible.

Access mode switch transactions must not include xUDT input or output cells for the same `meta_type_hash`. In this rule, an xUDT cell means a cell whose type script is `Script{UDT_CODE_HASH_V1, Data2, meta_type_hash}`. This makes mode changes pure governance operations and avoids ambiguity about whether the old or new mode governs asset flow.

State transition constraints:

- `Disabled -> Blacklist`: output shards must form a valid full-domain Blacklist set.
- `Disabled -> Whitelist`: output shards must include at least one valid Whitelist shard.
- `Blacklist -> Whitelist`: output shards must form a valid Whitelist shard set; old Blacklist shards may be reclaimed.
- `Whitelist -> Blacklist`: output shards must form a valid full-domain Blacklist set.
- `Blacklist -> Disabled` and `Whitelist -> Disabled`: allowed; shards may be reclaimed.

Blacklist shard invariants:

- cover the full domain `[00..00, FF..FF]`
- ordered and non-overlapping
- nibble aligned
- under capacity limits
- updated only through allowed Insert/Delete/Split/Merge diffs

Whitelist shard invariants:

- ordered and non-overlapping
- nibble aligned
- at least one shard when enabling Whitelist
- missing coverage fails closed

AccessList shard updates require `access_authority`. If a transaction also updates Meta, the input Meta's old `access_authority` is authoritative for the transaction.

## Required Security Checks

- Meta uniqueness: on any path that requires Meta, there must be exactly one visible Meta with `type_hash == meta_type_hash`. xUDT always requires Meta; sUDT transfer and user destruction do not.
- Meta lock whitelist check.
- Strict Meta data decoding and `config_flags` legality.
- UDT amount length exactly 16 bytes.
- `current_supply` length exactly 16 bytes.
- All u128 sums and deltas reject overflow and underflow.
- `UDTType` must not depend on `META_CODE_HASH_V1`.
- `MetaType` must hardcode the matching `UDT_CODE_HASH_V1`.
- `Tracked` supply tracks only mint and protocol burn, not user destruction.
- AccessList shards must match `ACCESSLIST_CODE_HASH_V1` and `meta_type_hash`.
- Access mode switching transactions must not include xUDT cells for the same token.
- Extensions fail closed.
- `ScriptAttr` location and script-shape rules remain unchanged.

## Test Vectors

Binding and creation:

- UDT args must be `meta_type_hash`.
- wrong UDT args rejects.
- tracked initial supply equals initial UDT outputs.
- tracked initial supply mismatch rejects.
- untracked nonzero `current_supply` rejects.
- sUDT rejects xUDT-only flag bits.
- xUDT rejects reserved bits and `bit1=0, bit2=1`.

Supply:

- sUDT tracked mint updates supply with authorization.
- sUDT mint without authorization rejects.
- sUDT user destruction passes without Meta and leaves supply unchanged.
- sUDT protocol burn requires authorization and reduces supply.
- protocol burn underflow rejects.
- supply overflow rejects.
- xUDT covers the same mint, protocol burn, and user destruction cases.

Meta updates:

- changing `config_flags.bit0` rejects.
- untracked supply change rejects.
- combined updates require all old authorities.
- setting an authority to `None` is irreversible.

xUDT policy:

- paused rejects transfer and mint.
- paused allows user destruction.
- access-enabled transfer checks Blacklist and Whitelist.
- user destruction skips AccessList and extensions.
- access mode switch with xUDT cells rejects.
- Disabled to Blacklist requires full-domain shards.
- Disabled to Whitelist requires at least one valid shard.
- Whitelist to Blacklist requires full-domain Blacklist shards.
- AccessList updates require `access_authority`.

Existing V1 coverage remains required:

- multiple Meta injection rejects.
- non-whitelisted Meta lock rejects.
- amount decode and sum overflow reject.
- Blacklist coverage holes reject.
- Whitelist missing shards fail closed.
- shard overlap, ordering, nibble alignment, capacity, split, and merge cases.
- extensions success, rejection, ordering, duplicate, over-limit, and location 3/4 runtime paths.

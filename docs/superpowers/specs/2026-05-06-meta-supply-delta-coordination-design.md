# Meta Supply Delta Coordination Design

## Goal

Make tracked `current_supply` updates provably correspond to same-transaction UDT amount deltas, even when a transaction consumes only the meta cell and no UDT type script would otherwise execute.

This closes the gap where a holder of `mint_authority` can update `current_supply` directly through the meta type script without minting or burning any matching UDT cells.

## Problem

The current split is asymmetric:

- `sudt` and `xudt` type scripts compute their own group input/output amount delta.
- On mint or protocol burn, the UDT type script checks authority and checks tracked meta supply is updated by the same delta.
- `sudt-meta` and `xudt-meta` allow `current_supply` changes when `mint_authority` is satisfied.

That means a transaction can consume a tracked meta cell, satisfy `mint_authority`, change `current_supply`, and include no matching UDT cells. In that case the UDT type script for the token never runs, so no script checks that the supply field corresponds to actual token movement.

A weaker fix of requiring "some UDT cell" is not enough. A transaction could include a same-token UDT input/output pair with equal amounts, causing the UDT type script to take the transfer path while the meta supply changes independently.

## Ownership Rules

### UDT Type Owns Amount Authorization

`sudt` and `xudt` continue to own token amount non-conservation checks for their own type groups:

- `output_amount > input_amount` is mint.
- `input_amount > output_amount` is burn or user destruction, depending on each contract's existing rules.
- Protocol mint/burn requires `mint_authority`.
- When supply tracking is enabled and protocol mint/burn is selected, the UDT type script requires input/output meta cells and checks `current_supply` changes by the group amount delta.

These checks prevent minting or protocol-burning tokens without updating tracked meta supply.

### Meta Type Owns Supply Field Consistency

`sudt-meta` and `xudt-meta` own the meta cell field invariant:

- On create, tracked `current_supply` must equal the sum of same-token UDT outputs. This behavior already exists.
- On update, tracked `output.current_supply` must equal `input.current_supply + output_udt_sum - input_udt_sum`, where sums scan all transaction inputs and outputs for same-token UDT cells.
- If the transaction has no same-token UDT amount delta, tracked `current_supply` must not change.
- If UDT sums overflow or a same-token UDT cell has less than 16 bytes of data, reject with the meta contract's existing supply error.
- The supply tracking mode bit remains immutable.
- In untracked mode, `current_supply` remains required to be zero.
- Changing `current_supply` or `mint_authority` still requires `mint_authority`.

These checks prevent changing tracked meta supply without corresponding token movement.

## Same-Token UDT Cell Definition

For both meta contracts, a same-token UDT cell is any input or output cell whose type script satisfies:

- `hash_type == Data2`
- `args == meta_type_hash`
- `code_hash == SUDT_CODE_HASH` for `sudt-meta`, or `code_hash == XUDT_CODE_HASH` for `xudt-meta`

Amount decoding uses the first 16 bytes as little-endian `u128` and rejects data shorter than 16 bytes.

The scan is over `Source::Input` and `Source::Output`, not group sources, because the meta type script group contains meta cells, not UDT cells.

## Delta Formula

For tracked meta updates:

```text
input_udt_sum = sum same-token UDT amounts in Source::Input
output_udt_sum = sum same-token UDT amounts in Source::Output

if output_udt_sum >= input_udt_sum:
    expected_supply = input.current_supply + (output_udt_sum - input_udt_sum)
else:
    expected_supply = input.current_supply - (input_udt_sum - output_udt_sum)

require output.current_supply == expected_supply
```

Overflow and underflow reject.

## xUDT User Destruction Interaction

Current xUDT semantics are preserved:

- If `output_amount < input_amount` and a visible meta appears in `CellDep`, xUDT treats the amount decrease as user destruction and returns before protocol burn checks.
- If no `CellDep` meta is present and an input meta is consumed, xUDT treats the decrease as protocol burn and runs `mint_authority`, access, and extension checks.

With the new meta update invariant, a transaction that also consumes the meta cell and decreases `current_supply` must still prove that the same transaction contains a matching UDT amount decrease and must satisfy the meta update's `mint_authority` requirement.

This does not force all user destruction to update supply. A pure user destruction transaction that does not consume the meta cell keeps the existing behavior: the meta type script does not run and tracked supply is unchanged.

## Why Both Sides Check

The checks are intentionally redundant across script boundaries:

- UDT type checks protect token conservation when token cells are being changed.
- Meta type checks protect the meta supply field when the meta cell is being changed.

Either side alone is incomplete:

- Only UDT type checks miss meta-only supply updates.
- Only meta type checks miss token mint/burn transactions that omit a tracked meta update.

## Out of Scope

- No Molecule schema changes.
- No authority ABI changes.
- No change to untracked supply semantics.
- No change to xUDT access-list state machine rules.
- No change to xUDT extension operation dispatch except preserving the existing user-destruction/protocol-burn split.
- No requirement that user destruction updates tracked supply.

## Required Tests

New tests must prove:

- `sudt-meta` rejects tracked `current_supply` increase with no same-token UDT delta, even when `mint_authority` is satisfied.
- `sudt-meta` rejects tracked `current_supply` decrease with no same-token UDT delta, even when `mint_authority` is satisfied.
- `sudt-meta` accepts a tracked supply increase when same-token UDT outputs increase by exactly the same amount.
- `sudt-meta` accepts a tracked supply decrease when same-token UDT inputs decrease by exactly the same amount.
- `sudt-meta` rejects mismatched tracked supply delta and same-token UDT delta.
- `xudt-meta` has the same reject/accept coverage for tracked supply increases and decreases.
- Fake Data2 UDT cells with the wrong code hash remain ignored.
- Existing UDT type mint/burn tests still pass, proving the old forward checks remain intact.

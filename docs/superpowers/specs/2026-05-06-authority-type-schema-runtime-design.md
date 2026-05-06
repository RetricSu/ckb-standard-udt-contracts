# Authority Type Schema and Runtime Design

## Goal

Replace `ScriptAttr` / `ScriptLocation` with clearer `Authority` / `AuthorityType` naming across Molecule schema, host types, contracts, tests, and specification text. All authority fields must support the same five verification modes: input lock, input type, output type, dynamic linking, and spawn. Split xUDT extensions into dedicated `Extension` / `ExtensionType` schema types because extensions are executable validation plugins, not authorization rules.

This is a breaking cleanup. The repository does not need to preserve legacy `ScriptAttr` compatibility.

## Terminology

`Authority` is a rule that proves a privileged operation is authorized.

`AuthorityType` is the verification method:

| Value | Name | Meaning |
| --- | --- | --- |
| `0` | `InputLock` | At least one input lock hash equals `script_hash`. |
| `1` | `InputType` | At least one input type hash equals `script_hash`. |
| `2` | `OutputType` | At least one output type hash equals `script_hash`. |
| `3` | `DynamicLinking` | Load and call an authority plugin; return `0` means pass. |
| `4` | `Spawn` | Spawn an authority plugin; exit code `0` means pass. |

## Molecule Schema

Rename the schema types and fields:

```mol
table Authority {
    authority_type: byte,
    script_hash: Byte32,
    script: ScriptOpt,
}

option AuthorityOpt (Authority);
vector AuthorityVec <Authority>;

table Extension {
    extension_type: byte,
    script: Script,
}

vector ExtensionVec <Extension>;
```

Replace all metadata fields:

```mol
mint_authority: AuthorityOpt,
metadata_authority: AuthorityOpt,
access_authority: AuthorityOpt,
extensions: ExtensionVec,
```

`extensions` is a list of executable extension plugins. It does not reuse `Authority` because `InputLock`, `InputType`, and `OutputType` are not executable extension modes.

## Encoding Rules

For `authority_type in {0,1,2}`:

- `script` MUST be `None`.
- `script_hash` is the target lock/type hash.

For `authority_type in {3,4}`:

- `script` MUST be `Some(Script)`.
- `script.calc_script_hash()` MUST equal `script_hash`.
- The script must be available through transaction `cell_deps` using the normal CKB loading rules.

Any other `authority_type` value is invalid metadata.

## Host Types

`lib/types` should expose:

```rust
pub enum AuthorityType {
    InputLock,
    InputType,
    OutputType,
    DynamicLinking,
    Spawn,
}

pub struct Authority {
    pub authority_type: AuthorityType,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}
```

The public import path remains `standard_udt_types::metadata::{Authority, AuthorityType, ...}`.

`ScriptAttr` and `ScriptLocation` should be removed, not aliased.

`lib/types` should also expose:

```rust
pub enum ExtensionType {
    DynamicLinking,
    Spawn,
}

pub struct Extension {
    pub extension_type: ExtensionType,
    pub script: Script,
}
```

## Contract Parsed Types

Every contract-local parsed authority must retain all schema fields:

```rust
pub struct ParsedAuthority {
    pub authority_type: u8,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}
```

The exact local name can be `Authority` if unambiguous inside the contract, but it must not drop `script`.

This applies to:

- `sudt` mint authority
- `sudt-meta` mint and metadata authorities
- `xudt` mint authority
- `xudt-meta` mint, metadata, and access authorities
- `access-list` access authority

## Authority Runtime

All authority checks use one shared runtime behavior:

```rust
pub fn require_authority(authority: Option<&Authority>) -> Result<(), Error>
pub fn check_authority(authority: &Authority) -> Result<bool, Error>
```

Mode behavior:

- `InputLock`: scan input lock hashes.
- `InputType`: scan input type hashes.
- `OutputType`: scan output type hashes.
- `DynamicLinking`: call authority plugin.
- `Spawn`: spawn authority plugin.

`None` means the operation is permanently disabled and must return `AuthorityMissing`.

Invalid authority encoding returns `InvalidMetaData`. Plugin execution failures or nonzero plugin results return `AuthorityFailed` for authority fields.

## Authority Plugin ABI

Authority plugins should use a minimal ABI independent from xUDT extension operation context.

Dynamic linking:

```c
int eudt_authorize(const unsigned char *script_hash,
                   const unsigned char *args,
                   unsigned long args_len);
```

Rules:

- `script_hash` points to the 32-byte authority script hash.
- `args` and `args_len` are `authority.script.args`.
- Return `0` to authorize; any other return value denies.

Spawn:

```text
argv[0] = hex(authority.script_hash)
argv[1] = hex(authority.script.args)
```

Rules:

- Exit `0` to authorize.
- Any nonzero exit denies.

This ABI is intentionally separate from xUDT extension ABI. Extensions still use `eudt_validate(...)` and extension operation context.

## xUDT Extensions

`extensions` should be renamed from `ScriptAttrVec` to `ExtensionVec` and use a dedicated `Extension` table:

- `extension_type = 0` means dynamic linking.
- `extension_type = 1` means spawn.
- There is no `script_hash` field; the host computes `script.calc_script_hash()` when it needs the extension identity.
- Sorting and duplicate checks use `(extension_type, script.calc_script_hash())`.
- `InputLock`, `InputType`, and `OutputType` are not valid extension concepts.
- Extension ABI remains:

```c
int eudt_validate(const unsigned char *script_hash,
                  unsigned char op_type,
                  unsigned char ext_index,
                  const unsigned char *ext_data_ptr,
                  unsigned long ext_data_len,
                  unsigned char mint_authority_checked);
```

## Tests

Tests must prove:

- Schema-generated names are `Authority`, `AuthorityOpt`, `AuthorityVec`.
- Schema-generated names are `Extension` and `ExtensionVec`.
- Host metadata builders use `Authority`, `AuthorityType`, `Extension`, and `ExtensionType`.
- `authority_type` 0/1/2 still authorize through input/output scanning.
- `authority_type` 3 dynamic linking works for:
  - sUDT mint authority
  - sUDT metadata authority
  - xUDT mint authority
  - xUDT metadata/access authority
  - AccessList access authority
- `authority_type` 4 spawn works for the same authority domains.
- Missing plugin script, hash mismatch, invalid authority shape, plugin deny, and plugin execution error fail closed.
- xUDT extensions still pass existing dynamic linking/spawn tests after the `Extension` split.

## Out of Scope

- No backward compatibility with `ScriptAttr`.
- No new authority types beyond values `0..=4`.
- No changes to UDT/meta binding, supply tracking, access list range semantics, or amount decoding.

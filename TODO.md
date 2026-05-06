# TODO

## Status

- [x] Generate script crates from `ckb-script-templates`.
- [x] Implement shared metadata schemas and Rust bindings.
- [x] Implement shared script utilities.
- [x] Implement sUDT Meta.
- [x] Implement sUDT.
- [x] Implement xUDT Meta.
- [x] Implement AccessList.
- [x] Implement xUDT.
- [x] Implement dynamic-linking and spawn extension tests.
- [x] Rename ScriptAttr/ScriptLocation to Authority/AuthorityType and split xUDT extensions into Extension/ExtensionType.
- [x] Unify authority runtime so input_lock/input_type/output_type/dynamic_linking/spawn work across sUDT, xUDT, Meta, and AccessList authority fields.
- [x] Add dynamic-linking and spawn authority plugin coverage for mint, metadata, access-mode, and access-list update paths.
- [x] Publish project docs and final standard draft.
- [x] Run final debug and release verification.

## Open Deployment Constants

- `SUDT_CODE_HASH`
- `XUDT_CODE_HASH`
- `ACCESS_LIST_CODE_HASH`
- `ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST`

These values are deployment-specific. The root `Makefile` injects contract code hashes during local builds where dependencies are build-order dependent.

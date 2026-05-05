# TODO

## Status

- [x] Generate script crates from `ckb-script-templates`.
- [x] Implement shared metadata schemas and Rust bindings.
- [x] Implement shared script utilities.
- [x] Implement enhanced sUDT Meta.
- [x] Implement enhanced sUDT.
- [x] Implement enhanced xUDT Meta.
- [x] Implement AccessList.
- [x] Implement enhanced xUDT.
- [x] Implement dynamic-linking and spawn extension tests.
- [x] Publish project docs and final standard draft.
- [x] Run final debug and release verification.

## Open Deployment Constants

- `ENHANCED_SUDT_CODE_HASH`
- `ENHANCED_XUDT_CODE_HASH`
- `ACCESS_LIST_CODE_HASH`
- `ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST`

These values are deployment-specific. The root `Makefile` injects contract code hashes during local builds where dependencies are build-order dependent.
